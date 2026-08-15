use super::types::*;
use super::AppState;
use crate::pty::{LayoutNode, SplitDirection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;

pub async fn split_tab(
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
            .spawn_with_cwd(tab_id.clone(), profile, window_id.clone(), None, target_session.elevated)
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

pub async fn unsplit_tab(
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

pub async fn explode_tab(
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

pub async fn update_layout_ratio(
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

pub async fn get_window_layout(
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
