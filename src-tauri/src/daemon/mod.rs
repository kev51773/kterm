use crate::pty::PtyManager;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{Manager, WebviewWindowBuilder};
use tower_http::cors::{Any, CorsLayer};

static TAB_COUNTER: AtomicU32 = AtomicU32::new(102);
static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(2);

#[derive(Clone)]
pub struct AppState {
    pub pty_manager: PtyManager,
    pub app_handle: Option<tauri::AppHandle>,
    pub window_titles: Arc<Mutex<HashMap<String, String>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TabInfo {
    pub id: String,
    pub pid: u32,
    pub profile: String,
    pub window_id: String,
    pub title: String,
    pub badge: Option<String>,
    pub color: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateTabRequest {
    pub profile: Option<String>,
    pub window: Option<String>,
}

#[derive(Deserialize)]
pub struct SendTextRequest {
    pub targets: Vec<String>,
    pub command: String,
}

#[derive(Deserialize)]
pub struct SetTitleRequest {
    pub targets: Vec<String>,
    pub title: String,
}

#[derive(Deserialize)]
pub struct SetWindowTitleRequest {
    pub window: Option<String>,
    pub title: String,
}

#[derive(Deserialize)]
pub struct SetBadgeRequest {
    pub targets: Vec<String>,
    pub badge: String,
}

#[derive(Deserialize)]
pub struct SetColorRequest {
    pub targets: Vec<String>,
    pub color: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct TargetTabRequest {
    pub targets: Vec<String>,
    pub force: Option<bool>,
}

#[derive(Deserialize)]
pub struct ResizeRequest {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Serialize)]
pub struct WindowInfo {
    pub id: String,
    pub label: String,
    pub title: String,
}

pub async fn run_server(addr_str: &str, state: AppState) {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/tabs", get(list_tabs).post(create_tab))
        .route("/tabs/send", post(send_text))
        .route("/tabs/title", post(set_title))
        .route("/tabs/badge", post(set_badge))
        .route("/tabs/color", post(set_color))
        .route("/tabs/close", post(close_tabs))
        .route("/tabs/focus", post(focus_tabs))
        .route("/tabs/:id/resize", post(resize_tab))
        .route("/windows", get(list_windows).post(create_window))
        .route("/windows/title", post(set_window_title))
        .route("/tabs/:id/ws", get(ws_handler))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = addr_str.parse().expect("Invalid daemon address");
    tracing::info!("Starting Axum daemon on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind daemon TCP listener");
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn list_tabs(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<TabInfo>> {
    let window_filter = params.get("window").map(|s| s.as_str());
    let sessions = state.pty_manager.list_by_window(window_filter);
    let tabs = sessions
        .into_iter()
        .map(|s| {
            let title = s.title.lock().unwrap().clone();
            let badge = s.badge.lock().unwrap().clone();
            let color = s.color.lock().unwrap().clone();
            TabInfo {
                id: s.id.clone(),
                pid: s.pid,
                profile: s.profile.clone(),
                window_id: s.window_id.clone(),
                title,
                badge,
                color,
            }
        })
        .collect();
    Json(tabs)
}

async fn create_tab(
    State(state): State<AppState>,
    payload: Option<Json<CreateTabRequest>>,
) -> Result<Json<TabInfo>, (StatusCode, String)> {
    let profile = payload
        .as_ref()
        .and_then(|p| p.profile.clone())
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| "powershell".to_string());

    let window_id = payload
        .as_ref()
        .and_then(|p| p.window.clone())
        .filter(|w| !w.trim().is_empty())
        .unwrap_or_else(|| "win-1".to_string());

    let tab_id = format!("tab-{}", TAB_COUNTER.fetch_add(1, Ordering::SeqCst));
    let session = state
        .pty_manager
        .spawn(tab_id.clone(), profile.clone(), window_id.clone())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let title = session.title.lock().unwrap().clone();
    let badge = session.badge.lock().unwrap().clone();
    let color = session.color.lock().unwrap().clone();

    Ok(Json(TabInfo {
        id: session.id.clone(),
        pid: session.pid,
        profile: session.profile.clone(),
        window_id: session.window_id.clone(),
        title,
        badge,
        color,
    }))
}

async fn send_text(
    State(state): State<AppState>,
    Json(req): Json<SendTextRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state.pty_manager.resolve_tabs(&req.targets);
    if sessions.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No matching tabs found".to_string()));
    }

    let mut text_to_send = req.command;
    if !text_to_send.ends_with('\r') && !text_to_send.ends_with('\n') {
        text_to_send.push('\r');
    }

    for session in sessions {
        let mut writer = session.writer.lock().unwrap();
        let _ = writer.write_all(text_to_send.as_bytes());
        let _ = writer.flush();
    }

    Ok(StatusCode::OK)
}

async fn set_title(
    State(state): State<AppState>,
    Json(req): Json<SetTitleRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state.pty_manager.resolve_tabs(&req.targets);
    if sessions.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No matching tabs found".to_string()));
    }

    for session in sessions {
        *session.title.lock().unwrap() = req.title.clone();
    }

    Ok(StatusCode::OK)
}

async fn set_window_title(
    State(state): State<AppState>,
    Json(req): Json<SetWindowTitleRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let target_win = req
        .window
        .clone()
        .filter(|w| !w.trim().is_empty())
        .unwrap_or_else(|| "win-1".to_string());

    let formatted_title = format!("kterm.exe - {} - {}", req.title, target_win);

    state
        .window_titles
        .lock()
        .unwrap()
        .insert(target_win.clone(), formatted_title.clone());

    if let Some(app) = &state.app_handle {
        if let Some(win) = app.get_webview_window(&target_win) {
            let _ = win.set_title(&formatted_title);
        }
    }

    Ok(StatusCode::OK)
}

async fn set_badge(
    State(state): State<AppState>,
    Json(req): Json<SetBadgeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state.pty_manager.resolve_tabs(&req.targets);
    if sessions.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No matching tabs found".to_string()));
    }

    for session in sessions {
        *session.badge.lock().unwrap() = Some(req.badge.clone());
    }

    Ok(StatusCode::OK)
}

async fn set_color(
    State(state): State<AppState>,
    Json(req): Json<SetColorRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state.pty_manager.resolve_tabs(&req.targets);
    if sessions.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No matching tabs found".to_string()));
    }

    for session in sessions {
        *session.color.lock().unwrap() = Some(req.color.clone());
    }

    Ok(StatusCode::OK)
}

async fn close_tabs(
    State(state): State<AppState>,
    Json(req): Json<TargetTabRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state.pty_manager.resolve_tabs(&req.targets);
    if sessions.is_empty() {
        return Err((StatusCode::NOT_FOUND, "No matching tabs found".to_string()));
    }

    for session in sessions {
        state.pty_manager.close(&session.id);
    }

    Ok(StatusCode::OK)
}

async fn resize_tab(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ResizeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if let Some(session) = state.pty_manager.get(&id) {
        session.resize(req.rows, req.cols);
        Ok(StatusCode::OK)
    } else {
        Err((StatusCode::NOT_FOUND, "Tab not found".to_string()))
    }
}

async fn focus_tabs() -> StatusCode {
    StatusCode::OK
}

async fn list_windows(State(state): State<AppState>) -> Json<Vec<WindowInfo>> {
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

async fn create_window(
    State(state): State<AppState>,
) -> Result<Json<WindowInfo>, (StatusCode, String)> {
    if let Some(app) = &state.app_handle {
        let win_num = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
        let label = format!("win-{}", win_num);

        let window_title = format!("kterm.exe - A scriptable terminal - {}", label);

        let builder = WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::App(format!("index.html?window={}", label).into()),
        )
        .title(&window_title)
        .inner_size(900.0, 600.0);

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

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let session = state.pty_manager.get(&id);
    match session {
        Some(sess) => ws.on_upgrade(move |socket| handle_websocket(socket, sess)),
        None => {
            let profile = "powershell".to_string();
            match state.pty_manager.spawn(id.clone(), profile, "win-1".to_string()) {
                Ok(sess) => ws.on_upgrade(move |socket| handle_websocket(socket, sess)),
                Err(_) => (StatusCode::NOT_FOUND, "Tab missing and spawn failed").into_response(),
            }
        }
    }
}

#[derive(Deserialize)]
struct WsResizeMsg {
    r#type: String,
    cols: u16,
    rows: u16,
}

async fn handle_websocket(socket: WebSocket, session: Arc<crate::pty::PtySession>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut rx = session.tx.subscribe();
    let writer = session.writer.clone();
    let session_clone = session.clone();

    // Replay initial history buffer on connect to guarantee startup prompt is never missed
    let history = session.get_output_history();
    if !history.is_empty() {
        if let Ok(text) = String::from_utf8(history) {
            let _ = ws_sender.send(Message::Text(text)).await;
        }
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(bytes) = rx.recv().await {
            if let Ok(text) = String::from_utf8(bytes) {
                if ws_sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Text(text) = msg {
                if text.starts_with('{') && text.contains("\"resize\"") {
                    if let Ok(resize_msg) = serde_json::from_str::<WsResizeMsg>(&text) {
                        if resize_msg.r#type == "resize" {
                            session_clone.resize(resize_msg.rows, resize_msg.cols);
                            continue;
                        }
                    }
                }
                let mut guard = writer.lock().unwrap();
                let _ = guard.write_all(text.as_bytes());
                let _ = guard.flush();
            } else if let Message::Binary(data) = msg {
                let mut guard = writer.lock().unwrap();
                let _ = guard.write_all(&data);
                let _ = guard.flush();
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
