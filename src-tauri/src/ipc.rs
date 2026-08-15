use crate::daemon::{
    ApplySessionRequest, AppState, CreateTabRequest, SetBadgeRequest, SetColorRequest, SetTitleRequest,
    TargetTabRequest, TabInfo,
};
use crate::pty::LayoutNode;
use serde_json::{json, Value};
use std::io::Write;
use tauri::ipc::Channel;
use tauri::Manager;
use tauri::State;

#[tauri::command]
pub fn list_tabs(state: State<'_, AppState>, window: Option<String>) -> Vec<TabInfo> {
    let sessions = state.pty_manager.list_by_window(window.as_deref());
    sessions
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
        .collect()
}

#[tauri::command]
pub async fn create_tab(
    state: State<'_, AppState>,
    payload: Option<CreateTabRequest>,
) -> Result<TabInfo, String> {
    match crate::daemon::create_tab(axum::extract::State((*state).clone()), payload.map(axum::Json)).await {
        Ok(tab) => Ok(tab.0),
        Err((_, err)) => Err(err),
    }
}

#[tauri::command]
pub async fn close_tabs(
    state: State<'_, AppState>,
    targets: Vec<String>,
    window: Option<String>,
) -> Result<(), String> {
    let req = TargetTabRequest {
        targets,
        window,
        force: None,
    };
    match crate::daemon::close_tabs(axum::extract::State((*state).clone()), axum::Json(req)).await {
        Ok(_) => Ok(()),
        Err((_, err)) => Err(err),
    }
}

#[tauri::command]
pub async fn split_tab(
    state: State<'_, AppState>,
    tab_id: String,
    direction: String,
    profile: Option<String>,
) -> Result<Value, String> {
    let req = crate::daemon::SplitTabRequest {
        direction: Some(direction),
        profile,
        move_tab_id: None,
    };
    match crate::daemon::split_tab(
        axum::extract::State((*state).clone()),
        axum::extract::Path(tab_id),
        Some(axum::Json(req)),
    )
    .await
    {
        Ok(res) => Ok(json!(res.0)),
        Err((_, err)) => Err(err),
    }
}

#[tauri::command]
pub async fn unsplit_tab(
    state: State<'_, AppState>,
    tab_id: String,
) -> Result<(), String> {
    match crate::daemon::unsplit_tab(axum::extract::State((*state).clone()), axum::extract::Path(tab_id)).await {
        Ok(_) => Ok(()),
        Err((_, err)) => Err(err),
    }
}

#[tauri::command]
pub async fn set_title(state: State<'_, AppState>, payload: SetTitleRequest) -> Result<(), String> {
    match crate::daemon::set_title(axum::extract::State((*state).clone()), axum::Json(payload)).await {
        Ok(_) => Ok(()),
        Err((_, err)) => Err(err),
    }
}

#[tauri::command]
pub async fn set_badge(state: State<'_, AppState>, payload: SetBadgeRequest) -> Result<(), String> {
    match crate::daemon::set_badge(axum::extract::State((*state).clone()), axum::Json(payload)).await {
        Ok(_) => Ok(()),
        Err((_, err)) => Err(err),
    }
}

#[tauri::command]
pub async fn set_color(state: State<'_, AppState>, payload: SetColorRequest) -> Result<(), String> {
    match crate::daemon::set_color(axum::extract::State((*state).clone()), axum::Json(payload)).await {
        Ok(_) => Ok(()),
        Err((_, err)) => Err(err),
    }
}

#[tauri::command]
pub fn open_target(target: String) -> Result<(), String> {
    let t = target.trim();
    if t.is_empty() {
        return Err("empty target".to_string());
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", t])
            .spawn();
    }
    Ok(())
}

#[tauri::command]
pub fn reveal_target(path: String) -> Result<(), String> {
    let p = path.trim();
    if p.is_empty() {
        return Err("empty path".to_string());
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", p))
            .spawn();
    }
    Ok(())
}

#[tauri::command]
pub fn get_clipboard() -> String {
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
                        return text;
                    }
                }
                CloseClipboard();
            }
        }
    }
    String::new()
}

#[tauri::command]
pub fn update_ratio(
    state: State<'_, AppState>,
    split_id: String,
    ratio: f32,
) -> Result<(), String> {
    let mut layouts = state.window_layouts.lock().unwrap();
    for win_layouts in layouts.values_mut() {
        for node in win_layouts.iter_mut() {
            if node.update_ratio(&split_id, ratio) {
                return Ok(());
            }
        }
    }
    Err("Split ID not found".to_string())
}

#[tauri::command]
pub fn get_window_layout(state: State<'_, AppState>, window: Option<String>) -> Vec<LayoutNode> {
    let win_id = window.unwrap_or_else(|| "win-1".to_string());
    state
        .window_layouts
        .lock()
        .unwrap()
        .get(&win_id)
        .cloned()
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_config() -> crate::config::AppConfig {
    crate::config::AppConfig::load()
}

#[tauri::command]
pub fn update_config(config: crate::config::AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_window(state: State<'_, AppState>, window: Option<String>) -> Result<(), String> {
    let win_id = window.unwrap_or_else(|| "win-1".to_string());
    state.cleanup_window(&win_id);
    if let Some(app) = &state.app_handle {
        if let Some(w) = app.get_webview_window(&win_id) {
            let _ = w.close();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn send_pty_input(
    state: State<'_, AppState>,
    tab_id: String,
    data: String,
) -> Result<(), String> {
    let session = state
        .pty_manager
        .get_in_window(&tab_id, None)
        .ok_or_else(|| "Tab not found".to_string())?;
    let mut guard = session.writer.lock().unwrap();
    guard.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    guard.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn resize_pty(
    state: State<'_, AppState>,
    tab_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let session = state
        .pty_manager
        .get_in_window(&tab_id, None)
        .ok_or_else(|| "Tab not found".to_string())?;
    session.resize(rows.clamp(1, 500), cols.clamp(1, 1000));
    Ok(())
}

#[tauri::command]
pub async fn attach_pty(
    state: State<'_, AppState>,
    tab_id: String,
    on_data_channel: Channel<String>,
) -> Result<(), String> {
    let session = state
        .pty_manager
        .get_in_window(&tab_id, None)
        .ok_or_else(|| "Tab not found".to_string())?;

    let history = session.get_output_history();
    if !history.is_empty() {
        let text = String::from_utf8_lossy(&history).to_string();
        let _ = on_data_channel.send(text);
    }

    let mut rx = session.tx.subscribe();
    tokio::spawn(async move {
        while let Ok(bytes) = rx.recv().await {
            let text = String::from_utf8_lossy(&bytes).to_string();
            if on_data_channel.send(text).is_err() {
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn export_layout(state: State<'_, AppState>, window: Option<String>) -> Result<String, String> {
    let win_id = window.unwrap_or_else(|| "win-1".to_string());
    Ok(crate::exporter::export_yaml_layout(&state, &win_id))
}

#[tauri::command]
pub fn export_shortcut(path: String) -> Result<Value, String> {
    match crate::exporter::export_shortcut_for_yaml(&path) {
        Ok(shortcut_path) => Ok(json!({ "shortcut": shortcut_path })),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn apply_session(
    state: State<'_, AppState>,
    payload: ApplySessionRequest,
) -> Result<Value, String> {
    match crate::daemon::apply_session(axum::extract::State((*state).clone()), axum::Json(payload)).await {
        Ok(res) => Ok(res.0),
        Err((_, err)) => Err(err),
    }
}
