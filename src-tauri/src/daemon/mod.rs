use crate::pty::PtyManager;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

static TAB_COUNTER: AtomicU32 = AtomicU32::new(102);

#[derive(Clone)]
pub struct AppState {
    pub pty_manager: PtyManager,
}

#[derive(Serialize)]
pub struct TabInfo {
    pub id: String,
    pub pid: u32,
    pub profile: String,
}

#[derive(Deserialize)]
pub struct CreateTabRequest {
    pub profile: Option<String>,
}

#[derive(Deserialize)]
pub struct SendTextRequest {
    pub command: String,
}

pub async fn run_server(addr_str: &str, state: AppState) {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/tabs", get(list_tabs).post(create_tab))
        .route("/tabs/:id/send", post(send_text))
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

async fn list_tabs(State(state): State<AppState>) -> Json<Vec<TabInfo>> {
    let sessions = state.pty_manager.list();
    let tabs = sessions
        .into_iter()
        .map(|s| TabInfo {
            id: s.id.clone(),
            pid: s.pid,
            profile: s.profile.clone(),
        })
        .collect();
    Json(tabs)
}

async fn create_tab(
    State(state): State<AppState>,
    payload: Option<Json<CreateTabRequest>>,
) -> Result<Json<TabInfo>, (StatusCode, String)> {
    let profile = payload
        .and_then(|p| p.profile.clone())
        .unwrap_or_else(|| "powershell".to_string());

    let tab_id = format!("tab-{}", TAB_COUNTER.fetch_add(1, Ordering::SeqCst));
    let session = state
        .pty_manager
        .spawn(tab_id.clone(), profile.clone())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(TabInfo {
        id: session.id.clone(),
        pid: session.pid,
        profile: session.profile.clone(),
    }))
}

async fn send_text(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SendTextRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let session = state
        .pty_manager
        .get(&id)
        .ok_or((StatusCode::NOT_FOUND, "Tab not found".to_string()))?;

    let mut writer = session.writer.lock().unwrap();
    writer
        .write_all(req.command.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    writer
        .flush()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
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
            // Auto-spawn session if requested id does not exist yet
            let profile = "powershell".to_string();
            match state.pty_manager.spawn(id.clone(), profile) {
                Ok(sess) => ws.on_upgrade(move |socket| handle_websocket(socket, sess)),
                Err(_) => (StatusCode::NOT_FOUND, "Tab missing and spawn failed").into_response(),
            }
        }
    }
}

async fn handle_websocket(socket: WebSocket, session: Arc<crate::pty::PtySession>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut rx = session.tx.subscribe();

    let writer = session.writer.clone();

    // Task to forward PTY output to WS
    let mut send_task = tokio::spawn(async move {
        while let Ok(bytes) = rx.recv().await {
            if let Ok(text) = String::from_utf8(bytes) {
                if ws_sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task to forward WS input to PTY writer
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Text(text) = msg {
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
