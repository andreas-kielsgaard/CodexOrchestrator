use super::domain::{OwnedProcessLaunch, OwnerObservation, OwnerRoute};
use std::{error::Error, fmt};

pub(crate) trait ProcessOwner: Send + Sync {
    fn launch(
        &self,
        route: &OwnerRoute,
        launches: &[OwnedProcessLaunch],
    ) -> Result<OwnerObservation, OwnershipError>;

    fn observe(&self, route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError>;

    fn focus(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Err(OwnershipError::new(
            OwnershipErrorKind::Unavailable,
            "visible-window focus is unavailable for this process owner",
        ))
    }

    fn terminate(&self, route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OwnershipErrorKind {
    AlreadyExists,
    Ambiguous,
    LaunchFailed,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OwnershipError {
    pub(crate) kind: OwnershipErrorKind,
    pub(crate) message: String,
}

impl OwnershipError {
    fn new(kind: OwnershipErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for OwnershipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for OwnershipError {}

#[cfg(windows)]
mod windows {
    use super::{OwnershipError, OwnershipErrorKind, ProcessOwner};
    use crate::worktree_runtime::domain::{OwnedProcessLaunch, OwnerObservation, OwnerRoute};
    use std::{
        collections::{HashMap, HashSet},
        ffi::{c_void, OsStr},
        mem::{size_of, zeroed},
        os::windows::ffi::OsStrExt,
        ptr::{null, null_mut},
        sync::Mutex,
        thread,
        time::{Duration, Instant},
    };
    use windows_sys::core::BOOL;
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, INVALID_HANDLE_VALUE,
            LPARAM,
        },
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            CreateFileW, FILE_APPEND_DATA, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_ALWAYS,
        },
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
                JobObjectBasicProcessIdList, JobObjectExtendedLimitInformation, OpenJobObjectW,
                QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
                JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_BASIC_PROCESS_ID_LIST,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            },
            Threading::{
                CreateProcessW, ResumeThread, TerminateProcess, CREATE_NO_WINDOW, CREATE_SUSPENDED,
                CREATE_UNICODE_ENVIRONMENT, PROCESS_INFORMATION, STARTF_USESTDHANDLES,
                STARTUPINFOW,
            },
        },
        UI::WindowsAndMessaging::{
            BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindow,
            GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow, GW_OWNER,
            SW_RESTORE,
        },
    };

    const JOB_OBJECT_QUERY_ACCESS: u32 = 0x0004;
    const JOB_OBJECT_TERMINATE_ACCESS: u32 = 0x0008;

    pub(crate) struct WindowsJobProcessOwner {
        handles: Mutex<HashMap<String, OwnedHandle>>,
        termination_timeout: Duration,
    }

    impl Default for WindowsJobProcessOwner {
        fn default() -> Self {
            Self {
                handles: Mutex::new(HashMap::new()),
                termination_timeout: Duration::from_secs(5),
            }
        }
    }

    impl ProcessOwner for WindowsJobProcessOwner {
        fn launch(
            &self,
            route: &OwnerRoute,
            launches: &[OwnedProcessLaunch],
        ) -> Result<OwnerObservation, OwnershipError> {
            validate_job_name(&route.job_name)?;
            let mut handles = self.handles.lock().map_err(|_| {
                OwnershipError::new(
                    OwnershipErrorKind::Unavailable,
                    "Windows Job Object handle registry is unavailable",
                )
            })?;
            if handles.contains_key(&route.job_name) {
                return Err(OwnershipError::new(
                    OwnershipErrorKind::AlreadyExists,
                    "the exact Job Object route is already owned by this application",
                ));
            }

            let wide_name = wide_null(&route.job_name);
            let handle = unsafe { CreateJobObjectW(null(), wide_name.as_ptr()) };
            if handle.is_null() {
                return Err(last_error(
                    OwnershipErrorKind::Unavailable,
                    "create named Job Object",
                ));
            }
            let owned = OwnedHandle(handle);
            if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
                return Err(OwnershipError::new(
                    OwnershipErrorKind::AlreadyExists,
                    "the named Job Object already exists; launch ownership is ambiguous",
                ));
            }

            configure_kill_on_close(owned.0)?;
            for launch in launches {
                if let Err(error) = launch_suspended_into_job(owned.0, launch) {
                    unsafe {
                        TerminateJobObject(owned.0, 1);
                    }
                    return Err(error);
                }
            }
            let observation = observe_handle(owned.0)?;
            if !observation.is_active() {
                return Err(OwnershipError::new(
                    OwnershipErrorKind::LaunchFailed,
                    "the Job Object contained no active process after launch",
                ));
            }
            handles.insert(route.job_name.clone(), owned);
            Ok(observation)
        }

        fn observe(&self, route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
            validate_job_name(&route.job_name)?;
            if let Some(handle) = self
                .handles
                .lock()
                .map_err(|_| {
                    OwnershipError::new(
                        OwnershipErrorKind::Unavailable,
                        "Windows Job Object handle registry is unavailable",
                    )
                })?
                .get(&route.job_name)
            {
                return observe_handle(handle.0);
            }
            let Some(handle) = open_job(&route.job_name, JOB_OBJECT_QUERY_ACCESS)? else {
                return Ok(OwnerObservation::Absent);
            };
            observe_handle(handle.0)
        }

        fn focus(&self, route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
            validate_job_name(&route.job_name)?;
            let handle = match self
                .handles
                .lock()
                .map_err(|_| {
                    OwnershipError::new(
                        OwnershipErrorKind::Unavailable,
                        "Windows Job Object handle registry is unavailable",
                    )
                })?
                .get(&route.job_name)
                .map(|owned| owned.0)
            {
                Some(handle) => handle,
                None => {
                    let Some(opened) = open_job(&route.job_name, JOB_OBJECT_QUERY_ACCESS)? else {
                        return Err(OwnershipError::new(
                            OwnershipErrorKind::Ambiguous,
                            "the exact Job Object is absent while focusing its review window",
                        ));
                    };
                    let observation = focus_owned_window(opened.0)?;
                    return Ok(observation);
                }
            };
            focus_owned_window(handle)
        }

        fn terminate(&self, route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
            validate_job_name(&route.job_name)?;
            let owned = self
                .handles
                .lock()
                .map_err(|_| {
                    OwnershipError::new(
                        OwnershipErrorKind::Unavailable,
                        "Windows Job Object handle registry is unavailable",
                    )
                })?
                .remove(&route.job_name);
            let handle = match owned {
                Some(handle) => handle,
                None => {
                    let Some(handle) = open_job(
                        &route.job_name,
                        JOB_OBJECT_QUERY_ACCESS | JOB_OBJECT_TERMINATE_ACCESS,
                    )?
                    else {
                        return Ok(OwnerObservation::Absent);
                    };
                    handle
                }
            };
            if unsafe { TerminateJobObject(handle.0, 1) } == 0 {
                return Err(last_error(
                    OwnershipErrorKind::Ambiguous,
                    "terminate exact named Job Object",
                ));
            }
            let deadline = Instant::now() + self.termination_timeout;
            loop {
                let observation = observe_handle(handle.0)?;
                if !observation.is_active() {
                    return Ok(OwnerObservation::Absent);
                }
                if Instant::now() >= deadline {
                    return Err(OwnershipError::new(
                        OwnershipErrorKind::Ambiguous,
                        "the exact named Job Object still has active processes after termination",
                    ));
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
    }

    struct OwnedHandle(HANDLE);

    unsafe impl Send for OwnedHandle {}

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    fn configure_kill_on_close(handle: HANDLE) -> Result<(), OwnershipError> {
        let mut information: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let result = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &information as *const _ as *const c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if result == 0 {
            Err(last_error(
                OwnershipErrorKind::Unavailable,
                "configure Job Object kill-on-close",
            ))
        } else {
            Ok(())
        }
    }

    fn launch_suspended_into_job(
        job: HANDLE,
        launch: &OwnedProcessLaunch,
    ) -> Result<(), OwnershipError> {
        let application = wide_null(launch.program.as_os_str());
        let mut command_line = wide_null(build_command_line(launch));
        let working_directory = wide_null(launch.working_directory.as_os_str());
        let environment = environment_block(&launch.environment);
        let log = open_inheritable_log(&launch.log_path)?;
        let mut startup: STARTUPINFOW = unsafe { zeroed() };
        startup.cb = size_of::<STARTUPINFOW>() as u32;
        startup.dwFlags = STARTF_USESTDHANDLES;
        startup.hStdOutput = log.0;
        startup.hStdError = log.0;
        let mut process: PROCESS_INFORMATION = unsafe { zeroed() };
        let created = unsafe {
            CreateProcessW(
                application.as_ptr(),
                command_line.as_mut_ptr(),
                null(),
                null(),
                1,
                CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
                environment.as_ptr() as *const c_void,
                working_directory.as_ptr(),
                &startup,
                &mut process,
            )
        };
        if created == 0 {
            return Err(last_error(
                OwnershipErrorKind::LaunchFailed,
                "create suspended owned process",
            ));
        }
        let process_handle = OwnedHandle(process.hProcess);
        let thread_handle = OwnedHandle(process.hThread);
        if unsafe { AssignProcessToJobObject(job, process_handle.0) } == 0 {
            unsafe {
                TerminateProcess(process_handle.0, 1);
            }
            return Err(last_error(
                OwnershipErrorKind::Ambiguous,
                "assign suspended process to exact Job Object",
            ));
        }
        if unsafe { ResumeThread(thread_handle.0) } == u32::MAX {
            unsafe {
                TerminateProcess(process_handle.0, 1);
            }
            return Err(last_error(
                OwnershipErrorKind::LaunchFailed,
                "resume owned process after Job Object assignment",
            ));
        }
        Ok(())
    }

    fn open_inheritable_log(path: &std::path::Path) -> Result<OwnedHandle, OwnershipError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                OwnershipError::new(
                    OwnershipErrorKind::Unavailable,
                    format!("create review log directory: {error}"),
                )
            })?;
        }
        let mut security = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: 1,
        };
        let wide = wide_null(path.as_os_str());
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_APPEND_DATA,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                &mut security,
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(last_error(
                OwnershipErrorKind::Unavailable,
                "open isolated review process log",
            ))
        } else {
            Ok(OwnedHandle(handle))
        }
    }

    struct FocusContext {
        process_ids: HashSet<u32>,
        window: HWND,
    }

    unsafe extern "system" fn find_owned_window(window: HWND, data: LPARAM) -> BOOL {
        let context = unsafe { &mut *(data as *mut FocusContext) };
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut process_id);
        }
        if context.process_ids.contains(&process_id)
            && unsafe { IsWindowVisible(window) } != 0
            && unsafe { GetWindow(window, GW_OWNER) }.is_null()
        {
            context.window = window;
            return 0;
        }
        1
    }

    fn focus_owned_window(handle: HANDLE) -> Result<OwnerObservation, OwnershipError> {
        let process_ids = job_process_ids(handle)?;
        let mut context = FocusContext {
            process_ids,
            window: null_mut(),
        };
        unsafe {
            EnumWindows(
                Some(find_owned_window),
                &mut context as *mut FocusContext as LPARAM,
            );
        }
        if context.window.is_null() {
            return Err(OwnershipError::new(
                OwnershipErrorKind::Ambiguous,
                "the owned review process tree has no visible top-level window",
            ));
        }
        unsafe {
            ShowWindow(context.window, SW_RESTORE);
            BringWindowToTop(context.window);
            SetForegroundWindow(context.window);
        }
        if unsafe { GetForegroundWindow() } != context.window {
            return Err(OwnershipError::new(
                OwnershipErrorKind::Ambiguous,
                "Windows did not grant foreground focus to the owned review window",
            ));
        }
        observe_handle(handle)
    }

    fn job_process_ids(handle: HANDLE) -> Result<HashSet<u32>, OwnershipError> {
        let capacity = 256usize;
        let bytes = size_of::<JOBOBJECT_BASIC_PROCESS_ID_LIST>()
            + capacity.saturating_sub(1) * size_of::<usize>();
        let mut buffer = vec![0u8; bytes];
        let result = unsafe {
            QueryInformationJobObject(
                handle,
                JobObjectBasicProcessIdList,
                buffer.as_mut_ptr() as *mut c_void,
                bytes as u32,
                null_mut(),
            )
        };
        if result == 0 {
            return Err(last_error(
                OwnershipErrorKind::Unavailable,
                "query review Job Object process list",
            ));
        }
        let information = unsafe { &*(buffer.as_ptr() as *const JOBOBJECT_BASIC_PROCESS_ID_LIST) };
        let count = information.NumberOfProcessIdsInList as usize;
        let first = information.ProcessIdList.as_ptr();
        Ok((0..count)
            .filter_map(|index| u32::try_from(unsafe { *first.add(index) }).ok())
            .collect())
    }

    fn observe_handle(handle: HANDLE) -> Result<OwnerObservation, OwnershipError> {
        let mut information: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { zeroed() };
        let result = unsafe {
            QueryInformationJobObject(
                handle,
                JobObjectBasicAccountingInformation,
                &mut information as *mut _ as *mut c_void,
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                null_mut(),
            )
        };
        if result == 0 {
            Err(last_error(
                OwnershipErrorKind::Ambiguous,
                "query exact named Job Object",
            ))
        } else {
            Ok(OwnerObservation::Owned {
                active_processes: information.ActiveProcesses,
            })
        }
    }

    fn open_job(name: &str, access: u32) -> Result<Option<OwnedHandle>, OwnershipError> {
        let wide_name = wide_null(name);
        let handle = unsafe { OpenJobObjectW(access, 0, wide_name.as_ptr()) };
        if handle.is_null() {
            let error = unsafe { GetLastError() };
            if error == windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND {
                Ok(None)
            } else {
                Err(OwnershipError::new(
                    OwnershipErrorKind::Ambiguous,
                    format!("open exact named Job Object failed with Windows error {error}"),
                ))
            }
        } else {
            Ok(Some(OwnedHandle(handle)))
        }
    }

    fn validate_job_name(name: &str) -> Result<(), OwnershipError> {
        if !name.starts_with("Local\\CodexOrchestrator.WorktreeRuntime.")
            || name.encode_utf16().any(|unit| unit == 0)
        {
            return Err(OwnershipError::new(
                OwnershipErrorKind::Ambiguous,
                "the durable Job Object route is invalid",
            ));
        }
        Ok(())
    }

    fn build_command_line(launch: &OwnedProcessLaunch) -> String {
        std::iter::once(launch.program.to_string_lossy().into_owned())
            .chain(launch.arguments.iter().cloned())
            .map(|argument| quote_windows_argument(&argument))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn quote_windows_argument(argument: &str) -> String {
        if !argument.is_empty()
            && !argument
                .chars()
                .any(|character| character.is_whitespace() || character == '"')
        {
            return argument.to_string();
        }
        let mut quoted = String::from("\"");
        let mut slashes = 0;
        for character in argument.chars() {
            match character {
                '\\' => slashes += 1,
                '"' => {
                    quoted.push_str(&"\\".repeat(slashes * 2 + 1));
                    quoted.push('"');
                    slashes = 0;
                }
                _ => {
                    quoted.push_str(&"\\".repeat(slashes));
                    slashes = 0;
                    quoted.push(character);
                }
            }
        }
        quoted.push_str(&"\\".repeat(slashes * 2));
        quoted.push('"');
        quoted
    }

    fn environment_block(environment: &HashMapLike) -> Vec<u16> {
        let mut block = Vec::new();
        let mut entries = environment.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(name, _)| name.to_uppercase());
        for (name, value) in entries {
            block.extend(format!("{name}={value}").encode_utf16());
            block.push(0);
        }
        block.push(0);
        if block.len() == 1 {
            block.push(0);
        }
        block
    }

    type HashMapLike = std::collections::BTreeMap<String, String>;

    fn wide_null(value: impl AsRef<OsStr>) -> Vec<u16> {
        value.as_ref().encode_wide().chain(Some(0)).collect()
    }

    fn last_error(kind: OwnershipErrorKind, operation: &str) -> OwnershipError {
        let error = unsafe { GetLastError() };
        OwnershipError::new(
            kind,
            format!("{operation} failed with Windows error {error}"),
        )
    }
}

#[cfg(windows)]
#[allow(unused_imports)]
pub(crate) use windows::WindowsJobProcessOwner;

#[cfg(not(windows))]
pub(crate) struct UnsupportedProcessOwner;

#[cfg(not(windows))]
impl ProcessOwner for UnsupportedProcessOwner {
    fn launch(
        &self,
        _route: &OwnerRoute,
        _launches: &[OwnedProcessLaunch],
    ) -> Result<OwnerObservation, OwnershipError> {
        Err(OwnershipError::new(
            OwnershipErrorKind::Unavailable,
            "worktree process-tree ownership is currently available only on Windows",
        ))
    }

    fn observe(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Err(OwnershipError::new(
            OwnershipErrorKind::Unavailable,
            "worktree process-tree ownership is currently available only on Windows",
        ))
    }

    fn terminate(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Err(OwnershipError::new(
            OwnershipErrorKind::Unavailable,
            "worktree process-tree ownership is currently available only on Windows",
        ))
    }
}
