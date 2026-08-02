//! Ownership and supervision for invocation-scoped operating-system processes.
//!
//! The supervisor owns each direct child from spawn through reap, including its stdout and stderr
//! reader threads in the normal path. After cancellation and observed direct-child exit, it bounds
//! still-open readers under a retained-cleanup limit rather than waiting forever. It deliberately
//! does not own persistence, provider protocol parsing, scheduling, or application lifecycle policy.
//! Callbacks are invoked without holding supervisor registry or child locks.
//!
//! SystemProcessFactory terminates only the direct process returned by std::process::Command::spawn.
//! Rust's portable process API does not guarantee descendant-tree termination. In particular,
//! Windows process-tree ownership requires a Job Object (or another platform-specific launcher)
//! supplied behind ChildProcessFactory; this implementation does not pretend that Child::kill
//! provides that guarantee.

mod monitoring;
mod supervisor;
mod system;

use crate::agent_sessions::domain::AgentInvocationId;
use std::{
    error::Error,
    fmt,
    io::{self, Read},
    path::PathBuf,
    sync::Arc,
};

#[allow(unused_imports)]
pub(crate) use supervisor::ProcessSupervisor;
#[allow(unused_imports)]
pub(crate) use system::SystemProcessFactory;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProcessLaunchSpec {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) working_directory: Option<PathBuf>,
    pub(crate) environment: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessOutputStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProcessOutput {
    pub(crate) stream: ProcessOutputStream,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProcessExit {
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<String>,
}

impl ProcessExit {
    fn succeeded(&self) -> bool {
        self.exit_code == Some(0) && self.signal.is_none()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessFailureKind {
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
pub(crate) enum ProcessTerminalOutcome {
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
pub(crate) trait ProcessEventSink: Send + Sync {
    fn on_output(&self, invocation_id: &AgentInvocationId, output: ProcessOutput);

    fn on_terminal(&self, invocation_id: &AgentInvocationId, outcome: ProcessTerminalOutcome);
}

/// Process control operations that may safely be called concurrently.
///
/// `try_wait` must be non-blocking. `wait_after_termination` is used only after termination has
/// been requested because a reader or wait operation failed. This contract describes the owned
/// process unit: the system implementation owns one direct child, while a future platform-specific
/// implementation may own a stronger unit such as a process tree without changing the supervisor.
pub(crate) trait SupervisedChild: Send + Sync {
    fn try_wait(&self) -> io::Result<Option<ProcessExit>>;

    fn terminate(&self) -> io::Result<()>;

    fn wait_after_termination(&self) -> io::Result<ProcessExit>;
}

pub(crate) struct SpawnedProcess {
    pub(crate) child: Arc<dyn SupervisedChild>,
    pub(crate) stdout: Box<dyn Read + Send>,
    pub(crate) stderr: Box<dyn Read + Send>,
}

/// Fakeable boundary around the platform-specific launch and child-control mechanism.
///
/// Stronger descendant ownership belongs in a replacement factory and matching
/// [`SupervisedChild`], not in provider adapters or `ProcessSupervisor`. The default system factory
/// intentionally promises direct-child ownership only.
pub(crate) trait ChildProcessFactory: Send + Sync {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SupervisorErrorKind {
    AlreadyActive,
    DuplicateInvocation,
    NotActive,
    SpawnFailed,
    CancellationFailed,
    ShuttingDown,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SupervisorError {
    pub(crate) kind: SupervisorErrorKind,
    pub(crate) message: String,
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
