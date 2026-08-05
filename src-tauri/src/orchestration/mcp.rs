//! Invocation-scoped MCP adapter for the one Plan Builder proposal semantic.
//!
//! This module deliberately owns the local listener and child-only configuration; it does not
//! alter provider-neutral Agent Session identity or expose persistence as generic CRUD.
use super::{
    application::OrchestrationApplication,
    confirmation::{
        InitiationConfirmationCoordinator, InitiationConfirmationError, InitiationRequestSource,
    },
    domain::{
        CapabilityProfileId, EpicPlanningDraftId, PlanBuilderProposal,
        PlanningDraftAgentSessionAssociationId, SaveEpicPlanProposalCommand, SaveProposalError,
    },
};
use axum::http::{header, StatusCode};
use bytes::Bytes;
use http_body_util::Empty;
use hyper::{server::conn::http1, service::service_fn, Response};
use hyper_util::rt::TokioIo;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use sha2::{Digest, Sha256};
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

const SUBMIT_TOOL: &str = "submit_epic_plan_proposal";
const INITIATE_TOOL: &str = "request_epic_initiation";

/// Child-scoped Codex configuration. The runtime/process port can append these `-c` values and
/// environment pair without learning any orchestration identity or endpoint semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodexMcpInjection {
    pub(crate) configuration_args: Vec<String>,
    pub(crate) environment: (String, String),
}

impl CodexMcpInjection {
    pub(crate) fn new(
        server_url: &str,
        bearer: String,
        enabled_tools: &[String],
        required: bool,
    ) -> Self {
        Self::new_named("plan_builder", server_url, bearer, enabled_tools, required)
    }

    pub(crate) fn new_named(
        scope: &str,
        server_url: &str,
        bearer: String,
        enabled_tools: &[String],
        required: bool,
    ) -> Self {
        let name = format!("{scope}_{}", uuid::Uuid::new_v4().simple());
        let variable = format!("CODEX_ORCHESTRATOR_MCP_{}", uuid::Uuid::new_v4().simple());
        let values = [
            format!("mcp_servers.{name}.url=\"{server_url}\""),
            format!("mcp_servers.{name}.bearer_token_env_var=\"{variable}\""),
            format!(
                "mcp_servers.{name}.enabled_tools={}",
                serde_json::to_string(enabled_tools).expect("tool names serialize")
            ),
            format!("mcp_servers.{name}.required={required}"),
            format!("mcp_servers.{name}.default_tools_approval_mode=\"approve\""),
            format!("mcp_servers.{name}.startup_timeout_sec=10"),
            format!("mcp_servers.{name}.tool_timeout_sec=300"),
        ];
        Self {
            configuration_args: values
                .into_iter()
                .flat_map(|value| ["-c".into(), value])
                .collect(),
            environment: (variable, bearer),
        }
    }

    /// The sole WorkspaceWrite exception: the exact same-Session Implementer reporting
    /// continuation needs its token-protected loopback MCP transport on codex-cli 0.144.
    pub(crate) fn work_unit_implementer_reporting(server_url: &str, bearer: String) -> Self {
        let tools = [
            "submit_implementation_outcome".to_string(),
            "complete_implementation_outcome".to_string(),
        ];
        let mut injection = Self::new_named(
            "work_unit_implementer_reporting",
            server_url,
            bearer,
            &tools,
            true,
        );
        injection.configuration_args.extend([
            "-c".to_string(),
            "sandbox_workspace_write.network_access=true".to_string(),
            "-c".to_string(),
            "features.network_proxy=true".to_string(),
        ]);
        injection
    }

    pub(crate) fn is_exact_work_unit_implementer_reporting_transport(&self) -> bool {
        if self.configuration_args.len() != 18
            || self.configuration_args.chunks_exact(2).any(|pair| pair[0] != "-c")
        {
            return false;
        }
        let values = self
            .configuration_args
            .chunks_exact(2)
            .map(|pair| pair[1].as_str())
            .collect::<Vec<_>>();
        let Some(name) = values.iter().find_map(|value| {
            value
                .strip_prefix("mcp_servers.")
                .and_then(|value| value.split_once(".url="))
                .and_then(|(name, url)| (!url.is_empty()).then_some(name))
        }) else {
            return false;
        };
        if !name.starts_with("work_unit_implementer_reporting_") {
            return false;
        }
        let expected = [
            format!("mcp_servers.{name}.bearer_token_env_var="),
            format!(
                "mcp_servers.{name}.enabled_tools=[\"submit_implementation_outcome\",\"complete_implementation_outcome\"]"
            ),
            format!("mcp_servers.{name}.required=true"),
            format!("mcp_servers.{name}.default_tools_approval_mode=\"approve\""),
            format!("mcp_servers.{name}.startup_timeout_sec=10"),
            format!("mcp_servers.{name}.tool_timeout_sec=300"),
            "sandbox_workspace_write.network_access=true".into(),
            "features.network_proxy=true".into(),
        ];
        values.iter().any(|value| {
            value
                .strip_prefix(&expected[0])
                .is_some_and(|variable| !variable.is_empty())
        }) && expected[1..].iter().all(|expected| values.contains(&expected.as_str()))
    }
}

#[derive(Clone)]
pub(crate) struct PlanBuilderInvocation {
    pub(crate) agent_session_id: crate::agent_sessions::domain::AgentSessionId,
    pub(crate) draft_id: EpicPlanningDraftId,
    pub(crate) profile_id: CapabilityProfileId,
    pub(crate) association_id: PlanningDraftAgentSessionAssociationId,
    pub(crate) actor_id: String,
    expected_revision: Option<String>,
    agent_invocation: Arc<(
        Mutex<Option<crate::agent_sessions::domain::AgentInvocationId>>,
        Condvar,
    )>,
}

impl PlanBuilderInvocation {
    pub(crate) fn new(
        agent_session_id: crate::agent_sessions::domain::AgentSessionId,
        draft_id: EpicPlanningDraftId,
        profile_id: CapabilityProfileId,
        association_id: PlanningDraftAgentSessionAssociationId,
        actor_id: String,
        expected_revision: Option<String>,
    ) -> Self {
        Self {
            agent_session_id,
            draft_id,
            profile_id,
            association_id,
            actor_id,
            expected_revision,
            agent_invocation: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    pub(crate) fn bind_agent_invocation(
        &self,
        agent_invocation_id: crate::agent_sessions::domain::AgentInvocationId,
    ) {
        let (binding, ready) = &*self.agent_invocation;
        if let Ok(mut binding) = binding.lock() {
            *binding = Some(agent_invocation_id);
            ready.notify_all();
        }
    }

    fn save_command(
        &self,
        proposal: PlanBuilderProposal,
    ) -> Result<SaveEpicPlanProposalCommand, SaveProposalError> {
        let agent_invocation_id = self.bound_agent_invocation()?;
        let canonical_payload = serde_json::to_vec(&proposal)
            .map_err(|error| SaveProposalError::Unavailable(error.to_string()))?;
        let mut digest = Sha256::new();
        digest.update(agent_invocation_id.as_str().as_bytes());
        digest.update([0]);
        digest.update(canonical_payload);
        Ok(SaveEpicPlanProposalCommand {
            epic_planning_draft_id: self.draft_id.clone(),
            capability_profile_id: self.profile_id.clone(),
            agent_session_association_id: self.association_id.clone(),
            agent_session_id: self.agent_session_id.as_str().to_string(),
            actor_id: self.actor_id.clone(),
            expected_revision: self.expected_revision.clone(),
            proposal,
            idempotency_key: format!("managed-proposal-{:x}", digest.finalize()),
        })
    }

    fn bound_agent_invocation(
        &self,
    ) -> Result<crate::agent_sessions::domain::AgentInvocationId, SaveProposalError> {
        let (binding, ready) = &*self.agent_invocation;
        let binding = binding.lock().map_err(|_| {
            SaveProposalError::Unavailable("managed invocation binding is unavailable".into())
        })?;
        let (binding, _) = ready
            .wait_timeout_while(binding, Duration::from_secs(5), |value| value.is_none())
            .map_err(|_| {
                SaveProposalError::Unavailable("managed invocation binding is unavailable".into())
            })?;
        binding.as_ref().cloned().ok_or_else(|| {
            SaveProposalError::Unavailable("managed invocation did not become ready".into())
        })
    }
}

#[derive(Clone)]
struct PlanBuilderMcp {
    application: Arc<OrchestrationApplication>,
    confirmations: Arc<InitiationConfirmationCoordinator>,
    invocation: PlanBuilderInvocation,
    tool_router: ToolRouter<Self>,
}

impl PlanBuilderMcp {
    fn new(
        application: Arc<OrchestrationApplication>,
        confirmations: Arc<InitiationConfirmationCoordinator>,
        invocation: PlanBuilderInvocation,
    ) -> Self {
        Self {
            application,
            confirmations,
            invocation,
            tool_router: Self::tool_router(),
        }
    }
    fn error(code: &str, guidance: &str) -> CallToolResult {
        // operation IDs are intentionally opaque and never contain credentials or storage detail.
        CallToolResult::error(vec![ContentBlock::text(serde_json::json!({"code": code, "guidance": guidance, "operationId": uuid::Uuid::new_v4().to_string()}).to_string())])
    }
}

#[tool_router]
impl PlanBuilderMcp {
    #[tool(
        description = "Create or revise the concise proposed-Sprint projection. Input is ONLY {suggestedEpicName?: string, sprints: 1..20 [{title: string, intendedMovement: string, concernSummaries: string[]}]}; concernSummaries is required for every Sprint (use [] when none). Do not send IDs, revision tokens, idempotency keys, Work Units, phases, objectives, risks, acceptance criteria, or broader plan aggregates. One successful response is final; do not retry it."
    )]
    fn submit_epic_plan_proposal(
        &self,
        Parameters(proposal): Parameters<PlanBuilderProposal>,
    ) -> CallToolResult {
        let command = match self.invocation.save_command(proposal) {
            Ok(command) => command,
            Err(error) => return tool_error(error),
        };
        match self.application.save_epic_plan_proposal(command) {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(serde_json::json!({"status": if value.idempotent_replay { "idempotent_replay" } else { "persisted" }, "guidance": "The proposal is durably recorded. Do not retry this successful submission."}).to_string())]),
            Err(error) => tool_error(error),
        }
    }

    #[tool(
        description = "Request user-confirmed Epic initiation for this managed Plan Builder session. Takes no input. The application derives all draft, session, role, invocation, revision, authority, and replay identities, then waits for explicit user confirmation. A request is not an applied initiation."
    )]
    fn request_epic_initiation(&self) -> CallToolResult {
        let agent_invocation_id = match self.invocation.bound_agent_invocation() {
            Ok(value) => value,
            Err(error) => return tool_error(error),
        };
        let expected_revision_token = match self
            .application
            .capture_agent_initiation_precondition(&self.invocation)
        {
            Ok(value) => value,
            Err(error) => return initiation_error(error.into()),
        };
        let mut digest = Sha256::new();
        digest.update(agent_invocation_id.as_str().as_bytes());
        digest.update([0]);
        digest.update(expected_revision_token.as_bytes());
        let command = super::domain::InitiateEpicCommand {
            epic_planning_draft_id: self.invocation.draft_id.clone(),
            expected_revision_token,
            actor_id: "application-user".into(),
            idempotency_key: format!("managed-initiation-{:x}", digest.finalize()),
        };
        let request = match self.confirmations.request(
            InitiationRequestSource::Agent {
                agent_session_id: self.invocation.agent_session_id.as_str().to_string(),
                agent_invocation_id: agent_invocation_id.as_str().to_string(),
            },
            command,
        ) {
            Ok(value) => value,
            Err(error) => return initiation_error(error),
        };
        match self
            .confirmations
            .wait_for_resolution(&request.request_id, Duration::from_secs(240))
        {
            Ok(resolution) => CallToolResult::success(vec![ContentBlock::text(
                serde_json::json!({
                    "status": "projected",
                    "requestId": resolution.request_id,
                    "guidance": "The user confirmed initiation and the durable initiation is projected. Materials and Epic Runner launch are separate later states."
                })
                .to_string(),
            )]),
            Err(error) => initiation_error(error),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PlanBuilderMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Discussion does not call tools. For build or rebuild, call submit_epic_plan_proposal exactly once with only its concise typed Sprint projection. Call request_epic_initiation with no input only when initiation is requested; it waits for explicit user confirmation. Tool exposure never substitutes for server-side authorization.")
    }
}

fn tool_error(error: SaveProposalError) -> CallToolResult {
    let (code, guidance) = match error {
        SaveProposalError::InvalidInput(message) => (
            "invalid_input",
            format!(
                "{message}. Correct that field and retry once with the published proposal shape."
            ),
        ),
        SaveProposalError::Forbidden => (
            "forbidden",
            "This invocation is not authorized for that draft.".into(),
        ),
        SaveProposalError::DraftNotFound => (
            "draft_not_found",
            "Use an existing authorized planning draft.".into(),
        ),
        SaveProposalError::RevisionConflict => (
            "revision_conflict",
            "The proposal changed during this invocation. Continue the discussion, then submit a rebuilt proposal in a new invocation.".into(),
        ),
        SaveProposalError::IdempotencyConflict => (
            "idempotency_conflict",
            "The managed submission identity conflicts with different proposal content. Continue the discussion, then submit once in a new invocation.".into(),
        ),
        SaveProposalError::Unavailable(_message) => {
            #[cfg(test)]
            eprintln!("managed Plan Builder MCP internal error: {_message}");
            (
                "internal_error",
                "Retry later or contact the application owner.".into(),
            )
        }
    };
    PlanBuilderMcp::error(code, &guidance)
}

fn initiation_error(error: InitiationConfirmationError) -> CallToolResult {
    let (code, guidance) = match error {
        InitiationConfirmationError::Rejected => (
            "user_rejected",
            "The user rejected Epic initiation. Continue planning or ask what should change before requesting again.".to_string(),
        ),
        InitiationConfirmationError::RejectedNotificationFailed(_) => (
            "user_rejected",
            "The user rejected Epic initiation, but the application could not publish the terminal notification. Initiation was not applied; continue planning or ask what should change.".to_string(),
        ),
        InitiationConfirmationError::ConfirmedButNotApplied(_) => (
            "confirmation_failed_before_apply",
            "The user confirmed, but the application could not complete the confirmation boundary and did not apply initiation. Ask the user to retry from current product state.".to_string(),
        ),
        InitiationConfirmationError::TimedOut => (
            "confirmation_timed_out",
            "No user confirmation was received before the bounded wait ended. The request was not accepted; request again later if initiation is still wanted.".to_string(),
        ),
        InitiationConfirmationError::RequestNotFound => (
            "confirmation_unavailable",
            "The confirmation request is no longer available. Request initiation again.".to_string(),
        ),
        InitiationConfirmationError::Apply(error) => match error {
            super::domain::InitiateEpicError::ProposalMissing => ("proposal_missing", "Submit a current structured proposal before requesting initiation.".to_string()),
            super::domain::InitiateEpicError::RevisionConflict => ("revision_conflict", "The proposal changed before confirmation. Review the current proposal and request initiation again.".to_string()),
            super::domain::InitiateEpicError::Canceled => ("canceled", "This planning draft was canceled and cannot be initiated.".to_string()),
            super::domain::InitiateEpicError::AlreadyInitiated => ("already_initiated", "This Epic is already durably initiated; do not request initiation again.".to_string()),
            super::domain::InitiateEpicError::Forbidden => ("forbidden", "This managed session is not authorized to initiate that draft.".to_string()),
            super::domain::InitiateEpicError::DraftNotFound => ("draft_not_found", "The authorized planning draft is unavailable.".to_string()),
            super::domain::InitiateEpicError::IdempotencyConflict => ("idempotency_conflict", "The managed initiation identity conflicts with different semantics. Start a new managed invocation.".to_string()),
            super::domain::InitiateEpicError::InvalidInput(_) | super::domain::InitiateEpicError::Unavailable(_) => ("internal_error", "The application could not process initiation. Retry later or contact the application owner.".to_string()),
        },
        InitiationConfirmationError::PersistedButIncomplete { .. } => (
            "initiation_persisted_reconciliation_required",
            "Epic initiation was durably applied, but later notification or projection observation was incomplete. Do not request initiation again; refresh product state or ask the application owner to reconcile it.".to_string(),
        ),
        InitiationConfirmationError::Unavailable(_) => (
            "internal_error",
            "The application confirmation boundary is unavailable. Retry later or contact the application owner.".to_string(),
        ),
    };
    PlanBuilderMcp::error(code, &guidance)
}

impl From<super::domain::InitiateEpicError> for InitiationConfirmationError {
    fn from(value: super::domain::InitiateEpicError) -> Self {
        Self::Apply(value)
    }
}

/// Starts a loopback-only listener for one managed invocation. The token is checked before rmcp
/// receives a request; Host and Origin are also checked by rmcp's explicit configuration.
pub(crate) fn start_server(
    application: Arc<OrchestrationApplication>,
    confirmations: Arc<InitiationConfirmationCoordinator>,
    invocation: PlanBuilderInvocation,
    bearer: String,
    origins: Vec<String>,
) -> io::Result<ManagedMcpServer> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let cancellation = CancellationToken::new();
    let server_cancel = cancellation.clone();
    let join = thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("MCP runtime");
        runtime.block_on(async move {
            let config =
                rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default()
                    .with_allowed_hosts([format!("127.0.0.1:{}", address.port())])
                    .with_allowed_origins(origins.clone())
                    .with_cancellation_token(server_cancel.clone());
            let service: rmcp::transport::streamable_http_server::StreamableHttpService<
                PlanBuilderMcp,
                rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
            > = rmcp::transport::streamable_http_server::StreamableHttpService::new(
                move || {
                    Ok(PlanBuilderMcp::new(
                        application.clone(),
                        confirmations.clone(),
                        invocation.clone(),
                    ))
                },
                Default::default(),
                config,
            );
            let expected = Arc::new(bearer);
            let allowed_host = format!("127.0.0.1:{}", address.port());
            let allowed_origins = Arc::new(origins);
            let listener = tokio::net::TcpListener::from_std(listener).expect("async MCP listener");
            loop {
                let accepted = tokio::select! {
                    _ = server_cancel.cancelled() => break,
                    accepted = listener.accept() => accepted,
                };
                let Ok((stream, _)) = accepted else {
                    continue;
                };
                let service = service.clone();
                let expected = expected.clone();
                let allowed_host = allowed_host.clone();
                let allowed_origins = allowed_origins.clone();
                tokio::spawn(async move {
                    let guard = service_fn(move |request| {
                        let service = service.clone();
                        let expected = expected.clone();
                        let allowed_host = allowed_host.clone();
                        let allowed_origins = allowed_origins.clone();
                        async move {
                            if let Some(status) = transport_denial(
                                &expected,
                                &allowed_host,
                                &allowed_origins,
                                &request,
                            ) {
                                return Ok::<_, std::convert::Infallible>(
                                    Response::builder()
                                        .status(status)
                                        .body(Empty::<Bytes>::new())
                                        .expect("denial response")
                                        .map(axum::body::Body::new),
                                );
                            }
                            let response = service
                                .oneshot(request)
                                .await
                                .expect("rmcp service response");
                            Ok::<_, std::convert::Infallible>(response.map(axum::body::Body::new))
                        }
                    });
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), guard)
                        .await;
                });
            }
        });
    });
    Ok(ManagedMcpServer {
        address,
        cancellation,
        join: Some(join),
    })
}

/// The only callable lifecycle seam until a later managed Plan Builder invocation trigger exists.
/// It creates a fresh credential, exposes it exclusively through the returned child configuration,
/// and owns deterministic listener shutdown.
pub(crate) fn start_managed_invocation(
    application: Arc<OrchestrationApplication>,
    confirmations: Arc<InitiationConfirmationCoordinator>,
    invocation: PlanBuilderInvocation,
    enabled_tools: &[String],
    required: bool,
    origins: Vec<String>,
) -> io::Result<ManagedPlanBuilderInvocation> {
    let bearer = uuid::Uuid::new_v4().simple().to_string();
    let server = start_server(
        application,
        confirmations,
        invocation.clone(),
        bearer.clone(),
        origins,
    )?;
    let injection = CodexMcpInjection::new(&server.url(), bearer, enabled_tools, required);
    Ok(ManagedPlanBuilderInvocation {
        server,
        injection,
        invocation,
    })
}

pub(crate) fn transport_denial<B>(
    expected: &str,
    allowed_host: &str,
    allowed_origins: &[String],
    request: &hyper::Request<B>,
) -> Option<StatusCode> {
    let valid = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.strip_prefix("Bearer ") == Some(expected));
    if !valid {
        return Some(StatusCode::UNAUTHORIZED);
    }
    if request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(allowed_host)
    {
        return Some(StatusCode::FORBIDDEN);
    }
    if let Some(origin) = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        if !allowed_origins.iter().any(|allowed| allowed == origin) {
            return Some(StatusCode::FORBIDDEN);
        }
    }
    None
}

pub(crate) struct ManagedMcpServer {
    address: SocketAddr,
    cancellation: CancellationToken,
    join: Option<thread::JoinHandle<()>>,
}
impl ManagedMcpServer {
    pub(crate) fn url(&self) -> String {
        format!("http://{}/mcp", self.address)
    }
    pub(crate) fn stop(mut self) {
        self.cancellation.cancel();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub(crate) struct ManagedPlanBuilderInvocation {
    server: ManagedMcpServer,
    pub(crate) injection: CodexMcpInjection,
    invocation: PlanBuilderInvocation,
}
impl ManagedPlanBuilderInvocation {
    pub(crate) fn bind_agent_invocation(
        &self,
        agent_invocation_id: crate::agent_sessions::domain::AgentInvocationId,
    ) {
        self.invocation.bind_agent_invocation(agent_invocation_id);
    }
    pub(crate) fn stop(self) {
        self.server.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::repository::{
        FILE_REVIEW_FACTS_SCHEMA, ORCHESTRATION_INITIATION_SCHEMA, ORCHESTRATION_SCHEMA,
    };
    use chrono::{TimeZone, Utc};
    use rusqlite::{params, Connection};
    use std::sync::{mpsc, Mutex};

    #[test]
    fn confirmation_notification_failures_are_actionable_agent_results() {
        for (error, expected_code) in [
            (
                InitiationConfirmationError::RejectedNotificationFailed(
                    "notification unavailable".into(),
                ),
                "user_rejected",
            ),
            (
                InitiationConfirmationError::ConfirmedButNotApplied(
                    "notification unavailable".into(),
                ),
                "confirmation_failed_before_apply",
            ),
        ] {
            let result = serde_json::to_value(initiation_error(error)).unwrap();
            assert_eq!(result["isError"], true);
            assert!(result.to_string().contains(expected_code));
            assert!(!result.to_string().contains("notification unavailable"));
        }
    }

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

    struct ChannelConfirmationNotifier(
        Mutex<mpsc::Sender<super::super::confirmation::InitiationConfirmationEvent>>,
    );
    impl super::super::confirmation::InitiationConfirmationNotifier for ChannelConfirmationNotifier {
        fn notify(
            &self,
            event: super::super::confirmation::InitiationConfirmationEvent,
        ) -> Result<(), String> {
            self.0
                .lock()
                .map_err(|_| "confirmation channel is unavailable".to_string())?
                .send(event)
                .map_err(|error| error.to_string())
        }
    }

    fn channel_confirmations(
        application: Arc<OrchestrationApplication>,
    ) -> (
        Arc<InitiationConfirmationCoordinator>,
        mpsc::Receiver<super::super::confirmation::InitiationConfirmationEvent>,
    ) {
        let (sender, receiver) = mpsc::channel();
        (
            InitiationConfirmationCoordinator::new(
                application,
                Arc::new(ChannelConfirmationNotifier(Mutex::new(sender))),
            ),
            receiver,
        )
    }

    #[test]
    fn codex_injection_is_child_scoped_and_contains_only_accepted_configuration() {
        let tools = vec![SUBMIT_TOOL.to_string(), INITIATE_TOOL.to_string()];
        let injection =
            CodexMcpInjection::new("http://127.0.0.1:5555/mcp", "secret".into(), &tools, true);
        assert_eq!(
            injection
                .configuration_args
                .iter()
                .filter(|value| value.as_str() == "-c")
                .count(),
            7
        );
        assert!(injection
            .configuration_args
            .iter()
            .any(|value| value.contains("bearer_token_env_var")));
        assert!(!injection
            .configuration_args
            .iter()
            .any(|value| value == "secret"));
        assert_eq!(injection.environment.1, "secret");
        assert!(!injection.configuration_args.iter().any(|value| {
            value == "features.network_proxy=true"
                || value == "sandbox_workspace_write.network_access=true"
        }));
        assert!(!injection.is_exact_work_unit_implementer_reporting_transport());
    }

    #[test]
    fn only_implementer_reporting_gets_the_workspace_write_loopback_exception() {
        let injection = CodexMcpInjection::work_unit_implementer_reporting(
            "http://127.0.0.1:5555/mcp",
            "secret".into(),
        );
        assert_eq!(
            injection
                .configuration_args
                .iter()
                .filter(|value| value.as_str() == "-c")
                .count(),
            9
        );
        assert!(injection.configuration_args.iter().any(|value| {
            value == "sandbox_workspace_write.network_access=true"
        }));
        assert!(injection
            .configuration_args
            .iter()
            .any(|value| value == "features.network_proxy=true"));
        let tools = injection
            .configuration_args
            .iter()
            .find(|value| value.contains(".enabled_tools="))
            .expect("managed tool allow list");
        assert!(tools.ends_with("[\"submit_implementation_outcome\",\"complete_implementation_outcome\"]"));
        assert!(injection
            .configuration_args
            .iter()
            .any(|value| value.contains("work_unit_implementer_reporting_")));
        assert!(injection.is_exact_work_unit_implementer_reporting_transport());

        let mut malformed = injection;
        malformed.configuration_args.pop();
        assert!(!malformed.is_exact_work_unit_implementer_reporting_transport());
    }

    #[test]
    fn realistic_build_contract_is_zero_calls_for_discussion_then_one_submission() {
        let (application, confirmations, invocation, repository) = test_application();
        let mcp = PlanBuilderMcp::new(application, confirmations, invocation);

        // Discussion is deliberately ordinary conversation; no semantic tool effect exists.
        let before = serde_json::to_value(repository.native_query().unwrap()).unwrap();
        assert!(before["proposalRevisions"].as_array().unwrap().is_empty());

        let saved = mcp.submit_epic_plan_proposal(Parameters(PlanBuilderProposal {
            suggested_epic_name: Some("Efficient build".into()),
            sprints: vec![super::super::domain::ProposedSprint {
                title: "Focused Sprint".into(),
                intended_movement: "Converge through the typed product boundary.".into(),
                concern_summaries: vec![],
            }],
        }));
        let saved = serde_json::to_value(saved).unwrap();
        assert_eq!(saved["isError"], false);
        assert!(saved.to_string().len() < 1_000);
        let after = serde_json::to_value(repository.native_query().unwrap()).unwrap();
        assert_eq!(after["proposalRevisions"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn rebuild_is_one_submission_with_a_precondition_captured_at_invocation_start() {
        let (application, confirmations, invocation, repository) = test_application();
        let first = PlanBuilderMcp::new(
            application.clone(),
            confirmations.clone(),
            invocation.clone(),
        );
        assert_eq!(
            serde_json::to_value(first.submit_epic_plan_proposal(Parameters(proposal("First"))))
                .unwrap()["isError"],
            false
        );
        let revision = serde_json::to_value(repository.native_query().unwrap()).unwrap()
            ["proposalRevisions"][0]["revisionToken"]
            .as_str()
            .unwrap()
            .to_string();
        let rebuild_invocation = PlanBuilderInvocation::new(
            invocation.agent_session_id.clone(),
            invocation.draft_id.clone(),
            invocation.profile_id.clone(),
            invocation.association_id.clone(),
            invocation.actor_id.clone(),
            Some(revision),
        );
        rebuild_invocation.bind_agent_invocation(
            crate::agent_sessions::domain::AgentInvocationId::new("agent-invocation-2")
                .expect("invocation"),
        );

        let rebuilt = PlanBuilderMcp::new(application, confirmations, rebuild_invocation)
            .submit_epic_plan_proposal(Parameters(proposal("Rebuilt")));

        assert_eq!(serde_json::to_value(rebuilt).unwrap()["isError"], false);
        assert_eq!(
            serde_json::to_value(repository.native_query().unwrap()).unwrap()["proposalRevisions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn forbidden_and_cross_session_invocations_cannot_use_plan_builder_tools() {
        let (application, confirmations, invocation, _) = test_application();
        for (session_id, profile_id, suffix) in [
            (
                invocation.agent_session_id.clone(),
                CapabilityProfileId::new("forbidden-profile").unwrap(),
                "profile",
            ),
            (
                crate::agent_sessions::domain::AgentSessionId::new("cross-session").unwrap(),
                invocation.profile_id.clone(),
                "session",
            ),
        ] {
            let forbidden = PlanBuilderInvocation::new(
                session_id,
                invocation.draft_id.clone(),
                profile_id,
                invocation.association_id.clone(),
                invocation.actor_id.clone(),
                None,
            );
            forbidden.bind_agent_invocation(
                crate::agent_sessions::domain::AgentInvocationId::new(format!(
                    "agent-invocation-forbidden-{suffix}"
                ))
                .unwrap(),
            );
            let mcp = PlanBuilderMcp::new(application.clone(), confirmations.clone(), forbidden);
            let result =
                serde_json::to_value(mcp.submit_epic_plan_proposal(Parameters(proposal("Denied"))))
                    .unwrap();
            assert_eq!(result["isError"], true);
            assert!(result.to_string().contains("forbidden"));

            let initiation = serde_json::to_value(mcp.request_epic_initiation()).unwrap();
            assert_eq!(initiation["isError"], true);
            assert!(initiation.to_string().contains("forbidden"));
        }
    }

    #[test]
    fn agent_initiation_waits_for_shared_confirmation_and_reports_rejection_or_projection() {
        for decision in [
            super::super::confirmation::UserInitiationDecision::Rejected,
            super::super::confirmation::UserInitiationDecision::Confirmed,
        ] {
            let (application, _, invocation, repository) = test_application();
            let (confirmations, events) = channel_confirmations(application.clone());
            let mcp = PlanBuilderMcp::new(application, confirmations.clone(), invocation);
            assert_eq!(
                serde_json::to_value(
                    mcp.submit_epic_plan_proposal(Parameters(proposal("Initiate")))
                )
                .unwrap()["isError"],
                false
            );
            let resolver = std::thread::spawn(move || {
                let requested = events.recv().unwrap();
                assert_eq!(
                    requested.state,
                    super::super::confirmation::InitiationConfirmationState::Requested
                );
                assert!(matches!(
                    requested.request.source,
                    InitiationRequestSource::Agent { .. }
                ));
                let before = serde_json::to_value(repository.native_query().unwrap()).unwrap();
                assert!(before["initiatedEpics"].as_array().unwrap().is_empty());
                let resolved = confirmations.resolve(&requested.request.request_id, decision);
                let mut states = vec![];
                let count = if matches!(
                    decision,
                    super::super::confirmation::UserInitiationDecision::Confirmed
                ) {
                    4
                } else {
                    1
                };
                for _ in 0..count {
                    states.push(events.recv().unwrap().state);
                }
                (resolved, states, repository)
            });

            let response = serde_json::to_value(mcp.request_epic_initiation()).unwrap();
            let (resolved, states, repository) = resolver.join().unwrap();
            let query = serde_json::to_value(repository.native_query().unwrap()).unwrap();
            if matches!(
                decision,
                super::super::confirmation::UserInitiationDecision::Rejected
            ) {
                assert_eq!(response["isError"], true);
                assert!(response.to_string().contains("user_rejected"));
                assert_eq!(resolved, Err(InitiationConfirmationError::Rejected));
                assert_eq!(
                    states,
                    [super::super::confirmation::InitiationConfirmationState::UserRejected]
                );
                assert!(query["initiatedEpics"].as_array().unwrap().is_empty());
            } else {
                assert_eq!(response["isError"], false);
                assert!(response.to_string().contains("projected"));
                assert!(resolved.is_ok());
                assert_eq!(
                    states,
                    [
                        super::super::confirmation::InitiationConfirmationState::UserConfirmed,
                        super::super::confirmation::InitiationConfirmationState::Applied,
                        super::super::confirmation::InitiationConfirmationState::Persisted,
                        super::super::confirmation::InitiationConfirmationState::Projected,
                    ]
                );
                assert_eq!(query["initiatedEpics"].as_array().unwrap().len(), 1);
            }
        }
    }

    #[tokio::test]
    async fn streamable_http_enforces_transport_and_persists_only_authorized_tool_effects() {
        let (application, confirmations, invocation, repository) = test_application();
        let server = start_server(
            application,
            confirmations,
            invocation.clone(),
            "test-bearer".into(),
            vec!["tauri://localhost".into()],
        )
        .expect("server");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .expect("client");
        let url = server.url();

        assert_eq!(
            post(&client, &url, None, None, None, initialize())
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            raw_status(
                server.address,
                "bad.example",
                Some("Bearer test-bearer"),
                None,
                &initialize()
            )
            .await,
            403
        );
        assert_eq!(
            raw_status(
                server.address,
                &format!("127.0.0.1:{}", server.address.port()),
                Some("Bearer test-bearer"),
                Some("https://wrong.example"),
                &initialize()
            )
            .await,
            403
        );

        let initialized = post(
            &client,
            &url,
            Some("Bearer test-bearer"),
            None,
            None,
            initialize(),
        )
        .await;
        assert_eq!(initialized.status(), StatusCode::OK);
        let session = initialized
            .headers()
            .get("mcp-session-id")
            .expect("session id")
            .to_str()
            .expect("session id text")
            .to_string();
        let initialized_notification = post(
            &client,
            &url,
            Some("Bearer test-bearer"),
            None,
            some_session(&session),
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
        )
        .await;
        assert_eq!(initialized_notification.status(), StatusCode::ACCEPTED);
        let listed = post(
            &client,
            &url,
            Some("Bearer test-bearer"),
            None,
            some_session(&session),
            jsonrpc(2, "tools/list", serde_json::json!({})),
        )
        .await;
        let list = response_json(listed).await;
        let tools = list["result"]["tools"].as_array().expect("tools");
        assert_eq!(tools.len(), 2);
        assert_eq!(
            tools
                .iter()
                .map(|tool| tool["name"].as_str().unwrap())
                .collect::<std::collections::BTreeSet<_>>(),
            [SUBMIT_TOOL, INITIATE_TOOL].into_iter().collect()
        );
        assert!(tools.iter().all(|tool| tool["inputSchema"].is_object()));
        assert!(tools.iter().all(|tool| !tool["inputSchema"]
            .to_string()
            .contains("epicPlanningDraftId")));
        let save_schema = &tools
            .iter()
            .find(|tool| tool["name"] == SUBMIT_TOOL)
            .unwrap()["inputSchema"];
        let initiation_schema = &tools
            .iter()
            .find(|tool| tool["name"] == INITIATE_TOOL)
            .unwrap()["inputSchema"];
        assert!(initiation_schema["properties"]
            .as_object()
            .is_none_or(|properties| properties.is_empty()));
        assert!(initiation_schema["required"]
            .as_array()
            .is_none_or(|required| required.is_empty()));
        assert_eq!(
            save_schema["$defs"]["ProposedSprint"]["required"],
            serde_json::json!(["title", "intendedMovement", "concernSummaries"])
        );
        assert_eq!(save_schema["additionalProperties"], false);
        assert_eq!(
            save_schema["$defs"]["ProposedSprint"]["additionalProperties"],
            false
        );
        assert_eq!(save_schema["properties"]["sprints"]["minItems"], 1);
        assert_eq!(save_schema["properties"]["sprints"]["maxItems"], 20);
        assert_eq!(
            save_schema["$defs"]["ProposedSprint"]["properties"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<Vec<_>>(),
            vec!["concernSummaries", "intendedMovement", "title"]
        );
        assert!(save_schema.to_string().len() < 1_000);
        assert!(tools[0]["description"].as_str().unwrap().len() < 1_200);
        assert!(list.to_string().len() < 4_000);

        let broad = response_json(
            post(
                &client,
                &url,
                Some("Bearer test-bearer"),
                None,
                some_session(&session),
                jsonrpc(
                    3,
                    "tools/call",
                    serde_json::json!({"name": SUBMIT_TOOL, "arguments": {"suggestedEpicName": "Broad", "sprints": [{"title": "Foundation", "intendedMovement": "Establish it.", "workUnits": []}]}}),
                ),
            )
            .await,
        )
        .await;
        assert_eq!(broad["result"]["isError"], true);
        let saved = response_json(
            post(
                &client,
                &url,
                Some("Bearer test-bearer"),
                None,
                some_session(&session),
                jsonrpc(
                    4,
                    "tools/call",
                    serde_json::json!({"name": SUBMIT_TOOL, "arguments": submit_args()}),
                ),
            )
            .await,
        )
        .await;
        assert_eq!(saved["result"]["isError"], false);
        assert_eq!(tool_payload(&saved)["status"], "persisted");
        let duplicate = response_json(
            post(
                &client,
                &url,
                Some("Bearer test-bearer"),
                None,
                some_session(&session),
                jsonrpc(
                    5,
                    "tools/call",
                    serde_json::json!({"name": SUBMIT_TOOL, "arguments": submit_args()}),
                ),
            )
            .await,
        )
        .await;
        assert_eq!(tool_payload(&duplicate)["status"], "idempotent_replay");
        let conflict = response_json(post(&client, &url, Some("Bearer test-bearer"), None, some_session(&session), jsonrpc(6, "tools/call", serde_json::json!({"name": SUBMIT_TOOL, "arguments": {"suggestedEpicName":"Changed", "sprints":[{"title":"Changed", "intendedMovement":"Changed", "concernSummaries":[]}]}}))).await).await;
        assert_eq!(tool_payload(&conflict)["code"], "revision_conflict");
        let invalid = response_json(
            post(
                &client,
                &url,
                Some("Bearer test-bearer"),
                None,
                some_session(&session),
                jsonrpc(
                    7,
                    "tools/call",
                    serde_json::json!({"name": SUBMIT_TOOL, "arguments": {"epicPlanningDraftId":"other-draft","sprints":[{"title":"x","intendedMovement":"x","concernSummaries":[]}]}}),
                ),
            )
            .await,
        )
        .await;
        assert_eq!(invalid["result"]["isError"], true);
        let query =
            serde_json::to_value(repository.native_query().expect("query")).expect("query json");
        assert_eq!(
            query["proposalRevisions"]
                .as_array()
                .expect("revisions")
                .len(),
            1
        );
        server.stop();
    }

    fn test_application() -> (
        Arc<OrchestrationApplication>,
        Arc<InitiationConfirmationCoordinator>,
        PlanBuilderInvocation,
        Arc<super::super::repository::SqliteOrchestrationRepository>,
    ) {
        let connection = Connection::open_in_memory().expect("db");
        crate::storage::configure_sqlite_connection(&connection).expect("policy");
        connection
            .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
            .expect("session schema");
        connection
            .execute_batch(ORCHESTRATION_SCHEMA)
            .expect("schema");
        connection
            .execute_batch(FILE_REVIEW_FACTS_SCHEMA)
            .expect("File Review schema");
        connection
            .execute_batch(ORCHESTRATION_INITIATION_SCHEMA)
            .expect("initiation schema");
        let now = Utc
            .with_ymd_and_hms(2026, 7, 15, 12, 0, 0)
            .single()
            .expect("time");
        connection.execute("INSERT INTO agent_sessions (id, title, availability, requested_options_json, created_at, updated_at) VALUES ('session-1', 'session', 'available', '{}', ?1, ?1)", params![now.to_rfc3339()]).expect("session");
        let repository = Arc::new(
            super::super::repository::SqliteOrchestrationRepository::new(connection)
                .expect("repository"),
        );
        let draft = EpicPlanningDraftId::new("epic-planning-draft-1").expect("draft");
        let profile = CapabilityProfileId::new("capability-profile-1").expect("profile");
        let association =
            PlanningDraftAgentSessionAssociationId::new("association-1").expect("association");
        repository
            .create_planning_draft(&draft, now)
            .expect("draft");
        repository
            .create_capability_profile(&profile, "active", now)
            .expect("profile");
        repository
            .associate_agent_session(&association, &draft, "session-1", "actor-1", now)
            .expect("association");
        repository
            .assign_profile(
                &draft,
                &profile,
                &association,
                Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0)
                    .single()
                    .expect("expiry"),
                now,
            )
            .expect("assignment");
        let invocation = PlanBuilderInvocation::new(
            crate::agent_sessions::domain::AgentSessionId::new("session-1").expect("session id"),
            draft,
            profile,
            association,
            "actor-1".into(),
            None,
        );
        invocation.bind_agent_invocation(
            crate::agent_sessions::domain::AgentInvocationId::new("agent-invocation-1")
                .expect("invocation"),
        );
        let application = Arc::new(OrchestrationApplication::new(repository.clone()));
        (
            application.clone(),
            confirmations(application),
            invocation,
            repository,
        )
    }
    fn initialize() -> String {
        jsonrpc(
            1,
            "initialize",
            serde_json::json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}),
        )
    }
    fn jsonrpc(id: u32, method: &str, params: serde_json::Value) -> String {
        serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string()
    }
    fn submit_args() -> serde_json::Value {
        serde_json::json!({"suggestedEpicName":"Test","sprints":[{"title":"Sprint","intendedMovement":"Move","concernSummaries":[]}]})
    }
    fn proposal(title: &str) -> PlanBuilderProposal {
        PlanBuilderProposal {
            suggested_epic_name: Some(title.into()),
            sprints: vec![super::super::domain::ProposedSprint {
                title: title.into(),
                intended_movement: "Move through the managed semantic boundary.".into(),
                concern_summaries: vec![],
            }],
        }
    }
    fn some_session(value: &str) -> Option<&str> {
        Some(value)
    }
    async fn post(
        client: &reqwest::Client,
        url: &str,
        bearer: Option<&str>,
        host: Option<&str>,
        session: Option<&str>,
        body: String,
    ) -> reqwest::Response {
        let mut request = client
            .post(url)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream");
        if let Some(value) = bearer {
            request = request.header("authorization", value);
        }
        if let Some(value) = host {
            request = request.header("host", value);
        }
        if let Some(value) = session {
            request = request.header("mcp-session-id", value);
        }
        request.body(body).send().await.expect("http")
    }
    async fn raw_status(
        address: SocketAddr,
        host: &str,
        bearer: Option<&str>,
        origin: Option<&str>,
        body: &str,
    ) -> u16 {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect");
        let authorization = bearer
            .map(|value| format!("Authorization: {value}\r\n"))
            .unwrap_or_default();
        let origin = origin
            .map(|value| format!("Origin: {value}\r\n"))
            .unwrap_or_default();
        let request = format!("POST /mcp HTTP/1.1\r\nHost: {host}\r\n{authorization}{origin}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        stream.write_all(request.as_bytes()).await.expect("write");
        let mut response = String::new();
        stream.read_to_string(&mut response).await.expect("read");
        response
            .split_whitespace()
            .nth(1)
            .expect("status")
            .parse()
            .expect("status integer")
    }
    async fn response_json(response: reqwest::Response) -> serde_json::Value {
        let status = response.status();
        let text = response.text().await.expect("body");
        let json = text
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .find(|line| !line.trim().is_empty())
            .unwrap_or(&text);
        serde_json::from_str(json)
            .unwrap_or_else(|error| panic!("json response ({status}, {text:?}): {error}"))
    }
    fn tool_payload(value: &serde_json::Value) -> serde_json::Value {
        let text = value["result"]["content"][0]["text"]
            .as_str()
            .expect("tool text");
        serde_json::from_str(text)
            .unwrap_or_else(|error| panic!("tool payload ({text:?}): {error}"))
    }
}
