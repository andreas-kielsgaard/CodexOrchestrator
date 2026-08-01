use super::{
    application::{
        FocusInstanceCommand, PrepareInstanceCommand, ReadInstanceQuery, RecoverInstanceCommand,
        RuntimeApplicationError, RuntimeApplicationErrorKind, RuntimeStartProgressSink,
        RuntimeStartStage, StartInstanceCommand, StopInstanceCommand, WorktreeRuntimeControl,
    },
    build_cache::{record_build, reusable_build},
    domain::{AuthoritySecret, InstanceId, InstanceSnapshot, InstanceState, RequestId},
    execution::{ActionExecutor, ActionProgressEvent, ActionProgressObserver, ExecutionError},
    planning::{
        action_plan, derive_identity, launch_plan, project_runtime, ActionKind, PlanningError,
        RuntimeSettings, SourceInspector, ToolchainPrograms,
    },
};
use std::{error::Error, fmt, path::PathBuf, sync::Arc};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct TestSourceRef(String);

impl TestSourceRef {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, TestInstanceError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/')
            })
        {
            return Err(TestInstanceError::new(
                TestInstanceErrorKind::InvalidRequest,
                "test source reference must be a bounded application identifier",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IsolatedTestRequest {
    pub(crate) source: TestSourceRef,
    pub(crate) purpose: String,
}

impl IsolatedTestRequest {
    pub(crate) fn new(
        source: TestSourceRef,
        purpose: impl Into<String>,
    ) -> Result<Self, TestInstanceError> {
        let purpose = purpose.into();
        if purpose.is_empty()
            || purpose.len() > 96
            || !purpose.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'.' | b'_' | b'-')
            })
        {
            return Err(TestInstanceError::new(
                TestInstanceErrorKind::InvalidRequest,
                "test purpose must be a short semantic label",
            ));
        }
        Ok(Self { source, purpose })
    }
}

/// Opaque to feature callers. Resource routes remain owned by this facade.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct TestInstanceHandle(InstanceId);

impl TestInstanceHandle {
    pub(crate) fn from_opaque(value: impl Into<String>) -> Result<Self, TestInstanceError> {
        InstanceId::new(value).map(Self).map_err(|error| {
            TestInstanceError::new(TestInstanceErrorKind::InvalidRequest, error.to_string())
        })
    }

    pub(crate) fn opaque_ref(&self) -> &str {
        self.0.as_str()
    }
}

/// Private evidence for application-owned consumers. Paths and Git facts never cross transport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedTestSource {
    pub(crate) worktree_path: PathBuf,
    pub(crate) current_object_id: String,
    pub(crate) source_fingerprint: String,
    pub(crate) clean: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestInstancePhase {
    Prepared,
    Starting,
    Running,
    Stopping,
    Stopped,
    Recovering,
    Recovered,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HealthState {
    NotObserved,
    Healthy,
    Unhealthy,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TestInstanceStatus {
    pub(crate) phase: TestInstancePhase,
    pub(crate) health: HealthState,
    pub(crate) stale: bool,
    pub(crate) source_current: bool,
    pub(crate) build_reusable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RequestedTestInstance {
    pub(crate) handle: TestInstanceHandle,
    pub(crate) status: TestInstanceStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestActionOutcome {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TestActionResult {
    pub(crate) outcome: TestActionOutcome,
    pub(crate) failed_step: Option<String>,
    pub(crate) status: TestInstanceStatus,
    pub(crate) reused: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestActionStage {
    SourceInspection,
    Typecheck,
    FrontendBuild,
    TauriCompileLink,
    BuildReuse,
    Finalizing,
}

pub(crate) struct TestActionProgress<'a> {
    pub(crate) stage: TestActionStage,
    pub(crate) output: Option<&'a str>,
}

pub(crate) trait TestActionProgressSink: Send + Sync {
    fn progress(&self, progress: TestActionProgress<'_>);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestStartStage {
    Reservation,
    SupportingServices,
    NativeStart,
    WaitingForWindow,
    Ready,
}

pub(crate) struct TestStartProgress<'a> {
    pub(crate) stage: TestStartStage,
    pub(crate) output: Option<&'a str>,
}

pub(crate) trait TestStartProgressSink: Send + Sync {
    fn progress(&self, progress: TestStartProgress<'_>);
}

struct NoopTestStartProgressSink;

impl TestStartProgressSink for NoopTestStartProgressSink {
    fn progress(&self, _progress: TestStartProgress<'_>) {}
}

struct NoopTestActionProgressSink;

impl TestActionProgressSink for NoopTestActionProgressSink {
    fn progress(&self, _progress: TestActionProgress<'_>) {}
}

/// Application-owned lookup. Feature callers pass a semantic source reference, never a path.
pub(crate) trait TestSourceResolver: Send + Sync {
    fn resolve(&self, source: &TestSourceRef) -> Result<PathBuf, TestInstanceError>;
}

/// The only caller-facing lifecycle port for an isolated application test instance.
pub(crate) trait WorktreeTestInstances: Send + Sync {
    fn request(
        &self,
        request: IsolatedTestRequest,
    ) -> Result<RequestedTestInstance, TestInstanceError>;
    fn build(&self, handle: &TestInstanceHandle) -> Result<TestActionResult, TestInstanceError>;
    fn build_with_progress(
        &self,
        handle: &TestInstanceHandle,
        progress: &dyn TestActionProgressSink,
    ) -> Result<TestActionResult, TestInstanceError> {
        let _ = progress;
        self.build(handle)
    }
    fn test(&self, handle: &TestInstanceHandle) -> Result<TestActionResult, TestInstanceError>;
    fn start(&self, handle: &TestInstanceHandle) -> Result<TestInstanceStatus, TestInstanceError>;
    fn start_with_progress(
        &self,
        handle: &TestInstanceHandle,
        progress: &dyn TestStartProgressSink,
    ) -> Result<TestInstanceStatus, TestInstanceError> {
        let _ = progress;
        self.start(handle)
    }
    fn status(&self, handle: &TestInstanceHandle) -> Result<TestInstanceStatus, TestInstanceError>;
    fn focus(&self, handle: &TestInstanceHandle) -> Result<TestInstanceStatus, TestInstanceError>;
    fn stop(&self, handle: &TestInstanceHandle) -> Result<TestInstanceStatus, TestInstanceError>;
    fn recover(&self, handle: &TestInstanceHandle)
        -> Result<TestInstanceStatus, TestInstanceError>;
    fn verified_source(
        &self,
        handle: &TestInstanceHandle,
    ) -> Result<VerifiedTestSource, TestInstanceError>;
}

pub(crate) struct WorktreeTestInstanceFacade {
    runtime: Arc<dyn WorktreeRuntimeControl>,
    sources: Arc<dyn TestSourceResolver>,
    inspector: Arc<dyn SourceInspector>,
    executor: Arc<dyn ActionExecutor>,
    settings: RuntimeSettings,
    programs: ToolchainPrograms,
    authority: AuthoritySecret,
}

impl WorktreeTestInstanceFacade {
    pub(crate) fn new(
        runtime: Arc<dyn WorktreeRuntimeControl>,
        sources: Arc<dyn TestSourceResolver>,
        inspector: Arc<dyn SourceInspector>,
        executor: Arc<dyn ActionExecutor>,
        settings: RuntimeSettings,
        programs: ToolchainPrograms,
        authority: AuthoritySecret,
    ) -> Result<Self, TestInstanceError> {
        settings.validate().map_err(planning_error)?;
        programs.validate().map_err(planning_error)?;
        Ok(Self {
            runtime,
            sources,
            inspector,
            executor,
            settings,
            programs,
            authority,
        })
    }

    fn snapshot(&self, handle: &TestInstanceHandle) -> Result<InstanceSnapshot, TestInstanceError> {
        self.runtime
            .read(ReadInstanceQuery {
                authority: self.authority.clone(),
                instance_id: handle.0.clone(),
            })
            .map_err(runtime_error)
    }

    fn require_current_source(&self, snapshot: &InstanceSnapshot) -> Result<(), TestInstanceError> {
        let identity = &snapshot.projected.identity;
        let observed = self
            .inspector
            .inspect(&identity.worktree_path, &self.programs)
            .map_err(planning_error)?;
        if observed.git_commit != identity.git_commit
            || observed.source_fingerprint != identity.source_fingerprint
            || observed.node_cache_key != snapshot.projected.projection.caches.node_key
            || observed.rust_cache_key != snapshot.projected.projection.caches.rust_key
        {
            return Err(TestInstanceError::new(
                TestInstanceErrorKind::Conflict,
                "the worktree source or toolchain changed; request a new isolated test instance",
            ));
        }
        Ok(())
    }

    fn action(
        &self,
        handle: &TestInstanceHandle,
        kind: ActionKind,
        progress: &dyn TestActionProgressSink,
    ) -> Result<TestActionResult, TestInstanceError> {
        progress.progress(TestActionProgress {
            stage: TestActionStage::SourceInspection,
            output: Some("Inspecting the selected source and isolated build identity."),
        });
        let snapshot = self.snapshot(handle)?;
        self.require_current_source(&snapshot)?;
        if !matches!(
            snapshot.projected.state,
            InstanceState::Prepared | InstanceState::Stopped | InstanceState::Recovered
        ) {
            return Err(TestInstanceError::new(
                TestInstanceErrorKind::InvalidState,
                "build and test require a non-running isolated instance",
            ));
        }
        let plan = action_plan(
            kind,
            &snapshot.projected.identity,
            &snapshot.projected.projection,
            &tauri_identifier(&snapshot.projected.identity.instance_id),
            &self.programs,
        )
        .map_err(planning_error)?;
        let tauri_identifier = tauri_identifier(&snapshot.projected.identity.instance_id);
        if kind == ActionKind::Build
            && reusable_build(
                &snapshot.projected.identity,
                &snapshot.projected.projection,
                &tauri_identifier,
            )
        {
            progress.progress(TestActionProgress {
                stage: TestActionStage::BuildReuse,
                output: Some(
                    "Verified the exact source, toolchain, launch identity, frontend output, and private executable; no compilation is required.",
                ),
            });
            return Ok(TestActionResult {
                outcome: TestActionOutcome::Passed,
                failed_step: None,
                status: semantic_status(&snapshot),
                reused: true,
            });
        }
        let observer = FacadeActionProgress { progress };
        let execution = self
            .executor
            .execute(&plan, &observer)
            .map_err(execution_error)?;
        progress.progress(TestActionProgress {
            stage: TestActionStage::Finalizing,
            output: Some("Recording the isolated build result."),
        });
        if kind == ActionKind::Build && execution.succeeded {
            record_build(
                &snapshot.projected.identity,
                &snapshot.projected.projection,
                &tauri_identifier,
            )
            .map_err(|message| {
                TestInstanceError::new(TestInstanceErrorKind::Unavailable, message)
            })?;
        }
        Ok(TestActionResult {
            outcome: if execution.succeeded {
                TestActionOutcome::Passed
            } else {
                TestActionOutcome::Failed
            },
            failed_step: execution.failed_step,
            status: semantic_status(&snapshot),
            reused: false,
        })
    }
}

impl WorktreeTestInstances for WorktreeTestInstanceFacade {
    fn request(
        &self,
        request: IsolatedTestRequest,
    ) -> Result<RequestedTestInstance, TestInstanceError> {
        let worktree = self.sources.resolve(&request.source)?;
        let worktree = worktree
            .canonicalize()
            .map_err(|error| unavailable("resolve test source", error))?;
        let source = self
            .inspector
            .inspect(&worktree, &self.programs)
            .map_err(planning_error)?;
        let internal =
            derive_identity(worktree, request.source.as_str(), &request.purpose, &source)
                .map_err(planning_error)?;
        let handle = TestInstanceHandle(internal.identity.instance_id.clone());

        match self.snapshot(&handle) {
            Ok(snapshot) => {
                return Ok(RequestedTestInstance {
                    handle,
                    status: semantic_status(&snapshot),
                });
            }
            Err(error) if error.kind == TestInstanceErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        for ports in self
            .settings
            .candidate_ports(internal.identity.instance_id.as_str())
            .map_err(planning_error)?
        {
            let projection = project_runtime(&self.settings, &internal.identity, &source, ports)
                .map_err(planning_error)?;
            let result = self.runtime.prepare(PrepareInstanceCommand {
                request_id: request_id("prepare")?,
                authority: self.authority.clone(),
                identity: internal.identity.clone(),
                projection,
            });
            match result {
                Ok(snapshot) => {
                    return Ok(RequestedTestInstance {
                        handle,
                        status: semantic_status(&snapshot),
                    });
                }
                Err(error) if error.kind == RuntimeApplicationErrorKind::PortLeaseConflict => {
                    continue;
                }
                Err(error) => return Err(runtime_error(error)),
            }
        }
        Err(TestInstanceError::new(
            TestInstanceErrorKind::Unavailable,
            "no isolated runtime port pair could be leased",
        ))
    }

    fn build(&self, handle: &TestInstanceHandle) -> Result<TestActionResult, TestInstanceError> {
        self.action(handle, ActionKind::Build, &NoopTestActionProgressSink)
    }

    fn build_with_progress(
        &self,
        handle: &TestInstanceHandle,
        progress: &dyn TestActionProgressSink,
    ) -> Result<TestActionResult, TestInstanceError> {
        self.action(handle, ActionKind::Build, progress)
    }

    fn test(&self, handle: &TestInstanceHandle) -> Result<TestActionResult, TestInstanceError> {
        self.action(handle, ActionKind::Test, &NoopTestActionProgressSink)
    }

    fn start(&self, handle: &TestInstanceHandle) -> Result<TestInstanceStatus, TestInstanceError> {
        self.start_with_progress(handle, &NoopTestStartProgressSink)
    }

    fn start_with_progress(
        &self,
        handle: &TestInstanceHandle,
        progress: &dyn TestStartProgressSink,
    ) -> Result<TestInstanceStatus, TestInstanceError> {
        let snapshot = self.snapshot(handle)?;
        self.require_current_source(&snapshot)?;
        let identifier = tauri_identifier(&snapshot.projected.identity.instance_id);
        if !reusable_build(
            &snapshot.projected.identity,
            &snapshot.projected.projection,
            &identifier,
        ) {
            return Err(TestInstanceError::new(
                TestInstanceErrorKind::BuildRequired,
                "the private build receipt or artifact hashes no longer match; build this instance again",
            ));
        }
        let launches = launch_plan(
            &snapshot.projected.identity,
            &snapshot.projected.projection,
            &identifier,
            &self.programs,
        )
        .map_err(planning_error)?;
        self.runtime
            .start_with_progress(
                StartInstanceCommand {
                    request_id: request_id("start")?,
                    authority: self.authority.clone(),
                    instance_id: handle.0.clone(),
                    launches,
                },
                &FacadeStartProgress { progress },
            )
            .map(|snapshot| semantic_status(&snapshot))
            .map_err(runtime_error)
    }

    fn status(&self, handle: &TestInstanceHandle) -> Result<TestInstanceStatus, TestInstanceError> {
        let snapshot = self.snapshot(handle)?;
        let mut status = semantic_status(&snapshot);
        match self.require_current_source(&snapshot) {
            Ok(()) => {}
            Err(error) if error.kind == TestInstanceErrorKind::Conflict => {
                status.source_current = false;
                status.build_reusable = false;
            }
            Err(error) => return Err(error),
        }
        if status.phase == TestInstancePhase::Running
            && !self
                .runtime
                .review_window_ready(ReadInstanceQuery {
                    authority: self.authority.clone(),
                    instance_id: handle.0.clone(),
                })
                .map_err(runtime_error)?
        {
            status.health = HealthState::Unhealthy;
        }
        Ok(status)
    }

    fn focus(&self, handle: &TestInstanceHandle) -> Result<TestInstanceStatus, TestInstanceError> {
        self.runtime
            .focus(FocusInstanceCommand {
                authority: self.authority.clone(),
                instance_id: handle.0.clone(),
            })
            .map(|snapshot| semantic_status(&snapshot))
            .map_err(runtime_error)
    }

    fn stop(&self, handle: &TestInstanceHandle) -> Result<TestInstanceStatus, TestInstanceError> {
        self.runtime
            .stop(StopInstanceCommand {
                request_id: request_id("stop")?,
                authority: self.authority.clone(),
                instance_id: handle.0.clone(),
            })
            .map_err(runtime_error)?;
        self.status(handle)
    }

    fn recover(
        &self,
        handle: &TestInstanceHandle,
    ) -> Result<TestInstanceStatus, TestInstanceError> {
        self.runtime
            .recover(RecoverInstanceCommand {
                request_id: request_id("recover")?,
                authority: self.authority.clone(),
                instance_id: handle.0.clone(),
            })
            .map_err(runtime_error)?;
        self.status(handle)
    }

    fn verified_source(
        &self,
        handle: &TestInstanceHandle,
    ) -> Result<VerifiedTestSource, TestInstanceError> {
        let snapshot = self.snapshot(handle)?;
        if snapshot.stale {
            return Err(TestInstanceError::new(
                TestInstanceErrorKind::Conflict,
                "the isolated runtime instance is stale",
            ));
        }
        let identity = &snapshot.projected.identity;
        let observed = self
            .inspector
            .inspect(&identity.worktree_path, &self.programs)
            .map_err(planning_error)?;
        if observed.git_commit != identity.git_commit
            || observed.source_fingerprint != identity.source_fingerprint
            || observed.node_cache_key != snapshot.projected.projection.caches.node_key
            || observed.rust_cache_key != snapshot.projected.projection.caches.rust_key
        {
            return Err(TestInstanceError::new(
                TestInstanceErrorKind::Conflict,
                "the prepared runtime source was replaced or superseded",
            ));
        }
        if !matches!(
            snapshot.projected.state,
            InstanceState::Prepared
                | InstanceState::Running
                | InstanceState::Stopped
                | InstanceState::Recovered
        ) {
            return Err(TestInstanceError::new(
                TestInstanceErrorKind::InvalidState,
                "the runtime source has not reached an accepted prepared state",
            ));
        }
        Ok(VerifiedTestSource {
            worktree_path: identity.worktree_path.clone(),
            current_object_id: identity.git_commit.clone(),
            source_fingerprint: identity.source_fingerprint.clone(),
            clean: observed.clean,
        })
    }
}

struct FacadeStartProgress<'a> {
    progress: &'a dyn TestStartProgressSink,
}

impl RuntimeStartProgressSink for FacadeStartProgress<'_> {
    fn progress(&self, stage: RuntimeStartStage, output: Option<&str>) {
        let stage = match stage {
            RuntimeStartStage::Reservation => TestStartStage::Reservation,
            RuntimeStartStage::SupportingServices => TestStartStage::SupportingServices,
            RuntimeStartStage::NativeStart => TestStartStage::NativeStart,
            RuntimeStartStage::WaitingForWindow => TestStartStage::WaitingForWindow,
            RuntimeStartStage::Ready => TestStartStage::Ready,
        };
        self.progress.progress(TestStartProgress { stage, output });
    }
}

struct FacadeActionProgress<'a> {
    progress: &'a dyn TestActionProgressSink,
}

impl ActionProgressObserver for FacadeActionProgress<'_> {
    fn progress(&self, event: ActionProgressEvent<'_>) {
        let (step, output) = match event {
            ActionProgressEvent::Started { step } => (step, None),
            ActionProgressEvent::Output { step, line } => (step, Some(line)),
            ActionProgressEvent::Finished { step, succeeded } => (
                step,
                Some(if succeeded {
                    "Stage completed."
                } else {
                    "Stage failed."
                }),
            ),
        };
        let stage = match step {
            "typecheck" => TestActionStage::Typecheck,
            "frontend build" => TestActionStage::FrontendBuild,
            "Tauri debug build" => TestActionStage::TauriCompileLink,
            _ => TestActionStage::Finalizing,
        };
        self.progress.progress(TestActionProgress { stage, output });
    }
}

fn semantic_status(snapshot: &InstanceSnapshot) -> TestInstanceStatus {
    let phase = match snapshot.projected.state {
        InstanceState::Prepared => TestInstancePhase::Prepared,
        InstanceState::LaunchPending => TestInstancePhase::Starting,
        InstanceState::Running => TestInstancePhase::Running,
        InstanceState::StopPending => TestInstancePhase::Stopping,
        InstanceState::Stopped => TestInstancePhase::Stopped,
        InstanceState::RecoveryPending => TestInstancePhase::Recovering,
        InstanceState::Recovered => TestInstancePhase::Recovered,
    };
    let health = match snapshot.observed.as_ref() {
        None => HealthState::NotObserved,
        Some(observed) if observed.health.healthy() => HealthState::Healthy,
        Some(observed) if observed.health.all_closed() => HealthState::Closed,
        Some(_) => HealthState::Unhealthy,
    };
    TestInstanceStatus {
        phase,
        health,
        stale: snapshot.stale,
        source_current: true,
        build_reusable: reusable_build(
            &snapshot.projected.identity,
            &snapshot.projected.projection,
            &tauri_identifier(&snapshot.projected.identity.instance_id),
        ),
    }
}

fn tauri_identifier(instance_id: &InstanceId) -> String {
    format!(
        "dev.codex-orchestrator.worktree.{}",
        instance_id.as_str().trim_start_matches("wt-")
    )
}

fn request_id(operation: &str) -> Result<RequestId, TestInstanceError> {
    RequestId::new(format!("{operation}-{}", Uuid::new_v4().simple())).map_err(|error| {
        TestInstanceError::new(
            TestInstanceErrorKind::Unavailable,
            format!("create internal request identity: {error}"),
        )
    })
}

fn planning_error(error: PlanningError) -> TestInstanceError {
    TestInstanceError::new(TestInstanceErrorKind::Unavailable, error.message)
}

fn execution_error(error: ExecutionError) -> TestInstanceError {
    TestInstanceError::new(TestInstanceErrorKind::Unavailable, error.message)
}

fn runtime_error(error: RuntimeApplicationError) -> TestInstanceError {
    let kind = match error.kind {
        RuntimeApplicationErrorKind::NotFound => TestInstanceErrorKind::NotFound,
        RuntimeApplicationErrorKind::Unauthorized => TestInstanceErrorKind::Unauthorized,
        RuntimeApplicationErrorKind::OperationInProgress => {
            TestInstanceErrorKind::OperationInProgress
        }
        RuntimeApplicationErrorKind::InvalidState | RuntimeApplicationErrorKind::NotStale => {
            TestInstanceErrorKind::InvalidState
        }
        RuntimeApplicationErrorKind::Conflict
        | RuntimeApplicationErrorKind::PortLeaseConflict
        | RuntimeApplicationErrorKind::IdempotencyConflict
        | RuntimeApplicationErrorKind::OwnershipAmbiguous => TestInstanceErrorKind::Conflict,
        RuntimeApplicationErrorKind::LaunchFailed
        | RuntimeApplicationErrorKind::HealthFailed
        | RuntimeApplicationErrorKind::Unavailable => TestInstanceErrorKind::Unavailable,
    };
    TestInstanceError::new(kind, error.message)
}

fn unavailable(operation: &str, error: impl fmt::Display) -> TestInstanceError {
    TestInstanceError::new(
        TestInstanceErrorKind::Unavailable,
        format!("{operation}: {error}"),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestInstanceErrorKind {
    InvalidRequest,
    NotFound,
    Unauthorized,
    InvalidState,
    OperationInProgress,
    Conflict,
    BuildRequired,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TestInstanceError {
    pub(crate) kind: TestInstanceErrorKind,
    pub(crate) message: String,
}

impl TestInstanceError {
    pub(crate) fn new(kind: TestInstanceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for TestInstanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for TestInstanceError {}
