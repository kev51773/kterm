use crate::exporter::export_yaml_layout;
use crate::pty::{LayoutNode, PtyManager, SplitDirection};
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

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(2);

#[derive(Clone)]
pub struct AppState {
    pub pty_manager: PtyManager,
    pub app_handle: Option<tauri::AppHandle>,
    pub window_titles: Arc<Mutex<HashMap<String, String>>>,
    pub window_layouts: Arc<Mutex<HashMap<String, Vec<LayoutNode>>>>,
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
    pub cols: u16,
    pub rows: u16,
}

#[derive(Deserialize)]
pub struct CreateTabRequest {
    pub profile: Option<String>,
    pub window: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

#[derive(Deserialize)]
pub struct SendTextRequest {
    pub targets: Vec<String>,
    pub command: String,
    pub window: Option<String>,
}

#[derive(Deserialize)]
pub struct SetTitleRequest {
    pub targets: Vec<String>,
    pub title: String,
    pub window: Option<String>,
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
    pub window: Option<String>,
}

#[derive(Deserialize)]
pub struct SetColorRequest {
    pub targets: Vec<String>,
    pub color: String,
    pub window: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct TargetTabRequest {
    pub targets: Vec<String>,
    pub force: Option<bool>,
    pub window: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ApplySessionRequest {
    pub yaml: Option<String>,
    pub file: Option<String>,
    pub suffix: Option<String>,
    pub suffix_auto: Option<bool>,
    pub dry_run: Option<bool>,
    pub window: Option<String>,
}

#[derive(Deserialize)]
pub struct CloseWindowRequest {
    pub window: String,
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

#[derive(Deserialize)]
pub struct SplitTabRequest {
    pub direction: Option<String>,
    pub profile: Option<String>,
    pub move_tab_id: Option<String>,
}

#[derive(Serialize)]
pub struct SplitTabResponse {
    pub split_id: String,
    pub new_tab_id: String,
    pub target_tab_id: String,
}

#[derive(Deserialize)]
pub struct UpdateRatioRequest {
    pub split_id: String,
    pub ratio: f32,
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
        .route("/windows/title", post(set_window_title))
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
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = addr_str.parse().expect("Invalid daemon address");
    tracing::info!("Starting Axum daemon on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind daemon TCP listener");
    axum::serve(listener, app).await.unwrap();
}

pub fn auto_close_tab(state: &AppState, tab_id: &str) {
    if let Some(sess) = state.pty_manager.get(tab_id) {
        let window_id = sess.window_id.clone();
        state.pty_manager.close(tab_id);

        let mut layouts = state.window_layouts.lock().unwrap();
        if let Some(win_layouts) = layouts.get_mut(&window_id) {
            let mut remove_indices = Vec::new();
            for (idx, node) in win_layouts.iter_mut().enumerate() {
                if matches!(node, LayoutNode::Pane { tab_id: ref tid } if tid == tab_id) {
                    remove_indices.push(idx);
                } else if node.contains_tab(tab_id) {
                    node.remove_tab(tab_id);
                }
            }
            for idx in remove_indices.into_iter().rev() {
                win_layouts.remove(idx);
            }
        }
    }
}


async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn list_tabs(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
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
                cols: *s.cols.lock().unwrap(),
                rows: *s.rows.lock().unwrap(),
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


    let cols = payload.as_ref().and_then(|p| p.cols).unwrap_or(120);
    let rows = payload.as_ref().and_then(|p| p.rows).unwrap_or(30);

    let tab_id = state.pty_manager.generate_next_tab_id_for_window(&window_id);
    let session = state
        .pty_manager
        .spawn_with_size_and_cwd(tab_id.clone(), profile.clone(), window_id.clone(), cols, rows, None)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let title = session.title.lock().unwrap().clone();
    let badge = session.badge.lock().unwrap().clone();
    let color = session.color.lock().unwrap().clone();

    {
        let mut layouts = state.window_layouts.lock().unwrap();
        let win_layouts = layouts.entry(window_id.clone()).or_insert_with(Vec::new);
        win_layouts.push(LayoutNode::Pane {
            tab_id: session.id.clone(),
        });
    }

    let cols = *session.cols.lock().unwrap();
    let rows = *session.rows.lock().unwrap();

    Ok(Json(TabInfo {
        id: session.id.clone(),
        pid: session.pid,
        profile: session.profile.clone(),
        window_id: session.window_id.clone(),
        title,
        badge,
        color,
        cols,
        rows,
    }))
}


async fn send_text(
    State(state): State<AppState>,
    Json(req): Json<SendTextRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state
        .pty_manager
        .resolve_tabs_strict(&req.targets, req.window.as_deref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

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
    let sessions = state
        .pty_manager
        .resolve_tabs_strict(&req.targets, req.window.as_deref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

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

async fn close_window(
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


async fn set_badge(
    State(state): State<AppState>,
    Json(req): Json<SetBadgeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state
        .pty_manager
        .resolve_tabs_strict(&req.targets, req.window.as_deref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    for session in sessions {
        *session.badge.lock().unwrap() = Some(req.badge.clone());
    }

    Ok(StatusCode::OK)
}

async fn set_color(
    State(state): State<AppState>,
    Json(req): Json<SetColorRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state
        .pty_manager
        .resolve_tabs_strict(&req.targets, req.window.as_deref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    for session in sessions {
        *session.color.lock().unwrap() = Some(req.color.clone());
    }

    Ok(StatusCode::OK)
}

async fn close_tabs(
    State(state): State<AppState>,
    Json(req): Json<TargetTabRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let sessions = state
        .pty_manager
        .resolve_tabs_strict(&req.targets, req.window.as_deref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let affected_windows: std::collections::HashSet<String> =
        sessions.iter().map(|s| s.window_id.clone()).collect();

    {
        let mut layouts = state.window_layouts.lock().unwrap();
        for session in &sessions {
            if let Some(win_layouts) = layouts.get_mut(&session.window_id) {
                let mut remove_indices = Vec::new();
                for (idx, node) in win_layouts.iter_mut().enumerate() {
                    if matches!(node, LayoutNode::Pane { tab_id } if tab_id == &session.id) {
                        remove_indices.push(idx);
                    } else if node.contains_tab(&session.id) {
                        node.remove_tab(&session.id);
                    }
                }
                for idx in remove_indices.into_iter().rev() {
                    win_layouts.remove(idx);
                }
            }
        }
    }

    for session in &sessions {
        state.pty_manager.close_in_window(&session.id, Some(&session.window_id));
    }

    // Rule 1: Close window if no tabs remain
    for win_id in affected_windows {
        let remaining = state.pty_manager.list_by_window(Some(&win_id));
        if remaining.is_empty() {
            state.window_titles.lock().unwrap().remove(&win_id);
            state.window_layouts.lock().unwrap().remove(&win_id);
            if let Some(app) = &state.app_handle {
                if let Some(win) = app.get_webview_window(&win_id) {
                    let _ = win.close();
                }
            }
        }
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
    Query(query): Query<std::collections::HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let win_param = query.get("window").map(|s| s.as_str());
    match state.pty_manager.get_in_window(&id, win_param) {
        Some(sess) => ws.on_upgrade(move |socket| handle_websocket(socket, sess)),
        None => (StatusCode::NOT_FOUND, "Tab not found").into_response(),
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

async fn split_tab(
    State(state): State<AppState>,
    Path(target_id): Path<String>,
    payload: Option<Json<SplitTabRequest>>,
) -> Result<Json<SplitTabResponse>, (StatusCode, String)> {
    let req = payload.map(|p| p.0).unwrap_or_else(|| SplitTabRequest {
        direction: None,
        profile: None,
        move_tab_id: None,
    });

    let dir_str = req.direction.as_deref().unwrap_or("right").to_lowercase();
    let (direction, insert_first) = match dir_str.as_str() {
        "up" | "split-up" | "top" => (SplitDirection::Vertical, true),
        "down" | "split-down" | "vertical" | "v" | "bottom" => (SplitDirection::Vertical, false),
        "left" | "split-left" => (SplitDirection::Horizontal, true),
        _ => (SplitDirection::Horizontal, false),
    };

    let resolved = state.pty_manager.resolve_tabs(&[target_id.clone()]);
    let target_session = resolved
        .first()
        .ok_or((StatusCode::NOT_FOUND, "Target tab not found".to_string()))?;
    let target_tab_id = target_session.id.clone();
    let window_id = target_session.window_id.clone();

    let new_tab_id = if let Some(move_id) = req.move_tab_id.filter(|m| !m.trim().is_empty()) {
        let move_sess = state
            .pty_manager
            .get(&move_id)
            .ok_or((StatusCode::NOT_FOUND, "Move tab not found".to_string()))?;

        let mut layouts = state.window_layouts.lock().unwrap();
        let win_layouts = layouts.entry(window_id.clone()).or_insert_with(Vec::new);
        win_layouts.retain(|node| match node {
            LayoutNode::Pane { tab_id } => tab_id != &move_id,
            _ => !node.contains_tab(&move_id),
        });
        move_sess.id.clone()
    } else {
        let profile = req.profile.unwrap_or_else(|| target_session.profile.clone());
        let tab_id = state.pty_manager.generate_next_tab_id_for_window(&window_id);
        let _ = state
            .pty_manager
            .spawn(tab_id.clone(), profile, window_id.clone())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        tab_id
    };

    let mut layouts = state.window_layouts.lock().unwrap();
    let win_layouts = layouts.entry(window_id).or_insert_with(Vec::new);
    let mut split_id = String::new();

    let mut found = false;
    for node in win_layouts.iter_mut() {
        if node.contains_tab(&target_tab_id) {
            node.split_at(&target_tab_id, direction, &new_tab_id, insert_first);
            if let LayoutNode::Split { id, .. } = node {
                split_id = id.clone();
            }
            found = true;
            break;
        }
    }

    if !found {
        let mut root = LayoutNode::Pane {
            tab_id: target_tab_id.clone(),
        };
        root.split_at(&target_tab_id, direction, &new_tab_id, insert_first);
        if let LayoutNode::Split { ref id, .. } = root {
            split_id = id.clone();
        }
        win_layouts.push(root);
    }


    Ok(Json(SplitTabResponse {
        split_id,
        new_tab_id,
        target_tab_id,
    }))
}

async fn unsplit_tab(
    State(state): State<AppState>,
    Path(target_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resolved = state.pty_manager.resolve_tabs(&[target_id.clone()]);
    let target_session = resolved
        .first()
        .ok_or((StatusCode::NOT_FOUND, "Target tab not found".to_string()))?;
    let target_tab_id = target_session.id.clone();
    let window_id = target_session.window_id.clone();

    let mut layouts = state.window_layouts.lock().unwrap();
    if let Some(win_layouts) = layouts.get_mut(&window_id) {
        for node in win_layouts.iter_mut() {
            if node.contains_tab(&target_tab_id) {
                node.unsplit_pane(&target_tab_id);
                break;
            }
        }
        win_layouts.push(LayoutNode::Pane {
            tab_id: target_tab_id.clone(),
        });
    }

    Ok(Json(serde_json::json!({ "unsplit_tab_id": target_tab_id })))
}

async fn explode_tab(
    State(state): State<AppState>,
    Path(target_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resolved = state.pty_manager.resolve_tabs(&[target_id.clone()]);
    let target_session = resolved
        .first()
        .ok_or((StatusCode::NOT_FOUND, "Target tab not found".to_string()))?;
    let target_tab_id = target_session.id.clone();
    let window_id = target_session.window_id.clone();

    let mut exploded = Vec::new();
    let mut layouts = state.window_layouts.lock().unwrap();
    if let Some(win_layouts) = layouts.get_mut(&window_id) {
        if let Some(pos) = win_layouts.iter().position(|n| n.contains_tab(&target_tab_id)) {
            let tree = win_layouts.remove(pos);
            exploded = tree.collect_tabs();
            for tid in &exploded {
                win_layouts.push(LayoutNode::Pane {
                    tab_id: tid.clone(),
                });
            }
        }
    }

    Ok(Json(serde_json::json!({ "exploded_tab_ids": exploded })))
}

async fn update_layout_ratio(
    State(state): State<AppState>,
    Json(req): Json<UpdateRatioRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut layouts = state.window_layouts.lock().unwrap();
    for win_layouts in layouts.values_mut() {
        for node in win_layouts.iter_mut() {
            if node.update_ratio(&req.split_id, req.ratio) {
                return Ok(StatusCode::OK);
            }
        }
    }
    Err((StatusCode::NOT_FOUND, "Split ID not found".to_string()))
}

async fn get_window_layout(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<LayoutNode>> {
    let window_id = params
        .get("window")
        .cloned()
        .unwrap_or_else(|| "win-1".to_string());
    let mut layouts = state.window_layouts.lock().unwrap();
    let win_layouts = layouts.entry(window_id.clone()).or_insert_with(Vec::new);

    let active_tabs = state.pty_manager.list_by_window(Some(&window_id));
    let active_tab_ids: Vec<String> = active_tabs.into_iter().map(|s| s.id.clone()).collect();

    // Prune dead tabs from layout tree
    let mut remove_indices = Vec::new();
    for (idx, node) in win_layouts.iter_mut().enumerate() {
        let node_tabs = node.collect_tabs();
        for dead_id in node_tabs {
            if !active_tab_ids.contains(&dead_id) {
                node.remove_tab(&dead_id);
            }
        }
        if matches!(node, LayoutNode::Pane { ref tab_id } if !active_tab_ids.contains(tab_id)) {
            remove_indices.push(idx);
        }
    }
    for idx in remove_indices.into_iter().rev() {
        win_layouts.remove(idx);
    }

    let mut tracked = Vec::new();
    for node in win_layouts.iter() {
        tracked.extend(node.collect_tabs());
    }
    for tab_id in active_tab_ids {
        if !tracked.contains(&tab_id) {
            win_layouts.push(LayoutNode::Pane { tab_id });
        }
    }

    Json(win_layouts.clone())
}

async fn apply_session(
    State(state): State<AppState>,
    Json(req): Json<ApplySessionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let content = if let Some(y) = req.yaml {
        y
    } else if let Some(f) = req.file {
        std::fs::read_to_string(&f)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read YAML file '{}': {}", f, e)))?
    } else {
        return Err((StatusCode::BAD_REQUEST, "No YAML content or file provided".to_string()));
    };

    let spec = crate::yaml::parse_yaml(&content)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let is_dry_run = req.dry_run.unwrap_or(false);
    let is_suffix_auto = req.suffix_auto.unwrap_or(false);

    let win_override = req.window.as_deref();
    let generated_target_id;
    let target_base_id = if let Some(w) = win_override.filter(|w| !w.trim().is_empty()) {
        w
    } else if let Some(w) = spec.window.id.as_deref().filter(|w| !w.trim().is_empty()) {
        w
    } else {
        generated_target_id = crate::yaml::generate_next_window_id(&state);
        &generated_target_id
    };

    if is_dry_run {
        crate::yaml::validate_yaml(&spec).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        let window_id = crate::yaml::resolve_window_id(&state, target_base_id, req.suffix.as_deref(), is_suffix_auto);
        {
            let titles = state.window_titles.lock().unwrap();
            let layouts = state.window_layouts.lock().unwrap();
            let exists = titles.contains_key(&window_id) || layouts.contains_key(&window_id);
            if exists {
                if !crate::yaml::is_window_untouched_initial(&state, &window_id) {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("Window '{}' already exists.", window_id),
                    ));
                }
            }
        }
        return Ok(Json(serde_json::json!({
            "status": "valid",
            "window_id": window_id
        })));
    }

    let win_id = crate::yaml::apply_yaml_spec(&state, &spec, win_override, req.suffix.as_deref(), is_suffix_auto)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    if let Some(app_handle) = &state.app_handle {
        if let Some(window) = app_handle.get_webview_window(&win_id) {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            let _ = window.eval("if (window.__triggerSyncTabs) window.__triggerSyncTabs();");
        }
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "window_id": win_id
    })))
}

async fn export_layout_endpoint(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let window_id = params
        .get("window")
        .cloned()
        .unwrap_or_else(|| "win-1".to_string());
    let yaml_str = export_yaml_layout(&state, &window_id);
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
        yaml_str,
    )
}

#[derive(serde::Deserialize)]
struct ExportShortcutReq {
    path: String,
}

async fn export_shortcut_endpoint(
    Json(payload): Json<ExportShortcutReq>,
) -> impl IntoResponse {
    match crate::exporter::export_shortcut_for_yaml(&payload.path) {
        Ok(shortcut_path) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "ok", "shortcut": shortcut_path })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(serde::Deserialize)]
struct ReadTabQuery {
    tail: Option<usize>,
    raw: Option<bool>,
}

async fn read_tab_buffer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ReadTabQuery>,
) -> impl IntoResponse {
    let session = match state.pty_manager.get(&id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Tab not found" }))),
    };

    let tail = params.tail.unwrap_or(50);
    let raw = params.raw.unwrap_or(false);

    let lines = session.ring_buffer.read_tail_lines(tail, !raw);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "tab_id": id,
            "tail": tail,
            "lines": lines
        })),
    )
}

#[derive(serde::Deserialize)]
struct WaitTabReq {
    pattern: Option<String>,
    is_prompt: Option<bool>,
    from_history: Option<bool>,
    timeout_sec: Option<u64>,
}

async fn wait_tab_output(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<WaitTabReq>,
) -> impl IntoResponse {
    let session = match state.pty_manager.get(&id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Tab not found" }))),
    };

    let timeout_sec = payload.timeout_sec.unwrap_or(30);
    let is_prompt = payload.is_prompt.unwrap_or(false);
    let pattern = payload.pattern.unwrap_or_default();
    let from_history = payload.from_history.unwrap_or(false);

    let start_offset = if from_history {
        0
    } else {
        session.ring_buffer.get_total_bytes_written()
    };

    let check_matched = |session: &crate::pty::PtySession| -> bool {
        if is_prompt {
            session.ring_buffer.matches_prompt()
        } else if !pattern.is_empty() {
            session.ring_buffer.contains_pattern_from_offset(start_offset, &pattern)
        } else {
            true
        }
    };

    if check_matched(&session) {
        return (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "ok", "matched": true, "elapsed_sec": 0.0 })),
        );
    }

    let mut rx = session.tx.subscribe();
    let start_time = std::time::Instant::now();
    let timeout_duration = std::time::Duration::from_secs(timeout_sec);

    let wait_result = tokio::time::timeout(timeout_duration, async {
        loop {
            match rx.recv().await {
                Ok(_) => {
                    if check_matched(&session) {
                        return Ok(true);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(false),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    if check_matched(&session) {
                        return Ok(true);
                    }
                }
            }
        }
    }).await;

    match wait_result {
        Ok(Ok(true)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "matched": true,
                "elapsed_sec": start_time.elapsed().as_secs_f32()
            })),
        ),
        Ok(Ok(false)) => (
            StatusCode::GONE,
            Json(serde_json::json!({ "error": "PTY output stream closed" })),
        ),
        Ok(Err(())) => unreachable!(),
        Err(_) => (
            StatusCode::REQUEST_TIMEOUT,
            Json(serde_json::json!({
                "error": "Wait timeout elapsed",
                "timeout_sec": timeout_sec
            })),
        ),
    }
}

async fn get_config() -> impl IntoResponse {
    let cfg = crate::config::AppConfig::load();
    (StatusCode::OK, Json(cfg))
}

async fn update_config(
    Json(new_cfg): Json<crate::config::AppConfig>,
) -> impl IntoResponse {
    match new_cfg.save() {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

async fn show_window(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
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

#[derive(Deserialize)]
pub struct ResizeWindowRequest {
    pub window: Option<String>,
    pub width: f64,
    pub height: f64,
}

async fn resize_window(
    State(state): State<AppState>,
    Json(payload): Json<ResizeWindowRequest>,
) -> impl IntoResponse {
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

async fn get_build_id() -> impl IntoResponse {
    let build_id = env!("CARGO_PKG_VERSION");
    (StatusCode::OK, Json(serde_json::json!({ "build_id": build_id })))
}

async fn shutdown_daemon() -> impl IntoResponse {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    (StatusCode::OK, Json(serde_json::json!({ "status": "shutting_down" })))
}

async fn get_clipboard() -> impl IntoResponse {
    #[cfg(windows)]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        extern "system" {
            fn OpenClipboard(hwnd: *mut std::ffi::c_void) -> i32;
            fn CloseClipboard() -> i32;
            fn GetClipboardData(format: u32) -> *mut std::ffi::c_void;
            fn GlobalLock(handle: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn GlobalUnlock(handle: *mut std::ffi::c_void) -> i32;
        }
        const CF_UNICODETEXT: u32 = 13;
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                let handle = GetClipboardData(CF_UNICODETEXT);
                if !handle.is_null() {
                    let ptr = GlobalLock(handle) as *const u16;
                    if !ptr.is_null() {
                        let mut len = 0;
                        while *ptr.add(len) != 0 {
                            len += 1;
                        }
                        let slice = std::slice::from_raw_parts(ptr, len);
                        let text = OsString::from_wide(slice).to_string_lossy().to_string();
                        GlobalUnlock(handle);
                        CloseClipboard();
                        return (StatusCode::OK, Json(serde_json::json!({ "text": text })));
                    }
                }
                CloseClipboard();
            }
        }
    }
    (StatusCode::OK, Json(serde_json::json!({ "text": "" })))
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






