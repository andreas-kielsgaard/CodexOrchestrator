use super::{ChildProcessFactory, ProcessExit, ProcessLaunchSpec, SpawnedProcess, SupervisedChild};
use std::{
    io::{self, Write},
    process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(crate) struct SystemProcessFactory;

impl ChildProcessFactory for SystemProcessFactory {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess> {
        spawn(spec, false)
    }
}

pub(crate) struct DuplexProcessFactory;

impl ChildProcessFactory for DuplexProcessFactory {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess> {
        spawn(spec, true)
    }
}

fn spawn(spec: &ProcessLaunchSpec, duplex: bool) -> io::Result<SpawnedProcess> {
    let mut command = Command::new(&spec.program);
    for key in &spec.remove_environment {
        command.env_remove(key);
    }
    command
        .args(&spec.args)
        .envs(spec.environment.iter().map(|(key, value)| (key, value)))
        .stdin(if duplex {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(working_directory) = &spec.working_directory {
        command.current_dir(working_directory);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000004); // CREATE_NO_WINDOW | CREATE_SUSPENDED
    }
    let mut child = command.spawn()?;
    #[cfg(windows)]
    let job = match super::windows_job::JobGuard::attach_and_resume(&child) {
        Ok(job) => job,
        Err(error) => {
            terminate_and_reap(&mut child);
            return Err(error);
        }
    };
    let stdin = child.stdin.take();
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            terminate_and_reap(&mut child);
            return Err(io::Error::other(
                "spawned process did not expose its piped stdout handle",
            ));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            terminate_and_reap(&mut child);
            return Err(io::Error::other(
                "spawned process did not expose its piped stderr handle",
            ));
        }
    };

    Ok(SpawnedProcess {
        child: Arc::new(SystemChild {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            #[cfg(windows)]
            job,
        }),
        stdout: Box::new(stdout),
        stderr: Box::new(stderr),
    })
}

fn terminate_and_reap(child: &mut Child) {
    let _ = child.kill();
    loop {
        match child.wait() {
            Ok(_) => return,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        }
    }
}

struct SystemChild {
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    #[cfg(windows)]
    job: super::windows_job::JobGuard,
}

impl SupervisedChild for SystemChild {
    fn write_input(&self, bytes: &[u8]) -> io::Result<()> {
        let mut input = self
            .stdin
            .lock()
            .map_err(|_| poisoned_lock("process input"))?;
        let input = input
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "Process input is closed"))?;
        input.write_all(bytes)?;
        input.flush()
    }

    fn close_input(&self) -> io::Result<()> {
        self.stdin
            .lock()
            .map_err(|_| poisoned_lock("process input"))?
            .take();
        Ok(())
    }
    fn try_wait(&self) -> io::Result<Option<ProcessExit>> {
        let exit = self
            .child
            .lock()
            .map_err(|_| poisoned_lock("system child"))?
            .try_wait()
            .map(|status| status.map(process_exit))?;
        #[cfg(windows)]
        if exit.is_some() {
            self.job.terminate()?;
        }
        Ok(exit)
    }

    fn terminate(&self) -> io::Result<()> {
        let mut child = self
            .child
            .lock()
            .map_err(|_| poisoned_lock("system child"))?;
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        #[cfg(windows)]
        {
            self.job.terminate()
        }
        #[cfg(not(windows))]
        {
            child.kill()
        }
    }

    fn wait_after_termination(&self) -> io::Result<ProcessExit> {
        let mut child = self
            .child
            .lock()
            .map_err(|_| poisoned_lock("system child"))?;
        loop {
            match child.wait() {
                Ok(status) => return Ok(process_exit(status)),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
    }
}

#[cfg(unix)]
fn process_exit(status: ExitStatus) -> ProcessExit {
    use std::os::unix::process::ExitStatusExt;

    ProcessExit {
        exit_code: status.code(),
        signal: status.signal().map(|signal| signal.to_string()),
    }
}

#[cfg(not(unix))]
fn process_exit(status: ExitStatus) -> ProcessExit {
    ProcessExit {
        exit_code: status.code(),
        signal: None,
    }
}

fn poisoned_lock(name: &str) -> io::Error {
    io::Error::other(format!("{name} lock was poisoned"))
}
#[cfg(all(test, windows))]
mod descendant_tests {
    use super::*;
    use std::io::BufRead;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    #[test]
    fn duplex_factory_terminates_descendants_with_its_owned_job() {
        let script = "$worker = Start-Process powershell.exe -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 120' -WindowStyle Hidden -PassThru; Write-Output $worker.Id; Start-Sleep -Seconds 120";
        let process = DuplexProcessFactory
            .spawn(&ProcessLaunchSpec {
                remove_environment: Vec::new(),
                program: "powershell.exe".into(),
                args: vec!["-NoProfile".into(), "-Command".into(), script.into()],
                working_directory: None,
                environment: vec![],
            })
            .unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let read = std::io::BufReader::new(process.stdout).read_line(&mut line);
            let _ = sender.send(read.map(|_| line));
        });
        let result = receiver.recv_timeout(std::time::Duration::from_secs(15));
        if result.is_err() {
            let _ = process.child.terminate();
        }
        let child_id: u32 = result.unwrap().unwrap().trim().parse().unwrap();
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | 0x00100000, 0, child_id) };
        assert!(
            !handle.is_null(),
            "owned descendant must be running before cancellation"
        );
        process.child.terminate().unwrap();
        process.child.wait_after_termination().unwrap();
        let waited = unsafe { WaitForSingleObject(handle, 5000) };
        unsafe {
            CloseHandle(handle);
        }
        assert_eq!(
            waited, 0,
            "descendant must exit when its invocation job is terminated"
        );
    }
}
