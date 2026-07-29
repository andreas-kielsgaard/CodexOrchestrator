use super::planning::{ActionPlan, PlanningError, ProcessCommand};
use std::{
    error::Error,
    fmt,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ActionExecution {
    pub(crate) succeeded: bool,
    pub(crate) failed_step: Option<String>,
}

pub(crate) trait ActionExecutor: Send + Sync {
    fn execute(
        &self,
        plan: &ActionPlan,
        progress: &dyn ActionProgressObserver,
    ) -> Result<ActionExecution, ExecutionError>;
}

pub(crate) enum ActionProgressEvent<'a> {
    Started { step: &'a str },
    Output { step: &'a str, line: &'a str },
    Finished { step: &'a str, succeeded: bool },
}

pub(crate) trait ActionProgressObserver: Send + Sync {
    fn progress(&self, event: ActionProgressEvent<'_>);
}

pub(crate) struct NoopActionProgressObserver;

impl ActionProgressObserver for NoopActionProgressObserver {
    fn progress(&self, _event: ActionProgressEvent<'_>) {}
}

pub(crate) struct SystemActionExecutor;

impl ActionExecutor for SystemActionExecutor {
    fn execute(
        &self,
        plan: &ActionPlan,
        progress: &dyn ActionProgressObserver,
    ) -> Result<ActionExecution, ExecutionError> {
        if let Some(parent) = plan.log_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| ExecutionError::context("create action log directory", error))?;
        }
        let log = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&plan.log_path)
            .map_err(|error| ExecutionError::context("open action log", error))?;
        let log = Arc::new(Mutex::new(log));
        for command in &plan.commands {
            {
                let mut log = log
                    .lock()
                    .map_err(|_| ExecutionError::new("action log lock was poisoned"))?;
                writeln!(log, "== {} ==", command.label)
                    .map_err(|error| ExecutionError::context("write action log", error))?;
            }
            progress.progress(ActionProgressEvent::Started {
                step: command.label,
            });
            let succeeded = execute(command, &log, progress)?;
            progress.progress(ActionProgressEvent::Finished {
                step: command.label,
                succeeded,
            });
            if !succeeded {
                return Ok(ActionExecution {
                    succeeded: false,
                    failed_step: Some(command.label.into()),
                });
            }
        }
        Ok(ActionExecution {
            succeeded: true,
            failed_step: None,
        })
    }
}

fn execute(
    command: &ProcessCommand,
    log: &Arc<Mutex<std::fs::File>>,
    progress: &dyn ActionProgressObserver,
) -> Result<bool, ExecutionError> {
    let mut child = Command::new(&command.program)
        .args(&command.arguments)
        .current_dir(&command.working_directory)
        .env_clear()
        .envs(&command.environment)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ExecutionError::context(format!("run {}", command.label), error))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ExecutionError::new("capture action stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ExecutionError::new("capture action stderr"))?;
    let (status_result, stdout_result, stderr_result) = thread::scope(|scope| {
        let stdout_handle = scope.spawn(|| pump(stdout, command.label, log.as_ref(), progress));
        let stderr_handle = scope.spawn(|| pump(stderr, command.label, log.as_ref(), progress));
        let status = child
            .wait()
            .map_err(|error| ExecutionError::context(format!("wait for {}", command.label), error));
        (
            status,
            stdout_handle
                .join()
                .unwrap_or_else(|_| Err(ExecutionError::new("action stdout reader panicked"))),
            stderr_handle
                .join()
                .unwrap_or_else(|_| Err(ExecutionError::new("action stderr reader panicked"))),
        )
    });
    let status = status_result?;
    stdout_result?;
    stderr_result?;
    Ok(status.success())
}

fn pump(
    reader: impl Read,
    step: &str,
    log: &Mutex<std::fs::File>,
    progress: &dyn ActionProgressObserver,
) -> Result<(), ExecutionError> {
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        let read = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| ExecutionError::context("read action output", error))?;
        if read == 0 {
            break;
        }
        while matches!(bytes.last(), Some(b'\n' | b'\r')) {
            bytes.pop();
        }
        let line = String::from_utf8_lossy(&bytes);
        {
            let mut log = log
                .lock()
                .map_err(|_| ExecutionError::new("action log lock was poisoned"))?;
            writeln!(log, "{line}")
                .map_err(|error| ExecutionError::context("write action output", error))?;
        }
        progress.progress(ActionProgressEvent::Output { step, line: &line });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Cursor};

    #[test]
    fn action_output_is_recorded_lossily_instead_of_failing_the_owned_action() {
        let directory = tempfile::tempdir().expect("directory");
        let path = directory.path().join("action.log");
        let log = Mutex::new(File::create(&path).expect("log"));
        pump(
            Cursor::new([b'o', b'k', b'\n', 0xff, b'\n']),
            "fixture",
            &log,
            &NoopActionProgressObserver,
        )
        .expect("lossy output");
        drop(log);
        let output = fs::read_to_string(path).expect("output");
        assert!(output.contains("ok"));
        assert!(output.contains('\u{fffd}'));
    }
}

impl From<PlanningError> for ExecutionError {
    fn from(error: PlanningError) -> Self {
        Self::new(error.message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExecutionError {
    pub(crate) message: String,
}

impl ExecutionError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn context(operation: impl AsRef<str>, error: impl fmt::Display) -> Self {
        Self::new(format!("{}: {error}", operation.as_ref()))
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ExecutionError {}
