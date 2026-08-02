use super::*;
use crate::agent_sessions::domain::AgentSessionId;
use std::{
    collections::VecDeque,
    io::Cursor,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Default)]
struct RecordingSink {
    outputs: Mutex<Vec<(AgentInvocationId, ProcessOutput)>>,
    terminals: Mutex<Vec<(AgentInvocationId, ProcessTerminalOutcome)>>,
    changed: Condvar,
}

#[derive(Default)]
struct BlockingTerminalSink {
    entered: Mutex<bool>,
    released: Mutex<bool>,
    changed: Condvar,
}

impl ProcessEventSink for BlockingTerminalSink {
    fn on_output(&self, _invocation_id: &AgentInvocationId, _output: ProcessOutput) {}

    fn on_terminal(&self, _invocation_id: &AgentInvocationId, _outcome: ProcessTerminalOutcome) {
        *self.entered.lock().expect("lock entered") = true;
        self.changed.notify_all();
        let mut released = self.released.lock().expect("lock released");
        while !*released {
            released = self.changed.wait(released).expect("wait for release");
        }
    }
}

impl BlockingTerminalSink {
    fn wait_until_entered(&self) {
        let mut entered = self.entered.lock().expect("lock entered");
        while !*entered {
            entered = self.changed.wait(entered).expect("wait for callback");
        }
    }

    fn release(&self) {
        *self.released.lock().expect("lock released") = true;
        self.changed.notify_all();
    }
}

impl ProcessEventSink for RecordingSink {
    fn on_output(&self, invocation_id: &AgentInvocationId, output: ProcessOutput) {
        self.outputs
            .lock()
            .expect("lock outputs")
            .push((invocation_id.clone(), output));
    }

    fn on_terminal(&self, invocation_id: &AgentInvocationId, outcome: ProcessTerminalOutcome) {
        self.terminals
            .lock()
            .expect("lock terminals")
            .push((invocation_id.clone(), outcome));
        self.changed.notify_all();
    }
}

impl RecordingSink {
    fn wait_for_terminal(&self, invocation_id: &AgentInvocationId) -> ProcessTerminalOutcome {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut terminals = self.terminals.lock().expect("lock terminals");
        loop {
            if let Some((_, outcome)) = terminals.iter().find(|(id, _)| id == invocation_id) {
                return outcome.clone();
            }
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .expect("timed out waiting for terminal outcome");
            let (next, timeout) = self
                .changed
                .wait_timeout(terminals, remaining)
                .expect("wait for terminal");
            terminals = next;
            assert!(
                !timeout.timed_out(),
                "timed out waiting for terminal outcome"
            );
        }
    }
}

enum SpawnPlan {
    Process {
        child: Arc<FakeChild>,
        stdout: Box<dyn Read + Send>,
        stderr: Box<dyn Read + Send>,
    },
    Failure(io::ErrorKind, &'static str),
}

#[derive(Default)]
struct FakeFactory {
    plans: Mutex<VecDeque<SpawnPlan>>,
    specs: Mutex<Vec<ProcessLaunchSpec>>,
}

impl FakeFactory {
    fn push_process(&self, child: Arc<FakeChild>) {
        self.plans
            .lock()
            .expect("lock plans")
            .push_back(SpawnPlan::Process {
                child,
                stdout: Box::new(Cursor::new(Vec::<u8>::new())),
                stderr: Box::new(Cursor::new(Vec::<u8>::new())),
            });
    }

    fn push_process_with_readers(
        &self,
        child: Arc<FakeChild>,
        stdout: Box<dyn Read + Send>,
        stderr: Box<dyn Read + Send>,
    ) {
        self.plans
            .lock()
            .expect("lock plans")
            .push_back(SpawnPlan::Process {
                child,
                stdout,
                stderr,
            });
    }

    fn push_failure(&self, kind: io::ErrorKind, message: &'static str) {
        self.plans
            .lock()
            .expect("lock plans")
            .push_back(SpawnPlan::Failure(kind, message));
    }
}

impl ChildProcessFactory for FakeFactory {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess> {
        self.specs.lock().expect("lock specs").push(spec.clone());
        match self.plans.lock().expect("lock plans").pop_front() {
            Some(SpawnPlan::Process {
                child,
                stdout,
                stderr,
            }) => Ok(SpawnedProcess {
                child,
                stdout,
                stderr,
            }),
            Some(SpawnPlan::Failure(kind, message)) => Err(io::Error::new(kind, message)),
            None => panic!("fake process plan was not configured"),
        }
    }
}

#[derive(Default)]
struct FakeChildState {
    exit: Option<ProcessExit>,
    terminate_error: Option<io::ErrorKind>,
    wait_query_error: Option<io::ErrorKind>,
}

#[derive(Default)]
struct FakeChild {
    state: Mutex<FakeChildState>,
    changed: Condvar,
    terminate_calls: AtomicUsize,
}

impl FakeChild {
    fn complete(&self, exit_code: i32) {
        self.state.lock().expect("lock child").exit = Some(ProcessExit {
            exit_code: Some(exit_code),
            signal: None,
        });
        self.changed.notify_all();
    }

    fn fail_next_termination(&self, kind: io::ErrorKind) {
        self.state.lock().expect("lock child").terminate_error = Some(kind);
    }

    fn fail_next_wait_query(&self, kind: io::ErrorKind) {
        self.state.lock().expect("lock child").wait_query_error = Some(kind);
    }
}

impl SupervisedChild for FakeChild {
    fn try_wait(&self) -> io::Result<Option<ProcessExit>> {
        let mut state = self.state.lock().expect("lock child");
        if let Some(kind) = state.wait_query_error.take() {
            return Err(io::Error::new(kind, "fake wait query failure"));
        }
        Ok(state.exit.clone())
    }

    fn terminate(&self) -> io::Result<()> {
        self.terminate_calls.fetch_add(1, Ordering::SeqCst);
        let mut state = self.state.lock().expect("lock child");
        if let Some(kind) = state.terminate_error.take() {
            return Err(io::Error::new(kind, "fake termination failure"));
        }
        state.exit.get_or_insert(ProcessExit {
            exit_code: Some(1),
            signal: None,
        });
        self.changed.notify_all();
        Ok(())
    }

    fn wait_after_termination(&self) -> io::Result<ProcessExit> {
        let mut state = self.state.lock().expect("lock child");
        while state.exit.is_none() {
            state = self.changed.wait(state).expect("wait for fake exit");
        }
        Ok(state.exit.clone().expect("exit checked"))
    }
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "broken pipe reader",
        ))
    }
}

#[derive(Clone, Default)]
struct BlockingReader {
    released: Arc<(Mutex<bool>, Condvar)>,
}

impl BlockingReader {
    fn release(&self) {
        let (released, changed) = &*self.released;
        *released.lock().expect("lock reader release") = true;
        changed.notify_all();
    }
}

impl Read for BlockingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        let (released, changed) = &*self.released;
        let mut released = released.lock().expect("lock reader release");
        while !*released {
            released = changed.wait(released).expect("wait for reader release");
        }
        Ok(0)
    }
}

fn session_id(value: &str) -> AgentSessionId {
    AgentSessionId::new(value).expect("session id")
}

fn invocation_id(value: &str) -> AgentInvocationId {
    AgentInvocationId::new(value).expect("invocation id")
}

fn spec() -> ProcessLaunchSpec {
    ProcessLaunchSpec {
        program: "fake-runtime".to_string(),
        args: vec!["--json".to_string()],
        working_directory: None,
        environment: Vec::new(),
    }
}

fn fixture() -> (Arc<FakeFactory>, Arc<RecordingSink>, ProcessSupervisor) {
    let factory = Arc::new(FakeFactory::default());
    let sink = Arc::new(RecordingSink::default());
    let supervisor = ProcessSupervisor::new(factory.clone(), sink.clone());
    (factory, sink, supervisor)
}

#[test]
fn allows_concurrent_processes_for_different_sessions() {
    let (factory, sink, supervisor) = fixture();
    let first_child = Arc::new(FakeChild::default());
    let second_child = Arc::new(FakeChild::default());
    factory.push_process(first_child.clone());
    factory.push_process(second_child.clone());
    let first = invocation_id("invocation-1");
    let second = invocation_id("invocation-2");

    supervisor
        .start(session_id("session-1"), first.clone(), spec())
        .expect("start first");
    supervisor
        .start(session_id("session-2"), second.clone(), spec())
        .expect("start second");

    assert_eq!(supervisor.active_count().expect("active count"), 2);
    first_child.complete(0);
    second_child.complete(0);
    assert_eq!(
        sink.wait_for_terminal(&first),
        ProcessTerminalOutcome::Exited(ProcessExit {
            exit_code: Some(0),
            signal: None
        })
    );
    assert_eq!(
        sink.wait_for_terminal(&second),
        ProcessTerminalOutcome::Exited(ProcessExit {
            exit_code: Some(0),
            signal: None
        })
    );
    assert_eq!(supervisor.active_count().expect("active count"), 0);
}

#[test]
fn rejects_a_second_active_invocation_for_the_same_session() {
    let (factory, _sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    factory.push_process(child.clone());
    let session = session_id("session-1");
    supervisor
        .start(session.clone(), invocation_id("invocation-1"), spec())
        .expect("start first");

    let error = supervisor
        .start(session, invocation_id("invocation-2"), spec())
        .expect_err("duplicate session must fail");

    assert_eq!(error.kind, SupervisorErrorKind::AlreadyActive);
    assert_eq!(factory.specs.lock().expect("lock specs").len(), 1);
    child.complete(0);
}

#[test]
fn spawn_failure_emits_failure_and_releases_the_session() {
    let (factory, sink, supervisor) = fixture();
    factory.push_failure(io::ErrorKind::NotFound, "runtime missing");
    let session = session_id("session-1");
    let first = invocation_id("invocation-1");

    let error = supervisor
        .start(session.clone(), first.clone(), spec())
        .expect_err("spawn must fail");

    assert_eq!(error.kind, SupervisorErrorKind::SpawnFailed);
    assert_eq!(
        sink.wait_for_terminal(&first),
        ProcessTerminalOutcome::Failed {
            kind: ProcessFailureKind::SpawnFailed,
            exit: None,
            message: "runtime missing".to_string(),
        }
    );
    let next_child = Arc::new(FakeChild::default());
    factory.push_process(next_child.clone());
    supervisor
        .start(session, invocation_id("invocation-2"), spec())
        .expect("session reservation was released");
    next_child.complete(0);
}

#[test]
fn cancellation_terminates_the_child_and_emits_canceled_once() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    factory.push_process(child.clone());
    let invocation = invocation_id("invocation-1");
    supervisor
        .start(session_id("session-1"), invocation.clone(), spec())
        .expect("start");

    supervisor.cancel(&invocation).expect("cancel");
    supervisor
        .cancel(&invocation)
        .expect("repeated cancel is idempotent");

    assert_eq!(child.terminate_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        sink.wait_for_terminal(&invocation),
        ProcessTerminalOutcome::Canceled {
            exit: Some(ProcessExit {
                exit_code: Some(1),
                signal: None
            })
        }
    );
    assert!(!supervisor.is_active(&invocation).expect("active state"));
    assert_eq!(sink.terminals.lock().expect("lock terminals").len(), 1);
}

#[test]
fn cancellation_settles_after_direct_exit_when_an_unowned_reader_stays_open() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    let stdout = BlockingReader::default();
    factory.push_process_with_readers(
        child,
        Box::new(stdout.clone()),
        Box::new(Cursor::new(Vec::<u8>::new())),
    );
    let invocation = invocation_id("invocation-cancellation-reader-boundary");
    supervisor
        .start(
            session_id("session-cancellation-reader-boundary"),
            invocation.clone(),
            spec(),
        )
        .expect("start");

    supervisor.cancel(&invocation).expect("cancel");

    assert!(matches!(
        sink.wait_for_terminal(&invocation),
        ProcessTerminalOutcome::Canceled { .. }
    ));
    assert!(!supervisor.is_active(&invocation).expect("active state"));
    assert_eq!(
        supervisor
            .retained_reader_count()
            .expect("retained reader count"),
        1
    );
    let shutdown = supervisor
        .shutdown()
        .expect_err("shutdown reports retained reader boundary without blocking");
    assert!(shutdown.message.contains("retained output reader"));
    stdout.release();
    let deadline = Instant::now() + Duration::from_secs(2);
    while supervisor
        .retained_reader_count()
        .expect("retained reader count")
        != 0
    {
        assert!(Instant::now() < deadline, "retained reader did not finish");
        thread::sleep(Duration::from_millis(10));
    }
    supervisor
        .shutdown()
        .expect("shutdown after reader cleanup");
}

#[test]
fn cancellation_refuses_to_exceed_retained_reader_capacity() {
    let (factory, sink, supervisor) = fixture();
    let first_child = Arc::new(FakeChild::default());
    let first_reader = BlockingReader::default();
    factory.push_process_with_readers(
        first_child,
        Box::new(first_reader.clone()),
        Box::new(Cursor::new(Vec::<u8>::new())),
    );
    let first = invocation_id("invocation-reader-capacity-first");
    supervisor
        .start(
            session_id("session-reader-capacity-first"),
            first.clone(),
            spec(),
        )
        .expect("start first");
    supervisor.cancel(&first).expect("cancel first");
    sink.wait_for_terminal(&first);

    let second_child = Arc::new(FakeChild::default());
    factory.push_process(second_child.clone());
    let second = invocation_id("invocation-reader-capacity-second");
    supervisor
        .start(
            session_id("session-reader-capacity-second"),
            second.clone(),
            spec(),
        )
        .expect("start second");
    let error = supervisor
        .cancel(&second)
        .expect_err("capacity must refuse another cancellation");
    assert_eq!(error.kind, SupervisorErrorKind::CancellationFailed);
    assert!(error.message.contains("cleanup capacity"));
    second_child.complete(0);
    sink.wait_for_terminal(&second);

    first_reader.release();
    let deadline = Instant::now() + Duration::from_secs(2);
    while supervisor
        .retained_reader_count()
        .expect("retained reader count")
        != 0
    {
        assert!(Instant::now() < deadline, "retained reader did not finish");
        thread::sleep(Duration::from_millis(10));
    }
    supervisor
        .shutdown()
        .expect("shutdown after retained cleanup");
}

#[test]
fn cancellation_failure_is_reported_after_the_child_is_accounted_for() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    child.fail_next_termination(io::ErrorKind::PermissionDenied);
    factory.push_process(child.clone());
    let invocation = invocation_id("invocation-1");
    supervisor
        .start(session_id("session-1"), invocation.clone(), spec())
        .expect("start");

    let error = supervisor
        .cancel(&invocation)
        .expect_err("cancel must report the child-control failure");
    assert_eq!(error.kind, SupervisorErrorKind::CancellationFailed);
    assert!(supervisor.is_active(&invocation).expect("active state"));

    child.complete(0);
    assert_eq!(
        sink.wait_for_terminal(&invocation),
        ProcessTerminalOutcome::Failed {
            kind: ProcessFailureKind::CancellationFailed,
            exit: Some(ProcessExit {
                exit_code: Some(0),
                signal: None
            }),
            message: "fake termination failure".to_string(),
        }
    );
    assert!(!supervisor.is_active(&invocation).expect("active state"));
}

#[test]
fn shutdown_retries_after_failed_cancellation_and_reports_interruption() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    child.fail_next_termination(io::ErrorKind::PermissionDenied);
    factory.push_process(child.clone());
    let invocation = invocation_id("invocation-1");
    supervisor
        .start(session_id("session-1"), invocation.clone(), spec())
        .expect("start");
    supervisor
        .cancel(&invocation)
        .expect_err("first termination fails");

    supervisor.shutdown().expect("shutdown retries termination");

    assert_eq!(child.terminate_calls.load(Ordering::SeqCst), 2);
    assert!(matches!(
        sink.wait_for_terminal(&invocation),
        ProcessTerminalOutcome::Interrupted { .. }
    ));
}

#[test]
fn shutdown_reports_termination_failure_only_after_the_child_exits() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    child.fail_next_termination(io::ErrorKind::PermissionDenied);
    factory.push_process(child.clone());
    let invocation = invocation_id("invocation-shutdown-failure");
    supervisor
        .start(
            session_id("session-shutdown-failure"),
            invocation.clone(),
            spec(),
        )
        .expect("start");

    let completing_child = child.clone();
    let completion = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        completing_child.complete(0);
    });
    let error = supervisor
        .shutdown()
        .expect_err("failed direct-child termination must be reported");
    completion.join().expect("completion thread");

    assert_eq!(error.kind, SupervisorErrorKind::CancellationFailed);
    assert!(matches!(
        sink.wait_for_terminal(&invocation),
        ProcessTerminalOutcome::Interrupted { .. }
    ));
    assert_eq!(supervisor.active_count().expect("active count"), 0);
}

#[test]
fn reader_failure_terminates_and_reaps_the_child() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    factory.push_process_with_readers(
        child.clone(),
        Box::new(FailingReader),
        Box::new(Cursor::new(Vec::<u8>::new())),
    );
    let invocation = invocation_id("invocation-1");

    supervisor
        .start(session_id("session-1"), invocation.clone(), spec())
        .expect("start");

    let outcome = sink.wait_for_terminal(&invocation);
    assert!(matches!(
        outcome,
        ProcessTerminalOutcome::Failed {
            kind: ProcessFailureKind::ReaderFailed,
            ..
        }
    ));
    assert_eq!(child.terminate_calls.load(Ordering::SeqCst), 1);
    assert!(!supervisor.is_active(&invocation).expect("active state"));
}

#[test]
fn wait_failure_terminates_and_reaps_before_emitting_failure() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    child.fail_next_wait_query(io::ErrorKind::PermissionDenied);
    factory.push_process(child.clone());
    let invocation = invocation_id("invocation-1");

    supervisor
        .start(session_id("session-1"), invocation.clone(), spec())
        .expect("start");

    let outcome = sink.wait_for_terminal(&invocation);
    assert!(matches!(
        outcome,
        ProcessTerminalOutcome::Failed {
            kind: ProcessFailureKind::WaitFailed,
            ..
        }
    ));
    assert_eq!(child.terminate_calls.load(Ordering::SeqCst), 1);
    assert!(!supervisor.is_active(&invocation).expect("active state"));
}

#[test]
fn nonzero_exit_is_failed_and_cleanup_allows_the_next_invocation() {
    let (factory, sink, supervisor) = fixture();
    let first_child = Arc::new(FakeChild::default());
    factory.push_process(first_child.clone());
    let session = session_id("session-1");
    let first = invocation_id("invocation-1");
    supervisor
        .start(session.clone(), first.clone(), spec())
        .expect("start first");
    first_child.complete(23);

    assert_eq!(
        sink.wait_for_terminal(&first),
        ProcessTerminalOutcome::Failed {
            kind: ProcessFailureKind::NonZeroExit,
            exit: Some(ProcessExit {
                exit_code: Some(23),
                signal: None
            }),
            message: "process exited with code 23".to_string(),
        }
    );
    let second_child = Arc::new(FakeChild::default());
    factory.push_process(second_child.clone());
    supervisor
        .start(session, invocation_id("invocation-2"), spec())
        .expect("start after exit cleanup");
    second_child.complete(0);
}

#[test]
fn shutdown_interrupts_and_accounts_for_every_owned_child() {
    let (factory, sink, supervisor) = fixture();
    let first_child = Arc::new(FakeChild::default());
    let second_child = Arc::new(FakeChild::default());
    factory.push_process(first_child.clone());
    factory.push_process(second_child.clone());
    let first = invocation_id("invocation-1");
    let second = invocation_id("invocation-2");
    supervisor
        .start(session_id("session-1"), first.clone(), spec())
        .expect("start first");
    supervisor
        .start(session_id("session-2"), second.clone(), spec())
        .expect("start second");

    supervisor.shutdown().expect("shutdown");

    assert_eq!(first_child.terminate_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_child.terminate_calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        sink.wait_for_terminal(&first),
        ProcessTerminalOutcome::Interrupted { .. }
    ));
    assert!(matches!(
        sink.wait_for_terminal(&second),
        ProcessTerminalOutcome::Interrupted { .. }
    ));
    assert_eq!(supervisor.active_count().expect("active count"), 0);
    let error = supervisor
        .start(
            session_id("session-3"),
            invocation_id("invocation-3"),
            spec(),
        )
        .expect_err("start after shutdown must fail");
    assert_eq!(error.kind, SupervisorErrorKind::ShuttingDown);
}

#[test]
fn graceful_shutdown_allows_natural_completion_before_escalating() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    factory.push_process(child.clone());
    let invocation = invocation_id("invocation-graceful");
    supervisor
        .start(session_id("session-graceful"), invocation.clone(), spec())
        .expect("start");

    let completing_child = child.clone();
    let completion = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        completing_child.complete(0);
    });
    supervisor
        .shutdown_with_grace_period(Duration::from_secs(1))
        .expect("graceful shutdown");
    completion.join().expect("completion thread");

    assert_eq!(child.terminate_calls.load(Ordering::SeqCst), 0);
    assert!(matches!(
        sink.wait_for_terminal(&invocation),
        ProcessTerminalOutcome::Exited(ProcessExit {
            exit_code: Some(0),
            signal: None,
        })
    ));
    assert_eq!(supervisor.active_count().expect("active count"), 0);
}

#[test]
fn shutdown_waits_for_the_terminal_callback_to_return() {
    let factory = Arc::new(FakeFactory::default());
    let sink = Arc::new(BlockingTerminalSink::default());
    let supervisor = Arc::new(ProcessSupervisor::new(factory.clone(), sink.clone()));
    let child = Arc::new(FakeChild::default());
    factory.push_process(child);
    supervisor
        .start(
            session_id("session-callback"),
            invocation_id("invocation-callback"),
            spec(),
        )
        .expect("start");

    let shutdown_supervisor = supervisor.clone();
    let shutdown = thread::spawn(move || shutdown_supervisor.shutdown());
    sink.wait_until_entered();
    assert!(!shutdown.is_finished());

    sink.release();
    shutdown.join().expect("shutdown thread").expect("shutdown");
    assert_eq!(supervisor.active_count().expect("active count"), 0);
}

#[test]
fn streams_stdout_bytes_before_terminal_outcome() {
    let (factory, sink, supervisor) = fixture();
    let child = Arc::new(FakeChild::default());
    factory.push_process_with_readers(
        child.clone(),
        Box::new(Cursor::new(b"one\ntwo\n".to_vec())),
        Box::new(Cursor::new(Vec::<u8>::new())),
    );
    let invocation = invocation_id("invocation-1");
    supervisor
        .start(session_id("session-1"), invocation.clone(), spec())
        .expect("start");
    child.complete(0);
    sink.wait_for_terminal(&invocation);

    assert_eq!(
        sink.outputs.lock().expect("lock outputs").as_slice(),
        &[(
            invocation,
            ProcessOutput {
                stream: ProcessOutputStream::Stdout,
                bytes: b"one\ntwo\n".to_vec(),
            }
        )]
    );
}
