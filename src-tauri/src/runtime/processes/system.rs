use super::{ChildProcessFactory, ProcessExit, ProcessLaunchSpec, SpawnedProcess, SupervisedChild};
use std::{
    io,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(crate) struct SystemProcessFactory;

impl ChildProcessFactory for SystemProcessFactory {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess> {
        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .envs(spec.environment.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(working_directory) = &spec.working_directory {
            command.current_dir(working_directory);
        }

        let mut child = command.spawn()?;
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
            }),
            stdout: Box::new(stdout),
            stderr: Box::new(stderr),
        })
    }
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
}

impl SupervisedChild for SystemChild {
    fn try_wait(&self) -> io::Result<Option<ProcessExit>> {
        self.child
            .lock()
            .map_err(|_| poisoned_lock("system child"))?
            .try_wait()
            .map(|status| status.map(process_exit))
    }

    fn terminate(&self) -> io::Result<()> {
        let mut child = self
            .child
            .lock()
            .map_err(|_| poisoned_lock("system child"))?;
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        child.kill()
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
