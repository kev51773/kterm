//! Test-mode window placement: pin every created window to a 100% (96 DPI)
//! monitor so screenshots come out at physical == logical pixels regardless
//! of the machine's display scaling. Without it, WebView2 screenshots are
//! physical pixels, so a 150% monitor yields a 1.5x image that pixel-compares
//! as a fake 100% diff against a 100%-DPI baseline.
//!
//! Gated by KTERM_TEST_FORCE_100DPI=1 (set by the autotest harness). When the
//! flag is on and no 100% monitor exists, window creation fails with a clear
//! error instead of silently producing flaky screenshots.

use std::sync::Mutex;
use windows_sys::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

struct MonitorInfo {
    work: RECT,
    dpi: u32,
}

static MONITORS: Mutex<Vec<MonitorInfo>> = Mutex::new(Vec::new());

unsafe extern "system" fn enum_monitor(
    hm: HMONITOR,
    _hdc: HDC,
    _lprc: *mut RECT,
    _lparam: LPARAM,
) -> BOOL {
    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let mut dpi_x: u32 = 96;
    let mut dpi_y: u32 = 96;
    if GetMonitorInfoW(hm, &mut mi) != 0 {
        GetDpiForMonitor(hm, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        if let Ok(mut monitors) = MONITORS.lock() {
            monitors.push(MonitorInfo {
                work: mi.rcWork,
                dpi: dpi_x,
            });
        }
    }
    1
}

/// In test mode, return the top-left logical position that centers a `w`×`h`
/// window on the first 100%-scale monitor's work area. Off mode: `Ok(None)`
/// (normal placement, no behavior change). Test mode with no 100% monitor:
/// `Err` — the caller fails window creation.
pub fn placement_for_window(w: f64, h: f64) -> Result<Option<(f64, f64)>, String> {
    if std::env::var("KTERM_TEST_FORCE_100DPI").as_deref() != Ok("1") {
        return Ok(None);
    }
    let mut monitors = MONITORS
        .lock()
        .map_err(|_| "display monitor enumeration lock poisoned")?;
    monitors.clear();
    let ok = unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_monitor),
            0,
        )
    };
    if ok == 0 {
        return Err("failed to enumerate monitors".to_string());
    }
    match monitors.iter().find(|m| m.dpi == 96) {
        Some(m) => {
            let x = m.work.left as f64 + ((m.work.right - m.work.left) as f64 - w) / 2.0;
            let y = m.work.top as f64 + ((m.work.bottom - m.work.top) as f64 - h) / 2.0;
            Ok(Some((x.max(0.0), y.max(0.0))))
        }
        None => Err(
            "KTERM_TEST_FORCE_100DPI: no 100% (96 DPI) monitor available — connect one or remove the flag"
                .to_string(),
        ),
    }
}
