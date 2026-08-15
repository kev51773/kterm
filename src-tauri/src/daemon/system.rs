use axum::{http::StatusCode, response::IntoResponse, Json};

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub async fn get_build_id() -> impl IntoResponse {
    let build_id = env!("CARGO_PKG_VERSION");
    (StatusCode::OK, Json(serde_json::json!({ "build_id": build_id })))
}

pub async fn shutdown_daemon() -> impl IntoResponse {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    (StatusCode::OK, Json(serde_json::json!({ "status": "shutting_down" })))
}

pub async fn get_clipboard() -> impl IntoResponse {
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
