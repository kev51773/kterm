use portable_pty::{Child, MasterPty, PtySize};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use std::io::Write;
use super::platform::is_process_alive;

pub type ExitCallback = Arc<dyn Fn(String) + Send + Sync>;

pub struct PtySession {
    pub id: String,
    pub pid: u32,
    pub profile: String,
    pub window_id: String,
    pub elevated: bool,
    pub title: Arc<Mutex<String>>,
    pub badge: Arc<Mutex<Option<String>>>,
    pub color: Arc<Mutex<Option<String>>>,
    pub cols: Arc<Mutex<u16>>,
    pub rows: Arc<Mutex<u16>>,
    #[allow(dead_code)]
    pub is_dead: Arc<Mutex<bool>>,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
    pub ring_buffer: Arc<crate::pty::ring_buffer::RingBuffer>,
    pub tx: broadcast::Sender<Vec<u8>>,
    pub(crate) _master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    // Wrapped in Option so the waiter thread can take() it (releasing the lock)
    // before calling the blocking .wait(). This prevents close() from deadlocking
    // when it tries to kill() through the same mutex.
    pub(crate) _child: Arc<Mutex<Option<Box<dyn Child + Send>>>>,
}

impl PtySession {
    pub fn resize(&self, rows: u16, cols: u16) {
        if rows > 0 && cols > 0 {
            *self.cols.lock().unwrap() = cols;
            *self.rows.lock().unwrap() = rows;
            let _ = self._master.lock().unwrap().resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn get_output_history(&self) -> Vec<u8> {
        self.output_buffer.lock().unwrap().clone()
    }

    pub(crate) fn kill_by_pid(&self) {
        #[cfg(windows)]
        {
            // Safe: we're only using the PID to open a handle and terminate.
            // PROCESS_TERMINATE = 0x0001
            extern "system" {
                fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
                fn TerminateProcess(handle: *mut std::ffi::c_void, exit_code: u32) -> i32;
                fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
            }
            unsafe {
                let handle = OpenProcess(0x0001, 0, self.pid);
                if !handle.is_null() {
                    TerminateProcess(handle, 1);
                    CloseHandle(handle);
                }
            }
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(self.pid as libc::pid_t, libc::SIGKILL);
        }
    }

    pub fn is_alive(&self) -> bool {
        if *self.is_dead.lock().unwrap() {
            return false;
        }
        if self.pid == 0 {
            return true;
        }
        is_process_alive(self.pid)
    }
}

unsafe impl Sync for PtySession {}
