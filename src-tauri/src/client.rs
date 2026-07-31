use crate::cli::CliArgs;
use serde_json::json;
use std::time::Duration;

pub fn is_daemon_running() -> bool {
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(300))
        .build();

    if let Ok(c) = client {
        if let Ok(res) = c.get("http://127.0.0.1:9999/health").send() {
            res.status().is_success()
        } else {
            false
        }
    } else {
        false
    }
}

pub fn handle_client_mode(args: &CliArgs) -> Result<(), String> {
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
                println!("{}", id);
            } else {
                println!("{}", serde_json::to_string_pretty(&val).unwrap());
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
            println!("{}", id);
        } else {
            println!("{}", serde_json::to_string_pretty(&val).unwrap());
        }
    } else {
        return Err(format!("Failed to spawn tab: {}", res.text().unwrap_or_default()));
    }

    Ok(())
}
