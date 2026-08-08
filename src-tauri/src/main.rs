mod cli;
mod client;
mod config;
mod daemon;
mod exporter;
mod pty;
mod yaml;

use clap::Parser;
use cli::CliArgs;
use daemon::AppState;
use pty::PtyManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        cli::print_help();
        std::process::exit(0);
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let raw_args: Vec<String> = std::env::args().collect();
    let mut modified_args = raw_args.clone();

    if raw_args.len() > 1 && !raw_args.iter().any(|a| a == "--apply" || a == "--export-layout") {
        for (idx, arg) in raw_args.iter().enumerate().skip(1) {
            if !arg.starts_with('-') && (arg.ends_with(".yaml") || arg.ends_with(".yml")) {
                let prev_arg = raw_args.get(idx - 1).map(|s| s.as_str());
                if prev_arg != Some("--export-layout") && prev_arg != Some("--apply") {
                    modified_args.insert(idx, "--apply".to_string());
                    break;
                }
            }
        }
    }

    let args = CliArgs::parse_from(modified_args);

    if args.daemon {
        run_host_daemon(args);
        return;
    }

    if let Err(e) = client::ensure_daemon_running() {
        eprintln!("Error starting kterm daemon: {}", e);
        std::process::exit(1);
    }

    let has_client_args = std::env::args().len() > 1;
    if has_client_args {
        if let Err(e) = client::handle_client_mode(&args) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        let _ = client::ensure_window_visible("win-1");
    }

    std::process::exit(0);
}

fn run_host_daemon(_args: CliArgs) {
    let pty_manager = PtyManager::new();

    let window_titles = Arc::new(Mutex::new(HashMap::new()));
    window_titles.lock().unwrap().insert(
        "win-1".to_string(),
        "kterm.exe - A scriptable terminal - win-1".to_string(),
    );

    let window_layouts = Arc::new(Mutex::new(HashMap::new()));

    let pty_manager_event = pty_manager.clone();
    let window_titles_event = window_titles.clone();
    let window_layouts_event = window_layouts.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {
            // Pre-flight purge for win-1 on daemon setup
            let sessions = pty_manager.list_by_window(Some("win-1"));
            for s in sessions {
                pty_manager.close(&s.id);
            }
            window_titles.lock().unwrap().remove("win-1");
            window_layouts.lock().unwrap().remove("win-1");

            let cfg = crate::config::AppConfig::load();
            let (default_w, default_h) = cfg.get_default_window_size();

            if let Some(window) = app.get_webview_window("win-1") {
                eprintln!("[kterm host] Primary window 'win-1' initialized");
                let _ = window.set_title("kterm.exe - A scriptable terminal - win-1");
                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: default_w, height: default_h }));
                let _ = window.show();
                let _ = window.center();
                let _ = window.set_focus();
            } else if let Some(window) = app.get_webview_window("main") {
                eprintln!("[kterm host] Window 'main' fallback initialized");
                let _ = window.set_title("kterm.exe - A scriptable terminal - win-1");
                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: default_w, height: default_h }));
                let _ = window.show();
                let _ = window.center();
                let _ = window.set_focus();
            } else {
                eprintln!("[kterm ERROR] Primary window NOT found in setup!");
            }

            let app_handle = app.handle().clone();
            let daemon_state = AppState {
                pty_manager,
                app_handle: Some(app_handle),
                window_titles,
                window_layouts,
            };

            tauri::async_runtime::spawn(async move {
                daemon::run_server("127.0.0.1:9999", daemon_state).await;
            });
            Ok(())
        })
        .on_window_event(move |window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let win_label = window.label();
                eprintln!("[kterm] Window '{}' destroyed. Cleaning up PTY sessions.", win_label);

                let sessions = pty_manager_event.list_by_window(Some(win_label));
                for s in sessions {
                    pty_manager_event.close_in_window(&s.id, Some(win_label));
                }
                window_titles_event.lock().unwrap().remove(win_label);
                window_layouts_event.lock().unwrap().remove(win_label);

                let app = window.app_handle();
                let remaining = app.webview_windows();
                if remaining.is_empty() {
                    eprintln!("[kterm] All windows closed. Terminating background daemon.");
                    std::process::exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
