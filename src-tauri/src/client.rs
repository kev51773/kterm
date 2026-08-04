use crate::cli::CliArgs;
use serde_json::json;
use std::io::Write;
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
        return Ok(());
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
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let base_url = "http://127.0.0.1:9999";

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
                    return Err(format!("Split failed: {}", res.text().unwrap_or_default()));
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
                    return Err(format!("Unsplit failed: {}", res.text().unwrap_or_default()));
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
                    return Err(format!("Explode split failed: {}", res.text().unwrap_or_default()));
                }
            }
            return Ok(());
        }

        return Ok(());
    }

    // Action on selected tabs
    if !args.select_tab.is_empty() {
        let targets = &args.select_tab;

        if let Some(text_vec) = &args.send_text {
            let text = text_vec.join(" ");
            let body = json!({
                "targets": targets,
                "command": text,
            });
            let res = client
                .post(format!("{}/tabs/send", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to send text: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Send text failed: {}", res.text().unwrap_or_default()));
            }
        }

        if let Some(title_vec) = &args.send_title {
            let title = title_vec.join(" ");
            let body = json!({
                "targets": targets,
                "title": title,
            });
            let res = client
                .post(format!("{}/tabs/title", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to set title: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Set title failed: {}", res.text().unwrap_or_default()));
            }
        }

        if let Some(badge) = &args.set_badge {
            let body = json!({
                "targets": targets,
                "badge": badge,
            });
            let res = client
                .post(format!("{}/tabs/badge", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to set badge: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Set badge failed: {}", res.text().unwrap_or_default()));
            }
        }

        if let Some(color) = &args.set_color {
            let body = json!({
                "targets": targets,
                "color": color,
            });
            let res = client
                .post(format!("{}/tabs/color", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to set color: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Set color failed: {}", res.text().unwrap_or_default()));
            }
        }

        if args.focus {
            let body = json!({
                "targets": targets,
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
            });
            let res = client
                .post(format!("{}/tabs/close", base_url))
                .json(&body)
                .send()
                .map_err(|e| format!("Failed to close tab: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("Close tab failed: {}", res.text().unwrap_or_default()));
            }
        }

        return Ok(());
    }



    // Export current window layout to a script file
    if let Some(out_path) = &args.export_script {
        let window = args.window.clone().unwrap_or_else(|| "win-1".to_string());
        let url = format!(
            "{}/export?window={}&format={}",
            base_url, window, args.format
        );
        let res = client
            .get(&url)
            .send()
            .map_err(|e| format!("Failed to request export: {}", e))?;
        if !res.status().is_success() {
            return Err(format!("Export failed: {}", res.text().unwrap_or_default()));
        }
        let script = res.text().map_err(|e| e.to_string())?;
        std::fs::write(out_path, script)
            .map_err(|e| format!("Failed to write script to '{}': {}", out_path, e))?;
        println!("Layout script written to: {}", out_path);
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
