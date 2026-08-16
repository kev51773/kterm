use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tower_http::cors::{Any, CorsLayer};

use crate::daemon::{AppState, TabInfo, WsResizeMsg};

pub static MOBILE_KEYBOARD_JS: &str = include_str!("../../web/mobile-keyboard.js");
pub static INDEX_HTML: &str = include_str!("../../web/index.html");

#[derive(Clone, Serialize, Deserialize)]
pub struct RemoteStatus {
    pub enabled: bool,
    pub port: u16,
    pub address: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginPayload {
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

pub struct RemoteServerManager {
    pub enabled: bool,
    pub port: u16,
    pub password: String,
    pub token: String,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Default for RemoteServerManager {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8080,
            password: String::new(),
            token: String::new(),
            shutdown_tx: None,
        }
    }
}

pub type SharedRemoteManager = Arc<Mutex<RemoteServerManager>>;

fn generate_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{:x}{:x}", nanos, std::process::id())
}

#[tauri::command]
pub fn get_remote_status(manager: tauri::State<'_, SharedRemoteManager>) -> RemoteStatus {
    let mgr = manager.lock().unwrap();
    let addr = if mgr.enabled {
        Some(format!("http://0.0.0.0:{}", mgr.port))
    } else {
        None
    };
    RemoteStatus {
        enabled: mgr.enabled,
        port: mgr.port,
        address: addr,
    }
}

#[tauri::command]
pub async fn enable_remote_server(
    manager: tauri::State<'_, SharedRemoteManager>,
    app_state: tauri::State<'_, AppState>,
    port: u16,
    password: String,
) -> Result<RemoteStatus, String> {
    let mut mgr = manager.lock().unwrap();

    // Stop any running server instance
    if let Some(tx) = mgr.shutdown_tx.take() {
        let _ = tx.send(());
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let token = generate_token();

    mgr.enabled = true;
    mgr.port = port;
    mgr.password = password;
    mgr.token = token.clone();
    mgr.shutdown_tx = Some(shutdown_tx);

    let shared_mgr = manager.inner().clone();
    let app_state_inner = app_state.inner().clone();

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let router = Router::new()
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/mobile-keyboard.js", get(serve_keyboard_js))
        .route("/api/auth/check", get(handle_auth_check))
        .route("/api/login", post(handle_login))
        .route("/api/tabs", get(handle_list_tabs))
        .route("/api/tabs/new", post(handle_create_tab))
        .route("/ws/remote", get(handle_remote_ws))
        .layer(cors)
        .with_state((shared_mgr, app_state_inner));

    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[remote] Failed to bind port {}: {}", port, e);
                return;
            }
        };
        eprintln!("[remote] Listening on http://0.0.0.0:{}", port);
        let _ = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
        eprintln!("[remote] Stopped.");
    });

    Ok(RemoteStatus {
        enabled: true,
        port,
        address: Some(format!("http://0.0.0.0:{}", port)),
    })
}

#[tauri::command]
pub fn disable_remote_server(manager: tauri::State<'_, SharedRemoteManager>) -> RemoteStatus {
    let mut mgr = manager.lock().unwrap();
    if let Some(tx) = mgr.shutdown_tx.take() {
        let _ = tx.send(());
    }
    mgr.enabled = false;
    RemoteStatus {
        enabled: false,
        port: mgr.port,
        address: None,
    }
}

async fn serve_index() -> impl IntoResponse {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
        .header(header::PRAGMA, "no-cache")
        .header(header::EXPIRES, "0")
        .body(INDEX_HTML.to_string())
        .unwrap()
}

async fn serve_keyboard_js() -> impl IntoResponse {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
        .header(header::PRAGMA, "no-cache")
        .header(header::EXPIRES, "0")
        .body(MOBILE_KEYBOARD_JS.to_string())
        .unwrap()
}

async fn handle_auth_check(
    Query(query): Query<HashMap<String, String>>,
    State((shared_mgr, _)): State<(SharedRemoteManager, AppState)>,
) -> impl IntoResponse {
    let mgr = shared_mgr.lock().unwrap();
    let req_token = query.get("token").map(|s| s.as_str()).unwrap_or("");
    if !mgr.token.is_empty() && !req_token.is_empty() && req_token == mgr.token {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    }
}

async fn handle_login(
    State((shared_mgr, _)): State<(SharedRemoteManager, AppState)>,
    axum::Json(payload): axum::Json<LoginPayload>,
) -> Result<axum::Json<LoginResponse>, (StatusCode, &'static str)> {
    let mgr = shared_mgr.lock().unwrap();
    if !mgr.password.is_empty() && payload.password == mgr.password {
        Ok(axum::Json(LoginResponse {
            token: mgr.token.clone(),
        }))
    } else {
        Err((StatusCode::UNAUTHORIZED, "Invalid password"))
    }
}

async fn handle_list_tabs(
    Query(query): Query<HashMap<String, String>>,
    State((shared_mgr, app_state)): State<(SharedRemoteManager, AppState)>,
) -> Result<axum::Json<Vec<TabInfo>>, (StatusCode, String)> {
    let mgr = shared_mgr.lock().unwrap();
    let req_token = query.get("token").map(|s| s.as_str()).unwrap_or("");
    if mgr.token.is_empty() || req_token.is_empty() || req_token != mgr.token {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }

    let sessions = app_state.pty_manager.list_by_window(None);
    let tabs: Vec<TabInfo> = sessions
        .into_iter()
        .map(|s| TabInfo {
            id: s.id.clone(),
            pid: s.pid,
            profile: s.profile.clone(),
            window_id: s.window_id.clone(),
            title: s.title.lock().unwrap().clone(),
            badge: s.badge.lock().unwrap().clone(),
            color: s.color.lock().unwrap().clone(),
            cols: *s.cols.lock().unwrap(),
            rows: *s.rows.lock().unwrap(),
            elevated: s.elevated,
        })
        .collect();

    Ok(axum::Json(tabs))
}

async fn handle_create_tab(
    Query(query): Query<HashMap<String, String>>,
    State((shared_mgr, app_state)): State<(SharedRemoteManager, AppState)>,
) -> Result<axum::Json<TabInfo>, (StatusCode, String)> {
    let mgr = shared_mgr.lock().unwrap();
    let req_token = query.get("token").map(|s| s.as_str()).unwrap_or("");
    if mgr.token.is_empty() || req_token.is_empty() || req_token != mgr.token {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }

    let window_id = "win-1".to_string();
    let profile = "powershell".to_string();
    let tab_id = app_state.pty_manager.generate_next_tab_id_for_window(&window_id);
    let session = app_state
        .pty_manager
        .spawn_with_size_and_cwd(tab_id, profile, window_id, 120, 30, None, false)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let title = session.title.lock().unwrap().clone();
    let badge = session.badge.lock().unwrap().clone();
    let color = session.color.lock().unwrap().clone();

    let cols = *session.cols.lock().unwrap();
    let rows = *session.rows.lock().unwrap();

    Ok(axum::Json(TabInfo {
        id: session.id.clone(),
        pid: session.pid,
        profile: session.profile.clone(),
        window_id: session.window_id.clone(),
        title,
        badge,
        color,
        cols,
        rows,
        elevated: session.elevated,
    }))
}

async fn handle_remote_ws(
    ws: WebSocketUpgrade,
    Query(query): Query<HashMap<String, String>>,
    State((shared_mgr, app_state)): State<(SharedRemoteManager, AppState)>,
) -> impl IntoResponse {
    let mgr = shared_mgr.lock().unwrap();
    let req_token = query.get("token").map(|s| s.as_str()).unwrap_or("");
    if mgr.token.is_empty() || req_token.is_empty() || req_token != mgr.token {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let tab_id = match query.get("tab_id") {
        Some(id) => id.clone(),
        None => return (StatusCode::BAD_REQUEST, "Missing tab_id").into_response(),
    };

    match app_state.pty_manager.get_in_window(&tab_id, None) {
        Some(sess) => ws.on_upgrade(move |socket| handle_websocket_remote(socket, sess)),
        None => (StatusCode::NOT_FOUND, "Tab not found").into_response(),
    }
}

async fn handle_websocket_remote(socket: WebSocket, session: Arc<crate::pty::PtySession>) {
    let orig_cols = *session.cols.lock().unwrap();
    let orig_rows = *session.rows.lock().unwrap();

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut rx = session.tx.subscribe();
    let writer = session.writer.clone();
    let session_clone = session.clone();

    let history = session.get_output_history();
    if !history.is_empty() {
        let text = String::from_utf8_lossy(&history).to_string();
        let _ = ws_sender.send(Message::Text(text)).await;
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(bytes) = rx.recv().await {
            let text = String::from_utf8_lossy(&bytes).to_string();
            if ws_sender.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Text(text) = msg {
                if text.starts_with('{') && text.contains("\"resize\"") {
                    if let Ok(resize_msg) = serde_json::from_str::<WsResizeMsg>(&text) {
                        if resize_msg.r#type == "resize" {
                            let cols = resize_msg.cols.clamp(1, 1000);
                            let rows = resize_msg.rows.clamp(1, 500);
                            session_clone.resize(rows, cols);
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

    // Restore original desktop terminal size on mobile disconnect (Option B)
    session.resize(orig_rows, orig_cols);
}
