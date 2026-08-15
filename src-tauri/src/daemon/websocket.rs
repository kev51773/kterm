use super::types::*;
use super::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let win_param = query.get("window").map(|s| s.as_str());
    match state.pty_manager.get_in_window(&id, win_param) {
        Some(sess) => ws.on_upgrade(move |socket| handle_websocket(socket, sess)),
        None => (StatusCode::NOT_FOUND, "Tab not found").into_response(),
    }
}

pub async fn handle_websocket(socket: WebSocket, session: Arc<crate::pty::PtySession>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut rx = session.tx.subscribe();
    let writer = session.writer.clone();
    let session_clone = session.clone();

    // Replay initial history buffer on connect to guarantee startup prompt is never missed
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
