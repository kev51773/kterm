// Prevents additional console window on Windows in release, do not remove!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod client;
mod daemon;
mod pty;

use clap::Parser;
use cli::CliArgs;
use daemon::AppState;
use pty::PtyManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    tracing_subscriber::fmt::init();

    let args = CliArgs::parse();

    if client::is_daemon_running() {
        if let Err(e) = client::handle_client_mode(&args) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    // HOST MODE: Spawns Axum daemon server and launches Tauri GUI
    let pty_manager = PtyManager::new();

    let initial_profile = args
        .profile
        .clone()
        .unwrap_or_else(|| "powershell".to_string());

    let _ = pty_manager.spawn("tab-101".to_string(), initial_profile, "win-1".to_string());

    let window_titles = Arc::new(Mutex::new(HashMap::new()));
    window_titles.lock().unwrap().insert(
        "win-1".to_string(),
        "kterm.exe - A scriptable terminal - win-1".to_string(),
    );

    let window_layouts = Arc::new(Mutex::new(HashMap::new()));
    window_layouts.lock().unwrap().insert(
        "win-1".to_string(),
        vec![pty::LayoutNode::Pane {
            tab_id: "tab-101".to_string(),
        }],
    );

    tauri::Builder::default()
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("win-1") {
                println!("[kterm host] Primary window 'win-1' initialized");
                let _ = window.set_title("kterm.exe - A scriptable terminal - win-1");
                let _ = window.show();
                let _ = window.center();
                let _ = window.set_focus();
            } else if let Some(window) = app.get_webview_window("main") {
                println!("[kterm host] Window 'main' fallback initialized");
                let _ = window.set_title("kterm.exe - A scriptable terminal - win-1");
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
