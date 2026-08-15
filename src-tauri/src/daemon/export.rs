use super::types::*;
use super::AppState;
use crate::exporter::export_yaml_layout;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::collections::HashMap;

pub async fn export_layout_endpoint(
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

pub async fn export_shortcut_endpoint(
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
