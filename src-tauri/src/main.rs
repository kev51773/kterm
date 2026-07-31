// Prevents additional console window on Windows in release, do not remove!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod daemon;
mod pty;

use daemon::AppState;
use pty::PtyManager;
use tauri::Manager;

fn main() {
    tracing_subscriber::fmt::init();

    let pty_manager = PtyManager::new();
    // Pre-spawn default initial session (tab-101)
    let _ = pty_manager.spawn("tab-101".to_string(), "powershell".to_string());

    let app_state = AppState { pty_manager };

    tauri::Builder::default()
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                println!("[kterm] Found window 'main', unhiding, centering, and setting always_on_top");
                let _ = window.show();
                let _ = window.center();
                let _ = window.set_focus();
                let _ = window.set_always_on_top(true);
            } else {
                eprintln!("[kterm ERROR] Window 'main' NOT found in setup!");
            }

            let daemon_state = app_state.clone();
            tauri::async_runtime::spawn(async move {
                daemon::run_server("127.0.0.1:9999", daemon_state).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
