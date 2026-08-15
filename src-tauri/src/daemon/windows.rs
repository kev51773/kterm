use super::types::*;
use super::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{Manager, WebviewWindowBuilder};

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(2);

pub async fn list_windows(State(state): State<AppState>) -> Json<Vec<WindowInfo>> {
    let mut list = Vec::new();
    let titles_map = state.window_titles.lock().unwrap();

    if let Some(app) = &state.app_handle {
        let windows = app.webview_windows();
        for (label, _) in windows {
            let default_t = format!("kterm.exe - A scriptable terminal - {}", label);
            let t = titles_map.get(&label).cloned().unwrap_or(default_t);
            list.push(WindowInfo {
                id: label.clone(),
                label: label.clone(),
                title: t,
            });
        }
    } else {
        list.push(WindowInfo {
            id: "win-1".to_string(),
            label: "win-1".to_string(),
            title: "kterm.exe - A scriptable terminal - win-1".to_string(),
        });
    }

    Json(list)
}

pub async fn create_window(
    State(state): State<AppState>,
) -> Result<Json<WindowInfo>, (StatusCode, String)> {
    if let Some(app) = &state.app_handle {
        let win_num = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
        let label = format!("win-{}", win_num);
        state.cleanup_window(&label);

        let window_title = format!("kterm.exe - A scriptable terminal - {}", label);

        let (default_w, default_h) = crate::config::AppConfig::load().get_default_window_size();
        let builder = WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::App(format!("index.html?window={}", label).into()),
        )
        .title(&window_title)
        .inner_size(default_w, default_h);

        let builder = if let Ok(args) = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") {
            if !args.is_empty() {
                builder.additional_browser_args(&args)
            } else {
                builder
            }
        } else {
            builder
        };

        let window = builder
            .build()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let _ = window.show();
        let _ = window.set_focus();

        state
            .window_titles
            .lock()
            .unwrap()
            .insert(label.clone(), window_title.clone());

        Ok(Json(WindowInfo {
            id: label.clone(),
            label: label.clone(),
            title: window_title,
        }))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            "App handle not available".to_string(),
        ))
    }
}

pub async fn close_window(
    State(state): State<AppState>,
    Json(req): Json<CloseWindowRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let window_id = req.window;
    state.cleanup_window(&window_id);

    if let Some(app) = &state.app_handle {
        if let Some(win) = app.get_webview_window(&window_id) {
            let _ = win.close();
        }
    }
    Ok(StatusCode::OK)
}

pub async fn show_window(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    let win_id = payload
        .get("window")
        .and_then(|w| w.as_str())
        .unwrap_or("win-1");

    if let Some(app_handle) = &state.app_handle {
        if let Some(window) = app_handle.get_webview_window(win_id) {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            return (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })));
        }
    }
    (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Window not found" })))
}

pub async fn resize_window(
    State(state): State<AppState>,
    Json(payload): Json<ResizeWindowRequest>,
) -> impl axum::response::IntoResponse {
    let win_id = payload.window.as_deref().unwrap_or("win-1");
    if let Some(app_handle) = &state.app_handle {
        if let Some(window) = app_handle.get_webview_window(win_id) {
            let _ = window.show();
            let size = tauri::Size::Logical(tauri::LogicalSize {
                width: payload.width,
                height: payload.height,
            });
            let _ = window.set_size(size);
            return (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })));
        }
    }
    (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Window not found" })))
}
