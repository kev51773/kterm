use super::types::*;
use super::AppState;
use crate::pty::LayoutNode;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use tauri::Manager;

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

pub async fn apply_session(
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
