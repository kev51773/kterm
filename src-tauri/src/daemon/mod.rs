pub mod config;
pub mod export;
pub mod session;
pub mod splits;
pub mod system;
pub mod tabs;
pub mod types;
pub mod websocket;
pub mod windows;

pub use config::*;
pub use export::*;
pub use session::*;
pub use splits::*;
pub use system::*;
pub use tabs::*;
#[allow(unused_imports)]
pub use types::*;
pub use websocket::*;
pub use windows::*;

use crate::pty::{LayoutNode, PtyManager};
use axum::{
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub pty_manager: PtyManager,
    pub app_handle: Option<tauri::AppHandle>,
    pub window_titles: Arc<Mutex<HashMap<String, String>>>,
    pub window_layouts: Arc<Mutex<HashMap<String, Vec<LayoutNode>>>>,
}

impl AppState {
    pub fn cleanup_window(&self, window_id: &str) {
        self.pty_manager.close_all_for_window(window_id);
        self.window_titles.lock().unwrap().remove(window_id);
        self.window_layouts.lock().unwrap().remove(window_id);
    }

    pub fn cleanup_all_orphaned(&self) {
        self.pty_manager.prune_dead_sessions();
        if let Some(app) = &self.app_handle {
            let active_windows: std::collections::HashSet<String> = app
                .webview_windows()
                .keys()
                .cloned()
                .collect();

            let all_pty_sessions = self.pty_manager.list_by_window(None);
            let pty_windows: std::collections::HashSet<String> =
                all_pty_sessions.into_iter().map(|s| s.window_id.clone()).collect();

            for win_id in pty_windows {
                if !active_windows.contains(&win_id) {
                    self.cleanup_window(&win_id);
                }
            }
        }
    }
}

pub async fn run_server(addr_str: &str, state: AppState) {
    let state_clone = state.clone();
    state.pty_manager.set_exit_callback(move |tab_id| {
        auto_close_tab(&state_clone, &tab_id);
    });

    state.cleanup_all_orphaned();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/layout", get(get_window_layout))
        .route("/layout/ratio", post(update_layout_ratio))
        .route("/tabs", get(list_tabs).post(create_tab))
        .route("/tabs/send", post(send_text))
        .route("/tabs/title", post(set_title))
        .route("/tabs/badge", post(set_badge))
        .route("/tabs/color", post(set_color))
        .route("/tabs/close", post(close_tabs))
        .route("/tabs/focus", post(focus_tabs))
        .route("/tabs/:id/resize", post(resize_tab))
        .route("/tabs/:id/split", post(split_tab))
        .route("/tabs/:id/unsplit", post(unsplit_tab))
        .route("/tabs/:id/explode", post(explode_tab))
        .route("/tabs/:id/read", get(read_tab_buffer))
        .route("/tabs/:id/wait", post(wait_tab_output))
        .route("/windows", get(list_windows).post(create_window))
        .route("/windows/close", post(close_window))
        .route("/windows/show", post(show_window))
        .route("/windows/size", post(resize_window))
        .route("/tabs/:id/ws", get(ws_handler))
        .route("/apply", post(apply_session))
        .route("/export-layout", get(export_layout_endpoint))
        .route("/export", get(export_layout_endpoint))
        .route("/export-shortcut", post(export_shortcut_endpoint))
        .route("/create-shortcut", post(export_shortcut_endpoint))
        .route("/config", get(get_config).post(update_config))
        .route("/build_id", get(get_build_id))
        .route("/shutdown", post(shutdown_daemon))
        .route("/clipboard", get(get_clipboard))
        .route("/system/open", post(open_target))
        .route("/system/reveal", post(reveal_target))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = addr_str.parse().expect("Invalid daemon address");
    tracing::info!("Starting Axum daemon on {}", addr);

    let mut listener = None;
    for _ in 0..15 {
        match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }
    }
    let listener = listener.expect("Failed to bind daemon TCP listener");
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use crate::pty::layout::{LayoutNode, SplitDirection};

    #[test]
    fn test_split_node_tree() {
        let mut root = LayoutNode::Pane {
            tab_id: "tab-1".to_string(),
        };

        // Split right (Horizontal, false)
        assert!(root.split_at("tab-1", SplitDirection::Horizontal, "tab-2", false));
        assert!(root.contains_tab("tab-1"));
        assert!(root.contains_tab("tab-2"));

        // Split down on tab-2 (Vertical, false)
        assert!(root.split_at("tab-2", SplitDirection::Vertical, "tab-3", false));
        assert!(root.contains_tab("tab-3"));
    }
}
