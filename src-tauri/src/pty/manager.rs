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
    pub elevated: bool,
    pub title: Arc<Mutex<String>>,
    pub badge: Arc<Mutex<Option<String>>>,
    pub color: Arc<Mutex<Option<String>>>,
    pub cols: Arc<Mutex<u16>>,
    pub rows: Arc<Mutex<u16>>,
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
            *self.cols.lock().unwrap() = cols;
            *self.rows.lock().unwrap() = rows;
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

    pub fn is_alive(&self) -> bool {
        if *self.is_dead.lock().unwrap() {
            return false;
        }
        if self.pid == 0 {
            return true;
        }
        is_process_alive(self.pid)
    }
}

#[cfg(windows)]
pub fn is_app_elevated() -> bool {
    use std::ptr;
    extern "system" {
        fn OpenProcessToken(h: *mut std::ffi::c_void, access: u32, token: *mut *mut std::ffi::c_void) -> i32;
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetTokenInformation(token: *mut std::ffi::c_void, class: u32, info: *mut std::ffi::c_void, len: u32, return_len: *mut u32) -> i32;
        fn CloseHandle(h: *mut std::ffi::c_void) -> i32;
    }
    unsafe {
        let mut token: *mut std::ffi::c_void = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut token) != 0 {
            let mut elevation: u32 = 0;
            let mut size: u32 = 0;
            let res = GetTokenInformation(token, 20, &mut elevation as *mut u32 as *mut std::ffi::c_void, std::mem::size_of::<u32>() as u32, &mut size);
            CloseHandle(token);
            return res != 0 && elevation != 0;
        }
    }
    false
}

#[cfg(not(windows))]
pub fn is_app_elevated() -> bool {
    false
}

pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(windows)]
    {
        extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
            fn GetExitCodeProcess(handle: *mut std::ffi::c_void, exit_code: *mut u32) -> i32;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }
        unsafe {
            let handle = OpenProcess(0x1000, 0, pid);
            if handle.is_null() {
                return false;
            }
            let mut exit_code: u32 = 0;
            let res = GetExitCodeProcess(handle, &mut exit_code);
            CloseHandle(handle);
            res != 0 && exit_code == 259
        }
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as libc::pid_t, 0) == 0
    }
}
#[cfg(windows)]
pub fn run_elevated_pty_bridge(id: &str, profile: &str) {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    // The id is passed as "tab_id:uuid" to ensure unique pipe names
    let parts: Vec<&str> = id.split(':').collect();
    let pipe_id = if parts.len() == 2 { parts[1] } else { id };

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

#[cfg(windows)]
pub fn create_win32_named_pipe_handle(pipe_name: &str) -> Result<*mut std::ffi::c_void, String> {
    use std::os::windows::ffi::OsStrExt;

    #[allow(non_snake_case)]
    #[repr(C)]
    struct SECURITY_ATTRIBUTES {
        nLength: u32,
        lpSecurityDescriptor: *mut std::ffi::c_void,
        bInheritHandle: i32,
    }

    extern "system" {
        fn CreateNamedPipeW(
            name: *const u16,
            open_mode: u32,
            pipe_mode: u32,
            max_instances: u32,
            out_buffer_size: u32,
            in_buffer_size: u32,
            default_timeout: u32,
            security_attributes: *const SECURITY_ATTRIBUTES,
        ) -> *mut std::ffi::c_void;

        fn InitializeSecurityDescriptor(
            pSecurityDescriptor: *mut std::ffi::c_void,
            dwRevision: u32,
        ) -> i32;

        fn SetSecurityDescriptorDacl(
            pSecurityDescriptor: *mut std::ffi::c_void,
            bDaclPresent: i32,
            pDacl: *mut std::ffi::c_void,
            bDaclDefaulted: i32,
        ) -> i32;
    }

    let mut sd = [0u8; 40];
    unsafe {
        InitializeSecurityDescriptor(sd.as_mut_ptr() as *mut std::ffi::c_void, 1);
        SetSecurityDescriptorDacl(sd.as_mut_ptr() as *mut std::ffi::c_void, 1, std::ptr::null_mut(), 0);
    }

    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.as_mut_ptr() as *mut std::ffi::c_void,
        bInheritHandle: 0,
    };

    let full_path = format!("\\\\.\\pipe\\{}", pipe_name);
    let path_w: Vec<u16> = std::ffi::OsStr::new(&full_path).encode_wide().chain(std::iter::once(0)).collect();

    let handle = unsafe {
        CreateNamedPipeW(
            path_w.as_ptr(),
            3, // PIPE_ACCESS_DUPLEX
            0,
            255,
            65536,
            65536,
            0,
            &sa as *const SECURITY_ATTRIBUTES,
        )
    };

    let invalid_handle = -1isize as *mut std::ffi::c_void;
    if handle == invalid_handle {
        return Err("Failed to create named pipe server".to_string());
    }

    Ok(handle)
}

#[cfg(windows)]
pub fn connect_win32_named_pipe(handle: *mut std::ffi::c_void) -> Result<std::fs::File, String> {
    use std::os::windows::io::FromRawHandle;
    use std::sync::mpsc;
    use std::time::Duration;

    extern "system" {
        fn ConnectNamedPipe(handle: *mut std::ffi::c_void, overlapped: *mut std::ffi::c_void) -> i32;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        fn GetLastError() -> u32;
    }

    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let handle_val = handle as usize;

    std::thread::spawn(move || {
        let h = handle_val as *mut std::ffi::c_void;
        let connected = unsafe { ConnectNamedPipe(h, std::ptr::null_mut()) };
        if connected == 0 {
            let err = unsafe { GetLastError() };
            if err != 535 && err != 183 {
                let _ = tx.send(Err(format!("ConnectNamedPipe failed with error {}", err)));
                return;
            }
        }
        let _ = tx.send(Ok(()));
    });

    match rx.recv_timeout(Duration::from_secs(30)) {
        Ok(Ok(())) => {
            unsafe { Ok(std::fs::File::from_raw_handle(handle)) }
        },
        Ok(Err(e)) => {
            unsafe { CloseHandle(handle); }
            Err(e)
        }
        Err(_) => {
            unsafe { CloseHandle(handle); }
            Err("Elevated process did not connect within 30 seconds".to_string())
        }
    }
}

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

#[cfg(windows)]
fn assign_pid_to_job(pid: u32) {
    if pid == 0 {
        return;
    }
    use std::sync::Once;
    static START: Once = Once::new();
    static mut JOB_HANDLE: *mut std::ffi::c_void = std::ptr::null_mut();

    unsafe {
        START.call_once(|| {
            extern "system" {
                fn CreateJobObjectW(lpJobAttributes: *mut std::ffi::c_void, lpName: *const u16) -> *mut std::ffi::c_void;
                fn SetInformationJobObject(
                    hJob: *mut std::ffi::c_void,
                    JobObjectInformationClass: i32,
                    lpJobObjectInformation: *const std::ffi::c_void,
                    cbJobObjectInformationLength: u32,
                ) -> i32;
            }

            #[repr(C)]
            struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
                per_process_user_time_limit: i64,
                per_job_user_time_limit: i64,
                limit_flags: u32,
                minimum_working_set_size: usize,
                maximum_working_set_size: usize,
                active_process_limit: u32,
                affinity: usize,
                priority_class: u32,
                scheduling_class: u32,
            }

            #[repr(C)]
            struct IO_COUNTERS {
                read_operation_count: u64,
                write_operation_count: u64,
                other_operation_count: u64,
                read_transfer_count: u64,
                write_transfer_count: u64,
                other_transfer_count: u64,
            }

            #[repr(C)]
            struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
                basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
                io_info: IO_COUNTERS,
                process_memory_limit: usize,
                job_memory_limit: usize,
                peak_process_memory_used: usize,
                peak_job_memory_used: usize,
            }

            const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
            const JOB_OBJECT_EXTENDED_LIMIT_INFO_CLASS: i32 = 9;

            let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if !job.is_null() {
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job,
                    JOB_OBJECT_EXTENDED_LIMIT_INFO_CLASS,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                JOB_HANDLE = job;
            }
        });

        if !JOB_HANDLE.is_null() {
            extern "system" {
                fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
                fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
                fn AssignProcessToJobObject(hJob: *mut std::ffi::c_void, hProcess: *mut std::ffi::c_void) -> i32;
            }
            let proc_handle = OpenProcess(0x1F0FFF, 0, pid);
            if !proc_handle.is_null() {
                AssignProcessToJobObject(JOB_HANDLE, proc_handle);
                CloseHandle(proc_handle);
            }
        }
    }
}
