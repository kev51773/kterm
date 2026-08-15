use axum::{http::StatusCode, response::IntoResponse, Json};

pub async fn get_config() -> impl IntoResponse {
    let cfg = crate::config::AppConfig::load();
    (StatusCode::OK, Json(cfg))
}

pub async fn update_config(
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
