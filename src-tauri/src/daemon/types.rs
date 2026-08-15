use serde::{Deserialize, Serialize};

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
    pub elevated: bool,
}

#[derive(Deserialize)]
pub struct CreateTabRequest {
    pub profile: Option<String>,
    pub window: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub admin: Option<bool>,
    pub elevated: Option<bool>,
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

#[derive(Deserialize)]
pub struct ResizeWindowRequest {
    pub window: Option<String>,
    pub width: f64,
    pub height: f64,
}

#[derive(Deserialize)]
pub struct ExportShortcutReq {
    pub path: String,
}

#[derive(Deserialize)]
pub struct ReadTabQuery {
    pub tail: Option<usize>,
    pub raw: Option<bool>,
}

#[derive(Deserialize)]
pub struct WaitTabReq {
    pub pattern: Option<String>,
    pub is_prompt: Option<bool>,
    pub from_history: Option<bool>,
    pub timeout_sec: Option<u64>,
}

#[derive(Deserialize)]
pub struct WsResizeMsg {
    pub r#type: String,
    pub cols: u16,
    pub rows: u16,
}
