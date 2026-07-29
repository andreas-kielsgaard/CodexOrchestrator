use super::{
    domain::{
        validate_launches, AuthoritySecret, InstanceId, InstanceIdentity, InstanceProjection,
        InstanceRecord, InstanceSnapshot, InstanceState, LaunchId, OwnedProcessLaunch,
        OwnerObservation, OwnerRoute, RequestId, RuntimeObservation,
    },
    health::HealthProbe,
    ownership::{OwnershipError, ProcessOwner, ReviewWindowExpectation},
    registry::{
        CommandStart, InstanceRegistry, PrepareRecord, RegistryError, RegistryErrorKind,
        StoredFailure,
    },
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{error::Error, fmt, sync::Arc, thread, time::Duration};
use uuid::Uuid;

pub(crate) struct PrepareInstanceCommand {
    pub(crate) request_id: RequestId,
    pub(crate) authority: AuthoritySecret,
    pub(crate) identity: InstanceIdentity,
    pub(crate) projection: InstanceProjection,
}

pub(crate) struct StartInstanceCommand {
    pub(crate) request_id: RequestId,
    pub(crate) authority: AuthoritySecret,
    pub(crate) instance_id: InstanceId,
    pub(crate) launches: Vec<OwnedProcessLaunch>,
}

pub(crate) struct StopInstanceCommand {
    pub(crate) request_id: RequestId,
    pub(crate) authority: AuthoritySecret,
    pub(crate) instance_id: InstanceId,
}

pub(crate) struct RecoverInstanceCommand {
    pub(crate) request_id: RequestId,
    pub(crate) authority: AuthoritySecret,
    pub(crate) instance_id: InstanceId,
}

pub(crate) struct FocusInstanceCommand {
    pub(crate) authority: AuthoritySecret,
    pub(crate) instance_id: InstanceId,
}

pub(crate) struct ReadInstanceQuery {
    pub(crate) authority: AuthoritySecret,
    pub(crate) instance_id: InstanceId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeStartStage {
    Reservation,
    SupportingServices,
    NativeStart,
    WaitingForWindow,
    Ready,
}

pub(crate) trait RuntimeStartProgressSink: Send + Sync {
    fn progress(&self, stage: RuntimeStartStage, output: Option<&str>);

    fn require_usable_window(&self) -> bool {
        true
    }
}

struct NoopRuntimeStartProgressSink;

impl RuntimeStartProgressSink for NoopRuntimeStartProgressSink {
    fn progress(&self, _stage: RuntimeStartStage, _output: Option<&str>) {}
}

/// Explicit application lifecycle ports. These methods do not schedule, approve, or continue work.
pub(crate) trait WorktreeRuntimeControl: Send + Sync {
    fn prepare(
        &self,
        command: PrepareInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError>;

    fn start(
        &self,
        command: StartInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError>;

    fn start_with_progress(
        &self,
        command: StartInstanceCommand,
        progress: &dyn RuntimeStartProgressSink,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        let _ = progress;
        self.start(command)
    }

    fn read(&self, query: ReadInstanceQuery) -> Result<InstanceSnapshot, RuntimeApplicationError>;

    fn review_window_ready(
        &self,
        query: ReadInstanceQuery,
    ) -> Result<bool, RuntimeApplicationError> {
        let _ = query;
        Ok(false)
    }

    fn focus(
        &self,
        command: FocusInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError>;

    fn stop(
        &self,
        command: StopInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError>;

    fn recover(
        &self,
        command: RecoverInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError>;
}

pub(crate) trait RuntimeClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

struct SystemRuntimeClock;

impl RuntimeClock for SystemRuntimeClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ObservationPolicy {
    pub(crate) attempts: usize,
    pub(crate) interval: Duration,
}

impl Default for ObservationPolicy {
    fn default() -> Self {
        Self {
            attempts: 600,
            interval: Duration::from_millis(500),
        }
    }
}

pub(crate) struct WorktreeRuntimeApplication {
    registry: Arc<dyn InstanceRegistry>,
    owner: Arc<dyn ProcessOwner>,
    health: Arc<dyn HealthProbe>,
    clock: Arc<dyn RuntimeClock>,
    observation_policy: ObservationPolicy,
}

impl WorktreeRuntimeApplication {
    pub(crate) fn system(
        registry: Arc<dyn InstanceRegistry>,
        owner: Arc<dyn ProcessOwner>,
        health: Arc<dyn HealthProbe>,
    ) -> Self {
        Self::new(
            registry,
            owner,
            health,
            Arc::new(SystemRuntimeClock),
            ObservationPolicy::default(),
        )
    }

    pub(crate) fn new(
        registry: Arc<dyn InstanceRegistry>,
        owner: Arc<dyn ProcessOwner>,
        health: Arc<dyn HealthProbe>,
        clock: Arc<dyn RuntimeClock>,
        observation_policy: ObservationPolicy,
    ) -> Self {
        Self {
            registry,
            owner,
            health,
            clock,
            observation_policy,
        }
    }

    fn authority_hash(authority: &AuthoritySecret) -> String {
        format!("{:x}", Sha256::digest(authority.expose().as_bytes()))
    }

    fn observe_record(
        &self,
        record: &InstanceRecord,
    ) -> Result<RuntimeObservation, RuntimeApplicationError> {
        let owner = match &record.owner_route {
            Some(route) => {
                validate_owner_route(&record.identity.instance_id, route)?;
                self.owner.observe(route).map_err(owner_error)?
            }
            None => OwnerObservation::Absent,
        };
        Ok(RuntimeObservation {
            owner,
            health: self.health.observe(&record.projection),
            observed_at: self.clock.now(),
        })
    }

    fn wait_for_started(
        &self,
        record: &InstanceRecord,
        route: &OwnerRoute,
        progress: &dyn RuntimeStartProgressSink,
    ) -> Result<RuntimeObservation, RuntimeApplicationError> {
        let expected = review_window_expectation(record);
        let ready_marker = record.projection.paths.app_data.join("review-window-ready");
        for attempt in 0..self.observation_policy.attempts.max(1) {
            let owner = self.owner.observe(route).map_err(owner_error)?;
            let observation = RuntimeObservation {
                owner,
                health: self.health.observe(&record.projection),
                observed_at: self.clock.now(),
            };
            let window = if progress.require_usable_window() {
                self.owner
                    .observe_review_window(route, &expected)
                    .map_err(owner_error)?
            } else {
                None
            };
            let application_ready = ready_marker.is_file();
            if observation.owner.is_active()
                && observation.health.healthy()
                && (!progress.require_usable_window()
                    || (window
                        .as_ref()
                        .is_some_and(|window| window.usable(&expected))
                        && application_ready))
            {
                progress.progress(
                    RuntimeStartStage::Ready,
                    Some("The owned worktree-build window is visible and application-ready."),
                );
                return Ok(observation);
            }
            if !observation.owner.is_active() {
                return Err(RuntimeApplicationError::new(
                    RuntimeApplicationErrorKind::LaunchFailed,
                    "the exact Job Object owner exited before both endpoints became healthy",
                ));
            }
            if attempt == 0 || attempt % 4 == 0 {
                let evidence = match window {
                    None => "Owned processes are active; waiting for the worktree-build window.",
                    Some(window) if window.title != expected.title => {
                        "An owned window exists, but its expected worktree-build identity is not ready."
                    }
                    Some(window) if !window.usable(&expected) => {
                        "The owned worktree-build window exists but is not yet usable at review size."
                    }
                    Some(_) if !application_ready => {
                        "The owned window is usable; waiting for the application surface to render."
                    }
                    Some(_) => "Waiting for isolated supporting services.",
                };
                progress.progress(RuntimeStartStage::WaitingForWindow, Some(evidence));
            }
            if attempt + 1 < self.observation_policy.attempts.max(1) {
                thread::sleep(self.observation_policy.interval);
            }
        }
        Err(RuntimeApplicationError::new(
            RuntimeApplicationErrorKind::HealthFailed,
            "owned processes were observed, but a usable application-ready worktree-build window did not appear before the readiness limit",
        ))
    }

    fn wait_for_stopped(
        &self,
        record: &InstanceRecord,
        route: &OwnerRoute,
    ) -> Result<RuntimeObservation, RuntimeApplicationError> {
        let mut latest = None;
        for attempt in 0..self.observation_policy.attempts.max(1) {
            let owner = self.owner.observe(route).map_err(owner_error)?;
            let observation = RuntimeObservation {
                owner,
                health: self.health.observe(&record.projection),
                observed_at: self.clock.now(),
            };
            if !observation.owner.is_active() && observation.health.all_closed() {
                return Ok(observation);
            }
            latest = Some(observation);
            if attempt + 1 < self.observation_policy.attempts.max(1) {
                thread::sleep(self.observation_policy.interval);
            }
        }
        let latest = latest.expect("at least one observation attempt");
        Err(RuntimeApplicationError::new(
            RuntimeApplicationErrorKind::OwnershipAmbiguous,
            format!(
                "stop is unproven: owner active={}, Vite reachable={}, status reachable={}",
                latest.owner.is_active(),
                latest.health.vite.reachable,
                latest.health.status.reachable
            ),
        ))
    }

    fn cleanup_failed_start(
        &self,
        command: &StartInstanceCommand,
        record: &InstanceRecord,
        route: &OwnerRoute,
        failure: RuntimeApplicationError,
    ) -> RuntimeApplicationError {
        let _ = self.owner.terminate(route);
        let cleanup = self.wait_for_stopped(record, route);
        let (recorded_failure, observation, terminal_state) = match cleanup {
            Ok(observation) => (
                failure.clone(),
                Some(observation),
                Some(InstanceState::Stopped),
            ),
            Err(cleanup_error) => (
                RuntimeApplicationError::new(
                    RuntimeApplicationErrorKind::OwnershipAmbiguous,
                    format!(
                        "{}; failed launch cleanup is unproven: {}",
                        failure.message, cleanup_error.message
                    ),
                ),
                None,
                None,
            ),
        };
        let _ = self.registry.fail_transition(
            &command.request_id,
            InstanceState::LaunchPending,
            terminal_state,
            StoredFailure {
                kind: error_kind_name(recorded_failure.kind).into(),
                message: recorded_failure.message.clone(),
            },
            observation,
        );
        recorded_failure
    }

    fn finish_terminal(
        &self,
        request_id: &RequestId,
        record: InstanceRecord,
        pending_state: InstanceState,
        terminal_state: InstanceState,
        transition_kind: &'static str,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        let route = required_owner_route(&record)?;
        self.owner.terminate(route).map_err(|error| {
            let application_error = owner_error(error);
            let _ = self.registry.fail_transition(
                request_id,
                pending_state,
                None,
                StoredFailure {
                    kind: error_kind_name(application_error.kind).into(),
                    message: application_error.message.clone(),
                },
                None,
            );
            application_error
        })?;
        let observation = self.wait_for_stopped(&record, route).map_err(|error| {
            let _ = self.registry.fail_transition(
                request_id,
                pending_state,
                None,
                StoredFailure {
                    kind: error_kind_name(error.kind).into(),
                    message: error.message.clone(),
                },
                None,
            );
            error
        })?;
        self.registry
            .complete_transition(
                request_id,
                pending_state,
                terminal_state,
                transition_kind,
                observation,
            )
            .map_err(registry_error)
    }
}

impl WorktreeRuntimeControl for WorktreeRuntimeApplication {
    fn prepare(
        &self,
        command: PrepareInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        command
            .identity
            .validate()
            .map_err(contract_error("invalid instance identity"))?;
        command
            .projection
            .validate()
            .map_err(contract_error("invalid instance projection"))?;
        let authority_hash = Self::authority_hash(&command.authority);
        let fingerprint = fingerprint(&(
            "prepare",
            &command.identity,
            &command.projection,
            &authority_hash,
        ))?;
        self.registry
            .prepare(PrepareRecord {
                request_id: &command.request_id,
                fingerprint: &fingerprint,
                identity: &command.identity,
                projection: &command.projection,
                authority_hash: &authority_hash,
                recorded_at: self.clock.now(),
            })
            .map_err(registry_error)
    }

    fn start(
        &self,
        command: StartInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        self.start_with_progress(command, &NoopRuntimeStartProgressSink)
    }

    fn start_with_progress(
        &self,
        command: StartInstanceCommand,
        progress: &dyn RuntimeStartProgressSink,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        progress.progress(
            RuntimeStartStage::Reservation,
            Some("Reserving the exact isolated lifecycle and ownership route."),
        );
        validate_launches(&command.launches)
            .map_err(contract_error("invalid owned process launches"))?;
        let authority_hash = Self::authority_hash(&command.authority);
        let fingerprint = fingerprint(&StartSemantics {
            operation: "start",
            instance_id: command.instance_id.as_str(),
            authority_hash: &authority_hash,
            launches: command.launches.iter().map(LaunchSemantics::from).collect(),
        })?;
        if let Some(replay) = self
            .registry
            .replay_command(
                &command.request_id,
                &fingerprint,
                &command.instance_id,
                "start",
            )
            .map_err(registry_error)?
        {
            return command_result(replay);
        }
        let prepared = self
            .registry
            .load_authorized(&command.instance_id, &authority_hash)
            .map_err(registry_error)?;
        let preflight_health = self.health.observe(&prepared.projection);
        if !preflight_health.all_closed() {
            return Err(RuntimeApplicationError::new(
                RuntimeApplicationErrorKind::OwnershipAmbiguous,
                "a projected runtime endpoint is already reachable before launch",
            ));
        }
        let launch_id = LaunchId::new(format!("launch-{}", Uuid::new_v4().simple()))
            .map_err(contract_error("create launch ID"))?;
        let route = OwnerRoute {
            job_name: job_name(&command.instance_id, &launch_id),
            launch_id,
        };
        let start = self
            .registry
            .begin_start(
                &command.request_id,
                &fingerprint,
                &command.instance_id,
                &authority_hash,
                &route,
                self.clock.now(),
            )
            .map_err(registry_error)?;
        let record = match start {
            CommandStart::Execute(record) => record,
            CommandStart::Replay(snapshot) | CommandStart::Noop(snapshot) => return Ok(snapshot),
            CommandStart::ReplayFailure(failure) => return Err(stored_failure(failure)),
        };
        let durable_route = required_owner_route(&record)?;
        if durable_route != &route {
            return Err(RuntimeApplicationError::new(
                RuntimeApplicationErrorKind::OwnershipAmbiguous,
                "durable launch route changed before process ownership began",
            ));
        }
        let ready_marker = record.projection.paths.app_data.join("review-window-ready");
        if ready_marker.exists() {
            std::fs::remove_file(&ready_marker).map_err(|error| {
                self.cleanup_failed_start(
                    &command,
                    &record,
                    &route,
                    RuntimeApplicationError::new(
                        RuntimeApplicationErrorKind::Unavailable,
                        format!("clear prior application readiness marker: {error}"),
                    ),
                )
            })?;
        }
        progress.progress(
            RuntimeStartStage::SupportingServices,
            Some("Starting isolated supporting services and the verified private build."),
        );
        let launched = self
            .owner
            .launch(&route, &command.launches)
            .map_err(|error| {
                self.cleanup_failed_start(&command, &record, &route, owner_error(error))
            })?;
        if !launched.is_active() {
            return Err(self.cleanup_failed_start(
                &command,
                &record,
                &route,
                RuntimeApplicationError::new(
                    RuntimeApplicationErrorKind::LaunchFailed,
                    "launch returned without an active owned process",
                ),
            ));
        }
        progress.progress(
            RuntimeStartStage::NativeStart,
            Some("The owned process tree is active; checking native window readiness."),
        );
        let observation = self
            .wait_for_started(&record, &route, progress)
            .map_err(|error| self.cleanup_failed_start(&command, &record, &route, error))?;
        match self.registry.complete_transition(
            &command.request_id,
            InstanceState::LaunchPending,
            InstanceState::Running,
            "start_observed",
            observation,
        ) {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                Err(self.cleanup_failed_start(&command, &record, &route, registry_error(error)))
            }
        }
    }

    fn read(&self, query: ReadInstanceQuery) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        let authority_hash = Self::authority_hash(&query.authority);
        let record = self
            .registry
            .load_authorized(&query.instance_id, &authority_hash)
            .map_err(registry_error)?;
        let observation = self.observe_record(&record)?;
        self.registry
            .record_observation(&query.instance_id, "health_observed", observation)
            .map_err(registry_error)
    }

    fn review_window_ready(
        &self,
        query: ReadInstanceQuery,
    ) -> Result<bool, RuntimeApplicationError> {
        let authority_hash = Self::authority_hash(&query.authority);
        let record = self
            .registry
            .load_authorized(&query.instance_id, &authority_hash)
            .map_err(registry_error)?;
        let Some(route) = record.owner_route.as_ref() else {
            return Ok(false);
        };
        validate_owner_route(&record.identity.instance_id, route)?;
        let expected = review_window_expectation(&record);
        let window = self
            .owner
            .observe_review_window(route, &expected)
            .map_err(owner_error)?;
        Ok(window
            .as_ref()
            .is_some_and(|window| window.usable(&expected))
            && record
                .projection
                .paths
                .app_data
                .join("review-window-ready")
                .is_file())
    }

    fn focus(
        &self,
        command: FocusInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        let authority_hash = Self::authority_hash(&command.authority);
        let record = self
            .registry
            .load_authorized(&command.instance_id, &authority_hash)
            .map_err(registry_error)?;
        if record.state != InstanceState::Running {
            return Err(RuntimeApplicationError::new(
                RuntimeApplicationErrorKind::InvalidState,
                "only a running review instance can be focused",
            ));
        }
        let route = required_owner_route(&record)?;
        let owner = self
            .owner
            .focus_review_window(route, &review_window_expectation(&record))
            .map_err(owner_error)?;
        let observation = RuntimeObservation {
            owner,
            health: self.health.observe(&record.projection),
            observed_at: self.clock.now(),
        };
        if !observation.owner.is_active() || !observation.health.healthy() {
            return Err(RuntimeApplicationError::new(
                RuntimeApplicationErrorKind::OwnershipAmbiguous,
                "the review window focus target is not a healthy owned instance",
            ));
        }
        self.registry
            .record_observation(&command.instance_id, "window_focus_observed", observation)
            .map_err(registry_error)
    }

    fn stop(
        &self,
        command: StopInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        let authority_hash = Self::authority_hash(&command.authority);
        let fingerprint = fingerprint(&("stop", command.instance_id.as_str(), &authority_hash))?;
        if let Some(replay) = self
            .registry
            .replay_command(
                &command.request_id,
                &fingerprint,
                &command.instance_id,
                "stop",
            )
            .map_err(registry_error)?
        {
            return command_result(replay);
        }
        let current = self
            .registry
            .load_authorized(&command.instance_id, &authority_hash)
            .map_err(registry_error)?;
        if !current.state.has_projected_owner() {
            let observation = self.observe_record(&current)?;
            self.registry
                .record_observation(
                    &command.instance_id,
                    "stop_noop_inspection_observed",
                    observation.clone(),
                )
                .map_err(registry_error)?;
            if observation.owner.is_active() || !observation.health.all_closed() {
                return Err(RuntimeApplicationError::new(
                    RuntimeApplicationErrorKind::OwnershipAmbiguous,
                    "a terminal durable state conflicts with current owner or endpoint evidence",
                ));
            }
        }
        match self
            .registry
            .begin_stop(
                &command.request_id,
                &fingerprint,
                &command.instance_id,
                &authority_hash,
                self.clock.now(),
            )
            .map_err(registry_error)?
        {
            CommandStart::Execute(record) => self.finish_terminal(
                &command.request_id,
                record,
                InstanceState::StopPending,
                InstanceState::Stopped,
                "stop_observed",
            ),
            CommandStart::Noop(snapshot) | CommandStart::Replay(snapshot) => Ok(snapshot),
            CommandStart::ReplayFailure(failure) => Err(stored_failure(failure)),
        }
    }

    fn recover(
        &self,
        command: RecoverInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        let authority_hash = Self::authority_hash(&command.authority);
        let fingerprint = fingerprint(&("recover", command.instance_id.as_str(), &authority_hash))?;
        if let Some(replay) = self
            .registry
            .replay_command(
                &command.request_id,
                &fingerprint,
                &command.instance_id,
                "recover",
            )
            .map_err(registry_error)?
        {
            return command_result(replay);
        }
        let record = self
            .registry
            .load_authorized(&command.instance_id, &authority_hash)
            .map_err(registry_error)?;
        let observation = self.observe_record(&record)?;
        self.registry
            .record_observation(
                &command.instance_id,
                "recovery_inspection_observed",
                observation.clone(),
            )
            .map_err(registry_error)?;
        if record.state.has_projected_owner() {
            if observation.owner.is_active() && observation.health.healthy() {
                return Err(RuntimeApplicationError::new(
                    RuntimeApplicationErrorKind::NotStale,
                    "the exact owner and both projected endpoints are healthy",
                ));
            }
            if !observation.owner.is_active() && !observation.health.all_closed() {
                return Err(RuntimeApplicationError::new(
                    RuntimeApplicationErrorKind::OwnershipAmbiguous,
                    "runtime endpoints are reachable without the exact durable Job Object owner",
                ));
            }
        } else if observation.owner.is_active() || !observation.health.all_closed() {
            return Err(RuntimeApplicationError::new(
                RuntimeApplicationErrorKind::OwnershipAmbiguous,
                "a terminal durable state conflicts with current owner or endpoint evidence",
            ));
        }
        match self
            .registry
            .begin_recovery(
                &command.request_id,
                &fingerprint,
                &command.instance_id,
                &authority_hash,
                self.clock.now(),
            )
            .map_err(registry_error)?
        {
            CommandStart::Execute(record) => self.finish_terminal(
                &command.request_id,
                record,
                InstanceState::RecoveryPending,
                InstanceState::Recovered,
                "recovery_observed",
            ),
            CommandStart::Noop(snapshot) | CommandStart::Replay(snapshot) => Ok(snapshot),
            CommandStart::ReplayFailure(failure) => Err(stored_failure(failure)),
        }
    }
}

#[derive(Serialize)]
struct StartSemantics<'a> {
    operation: &'static str,
    instance_id: &'a str,
    authority_hash: &'a str,
    launches: Vec<LaunchSemantics<'a>>,
}

#[derive(Serialize)]
struct LaunchSemantics<'a> {
    role: super::domain::ProcessRole,
    program: &'a std::path::Path,
    arguments: &'a [String],
    working_directory: &'a std::path::Path,
    environment: &'a std::collections::BTreeMap<String, String>,
    log_path: &'a std::path::Path,
}

impl<'a> From<&'a OwnedProcessLaunch> for LaunchSemantics<'a> {
    fn from(value: &'a OwnedProcessLaunch) -> Self {
        Self {
            role: value.role,
            program: &value.program,
            arguments: &value.arguments,
            working_directory: &value.working_directory,
            environment: &value.environment,
            log_path: &value.log_path,
        }
    }
}

fn fingerprint(value: &impl Serialize) -> Result<String, RuntimeApplicationError> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| {
            RuntimeApplicationError::new(
                RuntimeApplicationErrorKind::Unavailable,
                format!("serialize runtime command semantics: {error}"),
            )
        })
}

fn job_name(instance_id: &InstanceId, launch_id: &LaunchId) -> String {
    format!(
        "Local\\CodexOrchestrator.WorktreeRuntime.{}.{}",
        instance_id.as_str(),
        launch_id.as_str()
    )
}

fn review_window_expectation(record: &InstanceRecord) -> ReviewWindowExpectation {
    ReviewWindowExpectation {
        title: format!(
            "Codex Orchestrator [Worktree build: {}]",
            record.identity.review_name
        ),
        minimum_width: 900,
        minimum_height: 600,
    }
}

fn validate_owner_route(
    instance_id: &InstanceId,
    route: &OwnerRoute,
) -> Result<(), RuntimeApplicationError> {
    if route.job_name != job_name(instance_id, &route.launch_id) {
        return Err(RuntimeApplicationError::new(
            RuntimeApplicationErrorKind::OwnershipAmbiguous,
            "durable Job Object name does not match the instance and launch identity",
        ));
    }
    Ok(())
}

fn required_owner_route(record: &InstanceRecord) -> Result<&OwnerRoute, RuntimeApplicationError> {
    let route = record.owner_route.as_ref().ok_or_else(|| {
        RuntimeApplicationError::new(
            RuntimeApplicationErrorKind::OwnershipAmbiguous,
            "runtime state has no exact durable owner route",
        )
    })?;
    validate_owner_route(&record.identity.instance_id, route)?;
    Ok(route)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeApplicationErrorKind {
    NotFound,
    Unauthorized,
    Conflict,
    PortLeaseConflict,
    InvalidState,
    IdempotencyConflict,
    OperationInProgress,
    LaunchFailed,
    HealthFailed,
    OwnershipAmbiguous,
    NotStale,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeApplicationError {
    pub(crate) kind: RuntimeApplicationErrorKind,
    pub(crate) message: String,
}

impl RuntimeApplicationError {
    fn new(kind: RuntimeApplicationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RuntimeApplicationError {}

fn registry_error(error: RegistryError) -> RuntimeApplicationError {
    let kind = match error.kind {
        RegistryErrorKind::NotFound => RuntimeApplicationErrorKind::NotFound,
        RegistryErrorKind::Unauthorized => RuntimeApplicationErrorKind::Unauthorized,
        RegistryErrorKind::Conflict => RuntimeApplicationErrorKind::Conflict,
        RegistryErrorKind::PortLeaseConflict => RuntimeApplicationErrorKind::PortLeaseConflict,
        RegistryErrorKind::InvalidState => RuntimeApplicationErrorKind::InvalidState,
        RegistryErrorKind::IdempotencyConflict => RuntimeApplicationErrorKind::IdempotencyConflict,
        RegistryErrorKind::OperationInProgress => RuntimeApplicationErrorKind::OperationInProgress,
        RegistryErrorKind::Unavailable => RuntimeApplicationErrorKind::Unavailable,
    };
    RuntimeApplicationError::new(kind, error.message)
}

fn owner_error(error: OwnershipError) -> RuntimeApplicationError {
    RuntimeApplicationError::new(
        match error.kind {
            super::ownership::OwnershipErrorKind::AlreadyExists
            | super::ownership::OwnershipErrorKind::Ambiguous => {
                RuntimeApplicationErrorKind::OwnershipAmbiguous
            }
            super::ownership::OwnershipErrorKind::LaunchFailed => {
                RuntimeApplicationErrorKind::LaunchFailed
            }
            super::ownership::OwnershipErrorKind::Unavailable => {
                RuntimeApplicationErrorKind::Unavailable
            }
        },
        error.message,
    )
}

fn contract_error(
    operation: &'static str,
) -> impl FnOnce(super::domain::RuntimeContractError) -> RuntimeApplicationError {
    move |error| {
        RuntimeApplicationError::new(
            RuntimeApplicationErrorKind::Conflict,
            format!("{operation}: {error}"),
        )
    }
}

fn stored_failure(failure: StoredFailure) -> RuntimeApplicationError {
    let kind = match failure.kind.as_str() {
        "launch_failed" => RuntimeApplicationErrorKind::LaunchFailed,
        "health_failed" => RuntimeApplicationErrorKind::HealthFailed,
        "ownership_ambiguous" => RuntimeApplicationErrorKind::OwnershipAmbiguous,
        "not_stale" => RuntimeApplicationErrorKind::NotStale,
        "unauthorized" => RuntimeApplicationErrorKind::Unauthorized,
        "conflict" => RuntimeApplicationErrorKind::Conflict,
        "invalid_state" => RuntimeApplicationErrorKind::InvalidState,
        _ => RuntimeApplicationErrorKind::Unavailable,
    };
    RuntimeApplicationError::new(kind, failure.message)
}

fn command_result(command: CommandStart) -> Result<InstanceSnapshot, RuntimeApplicationError> {
    match command {
        CommandStart::Replay(snapshot) | CommandStart::Noop(snapshot) => Ok(snapshot),
        CommandStart::ReplayFailure(failure) => Err(stored_failure(failure)),
        CommandStart::Execute(_) => Err(RuntimeApplicationError::new(
            RuntimeApplicationErrorKind::Unavailable,
            "runtime replay unexpectedly requested execution",
        )),
    }
}

fn error_kind_name(kind: RuntimeApplicationErrorKind) -> &'static str {
    match kind {
        RuntimeApplicationErrorKind::NotFound => "not_found",
        RuntimeApplicationErrorKind::Unauthorized => "unauthorized",
        RuntimeApplicationErrorKind::Conflict => "conflict",
        RuntimeApplicationErrorKind::PortLeaseConflict => "port_lease_conflict",
        RuntimeApplicationErrorKind::InvalidState => "invalid_state",
        RuntimeApplicationErrorKind::IdempotencyConflict => "idempotency_conflict",
        RuntimeApplicationErrorKind::OperationInProgress => "operation_in_progress",
        RuntimeApplicationErrorKind::LaunchFailed => "launch_failed",
        RuntimeApplicationErrorKind::HealthFailed => "health_failed",
        RuntimeApplicationErrorKind::OwnershipAmbiguous => "ownership_ambiguous",
        RuntimeApplicationErrorKind::NotStale => "not_stale",
        RuntimeApplicationErrorKind::Unavailable => "unavailable",
    }
}
