use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub type ExitCallback = Arc<dyn Fn(String) + Send + Sync>;

pub struct PtySession {
    pub id: String,
    pub pid: u32,
    pub profile: String,
    pub window_id: String,
    pub title: Arc<Mutex<String>>,
    pub badge: Arc<Mutex<Option<String>>>,
    pub color: Arc<Mutex<Option<String>>>,
    #[allow(dead_code)]
    pub is_dead: Arc<Mutex<bool>>,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
    pub ring_buffer: Arc<crate::pty::ring_buffer::RingBuffer>,
    pub tx: broadcast::Sender<Vec<u8>>,
    _master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    // Wrapped in Option so the waiter thread can take() it (releasing the lock)
    // before calling the blocking .wait(). This prevents close() from deadlocking
    // when it tries to kill() through the same mutex.
    _child: Arc<Mutex<Option<Box<dyn Child + Send>>>>,
}

impl PtySession {
    pub fn resize(&self, rows: u16, cols: u16) {
        if rows > 0 && cols > 0 {
            let _ = self._master.lock().unwrap().resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn get_output_history(&self) -> Vec<u8> {
        self.output_buffer.lock().unwrap().clone()
    }

    fn kill_by_pid(&self) {
        #[cfg(windows)]
        {
            // Safe: we're only using the PID to open a handle and terminate.
            // PROCESS_TERMINATE = 0x0001
            extern "system" {
                fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
                fn TerminateProcess(handle: *mut std::ffi::c_void, exit_code: u32) -> i32;
                fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
            }
            unsafe {
                let handle = OpenProcess(0x0001, 0, self.pid);
                if !handle.is_null() {
                    TerminateProcess(handle, 1);
                    CloseHandle(handle);
                }
            }
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(self.pid as libc::pid_t, libc::SIGKILL);
        }
    }
}

unsafe impl Sync for PtySession {}

#[derive(Clone, Default)]
pub struct PtyManager {
    sessions: Arc<Mutex<HashMap<String, Arc<PtySession>>>>,
    on_exit: Arc<Mutex<Option<ExitCallback>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            on_exit: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_exit_callback<F>(&self, cb: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        *self.on_exit.lock().unwrap() = Some(Arc::new(cb));
    }

    pub fn spawn(
        &self,
        id: String,
        profile: String,
        window_id: String,
    ) -> Result<Arc<PtySession>, String> {
        self.spawn_with_cwd(id, profile, window_id, None)
    }

    pub fn spawn_with_cwd(
        &self,
        id: String,
        profile: String,
        window_id: String,
        cwd: Option<&str>,
    ) -> Result<Arc<PtySession>, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 30,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut cmd = match profile.to_lowercase().as_str() {
            "cmd" => CommandBuilder::new("cmd.exe"),
            "wsl" => CommandBuilder::new("wsl.exe"),
            "git-bash" | "bash" => {
                let mut c = CommandBuilder::new("C:\\Program Files\\Git\\bin\\bash.exe");
                c.arg("--login");
                c
            }
            _ => CommandBuilder::new("powershell.exe"),
        };

        if let Some(dir) = cwd {
            if !dir.trim().is_empty() {
                cmd.cwd(dir);
            }
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        let pid = child.process_id().unwrap_or(0);
        let writer = Arc::new(Mutex::new(
            pair.master.take_writer().map_err(|e| e.to_string())?,
        ));
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

        let (tx, _rx) = broadcast::channel::<Vec<u8>>(256);
        let tx_clone = tx.clone();
        let output_buffer = Arc::new(Mutex::new(Vec::<u8>::with_capacity(65536)));
        let output_buffer_clone = output_buffer.clone();
        let ring_buffer = Arc::new(crate::pty::RingBuffer::new());
        let ring_buffer_clone = ring_buffer.clone();

        let is_dead = Arc::new(Mutex::new(false));
        let is_dead_clone = is_dead.clone();
        let self_clone = self.clone();
        let id_clone = id.clone();

        // Wrap in Option: waiter takes ownership via .take(), releasing the lock before
        // calling the blocking .wait(). This lets close() acquire the lock independently.
        let child_arc: Arc<Mutex<Option<Box<dyn Child + Send>>>> =
            Arc::new(Mutex::new(Some(child)));
        let child_clone = child_arc.clone();

        let output_buffer_waiter = output_buffer.clone();
        let ring_buffer_waiter = ring_buffer.clone();
        let tx_waiter = tx.clone();

        // Reader thread for stdout/stderr streaming
        std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = &buf[..n];
                        ring_buffer_clone.append(chunk);
                        {
                            let mut guard = output_buffer_clone.lock().unwrap();
                            guard.extend_from_slice(chunk);
                            if guard.len() > 65536 {
                                let overflow = guard.len() - 65536;
                                guard.drain(..overflow);
                            }
                        }
                        let _ = tx_clone.send(chunk.to_vec());
                    }
                    Err(_) => break,
                }
            }
        });

        // Waiter thread: takes the child OUT of the Option (releasing the mutex immediately)
        // then waits. This is the key fix — the mutex is not held during .wait(), so
        // close() can always acquire it to kill the process without deadlocking.
        std::thread::spawn(move || {
            let child_opt = child_clone.lock().unwrap().take();
            let exit_code = child_opt
                .map(|mut c| c.wait().ok().map(|s| s.exit_code()).unwrap_or(0))
                .unwrap_or(0);

            if exit_code == 0 {
                let cb_opt = self_clone.on_exit.lock().unwrap().clone();
                if let Some(cb) = cb_opt {
                    cb(id_clone);
                } else {
                    self_clone.close(&id_clone);
                }
            } else {
                *is_dead_clone.lock().unwrap() = true;
                let msg = format!(
                    "\r\n\x1b[33m[Process exited with code {}. Click X or press Ctrl+D to close]\x1b[0m\r\n",
                    exit_code
                );
                let bytes = msg.as_bytes().to_vec();
                ring_buffer_waiter.append(&bytes);
                {
                    let mut guard = output_buffer_waiter.lock().unwrap();
                    guard.extend_from_slice(&bytes);
                }
                let _ = tx_waiter.send(bytes);
            }
        });

        let default_title = format!("{} ({})", profile, id);
        let session = Arc::new(PtySession {
            id: id.clone(),
            pid,
            profile,
            window_id: window_id.clone(),
            title: Arc::new(Mutex::new(default_title)),
            badge: Arc::new(Mutex::new(None)),
            color: Arc::new(Mutex::new(None)),
            is_dead,
            writer,
            output_buffer,
            ring_buffer,
            tx,
            _master: Arc::new(Mutex::new(pair.master)),
            _child: child_arc,
        });

        session.resize(30, 100);

        let map_key = format!("{}:{}", window_id, id);
        self.sessions.lock().unwrap().insert(map_key, session.clone());
        Ok(session)
    }

    pub fn get(&self, id: &str) -> Option<Arc<PtySession>> {
        self.get_in_window(id, None)
    }

    pub fn get_in_window(&self, id: &str, window_id: Option<&str>) -> Option<Arc<PtySession>> {
        let lock = self.sessions.lock().unwrap();
        if let Some(w) = window_id {
            if !w.is_empty() {
                let map_key = format!("{}:{}", w, id);
                if let Some(s) = lock.get(&map_key) {
                    return Some(s.clone());
                }
                return lock.values().find(|s| s.id == id && s.window_id == w).cloned();
            }
        }
        if let Some(s) = lock.get(id) {
            return Some(s.clone());
        }
        lock.values().find(|s| s.id == id).cloned()
    }

    #[allow(dead_code)]
    pub fn list(&self) -> Vec<Arc<PtySession>> {
        self.list_by_window(None)
    }

    pub fn list_by_window(&self, window_id: Option<&str>) -> Vec<Arc<PtySession>> {
        let lock = self.sessions.lock().unwrap();
        let mut list: Vec<Arc<PtySession>> = lock
            .values()
            .filter(|s| match window_id {
                Some(w) if !w.is_empty() => s.window_id == w,
                _ => true,
            })
            .cloned()
            .collect();

        list.sort_by(|a, b| {
            let num_a = a.id.trim_start_matches("tab-").parse::<u32>().unwrap_or(0);
            let num_b = b.id.trim_start_matches("tab-").parse::<u32>().unwrap_or(0);
            if num_a != num_b && num_a > 0 && num_b > 0 {
                num_a.cmp(&num_b)
            } else {
                a.id.cmp(&b.id)
            }
        });
        list
    }

    pub fn close(&self, id: &str) -> bool {
        let mut lock = self.sessions.lock().unwrap();
        let target_key = if lock.contains_key(id) {
            Some(id.to_string())
        } else {
            lock.iter().find(|(_, s)| s.id == id).map(|(k, _)| k.clone())
        };

        if let Some(key) = target_key {
            if let Some(session) = lock.remove(&key) {
                let child_opt = session._child.lock().unwrap().take();
                if let Some(mut child) = child_opt {
                    let _ = child.kill();
                } else {
                    session.kill_by_pid();
                }
                return true;
            }
        }
        false
    }

    pub fn resolve_tabs(&self, targets: &[String]) -> Vec<Arc<PtySession>> {
        let lock = self.sessions.lock().unwrap();
        let mut result = Vec::new();

        let has_active = targets.is_empty()
            || targets.iter().any(|t| t == "active" || t.is_empty());
        if has_active {
            if let Some(first) = lock.values().next() {
                result.push(first.clone());
                return result;
            }
        }

        for target in targets {
            if let Some(sess) = lock.get(target) {
                result.push(sess.clone());
                continue;
            }
            for sess in lock.values() {
                let title = sess.title.lock().unwrap().clone();
                if title.eq_ignore_ascii_case(target) {
                    result.push(sess.clone());
                    break;
                }
            }
        }
        result
    }

    pub fn resolve_tabs_strict(
        &self,
        targets: &[String],
        window_filter: Option<&str>,
    ) -> Result<Vec<Arc<PtySession>>, String> {
        let lock = self.sessions.lock().unwrap();
        let mut result = Vec::new();

        let has_active = targets.is_empty() || targets.iter().any(|t| t == "active" || t.is_empty());
        if has_active {
            if let Some(w) = window_filter {
                if let Some(first) = lock.values().find(|s| s.window_id == w) {
                    result.push(first.clone());
                    return Ok(result);
                }
            } else if let Some(first) = lock.values().next() {
                result.push(first.clone());
                return Ok(result);
            }
            return Err("Tab 'active' not found.".to_string());
        }

        for target in targets {
            let id_matches: Vec<_> = lock.values().filter(|s| &s.id == target).cloned().collect();
            let title_matches: Vec<_> = if id_matches.is_empty() {
                lock.values()
                    .filter(|s| s.title.lock().unwrap().eq_ignore_ascii_case(target))
                    .cloned()
                    .collect()
            } else {
                vec![]
            };

            let matches: Vec<_> = if !id_matches.is_empty() {
                id_matches
            } else {
                title_matches
            };

            if matches.is_empty() {
                return Err(format!("Tab '{}' not found.", target));
            }

            if let Some(w) = window_filter {
                let filtered: Vec<_> = matches.into_iter().filter(|s| s.window_id == w).collect();
                if filtered.is_empty() {
                    return Err(format!("Tab '{}' not found in window '{}'.", target, w));
                }
                result.extend(filtered);
            } else {
                let mut unique_windows: Vec<String> = matches.iter().map(|s| s.window_id.clone()).collect();
                unique_windows.sort();
                unique_windows.dedup();

                if unique_windows.len() > 1 {
                    let win_list = unique_windows
                        .iter()
                        .map(|w| format!("'{}'", w))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!(
                        "Ambiguous tab '{}' found in windows {}. Specify --window <win_id>.",
                        target, win_list
                    ));
                }
                result.extend(matches);
            }
        }

        Ok(result)
    }
}
