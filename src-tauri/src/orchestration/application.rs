use super::confirmation::InitiationConfirmationCoordinator;
use super::{
    domain::{SaveEpicPlanProposalCommand, SaveProposalError, SaveProposalResult},
    mcp::PlanBuilderInvocation,
    repository::{NativeQueryV2, SqliteOrchestrationRepository},
};
use crate::{
    agent_sessions::{
        application::{
            AgentSessionApplication, ApplicationInvocationLaunchEvidence,
            CreateAgentSessionCommand, SendAgentSessionMessageCommand,
            SendAgentSessionMessageResult, SendIdempotentApplicationAgentSessionMessageCommand,
        },
        domain::{
            AgentInvocation, AgentInvocationId, AgentInvocationInputProvenance,
            AgentRuntimeOptions, AgentSessionId,
        },
        ports::RuntimeLaunchExtension,
    },
    orchestration::mcp::{self, CodexMcpInjection, ManagedPlanBuilderInvocation},
};
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};

pub(crate) struct OrchestrationApplication {
    repository: Arc<SqliteOrchestrationRepository>,
}

/// Owns the one-invocation MCP capability for the managed Epic Plan Builder.  Durable planning
/// identity is bootstrapped separately from the short-lived transport credential.
pub(crate) struct ManagedPlanBuilderService {
    orchestration: Arc<OrchestrationApplication>,
    sessions: Arc<AgentSessionApplication>,
    registry: Arc<ManagedPlanBuilderRegistry>,
    confirmations: Arc<InitiationConfirmationCoordinator>,
    factory: Arc<dyn ManagedPlanBuilderInvocationFactory>,
    send_lock: Mutex<()>,
}

impl super::confirmation::ButtonInitiationContextScheduler for OrchestrationApplication {
    fn schedule(&self, initiation: &super::domain::InitiateEpicResult) -> Result<(), String> {
        self.repository
            .schedule_button_initiation_context(initiation)
            .map_err(|error| error.to_string())
    }
}

pub(crate) trait ManagedPlanBuilderInvocationHandle: Send {
    fn injection(&self) -> &CodexMcpInjection;
    fn bind_agent_invocation(&self, invocation_id: AgentInvocationId);
    fn stop(self: Box<Self>);
}
pub(crate) trait ManagedPlanBuilderInvocationFactory: Send + Sync {
    fn start(
        &self,
        application: Arc<OrchestrationApplication>,
        confirmations: Arc<InitiationConfirmationCoordinator>,
        invocation: PlanBuilderInvocation,
        enabled_tools: &[String],
        required: bool,
    ) -> Result<Box<dyn ManagedPlanBuilderInvocationHandle>, String>;
}
struct ProductionManagedInvocationFactory;
struct ProductionManagedInvocation(ManagedPlanBuilderInvocation);
impl ManagedPlanBuilderInvocationHandle for ProductionManagedInvocation {
    fn injection(&self) -> &CodexMcpInjection {
        &self.0.injection
    }
    fn bind_agent_invocation(&self, invocation_id: AgentInvocationId) {
        self.0.bind_agent_invocation(invocation_id);
    }
    fn stop(self: Box<Self>) {
        self.0.stop();
    }
}
impl ManagedPlanBuilderInvocationFactory for ProductionManagedInvocationFactory {
    fn start(
        &self,
        application: Arc<OrchestrationApplication>,
        confirmations: Arc<InitiationConfirmationCoordinator>,
        invocation: PlanBuilderInvocation,
        enabled_tools: &[String],
        required: bool,
    ) -> Result<Box<dyn ManagedPlanBuilderInvocationHandle>, String> {
        mcp::start_managed_invocation(
            application,
            confirmations,
            invocation,
            enabled_tools,
            required,
            vec!["tauri://localhost".into()],
        )
        .map(|managed| {
            Box::new(ProductionManagedInvocation(managed))
                as Box<dyn ManagedPlanBuilderInvocationHandle>
        })
        .map_err(|error| error.to_string())
    }
}

#[derive(Default)]
pub(crate) struct ManagedPlanBuilderRegistry {
    active: Mutex<HashMap<AgentInvocationId, Box<dyn ManagedPlanBuilderInvocationHandle>>>,
}
impl ManagedPlanBuilderRegistry {
    fn insert(
        &self,
        id: AgentInvocationId,
        invocation: Box<dyn ManagedPlanBuilderInvocationHandle>,
    ) -> Result<(), (String, Box<dyn ManagedPlanBuilderInvocationHandle>)> {
        let Ok(mut active) = self.active.lock() else {
            return Err((
                "managed Plan Builder registry is poisoned".to_string(),
                invocation,
            ));
        };
        active.insert(id, invocation);
        Ok(())
    }
    pub(crate) fn on_terminal(&self, invocation: &AgentInvocation) {
        if let Ok(mut active) = self.active.lock() {
            if let Some(managed) = active.remove(&invocation.id) {
                managed.stop();
            }
        }
    }
    pub(crate) fn shutdown(&self) {
        if let Ok(mut active) = self.active.lock() {
            for (_, managed) in active.drain() {
                managed.stop();
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn active_count(&self) -> usize {
        self.active.lock().map_or(0, |active| active.len())
    }
}

impl ManagedPlanBuilderService {
    pub(crate) fn new(
        orchestration: Arc<OrchestrationApplication>,
        sessions: Arc<AgentSessionApplication>,
        registry: Arc<ManagedPlanBuilderRegistry>,
        confirmations: Arc<InitiationConfirmationCoordinator>,
    ) -> Arc<Self> {
        Arc::new(Self {
            orchestration,
            sessions,
            registry,
            confirmations,
            factory: Arc::new(ProductionManagedInvocationFactory),
            send_lock: Mutex::new(()),
        })
    }

    #[cfg(test)]
    fn with_factory(
        orchestration: Arc<OrchestrationApplication>,
        sessions: Arc<AgentSessionApplication>,
        registry: Arc<ManagedPlanBuilderRegistry>,
        confirmations: Arc<InitiationConfirmationCoordinator>,
        factory: Arc<dyn ManagedPlanBuilderInvocationFactory>,
    ) -> Arc<Self> {
        Arc::new(Self {
            orchestration,
            sessions,
            registry,
            confirmations,
            factory,
            send_lock: Mutex::new(()),
        })
    }

    pub(crate) fn shutdown(&self) {
        self.registry.shutdown();
    }

    /// Reconciles the durable draft already created by the acknowledged managed send.
    pub(crate) fn reconcile_session(
        &self,
        session_id: AgentSessionId,
        title: Option<String>,
    ) -> Result<ManagedPlanBuilderDraft, String> {
        self.sessions
            .load_session(&session_id)
            .map_err(|error| error.to_string())?;
        let (draft_id, _, _) = self
            .orchestration
            .repository
            .bootstrap_managed_plan_builder(session_id.as_str())
            .map_err(|error| error.to_string())?;
        if title
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            self.orchestration
                .repository
                .update_planning_draft_title(
                    &draft_id,
                    session_id.as_str(),
                    title.as_deref(),
                    &format!("initial-title:{}", session_id.as_str()),
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(ManagedPlanBuilderDraft {
            draft_id: draft_id.as_str().into(),
            session_id: session_id.as_str().into(),
        })
    }

    pub(crate) fn update_title(
        &self,
        draft_id: &str,
        session_id: &str,
        title: Option<&str>,
        idempotency_key: &str,
    ) -> Result<(), String> {
        let draft = super::domain::EpicPlanningDraftId::new(draft_id).map_err(|e| e.to_string())?;
        self.orchestration
            .repository
            .update_planning_draft_title(&draft, session_id, title, idempotency_key)
            .map_err(|e| e.to_string())
    }

    pub(crate) fn cancel(
        &self,
        draft_id: &str,
        session_id: &str,
        idempotency_key: &str,
    ) -> Result<(), String> {
        let draft = super::domain::EpicPlanningDraftId::new(draft_id).map_err(|e| e.to_string())?;
        self.orchestration
            .repository
            .cancel_planning_draft(&draft, session_id, idempotency_key)
            .map_err(|e| e.to_string())
    }

    pub(crate) fn send(
        &self,
        session_id: Option<AgentSessionId>,
        submitted_text: String,
        title: Option<String>,
        _working_directory: Option<String>,
        requested_options: Option<AgentRuntimeOptions>,
    ) -> Result<SendAgentSessionMessageResult, String> {
        self.send_with_provenance(
            session_id,
            submitted_text,
            title,
            requested_options,
            AgentInvocationInputProvenance::User,
        )
    }

    fn send_with_provenance(
        &self,
        session_id: Option<AgentSessionId>,
        submitted_text: String,
        title: Option<String>,
        requested_options: Option<AgentRuntimeOptions>,
        input_provenance: AgentInvocationInputProvenance,
    ) -> Result<SendAgentSessionMessageResult, String> {
        let _send_guard = self
            .send_lock
            .lock()
            .map_err(|_| "managed Plan Builder send serialization is unavailable".to_string())?;
        let harness = super::conversation_harness::profile(
            super::conversation_harness::ConversationHarnessRole::EpicPlanBuilder,
        )?;
        let harness_options = harness.runtime_options();
        if requested_options.as_ref().is_some_and(|requested| {
            requested.model.is_some()
                || requested
                    .sandbox
                    .is_some_and(|sandbox| Some(sandbox) != harness_options.sandbox)
        }) {
            return Err(
                "Epic Plan Builder runtime options are fixed by its Conversation Harness".into(),
            );
        }
        let discovery_root = super::conversation_harness::epic_plan_builder_discovery_root()?;
        let session_id = match session_id {
            Some(id) => id,
            None => {
                self.sessions
                    .create_session(CreateAgentSessionCommand {
                        title,
                        working_directory: Some(discovery_root.clone()),
                        requested_options: harness_options.clone(),
                    })
                    .map_err(|error| error.to_string())?
                    .id
            }
        };
        let has_prior_invocation = !self
            .sessions
            .load_session(&session_id)
            .map_err(|error| error.to_string())?
            .invocations
            .is_empty();
        self.reconcile_plan_builder_context(&session_id)?;
        let (draft_id, profile_id, association_id) = self
            .orchestration
            .repository
            .bootstrap_managed_plan_builder(session_id.as_str())
            .map_err(|error| error.to_string())?;
        let expected_revision = self
            .orchestration
            .capture_plan_builder_precondition(
                &draft_id,
                &profile_id,
                &association_id,
                session_id.as_str(),
                "managed-plan-builder",
            )
            .map_err(|error| error.to_string())?;
        let managed_invocation = PlanBuilderInvocation::new(
            session_id.clone(),
            draft_id,
            profile_id,
            association_id,
            "managed-plan-builder".into(),
            expected_revision,
        );
        let managed = self.factory.start(
            self.orchestration.clone(),
            self.confirmations.clone(),
            managed_invocation,
            &harness.mcp.enabled_tools,
            harness.mcp.required,
        )?;
        let mut additional_args = harness.runtime_configuration_args();
        additional_args.extend(managed.injection().configuration_args.clone());
        let extension = RuntimeLaunchExtension {
            additional_args,
            environment: vec![managed.injection().environment.clone()],
            initial_prompt_prefix: None,
        };
        let invocation_id = self.sessions.allocate_application_invocation_id();
        let claim_id = format!("plan-builder-context-claim-{}", uuid::Uuid::new_v4());
        let pending_context = match self
            .orchestration
            .repository
            .claim_pending_plan_builder_context(
                session_id.as_str(),
                &claim_id,
                invocation_id.as_str(),
            ) {
            Ok(delivery) => delivery,
            Err(error) => {
                managed.stop();
                return Err(error.to_string());
            }
        };
        let mut extension = extension;
        extension.initial_prompt_prefix = pending_context
            .as_ref()
            .map(|delivery| crate::agent_sessions::ports::InitialPromptPrefix {
                source: "epic_plan_builder_button_initiation".into(),
                version: 1,
                content: format!(
                    "The application observed confirmed Epic initiation {} for Epic {} and durably projected it. Continue from that product fact; do not infer Bootstrap material acceptance, Epic Runner launch, or Sprint start.",
                    delivery.initiation_id, delivery.epic_id
                ),
            })
            .or_else(|| (!has_prior_invocation).then(|| harness.initial_prompt_prefix()));
        let command = SendIdempotentApplicationAgentSessionMessageCommand {
            invocation_id,
            message: SendAgentSessionMessageCommand {
                session_id: Some(session_id.clone()),
                submitted_text,
                title: None,
                working_directory: Some(discovery_root),
                requested_options: Some(harness_options),
            },
        };
        let result = match input_provenance {
            AgentInvocationInputProvenance::User => self
                .sessions
                .send_idempotent_user_message_with_launch_observation(command, Some(extension)),
            AgentInvocationInputProvenance::Application => self
                .sessions
                .send_idempotent_application_message_with_launch_observation(
                    command,
                    Some(extension),
                ),
        };
        match result {
            Ok(launch) => {
                let result = launch.acknowledgement;
                managed.bind_agent_invocation(result.invocation_id.clone());
                // A runtime is allowed to synchronously report a terminal outcome before its
                // launch call returns. In that case the notification preceded registry insertion,
                // so avoid retaining a listener that has already lost its owner.
                let terminal = self
                    .sessions
                    .load_session(&result.session_id)
                    .map_err(|error| error.to_string())?
                    .invocations
                    .iter()
                    .find(|history| history.invocation.id == result.invocation_id)
                    .is_some_and(|history| history.invocation.status.is_terminal());
                if terminal {
                    managed.stop();
                } else {
                    if let Err((error, managed)) =
                        self.registry.insert(result.invocation_id.clone(), managed)
                    {
                        // Retain no listener when ownership cannot be recorded.
                        managed.stop();
                        return Err(error);
                    }
                }
                if launch.launch_accepted {
                    if let Some(delivery) = pending_context.as_ref() {
                        self.orchestration
                            .repository
                            .consume_plan_builder_context(delivery)
                            .map_err(|error| error.to_string())?;
                    }
                } else if let Some(delivery) = pending_context.as_ref() {
                    self.orchestration
                        .repository
                        .release_plan_builder_context(delivery)
                        .map_err(|error| error.to_string())?;
                }
                Ok(result)
            }
            Err(error) => {
                if pending_context.is_some() {
                    self.reconcile_plan_builder_context(&session_id)
                        .map_err(|reconcile| format!("{error}; {reconcile}"))?;
                }
                managed.stop();
                Err(error.to_string())
            }
        }
    }

    fn reconcile_plan_builder_context(&self, session_id: &AgentSessionId) -> Result<(), String> {
        let Some(delivery) = self
            .orchestration
            .repository
            .load_claimed_plan_builder_context(session_id.as_str())
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let invocation_id = AgentInvocationId::new(&delivery.target_invocation_id)
            .map_err(|error| error.to_string())?;
        let provenance = self
            .sessions
            .load_session(session_id)
            .map_err(|error| error.to_string())?
            .invocations
            .into_iter()
            .find(|history| history.invocation.id == invocation_id)
            .map(|history| history.invocation.input_provenance);
        let evidence = match provenance {
            None => ApplicationInvocationLaunchEvidence::NeverPersisted,
            Some(AgentInvocationInputProvenance::User) => self
                .sessions
                .user_invocation_launch_evidence(&invocation_id, session_id)
                .map_err(|error| error.to_string())?,
            Some(AgentInvocationInputProvenance::Application) => self
                .sessions
                .application_invocation_launch_evidence(&invocation_id, session_id)
                .map_err(|error| error.to_string())?,
        };
        match evidence {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => self
                .orchestration
                .repository
                .consume_plan_builder_context(&delivery),
            ApplicationInvocationLaunchEvidence::NeverPersisted
            | ApplicationInvocationLaunchEvidence::PersistedNotAccepted => self
                .orchestration
                .repository
                .release_plan_builder_context(&delivery),
        }
        .map_err(|error| error.to_string())
    }

    pub(crate) fn request_plan(
        &self,
        session_id: Option<AgentSessionId>,
        title: Option<String>,
        _working_directory: Option<String>,
        requested_options: Option<AgentRuntimeOptions>,
    ) -> Result<SendAgentSessionMessageResult, String> {
        self.send_with_provenance(
            session_id,
            "Build the epic plan based on what we have discussed".into(),
            title,
            requested_options,
            AgentInvocationInputProvenance::Application,
        )
    }
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManagedPlanBuilderDraft {
    pub(crate) draft_id: String,
    pub(crate) session_id: String,
}

impl OrchestrationApplication {
    pub(crate) fn new(repository: Arc<SqliteOrchestrationRepository>) -> Self {
        Self { repository }
    }
    pub(crate) fn save_epic_plan_proposal(
        &self,
        command: SaveEpicPlanProposalCommand,
    ) -> Result<SaveProposalResult, SaveProposalError> {
        self.repository.save_epic_plan_proposal(command)
    }
    pub(crate) fn capture_plan_builder_precondition(
        &self,
        draft_id: &super::domain::EpicPlanningDraftId,
        profile_id: &super::domain::CapabilityProfileId,
        association_id: &super::domain::PlanningDraftAgentSessionAssociationId,
        agent_session_id: &str,
        actor_id: &str,
    ) -> Result<Option<String>, SaveProposalError> {
        self.repository.capture_plan_builder_precondition(
            draft_id,
            profile_id,
            association_id,
            agent_session_id,
            actor_id,
        )
    }
    pub(crate) fn native_query(&self) -> Result<NativeQueryV2, String> {
        self.repository.native_query()
    }
    pub(crate) fn initiate_epic(
        &self,
        command: super::domain::InitiateEpicCommand,
    ) -> Result<super::domain::InitiateEpicResult, super::domain::InitiateEpicError> {
        self.repository.initiate_epic(command)
    }
    pub(crate) fn capture_agent_initiation_precondition(
        &self,
        invocation: &PlanBuilderInvocation,
    ) -> Result<String, super::domain::InitiateEpicError> {
        self.repository.capture_agent_initiation_precondition(
            &invocation.draft_id,
            &invocation.profile_id,
            &invocation.association_id,
            invocation.agent_session_id.as_str(),
            &invocation.actor_id,
        )
    }
    pub(crate) fn initiation_is_projected(
        &self,
        initiation_id: &super::domain::EpicInitiationId,
    ) -> Result<bool, String> {
        self.repository.initiation_is_projected(initiation_id)
    }
    pub(crate) fn plan_builder_context(
        &self,
        invocation: &PlanBuilderInvocation,
    ) -> Result<serde_json::Value, SaveProposalError> {
        self.repository.plan_builder_context(
            &invocation.draft_id,
            &invocation.profile_id,
            &invocation.association_id,
            &invocation.actor_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::{
        application::{
            AgentSessionNotification, AgentSessionNotifier, SystemAgentSessionProviders,
        },
        domain::{AgentInvocationId, AgentInvocationTerminalStatus},
        ports::{
            AgentRuntime, AgentRuntimeUpdateSink, RuntimeInvocationMode, RuntimeInvocationOutcome,
            RuntimeInvocationPreflight, RuntimeInvocationRequest, RuntimePortError,
            RuntimePortErrorKind, RuntimeUpdate,
        },
        repository::SqliteAgentSessionRepository,
    };
    use rusqlite::{params, Connection};
    use std::{
        fs,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    struct ConfirmationNotifier;
    impl super::super::confirmation::InitiationConfirmationNotifier for ConfirmationNotifier {
        fn notify(
            &self,
            _: super::super::confirmation::InitiationConfirmationEvent,
        ) -> Result<(), String> {
            Ok(())
        }
    }
    fn confirmations(
        application: Arc<OrchestrationApplication>,
    ) -> Arc<InitiationConfirmationCoordinator> {
        InitiationConfirmationCoordinator::new(application, Arc::new(ConfirmationNotifier))
    }

    #[derive(Default)]
    struct Factory {
        injections: Mutex<Vec<CodexMcpInjection>>,
        stops: Arc<Mutex<usize>>,
    }
    struct Handle {
        injection: CodexMcpInjection,
        stops: Arc<Mutex<usize>>,
    }
    impl ManagedPlanBuilderInvocationHandle for Handle {
        fn injection(&self) -> &CodexMcpInjection {
            &self.injection
        }
        fn bind_agent_invocation(&self, _: AgentInvocationId) {}
        fn stop(self: Box<Self>) {
            *self.stops.lock().unwrap() += 1;
        }
    }
    impl ManagedPlanBuilderInvocationFactory for Factory {
        fn start(
            &self,
            _: Arc<OrchestrationApplication>,
            _: Arc<InitiationConfirmationCoordinator>,
            _: PlanBuilderInvocation,
            enabled_tools: &[String],
            required: bool,
        ) -> Result<Box<dyn ManagedPlanBuilderInvocationHandle>, String> {
            let n = self.injections.lock().unwrap().len();
            let injection = CodexMcpInjection::new(
                &format!("http://127.0.0.1:{}/mcp", 7000 + n),
                format!("bearer-{n}"),
                enabled_tools,
                required,
            );
            self.injections.lock().unwrap().push(injection.clone());
            Ok(Box::new(Handle {
                injection,
                stops: self.stops.clone(),
            }))
        }
    }
    #[derive(Clone, Copy, Default)]
    enum RuntimeMode {
        #[default]
        Active,
        PreflightError,
        LaunchError,
        Synchronous(AgentInvocationTerminalStatus),
    }
    #[derive(Default)]
    struct Runtime {
        requests: Mutex<Vec<RuntimeInvocationRequest>>,
        active: Mutex<Vec<(AgentInvocationId, Arc<dyn AgentRuntimeUpdateSink>)>>,
        mode: Mutex<RuntimeMode>,
    }
    impl AgentRuntime for Runtime {
        fn preflight_invocation(
            &self,
            _: RuntimeInvocationMode,
            requested: &AgentRuntimeOptions,
        ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            if matches!(*self.mode.lock().unwrap(), RuntimeMode::PreflightError) {
                return Err(RuntimePortError::new(
                    RuntimePortErrorKind::UnsupportedOptions,
                    "preflight failed",
                ));
            }
            Ok(RuntimeInvocationPreflight {
                effective_options: requested.clone(),
            })
        }
        fn start_invocation(
            &self,
            request: RuntimeInvocationRequest,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.launch(request, sink)
        }
        fn resume_invocation(
            &self,
            request: RuntimeInvocationRequest,
            _: crate::agent_sessions::domain::ExternalRuntimeContextId,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.launch(request, sink)
        }
        fn cancel_invocation(&self, _: &AgentInvocationId) -> Result<(), RuntimePortError> {
            Ok(())
        }
        fn shutdown(&self) -> Result<(), RuntimePortError> {
            Ok(())
        }
    }
    impl Runtime {
        fn launch(
            &self,
            request: RuntimeInvocationRequest,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.requests.lock().unwrap().push(request.clone());
            match *self.mode.lock().unwrap() {
                RuntimeMode::Active => {
                    self.active
                        .lock()
                        .unwrap()
                        .push((request.invocation_id, sink));
                    Ok(())
                }
                RuntimeMode::LaunchError => Err(RuntimePortError::new(
                    RuntimePortErrorKind::LaunchFailed,
                    "launch failed",
                )),
                RuntimeMode::Synchronous(status) => {
                    sink.emit_update(
                        &request.invocation_id,
                        RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                            status,
                            exit_code: None,
                            signal: None,
                            runtime_error: None,
                        }),
                    )?;
                    Ok(())
                }
                RuntimeMode::PreflightError => unreachable!(),
            }
        }
    }
    struct Notifier(Arc<ManagedPlanBuilderRegistry>);
    impl AgentSessionNotifier for Notifier {
        fn notify(&self, n: AgentSessionNotification) -> Result<(), String> {
            if let AgentSessionNotification::InvocationTerminal { invocation, .. } = n {
                self.0.on_terminal(&invocation);
            }
            Ok(())
        }
    }

    struct LiveNotifier {
        registry: Arc<ManagedPlanBuilderRegistry>,
        terminal: Arc<(Mutex<Option<AgentInvocation>>, std::sync::Condvar)>,
    }
    impl AgentSessionNotifier for LiveNotifier {
        fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
            if let AgentSessionNotification::InvocationTerminal { invocation, .. } = notification {
                self.registry.on_terminal(&invocation);
                let (terminal, ready) = &*self.terminal;
                *terminal.lock().map_err(|_| "terminal lock failed")? = Some(invocation);
                ready.notify_all();
            }
            Ok(())
        }
    }

    struct LivePlanBuilderHarness {
        _directory: tempfile::TempDir,
        database_path: PathBuf,
        service: Arc<ManagedPlanBuilderService>,
        runtime: Arc<crate::runtime::codex::CodexCliRuntime>,
        repository: Arc<SqliteOrchestrationRepository>,
        terminal: Arc<(Mutex<Option<AgentInvocation>>, std::sync::Condvar)>,
    }

    impl LivePlanBuilderHarness {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database_path = directory.path().join("active.sqlite");
            drop(crate::storage::open_active_database(&database_path).unwrap());
            let registry = Arc::new(ManagedPlanBuilderRegistry::default());
            let terminal = Arc::new((Mutex::new(None), std::sync::Condvar::new()));
            let runtime = Arc::new(crate::runtime::codex::CodexCliRuntime::system(
                "codex", None,
            ));
            let providers = Arc::new(SystemAgentSessionProviders);
            let sessions = Arc::new(AgentSessionApplication::new(
                Arc::new(SqliteAgentSessionRepository::open(&database_path).unwrap()),
                runtime.clone(),
                Arc::new(LiveNotifier {
                    registry: registry.clone(),
                    terminal: terminal.clone(),
                }),
                providers.clone(),
                providers,
                None,
            ));
            let repository = Arc::new(SqliteOrchestrationRepository::open(&database_path).unwrap());
            let orchestration = Arc::new(OrchestrationApplication::new(repository.clone()));
            let service = ManagedPlanBuilderService::new(
                orchestration.clone(),
                sessions,
                registry,
                confirmations(orchestration),
            );
            Self {
                _directory: directory,
                database_path,
                service,
                runtime,
                repository,
                terminal,
            }
        }

        fn send(
            &self,
            session_id: Option<AgentSessionId>,
            prompt: &str,
        ) -> SendAgentSessionMessageResult {
            *self.terminal.0.lock().unwrap() = None;
            let sent = self
                .service
                .send(
                    session_id,
                    prompt.into(),
                    Some("Live managed Plan Builder matrix".into()),
                    None,
                    None,
                )
                .unwrap();
            let finished = self.terminal.0.lock().unwrap();
            let (finished, wait) = self
                .terminal
                .1
                .wait_timeout_while(finished, std::time::Duration::from_secs(180), |value| {
                    value.is_none()
                })
                .unwrap();
            assert!(!wait.timed_out(), "installed Codex invocation timed out");
            assert_eq!(
                finished.as_ref().unwrap().status,
                crate::agent_sessions::domain::AgentInvocationStatus::Completed
            );
            sent
        }

        fn proposal_count(&self) -> usize {
            serde_json::to_value(self.repository.native_query().unwrap()).unwrap()
                ["proposalRevisions"]
                .as_array()
                .unwrap()
                .len()
        }

        fn completed_mcp_calls(&self, tool: &str) -> i64 {
            Connection::open(&self.database_path)
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM agent_session_runtime_events WHERE json_extract(raw_payload_json,'$.type')='item.completed' AND json_extract(raw_payload_json,'$.item.type')='mcp_tool_call' AND raw_payload_json LIKE ?1",
                    [format!("%{tool}%")],
                    |row| row.get(0),
                )
                .unwrap()
        }

        fn user_invocation_count(&self) -> i64 {
            Connection::open(&self.database_path)
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM agent_session_invocations WHERE input_provenance='user'",
                    [],
                    |row| row.get(0),
                )
                .unwrap()
        }
    }

    impl Drop for LivePlanBuilderHarness {
        fn drop(&mut self) {
            self.service.shutdown();
            let _ = self.runtime.shutdown();
        }
    }

    fn service_fixture(
        mode: RuntimeMode,
    ) -> (
        Arc<ManagedPlanBuilderService>,
        Arc<Runtime>,
        Arc<Factory>,
        Arc<ManagedPlanBuilderRegistry>,
        std::path::PathBuf,
    ) {
        let path = std::env::temp_dir().join(format!(
            "managed-plan-builder-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let connection = Connection::open(&path).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        drop(connection);
        let registry = Arc::new(ManagedPlanBuilderRegistry::default());
        let runtime = Arc::new(Runtime {
            mode: Mutex::new(mode),
            ..Default::default()
        });
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            Arc::new(SqliteAgentSessionRepository::open(&path).unwrap()),
            runtime.clone(),
            Arc::new(Notifier(registry.clone())),
            providers.clone(),
            providers,
            None,
        ));
        let orchestration = Arc::new(OrchestrationApplication::new(Arc::new(
            SqliteOrchestrationRepository::open(&path).unwrap(),
        )));
        let factory = Arc::new(Factory::default());
        let confirmation = confirmations(orchestration.clone());
        (
            ManagedPlanBuilderService::with_factory(
                orchestration,
                sessions,
                registry.clone(),
                confirmation,
                factory.clone(),
            ),
            runtime,
            factory,
            registry,
            path,
        )
    }

    fn reopen_service(
        path: &std::path::Path,
        runtime: Arc<Runtime>,
    ) -> Arc<ManagedPlanBuilderService> {
        let registry = Arc::new(ManagedPlanBuilderRegistry::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            Arc::new(SqliteAgentSessionRepository::open(path).unwrap()),
            runtime,
            Arc::new(Notifier(registry.clone())),
            providers.clone(),
            providers,
            None,
        ));
        sessions.reconcile_startup().unwrap();
        let orchestration = Arc::new(OrchestrationApplication::new(Arc::new(
            SqliteOrchestrationRepository::open(path).unwrap(),
        )));
        ManagedPlanBuilderService::with_factory(
            orchestration.clone(),
            sessions,
            registry,
            confirmations(orchestration),
            Arc::new(Factory::default()),
        )
    }

    fn prepare_button_context(
        service: &ManagedPlanBuilderService,
        runtime: &Runtime,
    ) -> SendAgentSessionMessageResult {
        let first = service
            .send(None, "Discuss the Epic first.".into(), None, None, None)
            .unwrap();
        let (first_id, first_sink) = runtime.active.lock().unwrap().remove(0);
        first_sink
            .emit_update(
                &first_id,
                RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                    status: AgentInvocationTerminalStatus::Completed,
                    exit_code: Some(0),
                    signal: None,
                    runtime_error: None,
                }),
            )
            .unwrap();
        let (draft, profile, association) = service
            .orchestration
            .repository
            .bootstrap_managed_plan_builder(first.session_id.as_str())
            .unwrap();
        let saved = service
            .orchestration
            .save_epic_plan_proposal(SaveEpicPlanProposalCommand {
                epic_planning_draft_id: draft.clone(),
                capability_profile_id: profile,
                agent_session_association_id: association,
                agent_session_id: first.session_id.as_str().into(),
                actor_id: "managed-plan-builder".into(),
                expected_revision: None,
                proposal: super::super::domain::PlanBuilderProposal {
                    suggested_epic_name: Some("Context Epic".into()),
                    sprints: vec![super::super::domain::ProposedSprint {
                        title: "Context Sprint".into(),
                        intended_movement: "Prove one-shot context delivery.".into(),
                        concern_summaries: vec![],
                    }],
                },
                idempotency_key: "context-proposal".into(),
            })
            .unwrap();
        let initiation = service
            .orchestration
            .initiate_epic(super::super::domain::InitiateEpicCommand {
                epic_planning_draft_id: draft,
                expected_revision_token: saved.revision_token,
                actor_id: "application-user".into(),
                idempotency_key: "context-initiation".into(),
            })
            .unwrap();
        service
            .orchestration
            .repository
            .schedule_button_initiation_context(&initiation)
            .unwrap();
        first
    }

    fn context_extension(
        delivery: &super::super::repository::PendingPlanBuilderContextDelivery,
    ) -> RuntimeLaunchExtension {
        RuntimeLaunchExtension {
            additional_args: Vec::new(),
            environment: Vec::new(),
            initial_prompt_prefix: Some(crate::agent_sessions::ports::InitialPromptPrefix {
                source: "epic_plan_builder_button_initiation".into(),
                version: 1,
                content: format!(
                    "The application observed confirmed Epic initiation {} for Epic {} and durably projected it.",
                    delivery.initiation_id, delivery.epic_id
                ),
            }),
        }
    }

    fn send_exact_user_query(
        service: &ManagedPlanBuilderService,
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        submitted_text: &str,
        extension: RuntimeLaunchExtension,
    ) -> (SendAgentSessionMessageResult, bool) {
        let result = service
            .sessions
            .send_idempotent_user_message_with_launch_observation(
                SendIdempotentApplicationAgentSessionMessageCommand {
                    invocation_id,
                    message: SendAgentSessionMessageCommand {
                        session_id: Some(session_id),
                        submitted_text: submitted_text.into(),
                        title: None,
                        working_directory: None,
                        requested_options: None,
                    },
                },
                Some(extension),
            )
            .unwrap();
        (result.acknowledgement, result.launch_accepted)
    }

    #[test]
    fn service_stops_on_preflight_launch_and_synchronous_terminal_paths() {
        for mode in [
            RuntimeMode::PreflightError,
            RuntimeMode::LaunchError,
            RuntimeMode::Synchronous(AgentInvocationTerminalStatus::Completed),
        ] {
            let (service, runtime, factory, registry, path) = service_fixture(mode);
            let result = service.send(None, "plan".into(), None, None, None).unwrap();
            let invocation = service
                .sessions
                .load_session(&result.session_id)
                .unwrap()
                .invocations
                .into_iter()
                .find(|history| history.invocation.id == result.invocation_id)
                .unwrap()
                .invocation;
            assert!(invocation.status.is_terminal());
            assert_eq!(registry.active_count(), 0);
            assert_eq!(*factory.stops.lock().unwrap(), 1);
            match mode {
                RuntimeMode::PreflightError => assert!(runtime.requests.lock().unwrap().is_empty()),
                RuntimeMode::LaunchError | RuntimeMode::Synchronous(_) => {
                    assert_eq!(runtime.requests.lock().unwrap().len(), 1)
                }
                _ => unreachable!(),
            }
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn button_context_preserves_submitted_text_retries_launch_failure_and_consumes_once() {
        let (service, runtime, _factory, _registry, path) = service_fixture(RuntimeMode::Active);
        let first = prepare_button_context(&service, &runtime);

        *runtime.mode.lock().unwrap() = RuntimeMode::LaunchError;
        let failed = service
            .send(
                Some(first.session_id.clone()),
                "Continue after initiation.".into(),
                None,
                None,
                None,
            )
            .unwrap();
        assert!(service
            .sessions
            .load_session(&first.session_id)
            .unwrap()
            .invocations
            .iter()
            .find(|history| history.invocation.id == failed.invocation_id)
            .unwrap()
            .invocation
            .status
            .is_terminal());
        *runtime.mode.lock().unwrap() = RuntimeMode::Active;
        let delivered = service
            .send(
                Some(first.session_id.clone()),
                "Retry the same original query.".into(),
                None,
                None,
                None,
            )
            .unwrap();
        let requests = runtime.requests.lock().unwrap();
        let failed_request = requests
            .iter()
            .find(|request| request.invocation_id == failed.invocation_id)
            .unwrap();
        let delivered_request = requests
            .iter()
            .find(|request| request.invocation_id == delivered.invocation_id)
            .unwrap();
        for request in [failed_request, delivered_request] {
            assert!(request.submitted_text.contains(
                "provenance=\"product_initial_prompt_prefix\" source=\"epic_plan_builder_button_initiation\" version=\"1\""
            ));
            assert!(request
                .submitted_text
                .contains("do not infer Bootstrap material acceptance"));
        }
        drop(requests);
        let history = service.sessions.load_session(&first.session_id).unwrap();
        assert_eq!(
            history
                .invocations
                .iter()
                .find(|item| item.invocation.id == delivered.invocation_id)
                .unwrap()
                .invocation
                .submitted_text,
            "Retry the same original query."
        );
        let connection = Connection::open(&path).unwrap();
        let facts: (String, String, String) = connection
            .query_row(
                "SELECT delivered_to_invocation_id,delivered_at,consumed_at FROM plan_builder_context_deliveries",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(facts.0, delivered.invocation_id.as_str());
        assert_eq!(facts.1, facts.2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn plan_action_keeps_application_provenance_and_reconciles_context_after_restart() {
        let (service, runtime, _factory, registry, path) = service_fixture(RuntimeMode::Active);
        let first = prepare_button_context(&service, &runtime);
        let discussion = service.sessions.load_session(&first.session_id).unwrap();
        assert_eq!(
            discussion.invocations[0].invocation.input_provenance,
            AgentInvocationInputProvenance::User
        );

        service
            .orchestration
            .repository
            .fail_next_plan_builder_context_consume();
        assert!(service
            .request_plan(Some(first.session_id.clone()), None, None, None)
            .unwrap_err()
            .contains("injected Plan Builder context consume failure"));
        let claimed = service
            .orchestration
            .repository
            .load_claimed_plan_builder_context(first.session_id.as_str())
            .unwrap()
            .unwrap();
        let target = AgentInvocationId::new(&claimed.target_invocation_id).unwrap();
        let history = service.sessions.load_session(&first.session_id).unwrap();
        let plan_action = &history
            .invocations
            .iter()
            .find(|item| item.invocation.id == target)
            .unwrap()
            .invocation;
        assert_eq!(
            plan_action.submitted_text,
            "Build the epic plan based on what we have discussed"
        );
        assert_eq!(
            plan_action.input_provenance,
            AgentInvocationInputProvenance::Application
        );
        let runtime_request = runtime
            .requests
            .lock()
            .unwrap()
            .iter()
            .find(|request| request.invocation_id == target)
            .unwrap()
            .submitted_text
            .clone();
        assert!(runtime_request.contains("epic_plan_builder_button_initiation"));
        assert!(runtime_request.contains("Build the epic plan based on what we have discussed"));

        runtime.active.lock().unwrap().clear();
        drop(service);
        drop(registry);
        let restarted = reopen_service(&path, runtime.clone());
        restarted
            .reconcile_plan_builder_context(&first.session_id)
            .unwrap();
        restarted
            .reconcile_plan_builder_context(&first.session_id)
            .unwrap();
        assert!(restarted
            .orchestration
            .repository
            .load_claimed_plan_builder_context(first.session_id.as_str())
            .unwrap()
            .is_none());
        let next = restarted
            .send(
                Some(first.session_id.clone()),
                "Continue with user-authored discussion.".into(),
                None,
                None,
                None,
            )
            .unwrap();
        let reopened = restarted.sessions.load_session(&first.session_id).unwrap();
        let user_query = &reopened
            .invocations
            .iter()
            .find(|item| item.invocation.id == next.invocation_id)
            .unwrap()
            .invocation;
        assert_eq!(
            user_query.input_provenance,
            AgentInvocationInputProvenance::User
        );
        assert_eq!(
            user_query.submitted_text,
            "Continue with user-authored discussion."
        );
        assert!(!runtime
            .requests
            .lock()
            .unwrap()
            .iter()
            .find(|request| request.invocation_id == next.invocation_id)
            .unwrap()
            .submitted_text
            .contains("epic_plan_builder_button_initiation"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn launched_context_claim_reopens_and_consumes_without_redelivery() {
        let (service, runtime, _factory, registry, path) = service_fixture(RuntimeMode::Active);
        let first = prepare_button_context(&service, &runtime);
        let target = service.sessions.allocate_application_invocation_id();
        let delivery = service
            .orchestration
            .repository
            .claim_pending_plan_builder_context(
                first.session_id.as_str(),
                "crash-after-launch-claim",
                target.as_str(),
            )
            .unwrap()
            .unwrap();
        let (launched, launch_accepted) = send_exact_user_query(
            &service,
            first.session_id.clone(),
            target.clone(),
            "Original crash-window query.",
            context_extension(&delivery),
        );
        assert!(launch_accepted);
        assert_eq!(launched.invocation_id, target);
        let durable = service.sessions.load_session(&first.session_id).unwrap();
        let durable = &durable
            .invocations
            .iter()
            .find(|item| item.invocation.id == target)
            .unwrap()
            .invocation;
        assert_eq!(durable.submitted_text, "Original crash-window query.");
        assert_eq!(
            durable.input_provenance,
            crate::agent_sessions::domain::AgentInvocationInputProvenance::User
        );

        runtime.active.lock().unwrap().clear();
        drop(service);
        drop(registry);
        let restarted = reopen_service(&path, runtime.clone());
        restarted
            .reconcile_plan_builder_context(&first.session_id)
            .unwrap();
        restarted
            .reconcile_plan_builder_context(&first.session_id)
            .unwrap();
        let next = restarted
            .send(
                Some(first.session_id.clone()),
                "Query after recovered delivery.".into(),
                None,
                None,
                None,
            )
            .unwrap();
        let requests = runtime.requests.lock().unwrap();
        let recovered = requests
            .iter()
            .find(|request| request.invocation_id == target)
            .unwrap();
        assert!(recovered
            .submitted_text
            .contains("epic_plan_builder_button_initiation"));
        let next_request = requests
            .iter()
            .find(|request| request.invocation_id == next.invocation_id)
            .unwrap();
        assert!(!next_request
            .submitted_text
            .contains("epic_plan_builder_button_initiation"));
        drop(requests);
        let facts: (String, String) = Connection::open(&path)
            .unwrap()
            .query_row(
                "SELECT delivered_to_invocation_id,consumed_at FROM plan_builder_context_deliveries",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(facts.0, target.as_str());
        assert!(!facts.1.is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn started_invocation_without_launch_acceptance_reopens_pending_and_retries() {
        let (service, runtime, _factory, registry, path) = service_fixture(RuntimeMode::Active);
        let first = prepare_button_context(&service, &runtime);
        let target = service.sessions.allocate_application_invocation_id();
        let delivery = service
            .orchestration
            .repository
            .claim_pending_plan_builder_context(
                first.session_id.as_str(),
                "crash-before-launch-acceptance",
                target.as_str(),
            )
            .unwrap()
            .unwrap();
        *runtime.mode.lock().unwrap() = RuntimeMode::PreflightError;
        let (_, launch_accepted) = send_exact_user_query(
            &service,
            first.session_id.clone(),
            target.clone(),
            "Original not-launched query.",
            context_extension(&delivery),
        );
        assert!(!launch_accepted);
        assert!(!runtime
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|request| request.invocation_id == target));
        Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE agent_session_invocations SET status='running', effective_options_json='{}', started_at=created_at, completed_at=NULL, exit_code=NULL, signal=NULL, runtime_error_json=NULL, updated_at=created_at WHERE id=?1",
                params![target.as_str()],
            )
            .unwrap();
        let durable = service.sessions.load_session(&first.session_id).unwrap();
        let durable = &durable
            .invocations
            .iter()
            .find(|item| item.invocation.id == target)
            .unwrap()
            .invocation;
        assert!(durable.started_at.is_some());
        assert_eq!(
            service
                .sessions
                .user_invocation_launch_evidence(&target, &first.session_id)
                .unwrap(),
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted
        );
        assert_eq!(durable.submitted_text, "Original not-launched query.");
        assert_eq!(
            durable.input_provenance,
            crate::agent_sessions::domain::AgentInvocationInputProvenance::User
        );

        drop(service);
        drop(registry);
        *runtime.mode.lock().unwrap() = RuntimeMode::Active;
        let restarted = reopen_service(&path, runtime.clone());
        let retry = restarted
            .send(
                Some(first.session_id.clone()),
                "Retry after no accepted launch.".into(),
                None,
                None,
                None,
            )
            .unwrap();
        assert_ne!(retry.invocation_id, target);
        let requests = runtime.requests.lock().unwrap();
        let retry_request = requests
            .iter()
            .find(|request| request.invocation_id == retry.invocation_id)
            .unwrap();
        assert!(retry_request
            .submitted_text
            .contains("epic_plan_builder_button_initiation"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn consume_failure_after_launch_reopens_and_reconciles_idempotently() {
        let (service, runtime, _factory, registry, path) = service_fixture(RuntimeMode::Active);
        let first = prepare_button_context(&service, &runtime);
        service
            .orchestration
            .repository
            .fail_next_plan_builder_context_consume();
        assert!(service
            .send(
                Some(first.session_id.clone()),
                "Query whose consume persistence fails.".into(),
                None,
                None,
                None,
            )
            .unwrap_err()
            .contains("injected Plan Builder context consume failure"));
        let target = service
            .orchestration
            .repository
            .load_claimed_plan_builder_context(first.session_id.as_str())
            .unwrap()
            .unwrap()
            .target_invocation_id;
        assert!(runtime
            .requests
            .lock()
            .unwrap()
            .iter()
            .find(|request| request.invocation_id.as_str() == target)
            .unwrap()
            .submitted_text
            .contains("epic_plan_builder_button_initiation"));

        runtime.active.lock().unwrap().clear();
        drop(service);
        drop(registry);
        let restarted = reopen_service(&path, runtime.clone());
        restarted
            .reconcile_plan_builder_context(&first.session_id)
            .unwrap();
        restarted
            .reconcile_plan_builder_context(&first.session_id)
            .unwrap();
        let next = restarted
            .send(
                Some(first.session_id),
                "Query after consume reconciliation.".into(),
                None,
                None,
                None,
            )
            .unwrap();
        let requests = runtime.requests.lock().unwrap();
        assert!(!requests
            .iter()
            .find(|request| request.invocation_id == next.invocation_id)
            .unwrap()
            .submitted_text
            .contains("epic_plan_builder_button_initiation"));
        drop(requests);
        let count: i64 = Connection::open(&path)
            .unwrap()
            .query_row(
                "SELECT count(*) FROM plan_builder_context_deliveries WHERE consumed_at IS NOT NULL AND delivered_to_invocation_id=?1",
                params![target],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn service_terminal_outcomes_remove_only_the_matching_handle() {
        for status in [
            AgentInvocationTerminalStatus::Completed,
            AgentInvocationTerminalStatus::Failed,
            AgentInvocationTerminalStatus::Canceled,
            AgentInvocationTerminalStatus::Interrupted,
        ] {
            let (service, runtime, factory, registry, path) = service_fixture(RuntimeMode::Active);
            let first = service.send(None, "one".into(), None, None, None).unwrap();
            let second = service.send(None, "two".into(), None, None, None).unwrap();
            let (id, sink) = runtime
                .active
                .lock()
                .unwrap()
                .iter()
                .find(|(id, _)| *id == first.invocation_id)
                .map(|(id, sink)| (id.clone(), sink.clone()))
                .unwrap();
            sink.emit_update(
                &id,
                RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                    status,
                    exit_code: None,
                    signal: None,
                    runtime_error: None,
                }),
            )
            .unwrap();
            assert_eq!(registry.active_count(), 1);
            assert_eq!(*factory.stops.lock().unwrap(), 1);
            let active_ids = runtime.active.lock().unwrap();
            assert!(active_ids.iter().any(|(id, _)| *id == second.invocation_id));
            drop(active_ids);
            service.shutdown();
            assert_eq!(*factory.stops.lock().unwrap(), 2);
            service.shutdown();
            assert_eq!(*factory.stops.lock().unwrap(), 2);
            let _ = fs::remove_file(path);
        }
    }
    #[test]
    fn service_reuses_durable_identity_rotates_child_credentials_and_cleans_terminal_and_shutdown()
    {
        let path = std::env::temp_dir().join(format!(
            "managed-plan-builder-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let connection = Connection::open(&path).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        drop(connection);
        let registry = Arc::new(ManagedPlanBuilderRegistry::default());
        let runtime = Arc::new(Runtime::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            Arc::new(SqliteAgentSessionRepository::open(&path).unwrap()),
            runtime.clone(),
            Arc::new(Notifier(registry.clone())),
            providers.clone(),
            providers,
            None,
        ));
        let orchestration_repo = Arc::new(SqliteOrchestrationRepository::open(&path).unwrap());
        let orchestration = Arc::new(OrchestrationApplication::new(orchestration_repo.clone()));
        let factory = Arc::new(Factory::default());
        let confirmation = confirmations(orchestration.clone());
        let service = ManagedPlanBuilderService::with_factory(
            orchestration.clone(),
            sessions.clone(),
            registry.clone(),
            confirmation.clone(),
            factory.clone(),
        );
        let first = service
            .send(
                None,
                "Discuss goals, ambiguity, and risks.".into(),
                Some("Plan".into()),
                None,
                None,
            )
            .unwrap();
        let reconciled = service
            .reconcile_session(first.session_id.clone(), Some("Plan".into()))
            .unwrap();
        assert_eq!(reconciled.session_id, first.session_id.as_str());
        let reconciled_query =
            serde_json::to_value(orchestration_repo.native_query().unwrap()).unwrap();
        assert_eq!(
            reconciled_query["planningDrafts"].as_array().unwrap().len(),
            1
        );
        assert_eq!(reconciled_query["planningDrafts"][0]["title"], "Plan");
        let (id, sink) = runtime.active.lock().unwrap().remove(0);
        sink.emit_update(
            &id,
            RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                status: AgentInvocationTerminalStatus::Completed,
                exit_code: Some(0),
                signal: None,
                runtime_error: None,
            }),
        )
        .unwrap();
        assert_eq!(registry.active_count(), 0);
        let restarted_service = ManagedPlanBuilderService::with_factory(
            orchestration,
            sessions.clone(),
            registry.clone(),
            confirmation,
            factory.clone(),
        );
        let second = restarted_service
            .send(
                Some(first.session_id.clone()),
                "Build and structure the plan.".into(),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(first.session_id, second.session_id);
        let query = serde_json::to_string(&orchestration_repo.native_query().unwrap()).unwrap();
        assert!(query.contains("\"initiatedEpics\":[]"));
        let native_query =
            serde_json::to_value(orchestration_repo.native_query().unwrap()).unwrap();
        assert_eq!(native_query["planningDrafts"].as_array().unwrap().len(), 1);
        assert_eq!(
            native_query["agentSessionAssociations"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(registry.active_count(), 1);
        let requests = runtime.requests.lock().unwrap();
        assert!(requests
            .iter()
            .all(|request| request.launch_extension.is_some()));
        assert!(requests.iter().all(|request| {
            request.options.sandbox
                == Some(crate::agent_sessions::domain::RuntimeSandboxMode::ReadOnly)
                && request.options.model.is_none()
        }));
        assert!(requests.iter().all(|request| {
            let args = &request.launch_extension.as_ref().unwrap().additional_args;
            args.iter().any(|arg| arg == "approval_policy=\"never\"")
                && args.iter().any(|arg| arg.ends_with(".required=true"))
                && args
                    .iter()
                    .any(|arg| arg.contains("request_epic_initiation"))
                && args
                    .iter()
                    .any(|arg| arg.contains("submit_epic_plan_proposal"))
        }));
        assert_ne!(requests[0].launch_extension, requests[1].launch_extension);
        let discovery_root = std::path::PathBuf::from(
            super::super::conversation_harness::epic_plan_builder_discovery_root().unwrap(),
        );
        assert!(requests
            .iter()
            .all(|request| { request.working_directory.as_deref() == discovery_root.to_str() }));
        assert!(requests[0]
            .submitted_text
            .starts_with("<application_context provenance=\"product_initial_prompt_prefix\" source=\"epic_plan_builder\" version=\"4\">")
            && requests[0].submitted_text.contains("canonical repository source: .agents/skills/epic-plan-builder/SKILL.md")
            && requests[0].submitted_text.contains("Request initiation only through request_epic_initiation")
            && requests[0].submitted_text.ends_with("<user_query>\nDiscuss goals, ambiguity, and risks.\n</user_query>"));
        assert_eq!(requests[1].submitted_text, "Build and structure the plan.");
        drop(requests);
        let persisted = sessions.load_session(&first.session_id).unwrap();
        assert_eq!(
            persisted.invocations[0].invocation.submitted_text,
            "Discuss goals, ambiguity, and risks."
        );
        let ordinary = sessions
            .send_message(SendAgentSessionMessageCommand {
                session_id: None,
                submitted_text: "ordinary".into(),
                title: None,
                working_directory: None,
                requested_options: None,
            })
            .unwrap();
        assert_eq!(
            runtime
                .requests
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .launch_extension,
            None
        );
        assert_eq!(
            runtime
                .requests
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .submitted_text,
            "ordinary"
        );
        assert_eq!(
            runtime
                .requests
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .working_directory,
            None
        );
        service.shutdown();
        restarted_service.shutdown();
        service.shutdown();
        assert_eq!(registry.active_count(), 0);
        assert_eq!(*factory.stops.lock().unwrap(), 2);
        let _ = ordinary;
        let _ = fs::remove_file(path);
    }

    #[tokio::test]
    async fn production_service_converges_child_mcp_save_into_durable_native_query_without_codex() {
        let path = std::env::temp_dir().join(format!(
            "managed-plan-builder-convergence-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let connection = Connection::open(&path).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        drop(connection);
        let registry = Arc::new(ManagedPlanBuilderRegistry::default());
        let runtime = Arc::new(Runtime::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            Arc::new(SqliteAgentSessionRepository::open(&path).unwrap()),
            runtime.clone(),
            Arc::new(Notifier(registry.clone())),
            providers.clone(),
            providers,
            None,
        ));
        let repository = Arc::new(SqliteOrchestrationRepository::open(&path).unwrap());
        let orchestration = Arc::new(OrchestrationApplication::new(repository.clone()));
        let service = ManagedPlanBuilderService::new(
            orchestration.clone(),
            sessions,
            registry.clone(),
            confirmations(orchestration),
        );

        let sent = service
            .send(
                None,
                "deterministic direct MCP selection".into(),
                Some("stable".into()),
                None,
                None,
            )
            .unwrap();
        let request = runtime.requests.lock().unwrap()[0].clone();
        let extension = request
            .launch_extension
            .expect("production child extension");
        let endpoint = extension
            .additional_args
            .iter()
            .find_map(|value| {
                value
                    .strip_prefix("mcp_servers.")
                    .and_then(|_| value.split_once(".url=\""))
                    .map(|(_, url)| url.trim_end_matches('"').to_string())
            })
            .expect("ephemeral MCP endpoint");
        let bearer = extension.environment[0].1.clone();

        let client = reqwest::Client::new();
        let initialized = mcp_post(&client, &endpoint, &bearer, None, mcp_rpc(1, "initialize", serde_json::json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"convergence","version":"1"}}))).await;
        let session = initialized
            .headers()
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(initialized.status().is_success());
        assert_eq!(
            mcp_post(
                &client,
                &endpoint,
                &bearer,
                Some(&session),
                serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"})
                    .to_string()
            )
            .await
            .status(),
            reqwest::StatusCode::ACCEPTED
        );
        let tools = mcp_response_json(
            mcp_post(
                &client,
                &endpoint,
                &bearer,
                Some(&session),
                mcp_rpc(2, "tools/list", serde_json::json!({})),
            )
            .await,
        )
        .await;
        let mut tool_names = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        tool_names.sort_unstable();
        assert_eq!(
            tool_names,
            vec!["request_epic_initiation", "submit_epic_plan_proposal"]
        );

        let arguments = serde_json::json!({"suggestedEpicName":"Converged proposal","sprints":[{"title":"Proof Sprint","intendedMovement":"Prove the managed boundary.","concernSummaries":["No model tool selection was run."]}]});
        let saved = mcp_response_json(
            mcp_post(
                &client,
                &endpoint,
                &bearer,
                Some(&session),
                mcp_rpc(
                    3,
                    "tools/call",
                    serde_json::json!({"name":"submit_epic_plan_proposal","arguments":arguments}),
                ),
            )
            .await,
        )
        .await;
        assert_eq!(saved["result"]["isError"], false);
        let retry = mcp_response_json(
            mcp_post(
                &client,
                &endpoint,
                &bearer,
                Some(&session),
                mcp_rpc(
                    4,
                    "tools/call",
                    serde_json::json!({"name":"submit_epic_plan_proposal","arguments":arguments}),
                ),
            )
            .await,
        )
        .await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(
                retry["result"]["content"][0]["text"].as_str().unwrap()
            )
            .unwrap()["status"],
            "idempotent_replay"
        );
        let rejected_hidden_scope = mcp_response_json(mcp_post(&client, &endpoint, &bearer, Some(&session), mcp_rpc(5, "tools/call", serde_json::json!({"name":"submit_epic_plan_proposal","arguments":{"epicPlanningDraftId":"other-draft","sprints":[{"title":"x","intendedMovement":"x","concernSummaries":[]}]}}))).await).await;
        assert_eq!(rejected_hidden_scope["result"]["isError"], true);

        let query = serde_json::to_value(repository.native_query().unwrap()).unwrap();
        assert_eq!(query["proposalRevisions"].as_array().unwrap().len(), 1);
        assert_eq!(query["recordedProposalEvents"].as_array().unwrap().len(), 1);
        assert_eq!(query["provenanceLinks"].as_array().unwrap().len(), 1);
        assert_eq!(query["initiatedEpics"], serde_json::json!([]));
        let (_, sink) = runtime.active.lock().unwrap()[0].clone();
        sink.emit_update(
            &sent.invocation_id,
            RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                status: AgentInvocationTerminalStatus::Completed,
                exit_code: Some(0),
                signal: None,
                runtime_error: None,
            }),
        )
        .unwrap();
        assert_eq!(registry.active_count(), 0);
        service.shutdown();
        let restarted = serde_json::to_value(
            SqliteOrchestrationRepository::open(&path)
                .unwrap()
                .native_query()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(restarted["proposalRevisions"].as_array().unwrap().len(), 1);
        assert_eq!(restarted["provenanceLinks"].as_array().unwrap().len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    #[ignore = "paid installed-Codex discussion proof; run only after deterministic MCP and runtime tests"]
    fn installed_codex_plan_builder_discussion_has_zero_semantic_calls() {
        let harness = LivePlanBuilderHarness::new();
        harness.send(
            None,
            "Discuss whether one or two Sprints would be safer for a small migration. Give only a short recommendation. This is ordinary discussion: do not build or submit a structured proposal and do not request initiation.",
        );
        assert_eq!(harness.proposal_count(), 0);
        assert_eq!(harness.user_invocation_count(), 1);
        assert_eq!(harness.completed_mcp_calls("submit_epic_plan_proposal"), 0);
        assert_eq!(harness.completed_mcp_calls("request_epic_initiation"), 0);
        eprintln!("live Plan Builder discussion: 0 proposals, 0 semantic calls");
    }

    #[test]
    #[ignore = "paid installed-Codex build/resume proof; run only after deterministic MCP and runtime tests"]
    fn installed_codex_plan_builder_build_and_rebuild_each_submit_once() {
        let harness = LivePlanBuilderHarness::new();
        let first = harness.send(
            None,
            "Build the structured Epic proposal now. Submit exactly one proposal named Live rebuild proof with one Sprint titled Initial convergence, intended movement Prove the first managed proposal, and no concern summaries. Do not request initiation. Stop after the successful proposal submission.",
        );
        assert_eq!(harness.proposal_count(), 1);
        assert_eq!(harness.completed_mcp_calls("submit_epic_plan_proposal"), 1);

        harness.send(
            Some(first.session_id),
            "Rebuild the structured Epic proposal now. Submit exactly one revised proposal named Live rebuild proof with one Sprint titled Revised convergence, intended movement Prove managed resume and revision, and no concern summaries. Do not request initiation. Stop after the successful proposal submission.",
        );
        assert_eq!(harness.proposal_count(), 2);
        assert_eq!(harness.user_invocation_count(), 2);
        assert_eq!(harness.completed_mcp_calls("submit_epic_plan_proposal"), 2);
        assert_eq!(harness.completed_mcp_calls("request_epic_initiation"), 0);
        let reopened = serde_json::to_value(
            SqliteOrchestrationRepository::open(&harness.database_path)
                .unwrap()
                .native_query()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(reopened["proposalRevisions"].as_array().unwrap().len(), 2);
        assert!(reopened["initiatedEpics"].as_array().unwrap().is_empty());
        eprintln!(
            "live Plan Builder build/rebuild: 2 proposals, 2 submit calls, 0 initiation calls"
        );
    }

    #[test]
    #[ignore = "paid installed-Codex proof; run only after deterministic MCP and runtime tests"]
    fn installed_codex_managed_plan_builder_persists_one_structured_proposal() {
        let path = std::env::temp_dir().join(format!(
            "managed-plan-builder-live-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let connection = Connection::open(&path).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        drop(connection);

        let registry = Arc::new(ManagedPlanBuilderRegistry::default());
        let terminal = Arc::new((Mutex::new(None), std::sync::Condvar::new()));
        let runtime = Arc::new(crate::runtime::codex::CodexCliRuntime::system(
            "codex", None,
        ));
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            Arc::new(SqliteAgentSessionRepository::open(&path).unwrap()),
            runtime.clone(),
            Arc::new(LiveNotifier {
                registry: registry.clone(),
                terminal: terminal.clone(),
            }),
            providers.clone(),
            providers,
            None,
        ));
        let repository = Arc::new(SqliteOrchestrationRepository::open(&path).unwrap());
        let orchestration = Arc::new(OrchestrationApplication::new(repository.clone()));
        let service = ManagedPlanBuilderService::new(
            orchestration.clone(),
            sessions,
            registry,
            confirmations(orchestration),
        );

        service
            .send(
                None,
                "Build the structured Epic proposal now. Submit exactly one proposal named Live managed proof with one Sprint titled Deterministic convergence, intended movement Prove the real managed Codex MCP path, and no concern summaries. Do not request initiation. Stop after the successful proposal submission."
                    .into(),
                Some("Live managed Plan Builder proof".into()),
                None,
                None,
            )
            .unwrap();
        let (finished, ready) = &*terminal;
        let finished = finished.lock().unwrap();
        let (finished, wait) = ready
            .wait_timeout_while(finished, std::time::Duration::from_secs(180), |value| {
                value.is_none()
            })
            .unwrap();
        assert!(!wait.timed_out(), "installed Codex invocation timed out");
        assert_eq!(
            finished.as_ref().unwrap().status,
            crate::agent_sessions::domain::AgentInvocationStatus::Completed
        );
        let query = serde_json::to_value(repository.native_query().unwrap()).unwrap();
        assert_eq!(query["proposalRevisions"].as_array().unwrap().len(), 1);
        assert!(query["initiatedEpics"].as_array().unwrap().is_empty());
        service.shutdown();
        runtime.shutdown().unwrap();
        let _ = fs::remove_file(path);
    }

    fn mcp_rpc(id: u32, method: &str, params: serde_json::Value) -> String {
        serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string()
    }

    async fn mcp_post(
        client: &reqwest::Client,
        endpoint: &str,
        bearer: &str,
        session: Option<&str>,
        body: String,
    ) -> reqwest::Response {
        let mut request = client
            .post(endpoint)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", format!("Bearer {bearer}"));
        if let Some(session) = session {
            request = request.header("mcp-session-id", session);
        }
        request.body(body).send().await.unwrap()
    }

    async fn mcp_response_json(response: reqwest::Response) -> serde_json::Value {
        let text = response.text().await.unwrap();
        let json = text
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .find(|line| !line.trim().is_empty())
            .unwrap_or(&text);
        serde_json::from_str(json).unwrap()
    }
}
