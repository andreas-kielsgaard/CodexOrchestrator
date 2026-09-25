//! Ownership and supervision for invocation-scoped operating-system processes.
//!
//! The supervisor owns each direct child from spawn through reap, including its stdout and stderr
//! reader threads. It deliberately does not own persistence, provider protocol parsing, scheduling,
//! or application lifecycle policy. Callbacks are invoked without holding supervisor registry or
//! child locks.
//!
//! On Windows, both system factories attach suspended children to a kill-on-close Job Object
//! before execution begins. The supervisor retains ownership until the process and readers settle.

pub mod json_lines;
mod monitoring;
mod supervisor;
mod system;
#[cfg(windows)]
pub mod windows_job;

use crate::contracts::domain::AgentInvocationId;
use std::{
    error::Error,
    fmt,
    io::{self, Read},
    path::PathBuf,
    sync::Arc,
};

#[allow(unused_imports)]
pub use supervisor::ProcessSupervisor;
pub use system::DuplexProcessFactory;
#[allow(unused_imports)]
pub use system::SystemProcessFactory;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessLaunchSpec {
    pub remove_environment: Vec<String>,
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub environment: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessOutputStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessOutput {
    pub stream: ProcessOutputStream,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessExit {
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
}

impl ProcessExit {
    fn succeeded(&self) -> bool {
        self.exit_code == Some(0) && self.signal.is_none()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessFailureKind {
    SpawnFailed,
    NonZeroExit,
    ReaderFailed,
    WaitFailed,
    CancellationFailed,
    SupervisorFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Terminal classification uses one fixed precedence after a successful spawn: shutdown,
/// cancellation (or cancellation failure), reader failure, wait failure, then exit status. Spawn
/// failure is emitted synchronously before any active process exists.
pub enum ProcessTerminalOutcome {
    Exited(ProcessExit),
    Failed {
        kind: ProcessFailureKind,
        exit: Option<ProcessExit>,
        message: String,
    },
    Canceled {
        exit: Option<ProcessExit>,
    },
    Interrupted {
        exit: Option<ProcessExit>,
    },
}

/// Receives raw process output and exactly one terminal outcome for every successfully reserved
/// invocation. Output calls for one invocation may come from two reader threads concurrently.
pub trait ProcessEventSink: Send + Sync {
    fn on_output(&self, invocation_id: &AgentInvocationId, output: ProcessOutput);

    fn on_terminal(&self, invocation_id: &AgentInvocationId, outcome: ProcessTerminalOutcome);
}

/// Process control operations that may safely be called concurrently.
///
/// `try_wait` must be non-blocking. `wait_after_termination` is used only after termination has
/// been requested because a reader or wait operation failed. This contract describes the owned
/// process unit: the system implementation owns one direct child, while a future platform-specific
/// implementation may own a stronger unit such as a process tree without changing the supervisor.
pub trait SupervisedChild: Send + Sync {
    fn write_input(&self, _bytes: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Process has no interactive input",
        ))
    }

    fn close_input(&self) -> io::Result<()> {
        Ok(())
    }

    fn try_wait(&self) -> io::Result<Option<ProcessExit>>;

    fn terminate(&self) -> io::Result<()>;

    fn wait_after_termination(&self) -> io::Result<ProcessExit>;
}

pub struct SpawnedProcess {
    pub child: Arc<dyn SupervisedChild>,
    pub stdout: Box<dyn Read + Send>,
    pub stderr: Box<dyn Read + Send>,
}

/// Fakeable boundary around the platform-specific launch and child-control mechanism.
///
/// Stronger descendant ownership belongs in a replacement factory and matching
/// [`SupervisedChild`], not in provider adapters or `ProcessSupervisor`. The default system factory
/// intentionally promises direct-child ownership only.
pub trait ChildProcessFactory: Send + Sync {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupervisorErrorKind {
    AlreadyActive,
    DuplicateInvocation,
    NotActive,
    SpawnFailed,
    CancellationFailed,
    ShuttingDown,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupervisorError {
    pub kind: SupervisorErrorKind,
    pub message: String,
}

impl SupervisorError {
    fn new(kind: SupervisorErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SupervisorError {}

#[cfg(test)]
mod tests;
