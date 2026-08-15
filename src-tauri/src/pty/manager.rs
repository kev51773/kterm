use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub use super::elevated::run_elevated_pty_bridge;
use super::platform::*;
use super::session::{ExitCallback, PtySession};

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

    #[allow(dead_code)]
    pub fn spawn(
        &self,
        id: String,
        profile: String,
        window_id: String,
    ) -> Result<Arc<PtySession>, String> {
        self.spawn_with_cwd(id, profile, window_id, None, false)
    }

    pub fn spawn_with_cwd(
        &self,
        id: String,
        profile: String,
        window_id: String,
        cwd: Option<&str>,
        elevated: bool,
    ) -> Result<Arc<PtySession>, String> {
        let cfg = crate::config::AppConfig::load();
        let cols = if cfg.default_cols > 0 { cfg.default_cols } else { 120 };
        let rows = if cfg.default_rows > 0 { cfg.default_rows } else { 30 };
        self.spawn_with_size_and_cwd(id, profile, window_id, cols, rows, cwd, elevated)
    }

    pub fn generate_next_tab_id_for_window(&self, window_id: &str) -> String {
        let lock = self.sessions.lock().unwrap();
        let mut max_id: u32 = 0;
        for s in lock.values() {
            if s.window_id == window_id {
                if let Some(num_str) = s.id.strip_prefix("tab-") {
                    if let Ok(num) = num_str.parse::<u32>() {
                        if num > max_id {
                            max_id = num;
                        }
                    }
                }
            }
        }
        let mut candidate_num = max_id + 1;
        loop {
            let candidate = format!("tab-{}", candidate_num);
            let exists = lock
                .values()
                .any(|s| s.window_id == window_id && s.id == candidate);
            if !exists {
                return candidate;
            }
            candidate_num += 1;
        }
    }

    pub fn spawn_with_size_and_cwd(
        &self,
        id: String,
        profile: String,
        window_id: String,
        cols: u16,
        rows: u16,
        cwd: Option<&str>,
        elevated: bool,
    ) -> Result<Arc<PtySession>, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: if rows > 0 { rows } else { 30 },
                cols: if cols > 0 { cols } else { 120 },
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let bash_path = if std::path::Path::new("C:\\Program Files\\Git\\bin\\bash.exe").exists() {
            "C:\\Program Files\\Git\\bin\\bash.exe".to_string()
        } else if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = format!("{}\\Programs\\Git\\bin\\bash.exe", local);
            if std::path::Path::new(&p).exists() {
                p
            } else {
                "bash.exe".to_string()
            }
        } else {
            "bash.exe".to_string()
        };

        let (writer, mut reader, child_arc, pid): (
            Arc<Mutex<Box<dyn Write + Send>>>,
            Box<dyn Read + Send>,
            Arc<Mutex<Option<Box<dyn Child + Send>>>>,
            u32,
        ) = if elevated && !is_app_elevated() {
            #[cfg(windows)]
            {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
                let pid = std::process::id();
                let pipe_uuid = format!("{}_{}", now, pid);
                
                let pipe_in_name = format!("kterm_pipe_in_{}", pipe_uuid);
                let pipe_out_name = format!("kterm_pipe_out_{}", pipe_uuid);

                let handle_in = create_win32_named_pipe_handle(&pipe_in_name)?;
                let handle_out = match create_win32_named_pipe_handle(&pipe_out_name) {
                    Ok(h) => h,
                    Err(e) => {
                        extern "system" { fn CloseHandle(h: *mut std::ffi::c_void) -> i32; }
                        unsafe { CloseHandle(handle_in); }
                        return Err(e);
                    }
                };

                let exe_path = std::env::current_exe()
                    .map_err(|e| {
                        extern "system" { fn CloseHandle(h: *mut std::ffi::c_void) -> i32; }
                        unsafe { CloseHandle(handle_in); CloseHandle(handle_out); }
                        format!("Failed to get executable path: {}", e)
                    })?;

                {
                    use std::os::windows::ffi::OsStrExt;
                    let path_w: Vec<u16> = exe_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                    let verb_w: Vec<u16> = std::ffi::OsStr::new("runas").encode_wide().chain(std::iter::once(0)).collect();
                    let params = format!("--elevated-pty-bridge \"{}:{}\" --profile \"{}\"", id, pipe_uuid, profile);
                    let params_w: Vec<u16> = std::ffi::OsStr::new(&params).encode_wide().chain(std::iter::once(0)).collect();

                    unsafe {
                        extern "system" {
                            fn ShellExecuteW(
                                hwnd: *mut std::ffi::c_void,
                                operation: *const u16,
                                file: *const u16,
                                parameters: *const u16,
                                directory: *const u16,
                                show_cmd: i32,
                            ) -> *mut std::ffi::c_void;
                            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
                        }
                        let result = ShellExecuteW(
                            std::ptr::null_mut(),
                            verb_w.as_ptr(),
                            path_w.as_ptr(),
                            params_w.as_ptr(),
                            std::ptr::null(),
                            0,
                        );
                        if (result as isize) <= 32 {
                            CloseHandle(handle_in);
                            CloseHandle(handle_out);
                            return Err("Failed to launch elevated process (UAC denied or not available)".to_string());
                        }
                    }
                }

                let file_in = connect_win32_named_pipe(handle_in)?;
                let file_out = connect_win32_named_pipe(handle_out)?;

                (
                    Arc::new(Mutex::new(Box::new(file_in) as Box<dyn Write + Send>)),
                    Box::new(file_out) as Box<dyn Read + Send>,
                    Arc::new(Mutex::new(None)),
                    0,
                )
            }
            #[cfg(not(windows))]
            {
                return Err("Elevation bridging is only supported on Windows".to_string());
            }
        } else {
            let mut cmd = match profile.to_lowercase().as_str() {
                "cmd" => {
                    let mut c = CommandBuilder::new("cmd.exe");
                    c.arg("/K");
                    c
                }
                "wsl" => CommandBuilder::new("wsl.exe"),
                "git-bash" | "bash" => {
                    let mut c = CommandBuilder::new(&bash_path);
                    c.arg("--login");
                    c.arg("-i");
                    c.env("TERM", "xterm-256color");
                    c.env("MSYSTEM", "MINGW64");
                    c
                }
                _ => {
                    let mut c = CommandBuilder::new("powershell.exe");
                    c.arg("-NoExit");
                    c
                }
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
            #[cfg(windows)]
            assign_pid_to_job(pid);

            let master_writer = pair.master.take_writer().map_err(|e| e.to_string())?;
            let master_reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

            (
                Arc::new(Mutex::new(master_writer as Box<dyn Write + Send>)),
                Box::new(master_reader) as Box<dyn Read + Send>,
                Arc::new(Mutex::new(Some(child))),
                pid,
            )
        };

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

        // Reader thread for stdout/stderr streaming
        std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        break;
                    },
                    Ok(n) => {
                        let chunk = &buf[..n];
                        ring_buffer_clone.append(chunk);
                        {
                            let mut guard = output_buffer_clone.lock().unwrap();
                            guard.extend_from_slice(chunk);
                            if guard.len() > 65536 {
                                let overflow = guard.len() - 65536;
                                guard.drain(0..overflow);
                            }
                        }
                        let _ = tx_clone.send(chunk.to_vec());
                    }
                    Err(_) => {
                        break;
                    },
                }
            }
            *is_dead_clone.lock().unwrap() = true;
            let cb_opt = self_clone.on_exit.lock().unwrap().clone();
            if let Some(cb) = cb_opt {
                cb(id_clone);
            } else {
                self_clone.close(&id_clone);
            }
        });

        let default_title = profile.clone();
        let is_admin_session = elevated || is_app_elevated();
        let session = Arc::new(PtySession {
            id: id.clone(),
            pid,
            profile,
            window_id: window_id.clone(),
            elevated: is_admin_session,
            title: Arc::new(Mutex::new(default_title)),
            badge: Arc::new(Mutex::new(None)),
            color: Arc::new(Mutex::new(None)),
            cols: Arc::new(Mutex::new(cols)),
            rows: Arc::new(Mutex::new(rows)),
            is_dead,
            writer,
            output_buffer,
            ring_buffer,
            tx,
            _master: Arc::new(Mutex::new(pair.master)),
            _child: child_arc,
        });

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

    pub fn prune_dead_sessions(&self) -> Vec<String> {
        let mut lock = self.sessions.lock().unwrap();
        let mut dead_keys = Vec::new();
        for (key, session) in lock.iter() {
            if !session.is_alive() {
                dead_keys.push(key.clone());
            }
        }
        let mut removed_ids = Vec::new();
        for key in dead_keys {
            if let Some(session) = lock.remove(&key) {
                removed_ids.push(session.id.clone());
            }
        }
        removed_ids
    }

    pub fn list_by_window(&self, window_id: Option<&str>) -> Vec<Arc<PtySession>> {
        self.prune_dead_sessions();
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
        self.close_in_window(id, None)
    }

    pub fn close_in_window(&self, id: &str, window_id: Option<&str>) -> bool {
        let mut lock = self.sessions.lock().unwrap();
        let target_key = if let Some(w) = window_id {
            if !w.is_empty() {
                let map_key = format!("{}:{}", w, id);
                if lock.contains_key(&map_key) {
                    Some(map_key)
                } else {
                    lock.iter()
                        .find(|(_, s)| s.id == id && s.window_id == w)
                        .map(|(k, _)| k.clone())
                }
            } else {
                None
            }
        } else {
            None
        };

        let target_key = target_key.or_else(|| {
            if lock.contains_key(id) {
                Some(id.to_string())
            } else {
                lock.iter().find(|(_, s)| s.id == id).map(|(k, _)| k.clone())
            }
        });

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

    pub fn close_all_for_window(&self, window_id: &str) {
        let sessions = self.list_by_window(Some(window_id));
        for s in sessions {
            self.close_in_window(&s.id, Some(window_id));
        }
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
            if let Some(sess) = lock.values().find(|s| &s.id == target) {
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
