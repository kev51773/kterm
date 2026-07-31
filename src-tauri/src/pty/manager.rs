use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub struct PtySession {
    pub id: String,
    pub pid: u32,
    pub profile: String,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub tx: broadcast::Sender<Vec<u8>>,
    _master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    _child: Arc<Mutex<Box<dyn Child + Send>>>,
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

    pub fn spawn(&self, id: String, profile: String) -> Result<Arc<PtySession>, String> {
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

        std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = tx_clone.send(buf[..n].to_vec());
                    }
                    Err(_) => break,
                }
            }
        });

        let session = Arc::new(PtySession {
            id: id.clone(),
            pid,
            profile,
            writer,
            tx,
            _master: Arc::new(Mutex::new(pair.master)),
            _child: Arc::new(Mutex::new(child)),
        });

        self.sessions.lock().unwrap().insert(id, session.clone());
        Ok(session)
    }

    pub fn get(&self, id: &str) -> Option<Arc<PtySession>> {
        self.sessions.lock().unwrap().get(id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<PtySession>> {
        self.sessions.lock().unwrap().values().cloned().collect()
    }
}
