use super::planning::{ActionPlan, PlanningError, ProcessCommand};
use std::{
    error::Error,
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    process::Command,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ActionExecution {
    pub(crate) succeeded: bool,
    pub(crate) failed_step: Option<String>,
}

pub(crate) trait ActionExecutor: Send + Sync {
    fn execute(&self, plan: &ActionPlan) -> Result<ActionExecution, ExecutionError>;
}

pub(crate) struct SystemActionExecutor;

impl ActionExecutor for SystemActionExecutor {
    fn execute(&self, plan: &ActionPlan) -> Result<ActionExecution, ExecutionError> {
        if let Some(parent) = plan.log_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| ExecutionError::context("create action log directory", error))?;
        }
        let mut log = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&plan.log_path)
            .map_err(|error| ExecutionError::context("open action log", error))?;
        for command in &plan.commands {
            writeln!(log, "== {} ==", command.label)
                .map_err(|error| ExecutionError::context("write action log", error))?;
            let output = execute(command)?;
            log.write_all(&output.stdout)
                .and_then(|_| log.write_all(&output.stderr))
                .map_err(|error| ExecutionError::context("write action output", error))?;
            if !output.status.success() {
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

fn execute(command: &ProcessCommand) -> Result<std::process::Output, ExecutionError> {
    Command::new(&command.program)
        .args(&command.arguments)
        .current_dir(&command.working_directory)
        .env_clear()
        .envs(&command.environment)
        .output()
        .map_err(|error| ExecutionError::context(format!("run {}", command.label), error))
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
