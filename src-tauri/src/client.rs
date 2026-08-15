use crate::cli::CliArgs;
use serde_json::json;
use std::time::Duration;

pub const PIPE_NAME: &str = r"\\.\pipe\kterm_daemon";

pub fn is_daemon_running() -> bool {
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(300) {
            match OpenOptions::new().read(true).write(true).open(PIPE_NAME) {
                Ok(_) => return true,
                Err(e) => {
                    // OS error 231 = ERROR_PIPE_BUSY. Pipe exists -> daemon is running!
                    if e.raw_os_error() == Some(231) {
                        return true;
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
            }
        }
        false
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn send_pipe_request(req: &serde_json::Value) -> Result<serde_json::Value, String> {
    use std::fs::OpenOptions;
    use std::io::{BufRead, BufReader, Write};

    let start = std::time::Instant::now();
    let mut file = loop {
        match OpenOptions::new().read(true).write(true).open(PIPE_NAME) {
            Ok(f) => break f,
            Err(e) => {
                // Retry on OS error 231 (ERROR_PIPE_BUSY) while daemon finishes previous connection
                if (e.raw_os_error() == Some(231) || e.kind() == std::io::ErrorKind::WouldBlock)
                    && start.elapsed() < Duration::from_secs(10)
                {
                    std::thread::sleep(Duration::from_millis(15));
                    continue;
                }
                return Err(format!("Failed to connect to kterm named pipe ({}): {}", PIPE_NAME, e));
            }
        }
    };

    let mut payload = serde_json::to_string(req).map_err(|e| e.to_string())?;
    payload.push('\n');
    file.write_all(payload.as_bytes()).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(file);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).map_err(|e| e.to_string())?;

    let resp: serde_json::Value = serde_json::from_str(&response_line)
        .map_err(|e| format!("Failed to parse JSON-RPC pipe response: {}", e))?;

    if resp.get("success").and_then(|v| v.as_bool()) == Some(true) {
        Ok(resp.get("data").cloned().unwrap_or(serde_json::Value::Null))
    } else {
        Err(resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

fn safe_println(msg: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stdout(), "{}", msg);
}

#[cfg(windows)]
fn spawn_daemon_detached(exe_path: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    #[repr(C)]
    struct StartupInfoW {
        cb: u32,
        reserved: *mut u16,
        desktop: *mut u16,
        title: *mut u16,
        x: u32,
        y: u32,
        x_size: u32,
        y_size: u32,
        x_count_chars: u32,
        y_count_chars: u32,
        fill_attribute: u32,
        flags: u32,
        show_window: u16,
        cb_reserved2: u16,
        lp_reserved2: *mut u8,
        h_std_input: *mut std::ffi::c_void,
        h_std_output: *mut std::ffi::c_void,
        h_std_error: *mut std::ffi::c_void,
    }

    #[repr(C)]
    struct ProcessInformation {
        h_process: *mut std::ffi::c_void,
        h_thread: *mut std::ffi::c_void,
        dw_process_id: u32,
        dw_thread_id: u32,
    }

    extern "system" {
        fn CreateProcessW(
            app_name: *const u16,
            cmd_line: *mut u16,
            proc_attr: *const std::ffi::c_void,
            thread_attr: *const std::ffi::c_void,
            inherit_handles: i32,
            dw_flags: u32,
            env: *const std::ffi::c_void,
            dir: *const u16,
            startup_info: *const StartupInfoW,
            proc_info: *mut ProcessInformation,
        ) -> i32;

        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }

    let mut exe_str: Vec<u16> = exe_path.as_os_str().encode_wide().collect();
    exe_str.push(0);

    let mut cmd_line: Vec<u16> = format!("\"{}\" --daemon", exe_path.to_string_lossy())
        .encode_utf16()
        .collect();
    cmd_line.push(0);

    let mut si: StartupInfoW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<StartupInfoW>() as u32;

    let mut pi: ProcessInformation = unsafe { std::mem::zeroed() };

    let flags: u32 = 0x00000008 | 0x00000200 | 0x01000000;

    let mut res = unsafe {
        CreateProcessW(
            exe_str.as_ptr(),
            cmd_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags,
            std::ptr::null(),
            std::ptr::null(),
            &si,
            &mut pi,
        )
    };

    if res == 0 {
        let fallback_flags: u32 = 0x00000008 | 0x00000200;
        res = unsafe {
            CreateProcessW(
                exe_str.as_ptr(),
                cmd_line.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                fallback_flags,
                std::ptr::null(),
                std::ptr::null(),
                &si,
                &mut pi,
            )
        };
    }

    if res != 0 {
        unsafe {
            if !pi.h_process.is_null() {
                CloseHandle(pi.h_process);
            }
            if !pi.h_thread.is_null() {
                CloseHandle(pi.h_thread);
            }
        }
        Ok(())
    } else {
        Err("Win32 CreateProcessW failed to spawn daemon".to_string())
    }
}

pub fn ensure_daemon_running() -> Result<(), String> {
    if is_daemon_running() {
        if let Ok(res) = send_pipe_request(&json!({ "action": "build_id" })) {
            let running_id = res.get("build_id").and_then(|v| v.as_str()).unwrap_or("");
            if running_id == env!("CARGO_PKG_VERSION") {
                return Ok(());
            }
            eprintln!(
                "[kterm] Stale daemon detected (version '{}' vs '{}'). Restarting...",
                running_id,
                env!("CARGO_PKG_VERSION")
            );
            let _ = send_pipe_request(&json!({ "action": "shutdown" }));
            std::thread::sleep(Duration::from_millis(300));
        } else {
            return Ok(());
        }
    }

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;

    #[cfg(windows)]
    spawn_daemon_detached(&exe_path)?;

    #[cfg(not(windows))]
    {
        let mut cmd = std::process::Command::new(&exe_path);
        cmd.arg("--daemon");
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        cmd.spawn().map_err(|e| format!("Failed to spawn daemon: {}", e))?;
    }

    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if is_daemon_running() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    Err("Timed out waiting for kterm host daemon to start".to_string())
}

pub fn handle_client_mode(args: &CliArgs) -> Result<(), String> {
    ensure_daemon_running()?;

    if let Some(apply_path) = &args.apply {
        let content = std::fs::read_to_string(apply_path)
            .map_err(|e| format!("Failed to read YAML session file '{}': {}", apply_path, e))?;

        let req = json!({
            "action": "apply_yaml",
            "yaml": content,
            "suffix": args.suffix.clone(),
            "suffix_auto": args.suffix_auto,
            "dry_run": args.dry_run,
            "window": args.window.clone(),
        });

        let val = send_pipe_request(&req)?;
        if args.dry_run {
            println!("YAML specification is valid.");
        } else if let Some(win_id) = val["window_id"].as_str() {
            safe_println(win_id);
        } else {
            safe_println(&serde_json::to_string_pretty(&val).unwrap());
        }
        return Ok(());
    }

    if args.get_layout {
        let window = args.window.clone().unwrap_or_else(|| "win-1".to_string());
        let req = json!({
            "action": "get_layout",
            "window": window,
        });
        let val = send_pipe_request(&req)?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap());
        return Ok(());
    }

    if args.get_config {
        let req = json!({ "action": "get_config" });
        let val = send_pipe_request(&req)?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap());
        return Ok(());
    }

    if let Some(out_path) = &args.export_layout {
        let window = args.window.clone().unwrap_or_else(|| "win-1".to_string());
        let req = json!({
            "action": "export_layout",
            "window": window,
        });
        let val = send_pipe_request(&req)?;
        let yaml_text = val.as_str().unwrap_or("");
        std::fs::write(out_path, yaml_text)
            .map_err(|e| format!("Failed to write layout to '{}': {}", out_path, e))?;
        println!("Layout written to: {}", out_path);
        match crate::exporter::export_shortcut_for_yaml(out_path) {
            Ok(lnk) => println!("Shortcut written to: {}", lnk),
            Err(e) => eprintln!("Warning: Failed to create shortcut: {}", e),
        }
        return Ok(());
    }

    if args.new_window {
        let req = json!({ "action": "create_window" });
        let val = send_pipe_request(&req)?;
        if let Some(id) = val["id"].as_str() {
            safe_println(id);
        } else {
            safe_println(&serde_json::to_string_pretty(&val).unwrap());
        }
        return Ok(());
    }

    if let Some(win_id) = &args.close_window {
        let req = json!({
            "action": "close_window",
            "window": win_id,
        });
        send_pipe_request(&req)?;
        return Ok(());
    }

    if args.list_windows {
        let req = json!({ "action": "list_windows" });
        let val = send_pipe_request(&req)?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&val).unwrap());
        } else {
            if let Some(windows) = val.as_array() {
                for w in windows {
                    println!(
                        "Window ID: {}, Label: {}",
                        w["id"].as_str().unwrap_or("-"),
                        w["label"].as_str().unwrap_or("-")
                    );
                }
            }
        }
        return Ok(());
    }

    if args.list_tabs {
        let req = json!({
            "action": "list_tabs",
            "window": args.window.clone(),
        });
        let val = send_pipe_request(&req)?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&val).unwrap());
        } else {
            if let Some(tabs) = val.as_array() {
                for t in tabs {
                    println!(
                        "ID: {}\tPID: {}\tWindow: {}\tProfile: {}\tTitle: {}\tBadge: {}\tColor: {}",
                        t["id"].as_str().unwrap_or("-"),
                        t["pid"].as_u64().unwrap_or(0),
                        t["window_id"].as_str().unwrap_or("-"),
                        t["profile"].as_str().unwrap_or("-"),
                        t["title"].as_str().unwrap_or("-"),
                        t["badge"].as_str().unwrap_or("none"),
                        t["color"].as_str().unwrap_or("none")
                    );
                }
            }
        }
        return Ok(());
    }

    if args.split_right || args.split_left || args.split_down || args.split_up || args.unsplit || args.explode_split {
        let targets = if args.select_tab.is_empty() {
            vec!["active".to_string()]
        } else {
            args.select_tab.clone()
        };

        if args.split_right || args.split_left || args.split_down || args.split_up {
            let direction = if args.split_left {
                "left"
            } else if args.split_up {
                "up"
            } else if args.split_down {
                "down"
            } else {
                "right"
            };
            for target in &targets {
                let req = json!({
                    "action": "split_tab",
                    "tab_id": target,
                    "direction": direction,
                    "profile": args.profile.clone(),
                    "move_tab_id": args.move_tab.clone(),
                });
                let val = send_pipe_request(&req)?;
                if let Some(id) = val["new_tab_id"].as_str() {
                    safe_println(id);
                } else {
                    safe_println(&serde_json::to_string_pretty(&val).unwrap());
                }
            }
            return Ok(());
        }

        if args.unsplit {
            for target in &targets {
                let req = json!({
                    "action": "unsplit_tab",
                    "tab_id": target,
                });
                send_pipe_request(&req)?;
            }
            return Ok(());
        }

        if args.explode_split {
            for target in &targets {
                let req = json!({
                    "action": "explode_tab",
                    "tab_id": target,
                });
                send_pipe_request(&req)?;
            }
            return Ok(());
        }

        return Ok(());
    }

    if !args.select_tab.is_empty() {
        let targets = &args.select_tab;
        let mut ran_action = false;

        if let Some(text_vec) = &args.send_text {
            ran_action = true;
            let text = text_vec.join(" ");
            let req = json!({
                "action": "send_text",
                "targets": targets,
                "command": text,
                "window": args.window.clone(),
            });
            send_pipe_request(&req)?;
        }

        if let Some(title_vec) = &args.send_title {
            ran_action = true;
            let title = title_vec.join(" ");
            let req = json!({
                "action": "set_title",
                "targets": targets,
                "title": title,
                "window": args.window.clone(),
            });
            send_pipe_request(&req)?;
        }

        if let Some(badge) = &args.set_badge {
            ran_action = true;
            let req = json!({
                "action": "set_badge",
                "targets": targets,
                "badge": badge,
                "window": args.window.clone(),
            });
            send_pipe_request(&req)?;
        }

        if let Some(color) = &args.set_color {
            ran_action = true;
            let req = json!({
                "action": "set_color",
                "targets": targets,
                "color": color,
                "window": args.window.clone(),
            });
            send_pipe_request(&req)?;
        }

        if args.focus {
            ran_action = true;
        }

        if args.close {
            let req = json!({
                "action": "close_tabs",
                "targets": targets,
                "force": args.force,
                "window": args.window.clone(),
            });
            send_pipe_request(&req)?;
            return Ok(());
        }

        let is_read = args.read_text.is_some() || args.tail.is_some() || args.raw;
        let is_wait = args.wait_for.is_some() || args.wait_for_prompt;

        if ran_action && !is_read && !is_wait {
            return Ok(());
        }
    }

    if args.read_text.is_some() || args.tail.is_some() || args.raw {
        let target_tab = match &args.read_text {
            Some(Some(t)) => t.clone(),
            _ => args.select_tab.first().cloned().unwrap_or_else(|| "tab-101".to_string()),
        };

        let tail = args.tail.unwrap_or(50);
        let raw = args.raw;
        let req = json!({
            "action": "read_text",
            "tab_id": target_tab,
            "tail": tail,
            "raw": raw,
        });
        let val = send_pipe_request(&req)?;
        if let Some(lines) = val["lines"].as_array() {
            for line in lines {
                if let Some(l) = line.as_str() {
                    safe_println(l);
                }
            }
        }
        return Ok(());
    }

    if args.wait_for.is_some() || args.wait_for_prompt {
        let target_tab = args
            .select_tab
            .first()
            .cloned()
            .unwrap_or_else(|| "tab-101".to_string());

        let timeout_sec = args.timeout.unwrap_or(30);
        let req = json!({
            "action": "wait_for",
            "tab_id": target_tab,
            "pattern": args.wait_for,
            "is_prompt": args.wait_for_prompt,
            "from_history": args.from_history,
            "timeout_sec": timeout_sec,
        });

        let val = send_pipe_request(&req)?;
        if args.json {
            safe_println(&serde_json::to_string_pretty(&val).unwrap());
        } else {
            println!("Matched output successfully.");
        }
        return Ok(());
    }

    // Default action: Spawn tab
    let req = json!({
        "action": "spawn_tab",
        "profile": args.profile.clone(),
        "window": args.window.clone(),
        "admin": args.admin,
        "elevated": args.admin,
    });
    let val = send_pipe_request(&req)?;
    if let Some(id) = val["id"].as_str() {
        safe_println(id);
    } else {
        safe_println(&serde_json::to_string_pretty(&val).unwrap());
    }

    Ok(())
}

pub fn ensure_window_visible(_win_id: &str) -> Result<(), String> {
    let cfg = crate::config::AppConfig::load();
    let default_profile = if cfg.default_profile.trim().is_empty() {
        "powershell"
    } else {
        &cfg.default_profile
    };

    let default_yaml = format!(
        "window:\n  id: null\ntabs:\n- id: null\n  profile: {}\n",
        default_profile
    );

    let req = json!({
        "action": "apply_yaml",
        "yaml": default_yaml,
        "suffix_auto": true,
    });

    let _ = send_pipe_request(&req);
    Ok(())
}
