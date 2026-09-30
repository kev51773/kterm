#[cfg(windows)]
pub fn is_app_elevated() -> bool {
    use std::ptr;
    extern "system" {
        fn OpenProcessToken(h: *mut std::ffi::c_void, access: u32, token: *mut *mut std::ffi::c_void) -> i32;
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetTokenInformation(token: *mut std::ffi::c_void, class: u32, info: *mut std::ffi::c_void, len: u32, return_len: *mut u32) -> i32;
        fn CloseHandle(h: *mut std::ffi::c_void) -> i32;
    }
    unsafe {
        let mut token: *mut std::ffi::c_void = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut token) != 0 {
            let mut elevation: u32 = 0;
            let mut size: u32 = 0;
            let res = GetTokenInformation(token, 20, &mut elevation as *mut u32 as *mut std::ffi::c_void, std::mem::size_of::<u32>() as u32, &mut size);
            CloseHandle(token);
            return res != 0 && elevation != 0;
        }
    }
    false
}

#[cfg(unix)]
pub fn is_app_elevated() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(any(windows, unix)))]
pub fn is_app_elevated() -> bool {
    false
}

pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(windows)]
    {
        extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
            fn GetExitCodeProcess(handle: *mut std::ffi::c_void, exit_code: *mut u32) -> i32;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }
        unsafe {
            let handle = OpenProcess(0x1000, 0, pid);
            if handle.is_null() {
                return false;
            }
            let mut exit_code: u32 = 0;
            let res = GetExitCodeProcess(handle, &mut exit_code);
            CloseHandle(handle);
            res != 0 && exit_code == 259
        }
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as libc::pid_t, 0) == 0
    }
}

#[cfg(windows)]
pub fn create_win32_named_pipe_handle(pipe_name: &str) -> Result<*mut std::ffi::c_void, String> {
    use std::os::windows::ffi::OsStrExt;

    #[allow(non_snake_case)]
    #[repr(C)]
    struct SECURITY_ATTRIBUTES {
        nLength: u32,
        lpSecurityDescriptor: *mut std::ffi::c_void,
        bInheritHandle: i32,
    }

    extern "system" {
        fn CreateNamedPipeW(
            name: *const u16,
            open_mode: u32,
            pipe_mode: u32,
            max_instances: u32,
            out_buffer_size: u32,
            in_buffer_size: u32,
            default_timeout: u32,
            security_attributes: *const SECURITY_ATTRIBUTES,
        ) -> *mut std::ffi::c_void;

        fn InitializeSecurityDescriptor(
            pSecurityDescriptor: *mut std::ffi::c_void,
            dwRevision: u32,
        ) -> i32;

        fn SetSecurityDescriptorDacl(
            pSecurityDescriptor: *mut std::ffi::c_void,
            bDaclPresent: i32,
            pDacl: *mut std::ffi::c_void,
            bDaclDefaulted: i32,
        ) -> i32;
    }

    let mut sd = [0u8; 40];
    unsafe {
        InitializeSecurityDescriptor(sd.as_mut_ptr() as *mut std::ffi::c_void, 1);
        SetSecurityDescriptorDacl(sd.as_mut_ptr() as *mut std::ffi::c_void, 1, std::ptr::null_mut(), 0);
    }

    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.as_mut_ptr() as *mut std::ffi::c_void,
        bInheritHandle: 0,
    };

    let full_path = format!("\\\\.\\pipe\\{}", pipe_name);
    let path_w: Vec<u16> = std::ffi::OsStr::new(&full_path).encode_wide().chain(std::iter::once(0)).collect();

    let handle = unsafe {
        CreateNamedPipeW(
            path_w.as_ptr(),
            3, // PIPE_ACCESS_DUPLEX
            0,
            255,
            65536,
            65536,
            0,
            &sa as *const SECURITY_ATTRIBUTES,
        )
    };

    let invalid_handle = -1isize as *mut std::ffi::c_void;
    if handle == invalid_handle {
        return Err("Failed to create named pipe server".to_string());
    }

    Ok(handle)
}

#[cfg(windows)]
pub fn connect_win32_named_pipe(handle: *mut std::ffi::c_void) -> Result<std::fs::File, String> {
    use std::os::windows::io::FromRawHandle;
    use std::sync::mpsc;
    use std::time::Duration;

    extern "system" {
        fn ConnectNamedPipe(handle: *mut std::ffi::c_void, overlapped: *mut std::ffi::c_void) -> i32;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        fn GetLastError() -> u32;
    }

    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let handle_val = handle as usize;

    std::thread::spawn(move || {
        let h = handle_val as *mut std::ffi::c_void;
        let connected = unsafe { ConnectNamedPipe(h, std::ptr::null_mut()) };
        if connected == 0 {
            let err = unsafe { GetLastError() };
            if err != 535 && err != 183 {
                let _ = tx.send(Err(format!("ConnectNamedPipe failed with error {}", err)));
                return;
            }
        }
        let _ = tx.send(Ok(()));
    });

    match rx.recv_timeout(Duration::from_secs(30)) {
        Ok(Ok(())) => {
            unsafe { Ok(std::fs::File::from_raw_handle(handle)) }
        },
        Ok(Err(e)) => {
            unsafe { CloseHandle(handle); }
            Err(e)
        }
        Err(_) => {
            unsafe { CloseHandle(handle); }
            Err("Elevated process did not connect within 30 seconds".to_string())
        }
    }
}

#[cfg(windows)]
pub fn assign_pid_to_job(pid: u32) {
    if pid == 0 {
        return;
    }
    use std::sync::Once;
    static START: Once = Once::new();
    static mut JOB_HANDLE: *mut std::ffi::c_void = std::ptr::null_mut();

    unsafe {
        START.call_once(|| {
            extern "system" {
                fn CreateJobObjectW(lpJobAttributes: *mut std::ffi::c_void, lpName: *const u16) -> *mut std::ffi::c_void;
                fn SetInformationJobObject(
                    hJob: *mut std::ffi::c_void,
                    JobObjectInformationClass: i32,
                    lpJobObjectInformation: *const std::ffi::c_void,
                    cbJobObjectInformationLength: u32,
                ) -> i32;
            }

            #[repr(C)]
            struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
                per_process_user_time_limit: i64,
                per_job_user_time_limit: i64,
                limit_flags: u32,
                minimum_working_set_size: usize,
                maximum_working_set_size: usize,
                active_process_limit: u32,
                affinity: usize,
                priority_class: u32,
                scheduling_class: u32,
            }

            #[repr(C)]
            struct IO_COUNTERS {
                read_operation_count: u64,
                write_operation_count: u64,
                other_operation_count: u64,
                read_transfer_count: u64,
                write_transfer_count: u64,
                other_transfer_count: u64,
            }

            #[repr(C)]
            struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
                basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
                io_info: IO_COUNTERS,
                process_memory_limit: usize,
                job_memory_limit: usize,
                peak_process_memory_used: usize,
                peak_job_memory_used: usize,
            }

            const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
            const JOB_OBJECT_EXTENDED_LIMIT_INFO_CLASS: i32 = 9;

            let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if !job.is_null() {
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job,
                    JOB_OBJECT_EXTENDED_LIMIT_INFO_CLASS,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                JOB_HANDLE = job;
            }
        });

        if !JOB_HANDLE.is_null() {
            extern "system" {
                fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
                fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
                fn AssignProcessToJobObject(hJob: *mut std::ffi::c_void, hProcess: *mut std::ffi::c_void) -> i32;
            }
            let proc_handle = OpenProcess(0x1F0FFF, 0, pid);
            if !proc_handle.is_null() {
                AssignProcessToJobObject(JOB_HANDLE, proc_handle);
                CloseHandle(proc_handle);
            }
        }
    }
}

#[cfg(unix)]
pub fn assign_pid_to_job(pid: u32) {
    if pid == 0 {
        return;
    }
    unsafe {
        let _ = libc::setpgid(pid as libc::pid_t, pid as libc::pid_t);
    }
}

#[cfg(not(any(windows, unix)))]
pub fn assign_pid_to_job(_pid: u32) {}
