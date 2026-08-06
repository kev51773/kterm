use crate::cli::CliArgs;
use serde_json::json;
use std::time::Duration;

pub fn is_daemon_running() -> bool {
    use std::net::{SocketAddr, TcpStream};
    let addr: SocketAddr = match "127.0.0.1:9999".parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
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

    // DETACHED_PROCESS = 0x8, CREATE_NEW_PROCESS_GROUP = 0x200, CREATE_BREAKAWAY_FROM_JOB = 0x01000000
    let flags: u32 = 0x00000008 | 0x00000200 | 0x01000000;

    let mut res = unsafe {
        CreateProcessW(
            exe_str.as_ptr(),
            cmd_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0, // bInheritHandles = FALSE (0) -> Zero handle inheritance!
            flags,
            std::ptr::null(),
            std::ptr::null(),
            &si,
            &mut pi,
        )
    };

    if res == 0 {
        // Fallback without CREATE_BREAKAWAY_FROM_JOB if job disallows breakaway
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
        // Check if running daemon matches current binary version
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(200))
            .build()
            .ok();
        if let Some(c) = client {
            if let Ok(res) = c.get("http://127.0.0.1:9999/build_id").send() {
                if let Ok(json) = res.json::<serde_json::Value>() {
                    let running_id = json.get("build_id").and_then(|v| v.as_str()).unwrap_or("");
                    if running_id == env!("CARGO_PKG_VERSION") {
                        return Ok(());
                    }
                    eprintln!("[kterm] Stale daemon detected (version '{}' vs '{}'). Restarting...", running_id, env!("CARGO_PKG_VERSION"));
                    let _ = c.post("http://127.0.0.1:9999/shutdown").send();
                    std::thread::sleep(Duration::from_millis(300));
                }
            }
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

    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let base_url = "http://127.0.0.1:9999";

    if let Some(apply_path) = &args.apply {
        let content = std::fs::read_to_string(apply_path)
            .map_err(|e| format!("Failed to read YAML session file '{}': {}", apply_path, e))?;

        let body = json!({
            "yaml": content,
            "suffix": args.suffix.clone(),
            "suffix_auto": args.suffix_auto,
            "dry_run": args.dry_run,
            "window": args.window.clone(),
        });

        let res = client
            .post(format!("{}/apply", base_url))
            .json(&body)
            .send()
            .map_err(|e| format!("Failed to send apply request: {}", e))?;

        if res.status().is_success() {
            let val: serde_json::Value = res.json().map_err(|e| e.to_string())?;
            if args.dry_run {
                println!("YAML specification is valid.");
            } else if let Some(win_id) = val["window_id"].as_str() {
                safe_println(win_id);
            } else {
                safe_println(&serde_json::to_string_pretty(&val).unwrap());
            }
        } else {
            return Err(res.text().unwrap_or_default());
        }
        return Ok(());
    }

    if let Some(out_path) = &args.export_layout {
        let window = args.window.clone().unwrap_or_else(|| "win-1".to_string());
        let url = format!("{}/export-layout?window={}", base_url, window);
        let res = client
            .get(&url)
            .send()
            .map_err(|e| format!("Failed to request export-layout: {}", e))?;
        if !res.status().is_success() {
            return Err(format!("Export layout failed: {}", res.text().unwrap_or_default()));
        }
        let yaml_text = res.text().map_err(|e| e.to_string())?;
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
        let res = client
            .post(format!("{}/windows", base_url))
            .send()
            .map_err(|e| format!("Failed to create window: {}", e))?;

        if res.status().is_success() {
            let val: serde_json::Value = res.json().map_err(|e| e.to_string())?;
            if let Some(id) = val["id"].as_str() {
                safe_println(id);
            } else {
                safe_println(&serde_json::to_string_pretty(&val).unwrap());
            }
        } else {
            return Err(format!("Failed to create window: {}", res.text().unwrap_or_default()));
        }
        return Ok(());
    }

    if let Some(title_vec) = &args.set_window_title {
        let title = title_vec.join(" ");
        let body = json!({
            "window": args.window.clone(),
            "title": title,
        });
        let res = client
            .post(format!("{}/windows/title", base_url))
            .json(&body)
            .send()
            .map_err(|e| format!("Failed to set window title: {}", e))?;
        if !res.status().is_success() {
            return Err(format!("Set window title failed: {}", res.text().unwrap_or_default()));
        }
        return Ok(());
    }

    if let Some(win_id) = &args.close_window {
        let body = json!({
            "window": win_id,
        });
        let res = client
            .post(format!("{}/windows/close", base_url))
            .json(&body)
            .send()
            .map_err(|e| format!("Failed to close window: {}", e))?;
        if !res.status().is_success() {
            return Err(format!("Close window failed: {}", res.text().unwrap_or_default()));
        }
        return Ok(());
    }

    if args.list_windows {
        let res = client
            .get(format!("{}/windows", base_url))
            .send()
            .map_err(|e| format!("Failed to send list-windows request: {}", e))?;

        if args.json {
            let json_val: serde_json::Value = res.json().map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&json_val).unwrap());
        } else {
            let windows: Vec<serde_json::Value> = res.json().map_err(|e| e.to_string())?;
            for w in windows {
                println!(
                    "Window ID: {}, Label: {}",
                    w["id"].as_str().unwrap_or("-"),
                    w["label"].as_str().unwrap_or("-")
                );
            }
        }
        return Ok(());
    }

    if args.list_tabs {
        let url = if let Some(w) = &args.window {
            format!("{}/tabs?window={}", base_url, w)
        } else {
            format!("{}/tabs", base_url)
        };
        let res = client
            .get(url)
            .send()
            .map_err(|e| format!("Failed to send list-tabs request: {}", e))?;

        if args.json {
            let json_val: serde_json::Value = res.json().map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&json_val).unwrap());
        } else {
            let tabs: Vec<serde_json::Value> = res.json().map_err(|e| e.to_string())?;
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
        return Ok(());
    }

    // Split, Unsplit, or Explode operations (default target to "active" if select_tab is empty)
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
                let body = json!({
                    "direction": direction,
                    "profile": args.profile.clone(),
                    "move_tab_id": args.move_tab.clone(),
                });
                let res = client
                    .post(format!("{}/tabs/{}/split", base_url, target))
                    .json(&body)
                    .send()
                    .map_err(|e| format!("Failed to split tab {}: {}", target, e))?;

                if res.status().is_success() {
                    let val: serde_json::Value = res.json().map_err(|e| e.to_string())?;
                    if let Some(id) = val["new_tab_id"].as_str() {
                        safe_println(id);
                    } else {
                        safe_println(&serde_json::to_string_pretty(&val).unwrap());
                    }
                } else {
                    return Err(res.text().unwrap_or_default());
                }
            }
            return Ok(());
        }


        if args.unsplit {
            for target in &targets {
                let res = client
                    .post(format!("{}/tabs/{}/unsplit", base_url, target))
                    .send()
                    .map_err(|e| format!("Failed to unsplit tab {}: {}", target, e))?;
                if !res.status().is_success() {
                    return Err(res.text().unwrap_or_default());
                }
            }
            return Ok(());
        }

        if args.explode_split {
            for target in &targets {
                let res = client
                    .post(format!("{}/tabs/{}/explode", base_url, target))
                    .send()
                    .map_err(|e| format!("Failed to explode split for tab {}: {}", target, e))?;
                if !res.status().is_success() {
                    return Err(res.text().unwrap_or_default());
                }
            }
            return Ok(());
        }

        return Ok(());
    }

    // Action on selected tabs
    if !args.select_tab.is_empty() {
        let targets = &args.select_tab;
        let mut ran_action = false;

        if let Some(text_vec) = &args.send_text {
            ran_action = true;
            let text = text_vec.join(" ");
            let body = json!({
                "targets": targets,
                "command": text,
                "window": args.window.clone(),
            });
            let res = client
                .post(format!("{}/tabs/send", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to send text: {}", e))?;
            if !res.status().is_success() {
                return Err(res.text().unwrap_or_default());
            }
        }

        if let Some(title_vec) = &args.send_title {
            ran_action = true;
            let title = title_vec.join(" ");
            let body = json!({
                "targets": targets,
                "title": title,
                "window": args.window.clone(),
            });
            let res = client
                .post(format!("{}/tabs/title", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to set title: {}", e))?;
            if !res.status().is_success() {
                return Err(res.text().unwrap_or_default());
            }
        }

        if let Some(badge) = &args.set_badge {
            ran_action = true;
            let body = json!({
                "targets": targets,
                "badge": badge,
                "window": args.window.clone(),
            });
            let res = client
                .post(format!("{}/tabs/badge", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to set badge: {}", e))?;
            if !res.status().is_success() {
                return Err(res.text().unwrap_or_default());
            }
        }

        if let Some(color) = &args.set_color {
            ran_action = true;
            let body = json!({
                "targets": targets,
                "color": color,
                "window": args.window.clone(),
            });
            let res = client
                .post(format!("{}/tabs/color", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to set color: {}", e))?;
            if !res.status().is_success() {
                return Err(res.text().unwrap_or_default());
            }
        }

        if args.focus {
            ran_action = true;
            let body = json!({
                "targets": targets,
                "window": args.window.clone(),
            });
            let _ = client
                .post(format!("{}/tabs/focus", base_url))
                .json(&body)
                .send();
        }

        if args.close {
            let body = json!({
                "targets": targets,
                "force": args.force,
                "window": args.window.clone(),
            });
            let res = client
                .post(format!("{}/tabs/close", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to close tab: {}", e))?;
            if !res.status().is_success() {
                return Err(res.text().unwrap_or_default());
            }
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
        let url = format!("{}/tabs/{}/read?tail={}&raw={}", base_url, target_tab, tail, raw);
        let res = client
            .get(&url)
            .send()
            .map_err(|e| format!("Failed to read tab text: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Read tab text failed: {}", res.text().unwrap_or_default()));
        }

        let val: serde_json::Value = res.json().map_err(|e| e.to_string())?;
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
        let body = json!({
            "pattern": args.wait_for,
            "is_prompt": args.wait_for_prompt,
            "from_history": args.from_history,
            "timeout_sec": timeout_sec,
        });

        let wait_client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(timeout_sec + 10))
            .build()
            .unwrap_or_else(|_| client.clone());

        let url = format!("{}/tabs/{}/wait", base_url, target_tab);
        let res = wait_client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| format!("Failed to wait for tab output: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Wait failed: {}", res.text().unwrap_or_default()));
        }

        if args.json {
            let val: serde_json::Value = res.json().map_err(|e| e.to_string())?;
            safe_println(&serde_json::to_string_pretty(&val).unwrap());
        } else {
            println!("Matched output successfully.");
        }
        return Ok(());
    }

    // Default action: Spawn new tab (with optional profile and window)
    let body = json!({
        "profile": args.profile.clone(),
        "window": args.window.clone(),
    });

    let res = client
        .post(format!("{}/tabs", base_url))
        .json(&body)
        .send()
        .map_err(|e| format!("Failed to spawn tab: {}", e))?;

    if res.status().is_success() {
        let val: serde_json::Value = res.json().map_err(|e| e.to_string())?;
        if let Some(id) = val["id"].as_str() {
            safe_println(id);
        } else {
            safe_println(&serde_json::to_string_pretty(&val).unwrap());
        }
    } else {
        return Err(format!("Failed to spawn tab: {}", res.text().unwrap_or_default()));
    }

    Ok(())
}

pub fn ensure_window_visible(win_id: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .post("http://127.0.0.1:9999/windows/show")
        .json(&json!({ "window": win_id }))
        .send();

    match res {
        Ok(r) if r.status().is_success() => Ok(()),
        _ => Err("Failed to show window".to_string()),
    }
}
