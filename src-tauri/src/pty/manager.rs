use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub struct PtySession {
    pub id: String,
    pub pid: u32,
    pub profile: String,
    pub window_id: String,
    pub title: Arc<Mutex<String>>,
    pub badge: Arc<Mutex<Option<String>>>,
    pub color: Arc<Mutex<Option<String>>>,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
    pub tx: broadcast::Sender<Vec<u8>>,
    _master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    _child: Arc<Mutex<Box<dyn Child + Send>>>,
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
}

unsafe impl Sync for PtySession {}

#[derive(Clone, Default)]
pub struct PtyManager {
    sessions: Arc<Mutex<HashMap<String, Arc<PtySession>>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn(
        &self,
        id: String,
        profile: String,
        window_id: String,
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
            "cmd" => {
                let mut c = CommandBuilder::new("cmd.exe");
                c.arg("/K");
                c
            }
            "wsl" => CommandBuilder::new("wsl.exe"),
            "git-bash" | "bash" => {
                let mut c = CommandBuilder::new("C:\\Program Files\\Git\\bin\\bash.exe");
                c.arg("--login");
                c
            }
            _ => {
                let mut c = CommandBuilder::new("powershell.exe");
                c.arg("-NoExit");
                c
            }
        };

        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        let pid = child.process_id().unwrap_or(0);
        let writer = Arc::new(Mutex::new(pair.master.take_writer().map_err(|e| e.to_string())?));
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

        let (tx, _rx) = broadcast::channel::<Vec<u8>>(256);
        let tx_clone = tx.clone();
        let output_buffer = Arc::new(Mutex::new(Vec::<u8>::with_capacity(65536)));
        let output_buffer_clone = output_buffer.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = &buf[..n];
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

        let default_title = format!("{} ({})", profile, id);
        let session = Arc::new(PtySession {
            id: id.clone(),
            pid,
            profile,
            window_id,
            title: Arc::new(Mutex::new(default_title)),
            badge: Arc::new(Mutex::new(None)),
            color: Arc::new(Mutex::new(None)),
            writer,
            output_buffer,
            tx,
            _master: Arc::new(Mutex::new(pair.master)),
            _child: Arc::new(Mutex::new(child)),
        });

        session.resize(30, 100);

        self.sessions.lock().unwrap().insert(id, session.clone());
        Ok(session)
    }

    pub fn get(&self, id: &str) -> Option<Arc<PtySession>> {
        self.sessions.lock().unwrap().get(id).cloned()
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
        if let Some(session) = lock.remove(id) {
            let _ = session._child.lock().unwrap().kill();
            true
        } else {
            false
        }
    }

    pub fn resolve_tabs(&self, targets: &[String]) -> Vec<Arc<PtySession>> {
        let lock = self.sessions.lock().unwrap();
        let mut result = Vec::new();
        for target in targets {
            if let Some(sess) = lock.get(target) {
                result.push(sess.clone());
                continue;
            }
            for sess in lock.values() {
                let title = sess.title.lock().unwrap().clone();
                if title.eq_ignore_ascii_case(target) {
                    result.push(sess.clone());
                }
            }
        }
        result
    }
}
