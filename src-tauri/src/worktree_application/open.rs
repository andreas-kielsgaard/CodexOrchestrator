use super::domain::{
    OpenOutcome, PhysicalWorktreeBuildResult, WorktreeApplicationError,
    WorktreeApplicationErrorKind, WorktreeApplicationLaunchContext,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};

trait OpenMechanism {
    fn activate_existing(&self, executable: &Path) -> Result<bool, WorktreeApplicationError>;
    fn launch_detached(
        &self,
        executable: &Path,
        context: &WorktreeApplicationLaunchContext,
    ) -> Result<(), WorktreeApplicationError>;
}

struct SystemOpenMechanism;

impl OpenMechanism for SystemOpenMechanism {
    fn activate_existing(&self, executable: &Path) -> Result<bool, WorktreeApplicationError> {
        activate_existing(executable)
    }

    fn launch_detached(
        &self,
        executable: &Path,
        context: &WorktreeApplicationLaunchContext,
    ) -> Result<(), WorktreeApplicationError> {
        launch_detached(executable, context)
    }
}

pub(super) fn open(
    build: &PhysicalWorktreeBuildResult,
    context: &WorktreeApplicationLaunchContext,
) -> Result<OpenOutcome, WorktreeApplicationError> {
    open_with(build, context, &SystemOpenMechanism)
}

fn open_with(
    build: &PhysicalWorktreeBuildResult,
    context: &WorktreeApplicationLaunchContext,
    mechanism: &dyn OpenMechanism,
) -> Result<OpenOutcome, WorktreeApplicationError> {
    let executable = canonical_executable(&build.executable)?;
    if mechanism.activate_existing(&executable)? {
        return Ok(OpenOutcome::ExistingWindowActivationRequested);
    }
    mechanism.launch_detached(&executable, context)?;
    Ok(OpenOutcome::DetachedLaunchStarted)
}

fn canonical_executable(path: &Path) -> Result<PathBuf, WorktreeApplicationError> {
    fs::canonicalize(path)
        .ok()
        .filter(|path| path.is_file())
        .ok_or_else(|| {
            WorktreeApplicationError::new(
                WorktreeApplicationErrorKind::ExecutableUnavailable,
                "The built application executable is unavailable.",
            )
        })
}

#[cfg(windows)]
fn activate_existing(executable: &Path) -> Result<bool, WorktreeApplicationError> {
    windows::activate_existing(executable)
}

#[cfg(not(windows))]
fn activate_existing(_executable: &Path) -> Result<bool, WorktreeApplicationError> {
    Ok(false)
}

fn launch_detached(
    executable: &Path,
    context: &WorktreeApplicationLaunchContext,
) -> Result<(), WorktreeApplicationError> {
    let mut command = Command::new(executable);
    command
        .current_dir(executable.parent().ok_or_else(open_failed)?)
        .envs(&context.environment)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_detached(&mut command);
    let mut child = command.spawn().map_err(|_| open_failed())?;
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(windows)]
fn configure_detached(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};

    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(unix)]
fn configure_detached(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(not(any(unix, windows)))]
fn configure_detached(_command: &mut Command) {}

fn open_failed() -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::OpenFailed,
        "The built application could not be opened.",
    )
}

#[cfg(windows)]
mod windows {
    use super::{open_failed, WorktreeApplicationError};
    use std::{fs, path::Path, ptr::null_mut};
    use windows_sys::{
        core::BOOL,
        Win32::{
            Foundation::{CloseHandle, HWND, LPARAM},
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
            },
            UI::WindowsAndMessaging::{
                BringWindowToTop, EnumWindows, GetWindow, GetWindowThreadProcessId, IsIconic,
                IsWindowVisible, SetForegroundWindow, ShowWindow, GW_OWNER, SW_RESTORE,
            },
        },
    };

    struct SearchContext {
        executable_identity: String,
        visible_window: HWND,
        fallback_window: HWND,
    }

    pub(super) fn activate_existing(executable: &Path) -> Result<bool, WorktreeApplicationError> {
        let mut context = SearchContext {
            executable_identity: executable_identity(executable).ok_or_else(open_failed)?,
            visible_window: null_mut(),
            fallback_window: null_mut(),
        };
        unsafe {
            EnumWindows(
                Some(find_exact_executable_window),
                &mut context as *mut SearchContext as LPARAM,
            );
        }
        let window = if context.visible_window.is_null() {
            context.fallback_window
        } else {
            context.visible_window
        };
        if window.is_null() {
            return Ok(false);
        }
        unsafe {
            if IsIconic(window) != 0 || IsWindowVisible(window) == 0 {
                ShowWindow(window, SW_RESTORE);
            }
            BringWindowToTop(window);
            SetForegroundWindow(window);
        }
        Ok(true)
    }

    unsafe extern "system" fn find_exact_executable_window(window: HWND, data: LPARAM) -> BOOL {
        let context = unsafe { &mut *(data as *mut SearchContext) };
        if !unsafe { GetWindow(window, GW_OWNER) }.is_null() {
            return 1;
        }
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut process_id);
        }
        if process_id == 0 {
            return 1;
        }
        let Some(path) = process_executable(process_id) else {
            return 1;
        };
        if executable_identity(&path).as_deref() == Some(&context.executable_identity) {
            if context.fallback_window.is_null() {
                context.fallback_window = window;
            }
            if unsafe { IsWindowVisible(window) } != 0 {
                context.visible_window = window;
                return 0;
            }
        }
        1
    }

    fn process_executable(process_id: u32) -> Option<std::path::PathBuf> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return None;
        }
        let mut buffer = vec![0_u16; 32_768];
        let mut length = buffer.len() as u32;
        let result =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
        unsafe {
            CloseHandle(process);
        }
        if result == 0 || length == 0 {
            return None;
        }
        buffer.truncate(length as usize);
        Some(std::path::PathBuf::from(String::from_utf16_lossy(&buffer)))
    }

    fn executable_identity(path: &Path) -> Option<String> {
        let path = fs::canonicalize(path).ok()?;
        let value = path.to_string_lossy().replace('\\', "/");
        Some(
            value
                .strip_prefix("//?/")
                .unwrap_or(&value)
                .trim_end_matches('/')
                .to_ascii_lowercase(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct RecordingMechanism {
        existing: bool,
        calls: Mutex<Vec<&'static str>>,
        launch_context: Mutex<Option<WorktreeApplicationLaunchContext>>,
    }

    impl OpenMechanism for RecordingMechanism {
        fn activate_existing(&self, _executable: &Path) -> Result<bool, WorktreeApplicationError> {
            self.calls.lock().unwrap().push("activate");
            Ok(self.existing)
        }

        fn launch_detached(
            &self,
            _executable: &Path,
            context: &WorktreeApplicationLaunchContext,
        ) -> Result<(), WorktreeApplicationError> {
            self.calls.lock().unwrap().push("launch");
            *self.launch_context.lock().unwrap() = Some(context.clone());
            Ok(())
        }
    }

    #[test]
    fn existing_exact_window_is_preferred_to_another_launch() {
        let build = fixture();
        let mechanism = RecordingMechanism {
            existing: true,
            calls: Mutex::new(Vec::new()),
            launch_context: Mutex::new(None),
        };
        let context = launch_context(&build.0);

        assert_eq!(
            open_with(&build.1, &context, &mechanism).unwrap(),
            OpenOutcome::ExistingWindowActivationRequested
        );
        assert_eq!(*mechanism.calls.lock().unwrap(), ["activate"]);
    }

    #[test]
    fn missing_exact_window_starts_a_detached_application() {
        let build = fixture();
        let mechanism = RecordingMechanism {
            existing: false,
            calls: Mutex::new(Vec::new()),
            launch_context: Mutex::new(None),
        };
        let context = launch_context(&build.0);

        assert_eq!(
            open_with(&build.1, &context, &mechanism).unwrap(),
            OpenOutcome::DetachedLaunchStarted
        );
        assert_eq!(*mechanism.calls.lock().unwrap(), ["activate", "launch"]);
        let launched = mechanism.launch_context.lock().unwrap().clone().unwrap();
        assert_eq!(launched.environment, context.environment);
    }

    fn fixture() -> (tempfile::TempDir, PhysicalWorktreeBuildResult) {
        let directory = tempfile::tempdir().unwrap();
        let worktree = directory.path().join("worktree");
        fs::create_dir(&worktree).unwrap();
        let executable = worktree.join(if cfg!(windows) {
            "sample-app.exe"
        } else {
            "sample-app"
        });
        fs::write(&executable, b"application").unwrap();
        let attempt_root = directory.path().to_path_buf();
        (
            directory,
            PhysicalWorktreeBuildResult {
                worktree_root: worktree.clone(),
                attempt_root: attempt_root.clone(),
                output_root: worktree.clone(),
                log_path: attempt_root.join("build.log"),
                executable,
            },
        )
    }

    fn launch_context(directory: &tempfile::TempDir) -> WorktreeApplicationLaunchContext {
        WorktreeApplicationLaunchContext::new([
            (
                std::ffi::OsString::from("APPLICATION_DATA"),
                directory.path().join("shared-data").into_os_string(),
            ),
            ("ACTIVE_BUILD".into(), "opaque-build".into()),
            ("ACTIVE_WORKTREE".into(), "opaque-worktree".into()),
        ])
        .unwrap()
    }
}
