//! Ignored, opt-in live smoke coverage for the Agent Session lifecycle.
//!
//! The driver is compiled only for tests and uses an owned temporary database and workspace. It is
//! ignored by default and refuses to discover or launch Codex without explicit environment opt-in.

use crate::{
    agent_sessions::{
        application::{
            AgentSessionApplication, AgentSessionNotification, AgentSessionNotifier,
            ApplicationInvocationLaunchEvidence, CancelAgentInvocationCommand,
            SendAgentSessionMessageCommand, SendIdempotentApplicationAgentSessionMessageCommand,
            SystemAgentSessionProviders,
        },
        domain::{
            AgentInvocation, AgentInvocationId, AgentInvocationStatus, AgentRuntimeEvent,
            AgentSessionId, NormalizedRuntimeEventKind,
        },
        ports::{AgentSessionHistory, AgentSessionRepository},
        repository::{SqliteAgentSessionRepository, AGENT_SESSION_SCHEMA},
    },
    runtime::codex::{CodexCliCapabilities, CodexCliCapabilityProbe, CodexCliRuntime},
    runtime::processes::ProcessLaunchSpec,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::hash_map::DefaultHasher,
    env,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tempfile::{tempdir, TempDir};

pub(crate) const LIVE_SMOKE_OPT_IN_ENV: &str = "CODEX_AGENT_SESSION_LIVE_SMOKE";
pub(crate) const LIVE_SMOKE_TIMEOUT_ENV: &str = "CODEX_AGENT_SESSION_LIVE_SMOKE_TIMEOUT_SECS";
pub(crate) const EVIDENCE_SCHEMA_VERSION: u32 = 2;
const DEFAULT_TIMEOUT_SECS: u64 = 180;
const MAX_TIMEOUT_SECS: u64 = 300;
const LIVE_INVOCATION_BUDGET: u8 = 4;
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiveSmokeConfig {
    pub(crate) enabled: bool,
    pub(crate) timeout: Duration,
}

impl LiveSmokeConfig {
    #[allow(dead_code)]
    pub(crate) fn from_environment() -> Result<Self, String> {
        Self::from_values(
            env::var(LIVE_SMOKE_OPT_IN_ENV).ok(),
            env::var(LIVE_SMOKE_TIMEOUT_ENV).ok(),
        )
    }

    fn from_values(opt_in: Option<String>, timeout: Option<String>) -> Result<Self, String> {
        let enabled = match opt_in.as_deref().map(str::trim) {
            None | Some("") | Some("0") | Some("false") | Some("no") => false,
            Some("1") | Some("true") | Some("yes") => true,
            Some(_value) => return Err(format!("{LIVE_SMOKE_OPT_IN_ENV} must be true/false")),
        };
        let seconds = timeout
            .as_deref()
            .map(str::parse::<u64>)
            .transpose()
            .map_err(|_| format!("{LIVE_SMOKE_TIMEOUT_ENV} must be an integer number of seconds"))?
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        if seconds == 0 || seconds > MAX_TIMEOUT_SECS {
            return Err(format!(
                "{LIVE_SMOKE_TIMEOUT_ENV} must be between 1 and {MAX_TIMEOUT_SECS} seconds"
            ));
        }
        Ok(Self {
            enabled,
            timeout: Duration::from_secs(seconds),
        })
    }
}

pub(crate) struct LiveSmokeEnvironment {
    root: TempDir,
    pub(crate) database_path: PathBuf,
    pub(crate) workspace_path: PathBuf,
}

impl LiveSmokeEnvironment {
    pub(crate) fn create() -> Result<Self, String> {
        let root = tempdir().map_err(|error| format!("create smoke temp root: {error}"))?;
        let database_path = root.path().join("agent-session-smoke.sqlite");
        let workspace_path = root.path().join("workspace");
        std::fs::create_dir(&workspace_path)
            .map_err(|error| format!("create smoke workspace: {error}"))?;
        validate_owned_path(root.path(), &database_path)?;
        validate_owned_path(root.path(), &workspace_path)?;
        Ok(Self {
            root,
            database_path,
            workspace_path,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        self.root.path()
    }
}

fn validate_owned_path(root: &Path, candidate: &Path) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("canonicalize smoke root: {error}"))?;
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| format!("resolve smoke path: {error}"))?
            .join(candidate)
    };
    let existing_parent = absolute
        .parent()
        .ok_or_else(|| "smoke path has no parent".to_string())?
        .canonicalize()
        .map_err(|error| format!("canonicalize smoke path parent: {error}"))?;
    if !existing_parent.starts_with(&root) {
        return Err("smoke path must remain inside its owned temporary root".to_string());
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceRecord {
    pub(crate) schema_version: u32,
    pub(crate) outcome: String,
    pub(crate) invocation_budget: u8,
    pub(crate) invocations_launched: u8,
    pub(crate) phases: Vec<PhaseEvidence>,
    pub(crate) cleanup: CleanupEvidence,
    pub(crate) final_invocations: Vec<FinalInvocationEvidence>,
    pub(crate) limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PhaseEvidence {
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) session_id_hash: Option<String>,
    pub(crate) invocation_id_hash: Option<String>,
    pub(crate) external_context_id_hash: Option<String>,
    pub(crate) resume_target_matches_persisted_context: Option<bool>,
    pub(crate) resume_target_differs_from_local_ids: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupEvidence {
    pub(crate) shutdown_completed: bool,
    pub(crate) active_direct_children: Option<usize>,
    pub(crate) cancellation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FinalInvocationEvidence {
    pub(crate) invocation_id_hash: String,
    pub(crate) durable_status: Option<String>,
    pub(crate) launch_accepted: bool,
    pub(crate) external_context_persisted: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) runtime_error_code: Option<String>,
    pub(crate) runtime_event_sources: Vec<String>,
    pub(crate) normalized_event_kinds: Vec<String>,
    pub(crate) launch_provenance: Option<LaunchProvenanceEvidence>,
}

/// Credential-free subset of the application-owned launch record. It intentionally omits every
/// configuration/environment value, working-directory value, prompt, URL, and provider payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchProvenanceEvidence {
    pub(crate) executable: Option<String>,
    pub(crate) invocation_mode: Option<String>,
    pub(crate) configuration_keys: Vec<String>,
    pub(crate) environment_keys: Vec<String>,
    pub(crate) working_directory_provided: Option<bool>,
    pub(crate) working_directory_absolute: Option<bool>,
    pub(crate) working_directory_extended_length_prefix: Option<bool>,
}

impl EvidenceRecord {
    pub(crate) fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

struct NoopNotifier;

impl AgentSessionNotifier for NoopNotifier {
    fn notify(&self, _notification: AgentSessionNotification) -> Result<(), String> {
        Ok(())
    }
}

struct LiveSmokeDriver {
    environment: LiveSmokeEnvironment,
    timeout: Duration,
    application: Option<AgentSessionApplication>,
    runtime: Option<Arc<CodexCliRuntime>>,
    capabilities: CodexCliCapabilities,
    observed_launches: Arc<Mutex<Vec<ProcessLaunchSpec>>>,
    known_invocations: Vec<AgentInvocationId>,
    evidence: EvidenceRecord,
}

impl LiveSmokeDriver {
    fn new(config: LiveSmokeConfig) -> Result<Self, String> {
        let environment = LiveSmokeEnvironment::create()?;
        // Capability discovery happens only after explicit opt-in. It may invoke `codex --help`,
        // but never starts an agent invocation.
        let capabilities = CodexCliCapabilityProbe::new("codex").discover();
        Ok(Self {
            environment,
            timeout: config.timeout,
            application: None,
            runtime: None,
            capabilities,
            observed_launches: Arc::new(Mutex::new(Vec::new())),
            known_invocations: Vec::new(),
            evidence: EvidenceRecord {
                schema_version: EVIDENCE_SCHEMA_VERSION,
                outcome: "running".into(),
                invocation_budget: LIVE_INVOCATION_BUDGET,
                invocations_launched: 0,
                phases: Vec::new(),
                cleanup: CleanupEvidence {
                    shutdown_completed: false,
                    active_direct_children: None,
                    cancellation: "not_attempted".into(),
                },
                final_invocations: Vec::new(),
                limitations: vec![
                    "Only direct Codex children are supervised; Windows descendant-tree cleanup is not claimed.".into(),
                    "Provider completion, context creation, rate limits, and cancellation timing are nondeterministic.".into(),
                ],
            },
        })
    }

    fn compose_fresh_application(&mut self) -> Result<(), String> {
        let new_database = !self.environment.database_path.exists();
        let connection = Connection::open(&self.environment.database_path)
            .map_err(|error| format!("open owned smoke database: {error}"))?;
        if new_database {
            connection
                .execute_batch(&format!("PRAGMA foreign_keys = ON; {AGENT_SESSION_SCHEMA}"))
                .map_err(|error| format!("initialize owned smoke database: {error}"))?;
        }
        let repository = Arc::new(
            SqliteAgentSessionRepository::new(connection)
                .map_err(|error| format!("compose smoke repository: {error}"))?,
        );
        let observed = self.observed_launches.clone();
        let runtime = Arc::new(
            CodexCliRuntime::system("codex", Some(self.capabilities.clone())).with_launch_observer(
                Arc::new(move |spec| {
                    observed
                        .lock()
                        .expect("live smoke observer lock")
                        .push(spec.clone())
                }),
            ),
        );
        let providers = Arc::new(SystemAgentSessionProviders);
        self.application = Some(AgentSessionApplication::new(
            repository,
            runtime.clone(),
            Arc::new(NoopNotifier),
            providers.clone(),
            providers,
            self.capabilities.version.clone(),
        ));
        self.runtime = Some(runtime);
        Ok(())
    }

    fn application(&self) -> Result<&AgentSessionApplication, String> {
        self.application
            .as_ref()
            .ok_or_else(|| "live smoke application is not composed".to_string())
    }

    fn send(
        &mut self,
        session_id: Option<AgentSessionId>,
        marker: &str,
    ) -> Result<(AgentSessionId, AgentInvocationId), String> {
        self.send_prompt(session_id, format!("Reply with only {marker}."))
    }

    fn send_prompt(
        &mut self,
        session_id: Option<AgentSessionId>,
        submitted_text: String,
    ) -> Result<(AgentSessionId, AgentInvocationId), String> {
        if self.evidence.invocations_launched >= LIVE_INVOCATION_BUDGET {
            return Err(format!(
                "live invocation budget of {LIVE_INVOCATION_BUDGET} exceeded"
            ));
        }
        let result = self
            .application()?
            .send_message(SendAgentSessionMessageCommand {
                session_id,
                submitted_text,
                title: Some("ignored live smoke".into()),
                working_directory: Some(
                    self.environment
                        .workspace_path
                        .to_string_lossy()
                        .into_owned(),
                ),
                requested_options: None,
            })
            .map_err(|error| format!("send live smoke prompt: {error}"))?;
        self.evidence.invocations_launched += 1;
        self.known_invocations.push(result.invocation_id.clone());
        Ok((result.session_id, result.invocation_id))
    }

    fn wait_for_terminal(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
    ) -> Result<AgentInvocation, String> {
        self.poll_invocation(session_id, invocation_id, |invocation| {
            invocation.status.is_terminal()
        })
    }

    fn poll_invocation(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        predicate: impl Fn(&AgentInvocation) -> bool,
    ) -> Result<AgentInvocation, String> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let history = self
                .application()?
                .load_session(session_id)
                .map_err(|error| error.to_string())?;
            let invocation = find_invocation(&history, invocation_id)?;
            if predicate(&invocation) {
                return Ok(invocation);
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "timed out after {}s waiting for invocation state",
                    self.timeout.as_secs()
                ));
            }
            thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())));
        }
    }

    fn wait_for_event_kind(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        kind: NormalizedRuntimeEventKind,
    ) -> Result<(), String> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let history = self
                .application()?
                .load_session(session_id)
                .map_err(|error| error.to_string())?;
            let invocation = history
                .invocations
                .iter()
                .find(|candidate| candidate.invocation.id == *invocation_id)
                .ok_or_else(|| format!("invocation {invocation_id} was not found"))?;
            if invocation.events.iter().any(|event| {
                event
                    .normalized
                    .as_ref()
                    .is_some_and(|event| event.kind == kind)
            }) {
                return Ok(());
            }
            if invocation.invocation.status.is_terminal() {
                return Err(format!(
                    "invocation {invocation_id} became terminal before {kind:?} evidence"
                ));
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "timed out after {}s waiting for {kind:?} evidence",
                    self.timeout.as_secs()
                ));
            }
            thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())));
        }
    }

    fn record_phase(
        &mut self,
        name: &str,
        status: &str,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        external_context_id: Option<&str>,
        resume_matches: Option<bool>,
        resume_differs: Option<bool>,
    ) {
        self.evidence.phases.push(PhaseEvidence {
            name: name.into(),
            status: status.into(),
            session_id_hash: Some(redact_id(session_id.as_str())),
            invocation_id_hash: Some(redact_id(invocation_id.as_str())),
            external_context_id_hash: external_context_id.map(redact_id),
            resume_target_matches_persisted_context: resume_matches,
            resume_target_differs_from_local_ids: resume_differs,
        });
    }

    fn close_current_runtime(&mut self) -> Result<(), String> {
        if let Some(application) = &self.application {
            application
                .shutdown_runtime()
                .map_err(|error| error.to_string())?;
        }
        let active = self
            .runtime
            .as_ref()
            .ok_or_else(|| "live smoke runtime is not composed".to_string())?
            .active_direct_child_count()
            .map_err(|error| error.to_string())?;
        self.evidence.cleanup.shutdown_completed = true;
        self.evidence.cleanup.active_direct_children = Some(active);
        if active != 0 {
            return Err(format!(
                "shutdown left {active} owned direct child process(es)"
            ));
        }
        self.application = None;
        self.runtime = None;
        Ok(())
    }

    fn execute(&mut self) -> Result<(), String> {
        self.compose_fresh_application()?;
        let (first_session, first_invocation) = self.send(None, "AS_SMOKE_FIRST")?;
        let first = self.wait_for_terminal(&first_session, &first_invocation)?;
        if first.status != AgentInvocationStatus::Completed {
            self.record_phase(
                "first_turn",
                "failed",
                &first_session,
                &first_invocation,
                None,
                None,
                None,
            );
            return Err(classify_terminal_failure("first turn", &first));
        }
        let first_history = self
            .application()?
            .load_session(&first_session)
            .map_err(|error| error.to_string())?;
        require_final_marker(&first_history, &first_invocation, "AS_SMOKE_FIRST")?;
        let external_context = first_history
            .session
            .runtime_binding
            .external_context_id
            .ok_or_else(|| {
                "first turn completed without persisted external Codex context".to_string()
            })?;
        self.record_phase(
            "first_turn",
            "completed",
            &first_session,
            &first_invocation,
            Some(external_context.as_str()),
            None,
            None,
        );

        self.close_current_runtime()?;
        self.compose_fresh_application()?;
        let reopened_history = self
            .application()?
            .load_session(&first_session)
            .map_err(|error| format!("load reopened first-turn history: {error}"))?;
        require_final_marker(&reopened_history, &first_invocation, "AS_SMOKE_FIRST")?;
        let (_, resume_invocation) = self.send(Some(first_session.clone()), "AS_SMOKE_RESUME")?;
        let resumed = self.wait_for_terminal(&first_session, &resume_invocation)?;
        let resumed_history = self
            .application()?
            .load_session(&first_session)
            .map_err(|error| error.to_string())?;
        require_final_marker(&resumed_history, &resume_invocation, "AS_SMOKE_RESUME")?;
        let observed_resume_target = self
            .observed_launches
            .lock()
            .map_err(|_| "live smoke observer lock poisoned".to_string())?
            .iter()
            .rev()
            .find(|spec| spec.args.iter().any(|arg| arg == "resume"))
            .and_then(observed_resume_target);
        let resume_matches = observed_resume_target.as_deref() == Some(external_context.as_str());
        let resume_differs = observed_resume_target.as_deref().is_some_and(|target| {
            target != first_session.as_str() && target != resume_invocation.as_str()
        });
        self.record_phase(
            "close_reopen_resume",
            if resumed.status == AgentInvocationStatus::Completed
                && resume_matches
                && resume_differs
            {
                "completed"
            } else {
                "failed"
            },
            &first_session,
            &resume_invocation,
            Some(external_context.as_str()),
            Some(resume_matches),
            Some(resume_differs),
        );
        if resumed.status != AgentInvocationStatus::Completed || !resume_matches || !resume_differs
        {
            return Err(classify_terminal_failure("resumed turn", &resumed));
        }

        let (concurrent_a_session, concurrent_a) = self.send_prompt(
            None,
            "Use a shell command to wait for 30 seconds, then reply with only AS_SMOKE_CONCURRENT_A."
                .into(),
        )?;
        let (concurrent_b_session, concurrent_b) = self.send_prompt(
            None,
            "Use a shell command to wait for 30 seconds, then reply with only AS_SMOKE_CANCEL_B."
                .into(),
        )?;
        let tool_started = self.wait_for_event_kind(
            &concurrent_b_session,
            &concurrent_b,
            NormalizedRuntimeEventKind::ToolActivity,
        );
        let current_a = self.application()?.load_session(&concurrent_a_session);
        let current_b = self.application()?.load_session(&concurrent_b_session);
        let both_active = current_a
            .ok()
            .and_then(|history| find_invocation(&history, &concurrent_a).ok())
            .is_some_and(|invocation| invocation.status.is_active())
            && current_b
                .ok()
                .and_then(|history| find_invocation(&history, &concurrent_b).ok())
                .is_some_and(|invocation| invocation.status.is_active());
        if tool_started.is_err() || !both_active {
            self.evidence.cleanup.cancellation = "cancellation_not_exercised_inconclusive".into();
            self.record_phase(
                "concurrency_and_cancellation",
                "inconclusive",
                &concurrent_b_session,
                &concurrent_b,
                None,
                None,
                None,
            );
            return Err("cancellation was not exercised because both sessions were not simultaneously active after durable tool-start evidence".into());
        }
        self.record_phase(
            "concurrent_sessions",
            "both_durably_running",
            &concurrent_a_session,
            &concurrent_a,
            None,
            None,
            None,
        );
        self.application()?
            .cancel_invocation(CancelAgentInvocationCommand {
                invocation_id: concurrent_b.clone(),
            })
            .map_err(|error| format!("cancel durable running invocation: {error}"))?;
        self.evidence.cleanup.cancellation = "requested_after_durable_running".into();
        let cancelled = self.wait_for_terminal(&concurrent_b_session, &concurrent_b)?;
        let concurrent = self.wait_for_terminal(&concurrent_a_session, &concurrent_a)?;
        let concurrent_ok = concurrent.status == AgentInvocationStatus::Completed;
        let cancellation_ok = cancelled.status == AgentInvocationStatus::Canceled;
        self.record_phase(
            "concurrency_and_cancellation",
            if concurrent_ok && cancellation_ok {
                "completed"
            } else {
                "failed"
            },
            &concurrent_b_session,
            &concurrent_b,
            None,
            None,
            None,
        );
        if !concurrent_ok || !cancellation_ok {
            return Err(format!(
                "concurrent status {:?}; cancellation status {:?}",
                concurrent.status, cancelled.status
            ));
        }
        self.close_current_runtime()?;
        self.evidence.outcome = "passed".into();
        Ok(())
    }

    fn execute_launch_acceptance_only(&mut self) -> Result<(), String> {
        self.compose_fresh_application()?;
        let invocation_id = self.application()?.allocate_application_invocation_id();
        let launch = self
            .application()?
            .send_idempotent_application_message_with_launch_observation(
                SendIdempotentApplicationAgentSessionMessageCommand {
                    invocation_id: invocation_id.clone(),
                    message: SendAgentSessionMessageCommand {
                        session_id: None,
                        submitted_text: "Reply with only PIP01D_LAUNCH_ACCEPTANCE.".into(),
                        title: Some("PIP-01D launch acceptance smoke".into()),
                        working_directory: Some(
                            self.environment.workspace_path.to_string_lossy().into_owned(),
                        ),
                        requested_options: None,
                    },
                },
                None,
            )
            .map_err(|error| format!("send application launch-acceptance prompt: {error}"))?;
        self.evidence.invocations_launched += 1;
        self.known_invocations.push(invocation_id.clone());
        let session_id = launch.acknowledgement.session_id;
        let launch = self
            .application()?
            .application_invocation_launch_evidence(&invocation_id, &session_id)
            .map_err(|error| format!("load launch acceptance evidence: {error}"))?;
        if launch != ApplicationInvocationLaunchEvidence::LaunchAccepted {
            return Err("real runtime did not durably accept the invocation launch".into());
        }
        let terminal = self.wait_for_terminal(&session_id, &invocation_id)?;
        let history = self
            .application()?
            .load_session(&session_id)
            .map_err(|error| format!("load launch-acceptance history: {error}"))?;
        self.record_phase(
            "launch_acceptance",
            "accepted",
            &session_id,
            &invocation_id,
            None,
            None,
            None,
        );
        let external_context = history.session.runtime_binding.external_context_id.ok_or_else(|| {
            self.record_phase(
                "external_context",
                "absent",
                &session_id,
                &invocation_id,
                None,
                None,
                None,
            );
            "launch-accepted invocation completed without persisted external Codex context".to_string()
        })?;
        self.record_phase(
            "launch_acceptance_and_external_context",
            if terminal.status == AgentInvocationStatus::Completed {
                "completed"
            } else {
                "failed"
            },
            &session_id,
            &invocation_id,
            Some(external_context.as_str()),
            None,
            None,
        );
        if terminal.status != AgentInvocationStatus::Completed {
            return Err(classify_terminal_failure("launch-acceptance turn", &terminal));
        }
        self.close_current_runtime()?;
        self.evidence.outcome = "passed".into();
        Ok(())
    }

    fn cleanup_after_failure(&mut self) {
        if self.close_current_runtime().is_err() {
            self.evidence.cleanup.shutdown_completed = false;
        }
    }

    fn capture_final_durable_state(&mut self) {
        let Ok(repository) = SqliteAgentSessionRepository::open(&self.environment.database_path)
        else {
            return;
        };
        self.evidence.final_invocations = self
            .known_invocations
            .iter()
            .map(|invocation_id| {
                let invocation = repository.get_invocation(invocation_id).ok().flatten();
                let events = repository.list_events(invocation_id).unwrap_or_default();
                let external_context_persisted = invocation
                    .as_ref()
                    .and_then(|invocation| repository.get_session(&invocation.session_id).ok())
                    .flatten()
                    .and_then(|session| session.runtime_binding.external_context_id)
                    .is_some();
                FinalInvocationEvidence {
                    invocation_id_hash: redact_id(invocation_id.as_str()),
                    durable_status: invocation
                        .as_ref()
                        .map(|invocation| format!("{:?}", invocation.status).to_ascii_lowercase()),
                    launch_accepted: repository
                        .invocation_launch_accepted_at(invocation_id)
                        .ok()
                        .flatten()
                        .is_some(),
                    external_context_persisted,
                    exit_code: invocation
                        .as_ref()
                        .and_then(|invocation| invocation.exit_code),
                    runtime_error_code: invocation
                        .as_ref()
                        .and_then(|invocation| invocation.runtime_error.as_ref())
                        .map(|failure| failure.code.clone()),
                    runtime_event_sources: event_source_names(&events),
                    normalized_event_kinds: normalized_event_kind_names(&events),
                    launch_provenance: sanitized_launch_provenance(&events),
                }
            })
            .collect();
    }

    fn write_evidence(&mut self) -> Result<(), String> {
        let json = self
            .evidence
            .to_json()
            .map_err(|error| format!("serialize smoke evidence: {error}"))?;
        let path = self
            .environment
            .root()
            .join("agent-session-live-smoke-evidence.json");
        std::fs::write(&path, &json).map_err(|error| format!("write smoke evidence: {error}"))?;
        println!("AGENT_SESSION_LIVE_SMOKE_EVIDENCE={json}");
        Ok(())
    }
}

fn find_invocation(
    history: &AgentSessionHistory,
    invocation_id: &AgentInvocationId,
) -> Result<AgentInvocation, String> {
    history
        .invocations
        .iter()
        .find(|candidate| candidate.invocation.id == *invocation_id)
        .map(|candidate| candidate.invocation.clone())
        .ok_or_else(|| "live smoke invocation disappeared from durable history".to_string())
}

fn require_final_marker(
    history: &AgentSessionHistory,
    invocation_id: &AgentInvocationId,
    marker: &str,
) -> Result<(), String> {
    let invocation = history
        .invocations
        .iter()
        .find(|candidate| candidate.invocation.id == *invocation_id)
        .ok_or_else(|| "live smoke invocation disappeared from durable history".to_string())?;
    let final_agent_message = invocation.events.iter().rev().find_map(|event| {
        event.normalized.as_ref().and_then(|normalized| {
            (normalized.kind == NormalizedRuntimeEventKind::AgentMessage)
                .then(|| normalized.text.as_deref())
                .flatten()
        })
    });
    if final_agent_message.is_some_and(|text| text.trim() == marker) {
        Ok(())
    } else {
        Err(format!(
            "invocation {invocation_id} completed without persisted final marker {marker}"
        ))
    }
}

fn observed_resume_target(spec: &ProcessLaunchSpec) -> Option<String> {
    let resume_index = spec.args.iter().position(|arg| arg == "resume")?;
    let prompt_index = spec.args.len().checked_sub(1)?;
    let target_index = prompt_index.checked_sub(1)?;
    (resume_index < target_index).then(|| spec.args[target_index].clone())
}

fn redact_id(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("hash:{:016x}", hasher.finish())
}

fn sanitized_launch_provenance(events: &[AgentRuntimeEvent]) -> Option<LaunchProvenanceEvidence> {
    events.iter().find_map(|event| {
        let payload = event.raw_payload.as_object()?;
        (payload.get("kind")?.as_str()? == "codex_launch_provenance").then(|| {
            let working_directory = payload.get("workingDirectory").and_then(Value::as_object);
            LaunchProvenanceEvidence {
                executable: payload
                    .get("executable")
                    .and_then(Value::as_str)
                    .filter(|value| matches!(*value, "codex" | "codex.exe"))
                    .map(str::to_owned),
                invocation_mode: payload
                    .get("invocationMode")
                    .and_then(Value::as_str)
                    .filter(|value| matches!(*value, "start" | "resume"))
                    .map(str::to_owned),
                configuration_keys: sanitized_key_names(payload.get("configurationKeys")),
                environment_keys: sanitized_key_names(payload.get("environmentKeys")),
                working_directory_provided: working_directory
                    .and_then(|value| value.get("provided"))
                    .and_then(Value::as_bool),
                working_directory_absolute: working_directory
                    .and_then(|value| value.get("absolute"))
                    .and_then(Value::as_bool),
                working_directory_extended_length_prefix: working_directory
                    .and_then(|value| value.get("extendedLengthPrefix"))
                    .and_then(Value::as_bool),
            }
        })
    })
}

fn event_source_names(events: &[AgentRuntimeEvent]) -> Vec<String> {
    let mut names = events
        .iter()
        .map(|event| format!("{:?}", event.source).to_ascii_lowercase())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn normalized_event_kind_names(events: &[AgentRuntimeEvent]) -> Vec<String> {
    let mut names = events
        .iter()
        .filter_map(|event| event.normalized.as_ref())
        .map(|normalized| format!("{:?}", normalized.kind).to_ascii_lowercase())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn sanitized_key_names(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|name| {
            !name.is_empty()
                && name.len() <= 128
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        })
        .map(str::to_owned)
        .collect()
}

fn classify_terminal_failure(phase: &str, invocation: &AgentInvocation) -> String {
    let details = invocation
        .runtime_error
        .as_ref()
        .map(|failure| {
            failure
                .details
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let classification = if ["quota", "rate limit", "usage limit", "rate_limit"]
        .iter()
        .any(|needle| details.to_ascii_lowercase().contains(needle))
    {
        "provider quota_or_rate_limit (not retried)"
    } else {
        "terminal failure"
    };
    format!("{phase} {classification}: {:?}", invocation.status)
}

#[test]
#[ignore = "requires CODEX_AGENT_SESSION_LIVE_SMOKE=true and launches real Codex invocations"]
fn agent_session_live_smoke_driver() {
    let config = LiveSmokeConfig::from_environment().expect("valid live smoke environment");
    assert!(
        config.enabled,
        "refusing live smoke: set {LIVE_SMOKE_OPT_IN_ENV}=true explicitly"
    );
    let mut driver = LiveSmokeDriver::new(config).expect("create live smoke driver");
    let result = driver.execute();
    if let Err(error) = &result {
        driver.evidence.outcome = "failed".into();
        driver.cleanup_after_failure();
        driver.evidence.phases.push(PhaseEvidence {
            name: "failure".into(),
            status: "failed".into(),
            session_id_hash: None,
            invocation_id_hash: None,
            external_context_id_hash: None,
            resume_target_matches_persisted_context: None,
            resume_target_differs_from_local_ids: None,
        });
        eprintln!("Agent Session live smoke failed: {error}");
    }
    driver.capture_final_durable_state();
    driver
        .write_evidence()
        .expect("write redacted live smoke evidence");
    result.expect("Agent Session live smoke lifecycle proof")
}

#[test]
#[ignore = "requires CODEX_AGENT_SESSION_LIVE_SMOKE=true and launches one real Codex invocation"]
fn agent_session_launch_acceptance_live_smoke_driver() {
    let config = LiveSmokeConfig::from_environment().expect("valid live smoke environment");
    assert!(
        config.enabled,
        "refusing live smoke: set {LIVE_SMOKE_OPT_IN_ENV}=true explicitly"
    );
    let mut driver = LiveSmokeDriver::new(config).expect("create live smoke driver");
    let result = driver.execute_launch_acceptance_only();
    if let Err(error) = &result {
        driver.evidence.outcome = "failed".into();
        driver.cleanup_after_failure();
        driver.evidence.phases.push(PhaseEvidence {
            name: "failure".into(),
            status: "failed".into(),
            session_id_hash: None,
            invocation_id_hash: None,
            external_context_id_hash: None,
            resume_target_matches_persisted_context: None,
            resume_target_differs_from_local_ids: None,
        });
        eprintln!("Agent Session launch-acceptance live smoke failed: {error}");
    }
    driver.capture_final_durable_state();
    driver
        .write_evidence()
        .expect("write redacted launch-acceptance live smoke evidence");
    result.expect("Agent Session launch-acceptance live smoke proof")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_requires_explicit_opt_in_and_bounds_timeout() {
        assert_eq!(
            LiveSmokeConfig::from_values(None, None).unwrap().enabled,
            false
        );
        assert!(LiveSmokeConfig::from_values(Some("maybe".into()), None).is_err());
        assert!(LiveSmokeConfig::from_values(Some("true".into()), Some("0".into())).is_err());
        assert!(LiveSmokeConfig::from_values(Some("true".into()), Some("301".into())).is_err());
        assert_eq!(
            LiveSmokeConfig::from_values(Some("yes".into()), Some("7".into()))
                .unwrap()
                .timeout,
            Duration::from_secs(7)
        );
    }

    #[test]
    fn environment_owns_only_temporary_database_and_workspace() {
        let environment = LiveSmokeEnvironment::create().expect("environment");
        assert!(environment.database_path.starts_with(environment.root()));
        assert!(environment.workspace_path.starts_with(environment.root()));
        assert!(validate_owned_path(environment.root(), Path::new("C:/normal/app.db")).is_err());
    }

    #[test]
    fn evidence_is_versioned_round_trippable_and_deterministic() {
        let evidence = EvidenceRecord {
            schema_version: EVIDENCE_SCHEMA_VERSION,
            outcome: "passed".into(),
            invocation_budget: LIVE_INVOCATION_BUDGET,
            invocations_launched: 4,
            phases: vec![PhaseEvidence {
                name: "resume".into(),
                status: "completed".into(),
                session_id_hash: Some(redact_id("local-session")),
                invocation_id_hash: Some(redact_id("local-invocation")),
                external_context_id_hash: Some(redact_id("external-thread")),
                resume_target_matches_persisted_context: Some(true),
                resume_target_differs_from_local_ids: Some(true),
            }],
            cleanup: CleanupEvidence {
                shutdown_completed: true,
                active_direct_children: Some(0),
                cancellation: "requested_after_durable_running".into(),
            },
            final_invocations: vec![FinalInvocationEvidence {
                invocation_id_hash: redact_id("local-invocation"),
                durable_status: Some("completed".into()),
                launch_accepted: true,
                external_context_persisted: true,
                exit_code: Some(0),
                runtime_error_code: None,
                runtime_event_sources: vec!["runtime".into(), "stdout".into()],
                normalized_event_kinds: vec!["runtimecontextestablished".into()],
                launch_provenance: Some(LaunchProvenanceEvidence {
                    executable: Some("codex".into()),
                    invocation_mode: Some("start".into()),
                    configuration_keys: vec!["mcp_servers.epic_runner.command".into()],
                    environment_keys: vec!["CODEX_OPAQUE_ENVIRONMENT_KEY".into()],
                    working_directory_provided: Some(true),
                    working_directory_absolute: Some(true),
                    working_directory_extended_length_prefix: Some(false),
                }),
            }],
            limitations: vec!["direct children only".into()],
        };
        let first = evidence.to_json().expect("json");
        assert_eq!(first, evidence.to_json().expect("same json"));
        assert_eq!(
            serde_json::from_str::<EvidenceRecord>(&first).unwrap(),
            evidence
        );
        assert!(first.starts_with(r#"{"schemaVersion":2,"outcome":"passed""#));
        assert!(!first.contains("external-thread"));
    }

    #[test]
    fn launch_provenance_evidence_keeps_only_safe_key_names_and_boolean_shape() {
        let event = AgentRuntimeEvent {
            id: crate::agent_sessions::domain::AgentRuntimeEventId::new("event-1").unwrap(),
            invocation_id: AgentInvocationId::new("invocation-1").unwrap(),
            sequence: 0,
            source: crate::agent_sessions::domain::AgentRuntimeEventSource::Runtime,
            raw_payload: serde_json::json!({
                "kind": "codex_launch_provenance",
                "executable": "codex",
                "invocationMode": "start",
                "configurationKeys": ["mcp_servers.epic_runner.command", "https://not-a-key"],
                "environmentKeys": ["CODEX_OPAQUE_ENVIRONMENT_KEY", "OPENAI_API_KEY"],
                "workingDirectory": {
                    "provided": true,
                    "absolute": true,
                    "extendedLengthPrefix": false,
                    "value": "C:/never-retained"
                },
                "sandbox": "read-only",
                "secret": "never-retained"
            }),
            normalized: None,
            recorded_at: chrono::Utc::now(),
        };

        let evidence = sanitized_launch_provenance(&[event]).expect("sanitized provenance");
        assert_eq!(evidence.executable.as_deref(), Some("codex"));
        assert_eq!(evidence.invocation_mode.as_deref(), Some("start"));
        assert_eq!(
            evidence.configuration_keys,
            vec!["mcp_servers.epic_runner.command"]
        );
        assert_eq!(
            evidence.environment_keys,
            vec!["CODEX_OPAQUE_ENVIRONMENT_KEY", "OPENAI_API_KEY"]
        );
        let rendered = serde_json::to_string(&evidence).unwrap();
        assert!(!rendered.contains("never-retained"));
        assert!(!rendered.contains("https://"));
        assert!(!rendered.contains("read-only"));
    }

    #[test]
    fn event_evidence_retains_only_source_and_normalized_kind_names() {
        let event = AgentRuntimeEvent {
            id: crate::agent_sessions::domain::AgentRuntimeEventId::new("event-2").unwrap(),
            invocation_id: AgentInvocationId::new("invocation-2").unwrap(),
            sequence: 1,
            source: crate::agent_sessions::domain::AgentRuntimeEventSource::Stdout,
            raw_payload: serde_json::json!({"providerPayload": "never-retained"}),
            normalized: Some(crate::agent_sessions::domain::NormalizedRuntimeEvent {
                kind: NormalizedRuntimeEventKind::RuntimeContextEstablished,
                text: Some("never-retained".into()),
                external_context_id: None,
                usage: None,
                details: Some(serde_json::json!({"url": "never-retained"})),
                tool_activity: None,
            }),
            recorded_at: chrono::Utc::now(),
        };
        assert_eq!(event_source_names(&[event.clone()]), vec!["stdout"]);
        assert_eq!(
            normalized_event_kind_names(&[event]),
            vec!["runtimecontextestablished"]
        );
    }

    #[test]
    fn resume_target_is_read_from_the_target_position_not_argument_membership() {
        let spec = ProcessLaunchSpec {
            program: "codex".into(),
            args: vec![
                "exec".into(),
                "resume".into(),
                "--model".into(),
                "persisted-context".into(),
                "actual-target".into(),
                "prompt mentions persisted-context".into(),
            ],
            working_directory: None,
            environment: Vec::new(),
        };
        assert_eq!(
            observed_resume_target(&spec).as_deref(),
            Some("actual-target")
        );
    }
}
