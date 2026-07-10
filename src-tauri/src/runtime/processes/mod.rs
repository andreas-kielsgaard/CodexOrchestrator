//! Ownership and supervision for invocation-scoped operating-system processes.
//!
//! The supervisor owns each direct child from spawn through reap, including its stdout and stderr
//! reader threads. It deliberately does not own persistence, provider protocol parsing, scheduling,
//! or application lifecycle policy. Callbacks are invoked without holding supervisor registry or
//! child locks.
//!
//! [`SystemProcessFactory`] terminates only the direct process returned by
//! [`std::process::Command::spawn`]. Rust's portable process API does not guarantee descendant-tree
//! termination. In particular, Windows process-tree ownership requires a Job Object (or another
//! platform-specific launcher) supplied behind [`ChildProcessFactory`]; this implementation does
//! not pretend that `Child::kill` provides that guarantee.

use crate::agent_sessions::domain::{AgentInvocationId, AgentSessionId};
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    io::{self, Read},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{mpsc, Arc, Condvar, Mutex, MutexGuard},
    thread::{self, JoinHandle},
    time::Duration,
};

const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);

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
/// been requested because a reader or wait operation failed.
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
pub(crate) trait ChildProcessFactory: Send + Sync {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess>;
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequestedTermination {
    Cancellation,
    Shutdown,
}

struct ActiveProcess {
    child: Arc<dyn SupervisedChild>,
    reader_threads: Mutex<Vec<JoinHandle<ReaderResult>>>,
    reader_results: Mutex<Option<mpsc::Receiver<ReaderResult>>>,
}

struct ActiveEntry {
    session_id: AgentSessionId,
    process: Option<Arc<ActiveProcess>>,
    requested_termination: Option<RequestedTermination>,
    termination_operations: usize,
    termination_error: Option<String>,
}

#[derive(Default)]
struct Registry {
    active: HashMap<AgentInvocationId, ActiveEntry>,
    sessions: HashMap<AgentSessionId, AgentInvocationId>,
    shutting_down: bool,
    terminal_callbacks_in_progress: usize,
}

struct SupervisorInner {
    factory: Arc<dyn ChildProcessFactory>,
    sink: Arc<dyn ProcessEventSink>,
    registry: Mutex<Registry>,
    changed: Condvar,
}

/// Owns all processes launched through it until they have exited and been reaped.
pub(crate) struct ProcessSupervisor {
    inner: Arc<SupervisorInner>,
}

impl ProcessSupervisor {
    pub(crate) fn system(sink: Arc<dyn ProcessEventSink>) -> Self {
        Self::new(Arc::new(SystemProcessFactory), sink)
    }

    pub(crate) fn new(
        factory: Arc<dyn ChildProcessFactory>,
        sink: Arc<dyn ProcessEventSink>,
    ) -> Self {
        Self {
            inner: Arc::new(SupervisorInner {
                factory,
                sink,
                registry: Mutex::new(Registry::default()),
                changed: Condvar::new(),
            }),
        }
    }

    pub(crate) fn start(
        &self,
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        spec: ProcessLaunchSpec,
    ) -> Result<(), SupervisorError> {
        self.reserve(session_id.clone(), invocation_id.clone())?;

        let spawned = match self.inner.factory.spawn(&spec) {
            Ok(spawned) => spawned,
            Err(error) => {
                self.finish_spawn_failure(&session_id, &invocation_id, &error);
                return Err(SupervisorError::new(
                    SupervisorErrorKind::SpawnFailed,
                    format!("failed to spawn invocation {invocation_id}: {error}"),
                ));
            }
        };

        let process = match start_readers(&self.inner, &invocation_id, spawned) {
            Ok(process) => process,
            Err((error, child, reader_threads)) => {
                let _ = child.terminate();
                let exit = child.wait_after_termination().ok();
                for reader in reader_threads {
                    let _ = reader.join();
                }
                self.finish_start_failure(
                    &session_id,
                    &invocation_id,
                    ProcessFailureKind::SupervisorFailed,
                    exit,
                    error.to_string(),
                );
                return Err(SupervisorError::new(
                    SupervisorErrorKind::Internal,
                    format!(
                        "failed to start output readers for invocation {invocation_id}: {error}"
                    ),
                ));
            }
        };

        let requested_termination = {
            let mut registry = self.lock_registry()?;
            let entry = registry.active.get_mut(&invocation_id).ok_or_else(|| {
                SupervisorError::new(
                    SupervisorErrorKind::Internal,
                    format!("invocation {invocation_id} lost its supervisor reservation"),
                )
            })?;
            entry.process = Some(process.clone());
            if entry.requested_termination.is_some() {
                entry.termination_operations += 1;
            }
            entry.requested_termination
        };

        let monitored_process = process.clone();
        let monitor = thread::Builder::new()
            .name(format!("agent-process-monitor-{invocation_id}"))
            .spawn({
                let inner = self.inner.clone();
                let invocation_id = invocation_id.clone();
                move || monitor_process(inner, invocation_id, monitored_process)
            });

        if let Err(error) = monitor {
            let termination_error = process
                .child
                .terminate()
                .err()
                .map(|error| error.to_string());
            let exit = process.child.wait_after_termination().ok();
            let reader_failure = join_readers(&process);
            self.finish_monitor_start_failure(
                &session_id,
                &invocation_id,
                exit,
                termination_error,
                reader_failure,
                error,
            );
            return Err(SupervisorError::new(
                SupervisorErrorKind::Internal,
                format!("failed to start monitor for invocation {invocation_id}"),
            ));
        }

        if requested_termination.is_some() {
            let result = process.child.terminate();
            self.record_termination_result(&invocation_id, result.as_ref().err());
        }

        Ok(())
    }

    pub(crate) fn cancel(&self, invocation_id: &AgentInvocationId) -> Result<(), SupervisorError> {
        let process = {
            let mut registry = self.lock_registry()?;
            if registry.shutting_down {
                return Err(SupervisorError::new(
                    SupervisorErrorKind::ShuttingDown,
                    "process supervisor is shutting down",
                ));
            }
            let entry = registry.active.get_mut(invocation_id).ok_or_else(|| {
                SupervisorError::new(
                    SupervisorErrorKind::NotActive,
                    format!("invocation {invocation_id} is not active"),
                )
            })?;
            if entry.requested_termination == Some(RequestedTermination::Cancellation) {
                return Ok(());
            }
            entry.requested_termination = Some(RequestedTermination::Cancellation);
            let process = entry.process.clone();
            if process.is_some() {
                entry.termination_operations += 1;
            }
            process
        };

        let Some(process) = process else {
            return Ok(());
        };
        let result = process.child.terminate();
        self.record_termination_result(invocation_id, result.as_ref().err());
        result.map_err(|error| {
            SupervisorError::new(
                SupervisorErrorKind::CancellationFailed,
                format!("failed to cancel invocation {invocation_id}: {error}"),
            )
        })
    }

    /// Stops accepting work, requests termination of every direct child, and waits until each
    /// process has been reaped and its terminal callback has returned. If direct-child termination
    /// fails, shutdown continues waiting for that child to exit before returning the control error;
    /// it never reports successful shutdown while silently detaching an owned process.
    pub(crate) fn shutdown(&self) -> Result<(), SupervisorError> {
        let processes = {
            let mut registry = self.lock_registry()?;
            registry.shutting_down = true;
            let mut processes = Vec::new();
            for (invocation_id, entry) in &mut registry.active {
                if entry.requested_termination != Some(RequestedTermination::Shutdown) {
                    entry.requested_termination = Some(RequestedTermination::Shutdown);
                    if let Some(process) = entry.process.clone() {
                        entry.termination_operations += 1;
                        processes.push((invocation_id.clone(), process));
                    }
                }
            }
            processes
        };

        let mut termination_failures = Vec::new();
        for (invocation_id, process) in processes {
            let result = process.child.terminate();
            if let Err(error) = &result {
                termination_failures.push(format!("{invocation_id}: {error}"));
            }
            self.record_termination_result(&invocation_id, result.as_ref().err());
        }

        let mut registry = self.lock_registry()?;
        while !registry.active.is_empty() || registry.terminal_callbacks_in_progress != 0 {
            registry = self.inner.changed.wait(registry).map_err(|_| {
                SupervisorError::new(
                    SupervisorErrorKind::Internal,
                    "process supervisor registry lock was poisoned while waiting for shutdown",
                )
            })?;
        }

        if termination_failures.is_empty() {
            Ok(())
        } else {
            Err(SupervisorError::new(
                SupervisorErrorKind::CancellationFailed,
                format!(
                    "failed to terminate one or more supervised processes: {}",
                    termination_failures.join("; ")
                ),
            ))
        }
    }

    pub(crate) fn active_count(&self) -> Result<usize, SupervisorError> {
        Ok(self.lock_registry()?.active.len())
    }

    pub(crate) fn is_active(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<bool, SupervisorError> {
        Ok(self.lock_registry()?.active.contains_key(invocation_id))
    }

    fn reserve(
        &self,
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
    ) -> Result<(), SupervisorError> {
        let mut registry = self.lock_registry()?;
        if registry.shutting_down {
            return Err(SupervisorError::new(
                SupervisorErrorKind::ShuttingDown,
                "process supervisor is shutting down",
            ));
        }
        if registry.active.contains_key(&invocation_id) {
            return Err(SupervisorError::new(
                SupervisorErrorKind::DuplicateInvocation,
                format!("invocation {invocation_id} is already active"),
            ));
        }
        if let Some(active_invocation_id) = registry.sessions.get(&session_id) {
            return Err(SupervisorError::new(
                SupervisorErrorKind::AlreadyActive,
                format!(
                    "session {session_id} already has active invocation {active_invocation_id}"
                ),
            ));
        }

        registry
            .sessions
            .insert(session_id.clone(), invocation_id.clone());
        registry.active.insert(
            invocation_id,
            ActiveEntry {
                session_id,
                process: None,
                requested_termination: None,
                termination_operations: 0,
                termination_error: None,
            },
        );
        Ok(())
    }

    fn finish_spawn_failure(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        error: &io::Error,
    ) {
        self.remove_and_emit(
            session_id,
            invocation_id,
            ProcessTerminalOutcome::Failed {
                kind: ProcessFailureKind::SpawnFailed,
                exit: None,
                message: error.to_string(),
            },
        );
    }

    fn finish_start_failure(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        kind: ProcessFailureKind,
        exit: Option<ProcessExit>,
        message: String,
    ) {
        self.remove_and_emit(
            session_id,
            invocation_id,
            ProcessTerminalOutcome::Failed {
                kind,
                exit,
                message,
            },
        );
    }

    fn finish_monitor_start_failure(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        exit: Option<ProcessExit>,
        termination_error: Option<String>,
        reader_failure: Option<String>,
        error: io::Error,
    ) {
        let mut details = vec![error.to_string()];
        if let Some(error) = termination_error {
            details.push(format!("termination failed: {error}"));
        }
        if let Some(error) = reader_failure {
            details.push(format!("reader failed: {error}"));
        }
        self.finish_start_failure(
            session_id,
            invocation_id,
            ProcessFailureKind::SupervisorFailed,
            exit,
            details.join("; "),
        );
    }

    fn remove_and_emit(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        outcome: ProcessTerminalOutcome,
    ) {
        let should_emit = if let Ok(mut registry) = self.inner.registry.lock() {
            let matches = registry
                .active
                .get(invocation_id)
                .is_some_and(|entry| &entry.session_id == session_id);
            if matches {
                registry.active.remove(invocation_id);
                if registry.sessions.get(session_id) == Some(invocation_id) {
                    registry.sessions.remove(session_id);
                }
                registry.terminal_callbacks_in_progress += 1;
                self.inner.changed.notify_all();
                true
            } else {
                false
            }
        } else {
            false
        };

        if should_emit {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.inner.sink.on_terminal(invocation_id, outcome)
            }));
            if let Ok(mut registry) = self.inner.registry.lock() {
                registry.terminal_callbacks_in_progress =
                    registry.terminal_callbacks_in_progress.saturating_sub(1);
                self.inner.changed.notify_all();
            }
        }
    }

    fn record_termination_result(
        &self,
        invocation_id: &AgentInvocationId,
        error: Option<&io::Error>,
    ) {
        if let Ok(mut registry) = self.inner.registry.lock() {
            if let Some(entry) = registry.active.get_mut(invocation_id) {
                entry.termination_operations = entry.termination_operations.saturating_sub(1);
                entry.termination_error = error.map(ToString::to_string);
            }
            self.inner.changed.notify_all();
        }
    }

    fn lock_registry(&self) -> Result<MutexGuard<'_, Registry>, SupervisorError> {
        self.inner.registry.lock().map_err(|_| {
            SupervisorError::new(
                SupervisorErrorKind::Internal,
                "process supervisor registry lock was poisoned",
            )
        })
    }
}

impl Drop for ProcessSupervisor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[derive(Debug)]
struct ReaderResult {
    stream: ProcessOutputStream,
    error: Option<String>,
}

fn start_readers(
    inner: &Arc<SupervisorInner>,
    invocation_id: &AgentInvocationId,
    spawned: SpawnedProcess,
) -> Result<
    Arc<ActiveProcess>,
    (
        io::Error,
        Arc<dyn SupervisedChild>,
        Vec<JoinHandle<ReaderResult>>,
    ),
> {
    let (sender, receiver) = mpsc::channel();
    let mut threads = Vec::with_capacity(2);

    for (stream, reader) in [
        (ProcessOutputStream::Stdout, spawned.stdout),
        (ProcessOutputStream::Stderr, spawned.stderr),
    ] {
        let sink = inner.sink.clone();
        let invocation_id = invocation_id.clone();
        let sender = sender.clone();
        let thread_name = match stream {
            ProcessOutputStream::Stdout => format!("agent-stdout-{invocation_id}"),
            ProcessOutputStream::Stderr => format!("agent-stderr-{invocation_id}"),
        };
        let handle = match thread::Builder::new().name(thread_name).spawn(move || {
            let error = read_output(reader, &sink, &invocation_id, stream)
                .err()
                .map(|error| error.to_string());
            let _ = sender.send(ReaderResult {
                stream,
                error: error.clone(),
            });
            ReaderResult { stream, error }
        }) {
            Ok(handle) => handle,
            Err(error) => return Err((error, spawned.child, threads)),
        };
        threads.push(handle);
    }
    drop(sender);

    Ok(Arc::new(ActiveProcess {
        child: spawned.child,
        reader_threads: Mutex::new(threads),
        reader_results: Mutex::new(Some(receiver)),
    }))
}

fn read_output(
    mut reader: Box<dyn Read + Send>,
    sink: &Arc<dyn ProcessEventSink>,
    invocation_id: &AgentInvocationId,
    stream: ProcessOutputStream,
) -> io::Result<()> {
    let mut buffer = [0_u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(length) => {
                let output = ProcessOutput {
                    stream,
                    bytes: buffer[..length].to_vec(),
                };
                let callback = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    sink.on_output(invocation_id, output)
                }));
                if callback.is_err() {
                    return Err(io::Error::other("process output callback panicked"));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

fn monitor_process(
    inner: Arc<SupervisorInner>,
    invocation_id: AgentInvocationId,
    process: Arc<ActiveProcess>,
) {
    let receiver = process
        .reader_results
        .lock()
        .ok()
        .and_then(|mut receiver| receiver.take());
    let mut first_reader_failure = None;
    let mut wait_failure = None;

    let exit = loop {
        if let Some(receiver) = &receiver {
            while let Ok(result) = receiver.try_recv() {
                if let Some(error) = result.error {
                    first_reader_failure.get_or_insert_with(|| {
                        format!("{:?} reader failed: {error}", result.stream)
                    });
                }
            }
        }

        if first_reader_failure.is_some() {
            let termination_error = process.child.terminate().err();
            match process.child.wait_after_termination() {
                Ok(exit) => break Some(exit),
                Err(error) => {
                    let mut message =
                        format!("failed to reap process after reader failure: {error}");
                    if let Some(termination_error) = termination_error {
                        message.push_str(&format!("; termination failed: {termination_error}"));
                    }
                    wait_failure = Some(message);
                    break None;
                }
            }
        }

        match process.child.try_wait() {
            Ok(Some(exit)) => break Some(exit),
            Ok(None) => thread::sleep(PROCESS_POLL_INTERVAL),
            Err(error) => {
                let termination_error = process.child.terminate().err();
                wait_failure = Some(format!("failed to query process exit: {error}"));
                match process.child.wait_after_termination() {
                    Ok(exit) => break Some(exit),
                    Err(error) => {
                        let message = wait_failure.get_or_insert_with(String::new);
                        message.push_str(&format!("; failed to reap process: {error}"));
                        if let Some(termination_error) = termination_error {
                            message.push_str(&format!("; termination failed: {termination_error}"));
                        }
                        break None;
                    }
                }
            }
        }
    };

    if let Some(reader_failure) = join_readers(&process) {
        first_reader_failure.get_or_insert(reader_failure);
    }

    finalize_process(
        &inner,
        &invocation_id,
        &process,
        exit,
        first_reader_failure,
        wait_failure,
    );
}

fn join_readers(process: &ActiveProcess) -> Option<String> {
    let Ok(mut readers) = process.reader_threads.lock() else {
        return Some("reader thread registry lock was poisoned".to_string());
    };
    let mut first_failure = None;
    for reader in readers.drain(..) {
        match reader.join() {
            Ok(result) => {
                if let Some(error) = result.error {
                    first_failure.get_or_insert_with(|| {
                        format!("{:?} reader failed: {error}", result.stream)
                    });
                }
            }
            Err(_) => {
                first_failure.get_or_insert_with(|| "process reader thread panicked".to_string());
            }
        }
    }
    first_failure
}

fn finalize_process(
    inner: &Arc<SupervisorInner>,
    invocation_id: &AgentInvocationId,
    process: &Arc<ActiveProcess>,
    exit: Option<ProcessExit>,
    reader_failure: Option<String>,
    wait_failure: Option<String>,
) {
    let (requested, termination_error) = {
        let Ok(mut registry) = inner.registry.lock() else {
            return;
        };
        loop {
            let Some(entry) = registry.active.get(invocation_id) else {
                return;
            };
            if entry
                .process
                .as_ref()
                .is_none_or(|active| !Arc::ptr_eq(active, process))
            {
                return;
            }
            if entry.termination_operations == 0 {
                break;
            }
            let Ok(next_registry) = inner.changed.wait(registry) else {
                return;
            };
            registry = next_registry;
        }

        let entry = registry
            .active
            .remove(invocation_id)
            .expect("active process was checked before removal");
        if registry.sessions.get(&entry.session_id) == Some(invocation_id) {
            registry.sessions.remove(&entry.session_id);
        }
        registry.terminal_callbacks_in_progress += 1;
        inner.changed.notify_all();
        (entry.requested_termination, entry.termination_error)
    };

    let outcome = terminal_outcome(
        requested,
        termination_error,
        exit,
        reader_failure,
        wait_failure,
    );
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        inner.sink.on_terminal(invocation_id, outcome)
    }));

    if let Ok(mut registry) = inner.registry.lock() {
        registry.terminal_callbacks_in_progress =
            registry.terminal_callbacks_in_progress.saturating_sub(1);
        inner.changed.notify_all();
    }
}

fn terminal_outcome(
    requested: Option<RequestedTermination>,
    termination_error: Option<String>,
    exit: Option<ProcessExit>,
    reader_failure: Option<String>,
    wait_failure: Option<String>,
) -> ProcessTerminalOutcome {
    match requested {
        Some(RequestedTermination::Shutdown) => ProcessTerminalOutcome::Interrupted { exit },
        Some(RequestedTermination::Cancellation) if termination_error.is_some() => {
            ProcessTerminalOutcome::Failed {
                kind: ProcessFailureKind::CancellationFailed,
                exit,
                message: termination_error.expect("termination error was checked"),
            }
        }
        Some(RequestedTermination::Cancellation) => ProcessTerminalOutcome::Canceled { exit },
        None if reader_failure.is_some() => ProcessTerminalOutcome::Failed {
            kind: ProcessFailureKind::ReaderFailed,
            exit,
            message: reader_failure.expect("reader failure was checked"),
        },
        None if wait_failure.is_some() => ProcessTerminalOutcome::Failed {
            kind: ProcessFailureKind::WaitFailed,
            exit,
            message: wait_failure.expect("wait failure was checked"),
        },
        None => match exit {
            Some(exit) if exit.succeeded() => ProcessTerminalOutcome::Exited(exit),
            Some(exit) => ProcessTerminalOutcome::Failed {
                kind: ProcessFailureKind::NonZeroExit,
                message: match (&exit.exit_code, &exit.signal) {
                    (_, Some(signal)) => format!("process exited on signal {signal}"),
                    (Some(code), None) => format!("process exited with code {code}"),
                    (None, None) => "process exited without an exit code".to_string(),
                },
                exit: Some(exit),
            },
            None => ProcessTerminalOutcome::Failed {
                kind: ProcessFailureKind::WaitFailed,
                exit: None,
                message: "process ended without an observable exit status".to_string(),
            },
        },
    }
}

fn poisoned_lock(name: &str) -> io::Error {
    io::Error::other(format!("{name} lock was poisoned"))
}

#[cfg(test)]
mod tests;
