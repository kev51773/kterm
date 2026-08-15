#[cfg(windows)]
pub fn run_elevated_pty_bridge(id: &str, profile: &str) {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use super::platform::assign_pid_to_job;

    // The id is passed as "tab_id:uuid" to ensure unique pipe names
    let parts: Vec<&str> = id.split(':').collect();
    let pipe_id = if parts.len() == 2 { parts[1] } else { id };

    // Validate pipe_id to prevent named pipe path traversal or injection
    if !pipe_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return;
    }

    let pipe_in_path = format!("\\\\.\\pipe\\kterm_pipe_in_{}", pipe_id);
    let pipe_out_path = format!("\\\\.\\pipe\\kterm_pipe_out_{}", pipe_id);

    let mut file_in = loop {
        match OpenOptions::new().read(true).write(true).open(&pipe_in_path) {
            Ok(f) => {
                break f;
            }
            Err(_) => {}
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };

    let mut file_out = loop {
        match OpenOptions::new().read(true).write(true).open(&pipe_out_path) {
            Ok(f) => {
                break f;
            }
            Err(_) => {}
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };

    let bash_path = if std::path::Path::new("C:\\Program Files\\Git\\bin\\bash.exe").exists() {
        "C:\\Program Files\\Git\\bin\\bash.exe".to_string()
    } else {
        "bash.exe".to_string()
    };

    let cmd = match profile.to_lowercase().as_str() {
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

    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 30,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(p) => { p },
        Err(_) => { return; },
    };

    let child = match pair.slave.spawn_command(cmd) {
        Ok(c) => { c },
        Err(_) => { return; },
    };

    let pid = child.process_id().unwrap_or(0);
    #[cfg(windows)]
    assign_pid_to_job(pid);

    let child_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(child)));
    let child_clone = child_arc.clone();

    let mut master_writer = match pair.master.take_writer() {
        Ok(w) => { w },
        Err(_) => { return; },
    };
    let mut master_reader = match pair.master.try_clone_reader() {
        Ok(r) => { r },
        Err(_) => { return; },
    };

    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = file_in.read(&mut buf) {
            if n == 0 {
                break;
            }
            if master_writer.write_all(&buf[..n]).is_err() {
                break;
            }
        }
        if let Ok(mut lock) = child_clone.lock() {
            if let Some(mut c) = lock.take() {
                let _ = c.kill();
            }
        }
        std::process::exit(0);
    });

    let mut buf = [0u8; 4096];
    while let Ok(n) = master_reader.read(&mut buf) {
        if n == 0 {
            break;
        }
        if file_out.write_all(&buf[..n]).is_err() {
            break;
        }
    }
    if let Ok(mut lock) = child_arc.lock() {
        if let Some(mut c) = lock.take() {
            let _ = c.kill();
        }
    }
    std::process::exit(0);
}

#[cfg(not(windows))]
pub fn run_elevated_pty_bridge(_pipe_name: &str, _profile: &str) {}
