use super::types::*;
use super::AppState;
use crate::pty::LayoutNode;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use std::io::Write;
use tauri::Manager;

pub async fn list_tabs(
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
                elevated: s.elevated,
            }
        })
        .collect();
    Json(tabs)
}

pub async fn create_tab(
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
    let elevated = payload
        .as_ref()
        .and_then(|p| p.admin.or(p.elevated))
        .unwrap_or(false);

    let cwd = payload
        .as_ref()
        .and_then(|p| p.cwd.clone())
        .filter(|c| !c.trim().is_empty());

    let tab_id = state.pty_manager.generate_next_tab_id_for_window(&window_id);
    let session = state
        .pty_manager
        .spawn_with_size_and_cwd(tab_id.clone(), profile.clone(), window_id.clone(), cols, rows, cwd.as_deref(), elevated)
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
        elevated: session.elevated,
    }))
}

pub async fn send_text(
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

pub async fn set_title(
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

pub async fn set_badge(
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

pub async fn set_color(
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

pub async fn close_tabs(
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

pub async fn resize_tab(
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

pub async fn focus_tabs() -> StatusCode {
    StatusCode::OK
}

pub async fn read_tab_buffer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ReadTabQuery>,
) -> impl axum::response::IntoResponse {
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

pub async fn wait_tab_output(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<WaitTabReq>,
) -> impl axum::response::IntoResponse {
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
