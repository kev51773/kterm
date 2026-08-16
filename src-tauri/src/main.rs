mod cli;
mod client;
mod config;
mod daemon;
mod exporter;
mod ipc;
mod named_pipe;
mod pty;
mod remote;
mod yaml;

use clap::Parser;
use cli::CliArgs;
use daemon::AppState;
use pty::PtyManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use daemon::{AppState, TabInfo};
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::time::Duration;

    static PORT_COUNTER: AtomicU16 = AtomicU16::new(19900);

    fn next_port() -> u16 {
        PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    fn make_state() -> AppState {
        AppState {
            pty_manager: PtyManager::new(),
            app_handle: None,
            window_titles: Arc::new(Mutex::new(HashMap::new())),
            window_layouts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn start_daemon() -> u16 {
        let port = next_port();
        let state = make_state();
        tokio::spawn(async move {
            daemon::run_server(&format!("127.0.0.1:{}", port), state).await;
        });
        let c = reqwest::Client::builder()
            .timeout(Duration::from_secs(2)).build().unwrap();
        for _ in 0..50 {
            if c.get(format!("http://127.0.0.1:{}/health", port)).send().await.is_ok() { return port; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("Daemon failed to start on port {}", port);
    }

    fn http(port: u16) -> reqwest::Client {
        let _ = port;
        reqwest::Client::builder().timeout(Duration::from_secs(30)).build().unwrap()
    }

    fn base(port: u16) -> String {
        format!("http://127.0.0.1:{}", port)
    }

    #[tokio::test]
    async fn health_check() {
        let port = start_daemon().await;
        let b = base(port);
        assert!(http(port).get(format!("{}/health", b)).send().await.unwrap().status().is_success());
    }

    #[tokio::test]
    async fn spawn_tab_default() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        assert_eq!(tab.profile, "powershell");
        assert!(!tab.id.is_empty());
    }

    #[tokio::test]
    async fn spawn_tab_cmd() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b))
            .json(&serde_json::json!({ "profile": "cmd" }))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        assert_eq!(tab.profile, "cmd");
    }

    #[tokio::test]
    async fn list_tabs_empty_initially() {
        let port = start_daemon().await;
        let b = base(port);
        let tabs: Vec<TabInfo> = http(port).get(format!("{}/tabs", b))
            .send().await.unwrap().json().await.unwrap();
        assert!(tabs.is_empty());
    }

    #[tokio::test]
    async fn list_tabs_after_spawn() {
        let port = start_daemon().await;
        let b = base(port);
        http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({})).send().await.unwrap();
        http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({ "profile": "cmd" })).send().await.unwrap();
        let tabs: Vec<TabInfo> = http(port).get(format!("{}/tabs", b))
            .send().await.unwrap().json().await.unwrap();
        assert!(tabs.len() >= 2);
    }

    #[tokio::test]
    async fn close_tab_removes() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        http(port).post(format!("{}/tabs/close", b))
            .json(&serde_json::json!({ "targets": [tab.id] }))
            .send().await.unwrap();
        let tabs: Vec<TabInfo> = http(port).get(format!("{}/tabs", b))
            .send().await.unwrap().json().await.unwrap();
        assert!(!tabs.iter().any(|t| t.id == tab.id));
    }

    #[tokio::test]
    async fn set_title() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        http(port).post(format!("{}/tabs/title", b))
            .json(&serde_json::json!({ "targets": [tab.id], "title": "My Server" }))
            .send().await.unwrap();
        let tabs: Vec<TabInfo> = http(port).get(format!("{}/tabs", b))
            .send().await.unwrap().json().await.unwrap();
        assert_eq!(tabs.iter().find(|t| t.id == tab.id).unwrap().title, "My Server");
    }

    #[tokio::test]
    async fn set_badge() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        http(port).post(format!("{}/tabs/badge", b))
            .json(&serde_json::json!({ "targets": [tab.id], "badge": "PROD" }))
            .send().await.unwrap();
        let tabs: Vec<TabInfo> = http(port).get(format!("{}/tabs", b))
            .send().await.unwrap().json().await.unwrap();
        assert_eq!(tabs.iter().find(|t| t.id == tab.id).unwrap().badge.as_deref(), Some("PROD"));
    }

    #[tokio::test]
    async fn set_color() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        http(port).post(format!("{}/tabs/color", b))
            .json(&serde_json::json!({ "targets": [tab.id], "color": "#E53935" }))
            .send().await.unwrap();
        let tabs: Vec<TabInfo> = http(port).get(format!("{}/tabs", b))
            .send().await.unwrap().json().await.unwrap();
        assert_eq!(tabs.iter().find(|t| t.id == tab.id).unwrap().color.as_deref(), Some("#E53935"));
    }

    #[tokio::test]
    async fn send_text() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        let r = http(port).post(format!("{}/tabs/send", b))
            .json(&serde_json::json!({ "targets": [tab.id], "command": "echo hello" }))
            .send().await.unwrap();
        assert!(r.status().is_success());
    }

    #[tokio::test]
    async fn read_output() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        http(port).post(format!("{}/tabs/send", b))
            .json(&serde_json::json!({ "targets": [tab.id], "command": "echo testoutput" }))
            .send().await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        let body: serde_json::Value = http(port)
            .get(format!("{}/tabs/{}/read?tail=10", b, tab.id))
            .send().await.unwrap().json().await.unwrap();
        assert!(!body["lines"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn split_right() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        let body: serde_json::Value = http(port)
            .post(format!("{}/tabs/{}/split", b, tab.id))
            .json(&serde_json::json!({ "direction": "right", "profile": "cmd" }))
            .send().await.unwrap().json().await.unwrap();
        let new_id = body["new_tab_id"].as_str().unwrap();
        assert!(!new_id.is_empty());
        assert_ne!(new_id, &tab.id);
    }

    #[tokio::test]
    async fn split_down() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        let r = http(port).post(format!("{}/tabs/{}/split", b, tab.id))
            .json(&serde_json::json!({ "direction": "down" })).send().await.unwrap();
        assert!(r.status().is_success());
    }

    #[tokio::test]
    async fn unsplit() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        let split: serde_json::Value = http(port)
            .post(format!("{}/tabs/{}/split", b, tab.id))
            .json(&serde_json::json!({ "direction": "right" }))
            .send().await.unwrap().json().await.unwrap();
        let new_id = split["new_tab_id"].as_str().unwrap();
        let r = http(port).post(format!("{}/tabs/{}/unsplit", b, new_id)).send().await.unwrap();
        assert!(r.status().is_success());
    }

    #[tokio::test]
    async fn explode() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        http(port).post(format!("{}/tabs/{}/split", b, tab.id))
            .json(&serde_json::json!({ "direction": "right" })).send().await.unwrap();
        let r = http(port).post(format!("{}/tabs/{}/explode", b, tab.id)).send().await.unwrap();
        assert!(r.status().is_success());
        let body: serde_json::Value = r.json().await.unwrap();
        assert!(body["exploded_tab_ids"].as_array().unwrap().len() >= 2);
    }

    #[tokio::test]
    async fn apply_yaml_dry_run() {
        let port = start_daemon().await;
        let b = base(port);
        let r = http(port).post(format!("{}/apply", b)).json(&serde_json::json!({
            "yaml": "window:\n  id: null\ntabs:\n- profile: powershell\n",
            "dry_run": true
        })).send().await.unwrap();
        assert!(r.status().is_success());
        let body: serde_json::Value = r.json().await.unwrap();
        assert_eq!(body["status"].as_str().unwrap(), "valid");
    }

    #[tokio::test]
    async fn apply_yaml_invalid_profile() {
        let port = start_daemon().await;
        let b = base(port);
        let r = http(port).post(format!("{}/apply", b)).json(&serde_json::json!({
            "yaml": "window:\n  id: null\ntabs:\n- profile: fish\n",
            "dry_run": true
        })).send().await.unwrap();
        assert!(r.status().is_client_error());
    }

    #[tokio::test]
    async fn apply_yaml_creates_tab() {
        let port = start_daemon().await;
        let b = base(port);
        let r = http(port).post(format!("{}/apply", b)).json(&serde_json::json!({
            "yaml": "window:\n  id: null\ntabs:\n- profile: powershell\n"
        })).send().await.unwrap();
        assert!(r.status().is_success());
        let body: serde_json::Value = r.json().await.unwrap();
        assert_eq!(body["status"].as_str().unwrap(), "ok");
    }

    #[tokio::test]
    async fn apply_yaml_with_splits() {
        let port = start_daemon().await;
        let b = base(port);
        let yaml = r#"window:
  id: null
tabs:
- profile: powershell
  splits:
  - direction: right
    profile: cmd
"#;
        let r = http(port).post(format!("{}/apply", b)).json(&serde_json::json!({ "yaml": yaml }))
            .send().await.unwrap();
        assert!(r.status().is_success());
        let body: serde_json::Value = r.json().await.unwrap();
        let win_id = body["window_id"].as_str().unwrap();
        let tabs: Vec<TabInfo> = http(port)
            .get(format!("{}/tabs?window={}", b, win_id))
            .send().await.unwrap().json().await.unwrap();
        assert!(tabs.len() >= 2);
    }

    #[tokio::test]
    async fn config_roundtrip() {
        let port = start_daemon().await;
        let b = base(port);
        let mut cfg: serde_json::Value = http(port).get(format!("{}/config", b))
            .send().await.unwrap().json().await.unwrap();
        let orig = cfg["default_cols"].as_u64().unwrap();
        cfg["default_cols"] = serde_json::json!(80);
        http(port).post(format!("{}/config", b)).json(&cfg).send().await.unwrap();
        let saved: serde_json::Value = http(port).get(format!("{}/config", b))
            .send().await.unwrap().json().await.unwrap();
        assert_eq!(saved["default_cols"].as_u64().unwrap(), 80);
        cfg["default_cols"] = serde_json::json!(orig);
        http(port).post(format!("{}/config", b)).json(&cfg).send().await.unwrap();
    }

    #[tokio::test]
    async fn build_id() {
        let port = start_daemon().await;
        let b = base(port);
        let body: serde_json::Value = http(port).get(format!("{}/build_id", b))
            .send().await.unwrap().json().await.unwrap();
        assert_eq!(body["build_id"].as_str().unwrap(), env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn layout_after_split() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        http(port).post(format!("{}/tabs/{}/split", b, tab.id))
            .json(&serde_json::json!({ "direction": "right" })).send().await.unwrap();
        let body: serde_json::Value = http(port)
            .get(format!("{}/layout?window=win-1", b))
            .send().await.unwrap().json().await.unwrap();
        assert!(body.as_array().unwrap().len() > 0);
        assert_eq!(body[0]["type"].as_str().unwrap(), "split");
    }

    #[tokio::test]
    async fn tab_not_found_404() {
        let port = start_daemon().await;
        let b = base(port);
        let r = http(port).post(format!("{}/tabs/tab-99999/split", b))
            .json(&serde_json::json!({ "direction": "right" })).send().await.unwrap();
        assert!(r.status().is_client_error());
    }

    #[tokio::test]
    async fn resize_tab() {
        let port = start_daemon().await;
        let b = base(port);
        let tab = http(port).post(format!("{}/tabs", b)).json(&serde_json::json!({}))
            .send().await.unwrap().json::<TabInfo>().await.unwrap();
        let r = http(port).post(format!("{}/tabs/{}/resize", b, tab.id))
            .json(&serde_json::json!({ "cols": 100, "rows": 25 })).send().await.unwrap();
        assert!(r.status().is_success());
    }
}

fn main() {
    let args_db: Vec<String> = std::env::args().collect();
    let wv_args = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    let wv_udf = std::env::var("WEBVIEW2_USER_DATA_FOLDER").unwrap_or_default();
    let _ = std::fs::write(
        format!(r"C:\Users\Kev\Desktop\kterm\autotest\tmp\launch-dump-{}.txt", std::process::id()),
        format!("{:?}\nWV_ARGS={}\nWV_UDF={}", args_db, wv_args, wv_udf),
    );

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

    if let Some(pipe_name) = &args.elevated_pty_bridge {
        let profile = args.profile.as_deref().unwrap_or("powershell");
        pty::manager::run_elevated_pty_bridge(pipe_name, profile);
        std::process::exit(0);
    }

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

fn run_host_daemon(args: CliArgs) {
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
    let apply_file = args.apply.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            ipc::list_tabs,
            ipc::create_tab,
            ipc::close_tabs,
            ipc::split_tab,
            ipc::unsplit_tab,
            ipc::set_title,
            ipc::set_badge,
            ipc::set_color,
            ipc::open_target,
            ipc::reveal_target,
            ipc::get_clipboard,
            ipc::update_ratio,
            ipc::get_window_layout,
            ipc::get_config,
            ipc::update_config,
            ipc::close_window,
            ipc::send_pty_input,
            ipc::resize_pty,
            ipc::attach_pty,
            ipc::export_layout,
            ipc::export_shortcut,
            ipc::apply_session,
            remote::get_remote_status,
            remote::enable_remote_server,
            remote::disable_remote_server,
        ])
        .setup(move |app| {
            app.manage(Arc::new(Mutex::new(remote::RemoteServerManager::default())));
            // Pre-flight purge for win-1 on daemon setup
            let sessions = pty_manager.list_by_window(Some("win-1"));
            for s in sessions {
                pty_manager.close(&s.id);
            }
            window_titles.lock().unwrap().remove("win-1");
            window_layouts.lock().unwrap().remove("win-1");

            let cfg = crate::config::AppConfig::load();
            let (default_w, default_h) = cfg.get_default_window_size();

            if app.get_webview_window("win-1").is_none() {
                if let Some(win_cfg) = app.config().app.windows.iter().find(|w| w.label == "win-1") {
                    let mut builder = tauri::WebviewWindowBuilder::from_config(app.handle(), win_cfg)
                        .map_err(|e| e.to_string())?;
                    if let Ok(args) = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") {
                        if !args.is_empty() {
                            builder = builder.additional_browser_args(&args);
                        }
                    }
                    if let Ok(udf) = std::env::var("WEBVIEW2_USER_DATA_FOLDER") {
                        if !udf.is_empty() {
                            builder = builder.data_directory(std::path::PathBuf::from(udf));
                        }
                    }
                    let _ = builder.build();
                }
            }

            if let Some(window) = app.get_webview_window("win-1") {
                let _ = window.set_title("kterm.exe - A scriptable terminal - win-1");
                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: default_w, height: default_h }));
                let _ = window.show();
                let _ = window.center();
                let _ = window.set_focus();
            } else if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("kterm.exe - A scriptable terminal - win-1");
                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: default_w, height: default_h }));
                let _ = window.show();
                let _ = window.center();
                let _ = window.set_focus();
            }

            let app_handle = app.handle().clone();
            let daemon_state = AppState {
                pty_manager,
                app_handle: Some(app_handle),
                window_titles,
                window_layouts,
            };

            let daemon_state_exit = daemon_state.clone();
            daemon_state.pty_manager.set_exit_callback(move |tab_id| {
                daemon::auto_close_tab(&daemon_state_exit, &tab_id);
            });

            app.manage(daemon_state.clone());

            let daemon_state_clone = daemon_state.clone();
            tauri::async_runtime::spawn(async move {
                named_pipe::start_pipe_server(daemon_state_clone).await;
            });

            if let Some(file_path) = apply_file {
                let daemon_state_apply = daemon_state.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
                    let content = std::fs::read_to_string(&file_path);
                    match content {
                        Ok(content) => match serde_yaml::from_str::<crate::yaml::YamlSessionSpec>(&content) {
                        Ok(spec) => {
                            let _ = crate::yaml::apply_yaml_spec(&daemon_state_apply, &spec, Some("win-1"), None, false);
                        }
                            Err(e) => { eprintln!("[kterm] yaml parse err: {}", e); }
                        },
                        Err(e) => { eprintln!("[kterm] read err: {} path={}", e, file_path); }
                    }
                });
            }

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
