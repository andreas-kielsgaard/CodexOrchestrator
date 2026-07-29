#[cfg(windows)]
fn main() -> Result<(), String> {
    use std::{env, os::windows::ffi::OsStrExt, path::PathBuf, ptr::null};
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            CreateProcessW, TerminateProcess, CREATE_NEW_PROCESS_GROUP, PROCESS_INFORMATION,
            STARTF_USESHOWWINDOW, STARTUPINFOW,
        },
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, SetWindowPos, ShowWindow,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE,
        },
    };

    let executable =
        PathBuf::from(env::args().nth(1).ok_or_else(|| {
            "Usage: worktree_review_background_launcher <absolute-exe>".to_string()
        })?);
    if !executable.is_absolute() || !executable.is_file() {
        return Err("The launcher executable must be an absolute regular file.".into());
    }
    let before = unsafe { GetForegroundWindow() };
    let mut before_process = 0;
    unsafe {
        GetWindowThreadProcessId(before, &mut before_process);
    }
    let application = executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    startup.dwFlags = STARTF_USESHOWWINDOW;
    startup.wShowWindow = SW_SHOWNOACTIVATE as u16;
    let mut process: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe {
        CreateProcessW(
            application.as_ptr(),
            std::ptr::null_mut(),
            null(),
            null(),
            0,
            CREATE_NEW_PROCESS_GROUP,
            null(),
            std::ptr::null(),
            &startup,
            &mut process,
        )
    } == 0
    {
        return Err("Start the isolated launcher without activation.".into());
    }
    unsafe {
        CloseHandle(process.hThread);
    }
    std::thread::sleep(std::time::Duration::from_secs(4));
    let hidden_after = unsafe { GetForegroundWindow() };
    if before != hidden_after {
        unsafe {
            TerminateProcess(process.hProcess, 1);
            CloseHandle(process.hProcess);
        }
        return Err(
            "The hidden isolated launcher changed foreground ownership; it was terminated.".into(),
        );
    }
    let window = match wait_for_launcher_window(process.dwProcessId) {
        Ok(window) => window,
        Err(error) => {
            unsafe {
                TerminateProcess(process.hProcess, 1);
                CloseHandle(process.hProcess);
            }
            return Err(error);
        }
    };
    unsafe {
        SetWindowPos(
            window,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        );
        ShowWindow(window, SW_SHOWNOACTIVATE);
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    let after = unsafe { GetForegroundWindow() };
    let mut after_process = 0;
    unsafe {
        GetWindowThreadProcessId(after, &mut after_process);
    }
    if before != after || before_process != after_process {
        unsafe {
            TerminateProcess(process.hProcess, 1);
            CloseHandle(process.hProcess);
        }
        return Err(format!(
            "The isolated launcher changed foreground ownership (before window {} process {}, after window {} process {}); it was terminated.",
            before as usize, before_process, after as usize, after_process
        ));
    }
    unsafe {
        CloseHandle(process.hProcess);
    }
    println!(
        "{{\"launcherProcessId\":{},\"foregroundBefore\":{{\"window\":{},\"processId\":{}}},\"foregroundAfter\":{{\"window\":{},\"processId\":{}}},\"foregroundUnchanged\":true}}",
        process.dwProcessId,
        before as usize,
        before_process,
        after as usize,
        after_process
    );
    Ok(())
}

#[cfg(windows)]
fn wait_for_launcher_window(process_id: u32) -> Result<*mut std::ffi::c_void, String> {
    use std::{ptr::null_mut, time::Instant};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };
    struct Context {
        process_id: u32,
        window: *mut std::ffi::c_void,
    }
    unsafe extern "system" fn visit(window: *mut std::ffi::c_void, data: isize) -> i32 {
        let context = unsafe { &mut *(data as *mut Context) };
        let mut owner = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut owner);
        }
        if owner != context.process_id {
            return 1;
        }
        let length = unsafe { GetWindowTextLengthW(window) };
        if length <= 0 {
            return 1;
        }
        let mut title = vec![0u16; length as usize + 1];
        unsafe {
            GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32);
        }
        if String::from_utf16_lossy(&title[..length as usize]).contains("Worktree Review Launcher")
        {
            context.window = window;
            return 0;
        }
        1
    }
    let deadline = Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let mut context = Context {
            process_id,
            window: null_mut(),
        };
        unsafe {
            EnumWindows(Some(visit), &mut context as *mut Context as isize);
        }
        if !context.window.is_null() {
            return Ok(context.window);
        }
        if Instant::now() >= deadline {
            return Err(
                "The hidden isolated launcher did not create its owned main window.".into(),
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(not(windows))]
fn main() -> Result<(), String> {
    Err("The non-activating launcher proof is Windows-only.".into())
}
