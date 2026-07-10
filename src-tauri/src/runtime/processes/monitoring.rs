use super::{
    supervisor::{RequestedTermination, SupervisorInner},
    ProcessEventSink, ProcessExit, ProcessFailureKind, ProcessOutput, ProcessOutputStream,
    ProcessTerminalOutcome, SpawnedProcess, SupervisedChild,
};
use crate::agent_sessions::domain::AgentInvocationId;
use std::{
    io::{self, Read},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(super) struct ActiveProcess {
    pub(super) child: Arc<dyn SupervisedChild>,
    pub(super) reader_threads: Mutex<Vec<JoinHandle<ReaderResult>>>,
    pub(super) reader_results: Mutex<Option<mpsc::Receiver<ReaderResult>>>,
}

#[derive(Debug)]
pub(super) struct ReaderResult {
    stream: ProcessOutputStream,
    error: Option<String>,
}

pub(super) fn start_readers(
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

pub(super) fn monitor_process(
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

pub(super) fn join_readers(process: &ActiveProcess) -> Option<String> {
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
