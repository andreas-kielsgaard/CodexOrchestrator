use super::{
    monitoring::{join_readers, monitor_process, start_readers, ActiveProcess, ReaderResult},
    system::SystemProcessFactory,
    ChildProcessFactory, ProcessEventSink, ProcessExit, ProcessFailureKind, ProcessLaunchSpec,
    ProcessTerminalOutcome, SupervisorError, SupervisorErrorKind,
};
use crate::agent_sessions::domain::{AgentInvocationId, AgentSessionId};
use std::{
    collections::HashMap,
    io,
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RequestedTermination {
    Cancellation,
    Shutdown,
}

pub(super) struct ActiveEntry {
    pub(super) session_id: AgentSessionId,
    pub(super) process: Option<Arc<ActiveProcess>>,
    pub(super) requested_termination: Option<RequestedTermination>,
    pub(super) reader_cleanup_slots: usize,
    pub(super) termination_operations: usize,
    pub(super) termination_error: Option<String>,
}

#[derive(Default)]
pub(super) struct Registry {
    pub(super) active: HashMap<AgentInvocationId, ActiveEntry>,
    pub(super) sessions: HashMap<AgentSessionId, AgentInvocationId>,
    pub(super) shutting_down: bool,
    pub(super) terminal_callbacks_in_progress: usize,
    pub(super) retained_reader_threads: Vec<thread::JoinHandle<ReaderResult>>,
    pub(super) reserved_reader_cleanup_slots: usize,
}

pub(super) struct SupervisorInner {
    pub(super) factory: Arc<dyn ChildProcessFactory>,
    pub(super) sink: Arc<dyn ProcessEventSink>,
    pub(super) registry: Mutex<Registry>,
    pub(super) changed: Condvar,
}

const RETAINED_READER_THREAD_LIMIT: usize = 2;

/// Owns each direct child through reap and bounds retained reader threads after cancellation.
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

    /// Reserves the invocation before launching its child process.
    ///
    /// If spawning fails after reservation, the supervisor emits exactly one terminal
    /// [`ProcessTerminalOutcome::Failed`] callback and also returns [`SupervisorError`]. Callers
    /// such as AS-05 must treat that callback as the terminal result and must not synthesize a
    /// duplicate completion from the returned error.
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
        self.reap_retained_reader_threads();
        let process = {
            let mut registry = self.lock_registry()?;
            if registry.shutting_down {
                return Err(SupervisorError::new(
                    SupervisorErrorKind::ShuttingDown,
                    "process supervisor is shutting down",
                ));
            }
            let already_canceling = registry
                .active
                .get(invocation_id)
                .ok_or_else(|| {
                    SupervisorError::new(
                        SupervisorErrorKind::NotActive,
                        format!("invocation {invocation_id} is not active"),
                    )
                })?
                .requested_termination
                == Some(RequestedTermination::Cancellation);
            if already_canceling {
                return Ok(());
            }
            if registry.retained_reader_threads.len()
                + registry.reserved_reader_cleanup_slots
                + RETAINED_READER_THREAD_LIMIT
                > RETAINED_READER_THREAD_LIMIT
            {
                return Err(SupervisorError::new(
                    SupervisorErrorKind::CancellationFailed,
                    "refusing cancellation: retained output-reader cleanup capacity is exhausted",
                ));
            }
            let process = {
                let entry = registry.active.get_mut(invocation_id).ok_or_else(|| {
                    SupervisorError::new(
                        SupervisorErrorKind::NotActive,
                        format!("invocation {invocation_id} is not active"),
                    )
                })?;
                entry.requested_termination = Some(RequestedTermination::Cancellation);
                entry.reader_cleanup_slots = RETAINED_READER_THREAD_LIMIT;
                let process = entry.process.clone();
                if process.is_some() {
                    entry.termination_operations += 1;
                }
                process
            };
            registry.reserved_reader_cleanup_slots += RETAINED_READER_THREAD_LIMIT;
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
        self.shutdown_with_grace_period(Duration::ZERO)
    }

    /// Stops accepting work and gives active children a bounded opportunity to finish naturally
    /// before requesting termination. Ownership is retained through escalation and reap.
    pub(crate) fn shutdown_with_grace_period(
        &self,
        grace_period: Duration,
    ) -> Result<(), SupervisorError> {
        let grace_deadline = Instant::now() + grace_period;
        let mut registry = self.lock_registry()?;
        registry.shutting_down = true;
        while (!registry.active.is_empty() || registry.terminal_callbacks_in_progress != 0)
            && Instant::now() < grace_deadline
        {
            let remaining = grace_deadline.saturating_duration_since(Instant::now());
            let (next, _) = self.inner.changed.wait_timeout(registry, remaining).map_err(|_| {
                SupervisorError::new(
                    SupervisorErrorKind::Internal,
                    "process supervisor registry lock was poisoned while awaiting graceful shutdown",
                )
            })?;
            registry = next;
        }

        let processes = {
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
        drop(registry);

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

        drop(registry);
        self.reap_retained_reader_threads();
        let retained = self.lock_registry()?.retained_reader_threads.len();
        if !termination_failures.is_empty() {
            Err(SupervisorError::new(
                SupervisorErrorKind::CancellationFailed,
                format!(
                    "failed to terminate one or more supervised processes: {}",
                    termination_failures.join("; ")
                ),
            ))
        } else if retained != 0 {
            Err(SupervisorError::new(
                SupervisorErrorKind::Internal,
                format!(
                    "shutdown completed direct-child cleanup but {retained} retained output reader(s) remain"
                ),
            ))
        } else {
            Ok(())
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

    #[cfg(test)]
    pub(crate) fn retained_reader_count(&self) -> Result<usize, SupervisorError> {
        self.reap_retained_reader_threads();
        Ok(self.lock_registry()?.retained_reader_threads.len())
    }

    fn reap_retained_reader_threads(&self) {
        let completed = {
            let Ok(mut registry) = self.inner.registry.lock() else {
                return;
            };
            let mut completed = Vec::new();
            let mut retained = Vec::new();
            for handle in registry.retained_reader_threads.drain(..) {
                if handle.is_finished() {
                    completed.push(handle);
                } else {
                    retained.push(handle);
                }
            }
            registry.retained_reader_threads = retained;
            completed
        };
        for handle in completed {
            let _ = handle.join();
        }
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
                reader_cleanup_slots: 0,
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

pub(super) fn retain_cancellation_readers(
    inner: &Arc<SupervisorInner>,
    invocation_id: &AgentInvocationId,
    process: &Arc<ActiveProcess>,
) -> bool {
    let Ok(mut readers) = process.reader_threads.lock() else {
        return false;
    };
    let retained = readers.drain(..).collect::<Vec<_>>();
    drop(readers);
    let Ok(mut registry) = inner.registry.lock() else {
        return false;
    };
    let Some(entry) = registry.active.get_mut(invocation_id) else {
        return false;
    };
    if entry
        .process
        .as_ref()
        .is_none_or(|active| !Arc::ptr_eq(active, process))
    {
        return false;
    }
    let slots = entry.reader_cleanup_slots;
    entry.reader_cleanup_slots = 0;
    registry.reserved_reader_cleanup_slots =
        registry.reserved_reader_cleanup_slots.saturating_sub(slots);
    registry.retained_reader_threads.extend(retained);
    inner.changed.notify_all();
    true
}

impl Drop for ProcessSupervisor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
