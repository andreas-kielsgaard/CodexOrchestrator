//! Durable post-confirmation preparation, semantic bootstrap completion, and Epic Runner launch.

use super::{
    conversation_harness::{self, ConversationHarnessRole},
    domain::PlanBuilderProposal,
    mcp::CodexMcpInjection,
};
use crate::agent_sessions::{
    application::{
        AgentSessionApplication, AgentSessionNotification, ApplicationInvocationLaunchEvidence,
        CreateAgentSessionCommand, CreateApplicationAgentSessionCommand,
        SendAgentSessionMessageCommand, SendIdempotentApplicationAgentSessionMessageCommand,
    },
    domain::{AgentInvocationId, AgentInvocationStatus, AgentSessionId},
    ports::RuntimeLaunchExtension,
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use http_body_util::Empty;
use hyper::{server::conn::http1, service::service_fn, Response};
use hyper_util::rt::TokioIo;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use rusqlite::{params, Connection, OptionalExtension};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    net::SocketAddr,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

pub(crate) const TRANSITION_QUERY_CONTRACT: &str = "epic-bootstrap-transition-query/v2";
const MAX_BOOTSTRAP_ATTEMPTS: i64 = 3;

pub(crate) const POST_CONFIRMATION_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS epic_bootstrap_transitions (
  initiation_id TEXT PRIMARY KEY,
  epic_id TEXT NOT NULL UNIQUE,
  proposal_revision_id TEXT NOT NULL,
  material_snapshot_hash TEXT NOT NULL,
  proposal_json TEXT NOT NULL CHECK (json_valid(proposal_json)),
  preparation_id TEXT NOT NULL UNIQUE,
  prepared_root TEXT NOT NULL UNIQUE,
  approved_plan_path TEXT NOT NULL UNIQUE,
  manifest_path TEXT NOT NULL UNIQUE,
  overview_path TEXT NOT NULL UNIQUE,
  runner_brief_path TEXT NOT NULL UNIQUE,
  bootstrap_session_id TEXT NOT NULL UNIQUE,
  bootstrap_invocation_id TEXT NOT NULL UNIQUE,
  runner_session_id TEXT NOT NULL UNIQUE,
  runner_invocation_id TEXT NOT NULL UNIQUE,
  prepared_at TEXT,
  bootstrap_session_created_at TEXT,
  bootstrap_launched_at TEXT,
  bootstrap_lifecycle_status TEXT,
  bootstrap_lifecycle_observed_at TEXT,
  semantic_completion_fact_id TEXT UNIQUE,
  semantic_completed_at TEXT,
  material_accepted_at TEXT,
  runner_session_created_at TEXT,
  runner_harness_key TEXT,
  runner_harness_version INTEGER,
  runner_harness_requested_at TEXT,
  runner_harness_applied_at TEXT,
  runner_launched_at TEXT,
  runner_lifecycle_status TEXT,
  runner_lifecycle_observed_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (initiation_id) REFERENCES epic_initiations(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS epic_bootstrap_completion_commands (
  id TEXT PRIMARY KEY,
  transition_id TEXT NOT NULL,
  agent_session_id TEXT NOT NULL,
  agent_invocation_id TEXT NOT NULL UNIQUE,
  payload_hash TEXT NOT NULL,
  payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (transition_id) REFERENCES epic_bootstrap_transitions(initiation_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS epic_bootstrap_completion_results (
  id TEXT PRIMARY KEY,
  command_id TEXT NOT NULL UNIQUE,
  inventory_json TEXT NOT NULL CHECK (json_valid(inventory_json)),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_bootstrap_completion_commands(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS epic_bootstrap_completion_facts (
  id TEXT PRIMARY KEY,
  transition_id TEXT NOT NULL UNIQUE,
  command_id TEXT NOT NULL UNIQUE,
  result_id TEXT NOT NULL UNIQUE,
  inventory_json TEXT NOT NULL CHECK (json_valid(inventory_json)),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (transition_id) REFERENCES epic_bootstrap_transitions(initiation_id) ON DELETE RESTRICT,
  FOREIGN KEY (command_id) REFERENCES epic_bootstrap_completion_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (result_id) REFERENCES epic_bootstrap_completion_results(id) ON DELETE RESTRICT
);
"#;

const POST_CONFIRMATION_RUNNER_HARNESS_SCHEMA: &str = r#"
ALTER TABLE epic_bootstrap_transitions ADD COLUMN runner_harness_key TEXT;
ALTER TABLE epic_bootstrap_transitions ADD COLUMN runner_harness_version INTEGER;
ALTER TABLE epic_bootstrap_transitions ADD COLUMN runner_harness_requested_at TEXT;
ALTER TABLE epic_bootstrap_transitions ADD COLUMN runner_harness_applied_at TEXT;
"#;

pub(crate) const POST_CONFIRMATION_ATTEMPT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS epic_bootstrap_attempts (
  id TEXT PRIMARY KEY,
  transition_id TEXT NOT NULL,
  ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
  agent_session_id TEXT NOT NULL,
  agent_invocation_id TEXT NOT NULL UNIQUE,
  launched_at TEXT,
  lifecycle_status TEXT,
  lifecycle_observed_at TEXT,
  semantic_completion_fact_id TEXT UNIQUE,
  semantic_completed_at TEXT,
  retry_disposition TEXT NOT NULL CHECK (retry_disposition IN ('active','retryable','retried','blocked','accepted')),
  retry_reason TEXT,
  retry_attempt_id TEXT UNIQUE,
  accepted_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (transition_id, ordinal),
  FOREIGN KEY (transition_id) REFERENCES epic_bootstrap_transitions(initiation_id) ON DELETE RESTRICT,
  FOREIGN KEY (retry_attempt_id) REFERENCES epic_bootstrap_attempts(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS epic_bootstrap_attempt_completion_commands (
  id TEXT PRIMARY KEY,
  attempt_id TEXT NOT NULL UNIQUE,
  agent_session_id TEXT NOT NULL,
  agent_invocation_id TEXT NOT NULL UNIQUE,
  payload_hash TEXT NOT NULL,
  payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (attempt_id) REFERENCES epic_bootstrap_attempts(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS epic_bootstrap_attempt_completion_results (
  id TEXT PRIMARY KEY,
  command_id TEXT NOT NULL UNIQUE,
  inventory_json TEXT NOT NULL CHECK (json_valid(inventory_json)),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_bootstrap_attempt_completion_commands(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS epic_bootstrap_attempt_completion_facts (
  id TEXT PRIMARY KEY,
  attempt_id TEXT NOT NULL UNIQUE,
  command_id TEXT NOT NULL UNIQUE,
  result_id TEXT NOT NULL UNIQUE,
  inventory_json TEXT NOT NULL CHECK (json_valid(inventory_json)),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (attempt_id) REFERENCES epic_bootstrap_attempts(id) ON DELETE RESTRICT,
  FOREIGN KEY (command_id) REFERENCES epic_bootstrap_attempt_completion_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (result_id) REFERENCES epic_bootstrap_attempt_completion_results(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_epic_bootstrap_attempts_one_accepted
ON epic_bootstrap_attempts(transition_id) WHERE accepted_at IS NOT NULL;

INSERT OR IGNORE INTO epic_bootstrap_attempts (
  id,transition_id,ordinal,agent_session_id,agent_invocation_id,launched_at,
  lifecycle_status,lifecycle_observed_at,semantic_completion_fact_id,semantic_completed_at,
  retry_disposition,retry_reason,accepted_at,created_at,updated_at
)
SELECT
  'epic-bootstrap-attempt-0-' || transition.initiation_id,
  transition.initiation_id,0,transition.bootstrap_session_id,transition.bootstrap_invocation_id,
  transition.bootstrap_launched_at,transition.bootstrap_lifecycle_status,
  transition.bootstrap_lifecycle_observed_at,transition.semantic_completion_fact_id,
  transition.semantic_completed_at,
  CASE
    WHEN transition.material_accepted_at IS NOT NULL THEN 'accepted'
    WHEN transition.bootstrap_lifecycle_status = 'interrupted' THEN 'retryable'
    WHEN transition.bootstrap_lifecycle_status IN ('failed','canceled') THEN 'blocked'
    WHEN transition.bootstrap_lifecycle_status = 'completed' AND transition.semantic_completion_fact_id IS NULL THEN 'blocked'
    ELSE 'active'
  END,
  CASE
    WHEN transition.bootstrap_lifecycle_status = 'interrupted' THEN 'startup_interrupted'
    WHEN transition.bootstrap_lifecycle_status IN ('failed','canceled') THEN 'terminal_without_retry_authority'
    WHEN transition.bootstrap_lifecycle_status = 'completed' AND transition.semantic_completion_fact_id IS NULL THEN 'completed_without_semantic_fact'
    ELSE NULL
  END,
  transition.material_accepted_at,transition.created_at,transition.updated_at
FROM epic_bootstrap_transitions transition;

INSERT OR IGNORE INTO epic_bootstrap_attempt_completion_commands (
  id,attempt_id,agent_session_id,agent_invocation_id,payload_hash,payload_json,recorded_at
)
SELECT command.id,attempt.id,command.agent_session_id,command.agent_invocation_id,
       command.payload_hash,command.payload_json,command.recorded_at
FROM epic_bootstrap_completion_commands command
JOIN epic_bootstrap_attempts attempt ON attempt.agent_invocation_id=command.agent_invocation_id;

INSERT OR IGNORE INTO epic_bootstrap_attempt_completion_results (
  id,command_id,inventory_json,recorded_at
)
SELECT result.id,result.command_id,result.inventory_json,result.recorded_at
FROM epic_bootstrap_completion_results result
JOIN epic_bootstrap_attempt_completion_commands command ON command.id=result.command_id;

INSERT OR IGNORE INTO epic_bootstrap_attempt_completion_facts (
  id,attempt_id,command_id,result_id,inventory_json,recorded_at
)
SELECT fact.id,attempt.id,fact.command_id,fact.result_id,fact.inventory_json,fact.recorded_at
FROM epic_bootstrap_completion_facts fact
JOIN epic_bootstrap_attempts attempt ON attempt.transition_id=fact.transition_id;
"#;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BootstrapMaterialInput {
    pub(crate) epic_overview_markdown: String,
    pub(crate) runner_brief_markdown: String,
}

impl BootstrapMaterialInput {
    fn validate(&self) -> Result<(), TransitionError> {
        validate_material_text(&self.epic_overview_markdown, "epicOverviewMarkdown")?;
        validate_material_text(&self.runner_brief_markdown, "runnerBriefMarkdown")
    }
}

fn validate_material_text(value: &str, field: &str) -> Result<(), TransitionError> {
    if value.trim().is_empty() || value.len() > 32_000 || value.contains('\0') {
        return Err(TransitionError::InvalidMaterial(format!(
            "{field} must be non-empty, NUL-free, and at most 32000 bytes"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MaterialInventoryItem {
    pub(crate) kind: String,
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) size_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SemanticCompletionResult {
    pub(crate) fact_id: String,
    pub(crate) inventory: Vec<MaterialInventoryItem>,
    pub(crate) idempotent_replay: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TransitionError {
    NotFound,
    Forbidden,
    IdentityMismatch(String),
    InvalidMaterial(String),
    IdempotencyConflict,
    Unavailable(String),
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("durable Epic initiation was not found"),
            Self::Forbidden => {
                formatter.write_str("the registered bootstrap invocation is not authorized")
            }
            Self::IdentityMismatch(message)
            | Self::InvalidMaterial(message)
            | Self::Unavailable(message) => formatter.write_str(message),
            Self::IdempotencyConflict => formatter.write_str(
                "bootstrap completion identity was already used for different material semantics",
            ),
        }
    }
}

#[derive(Clone, Debug)]
struct ConfirmedInitiationSnapshot {
    initiation_id: String,
    epic_id: String,
    proposal_revision_id: String,
    material_snapshot_hash: String,
    proposal_json: String,
    proposal: PlanBuilderProposal,
}

#[derive(Clone, Debug)]
struct TransitionRecord {
    initiation_id: String,
    epic_id: String,
    proposal_revision_id: String,
    material_snapshot_hash: String,
    proposal_json: String,
    proposal: PlanBuilderProposal,
    preparation_id: String,
    prepared_root: String,
    approved_plan_path: String,
    manifest_path: String,
    overview_path: String,
    runner_brief_path: String,
    bootstrap_session_id: String,
    bootstrap_invocation_id: String,
    runner_session_id: String,
    runner_invocation_id: String,
    prepared_at: Option<String>,
    bootstrap_session_created_at: Option<String>,
    bootstrap_launched_at: Option<String>,
    bootstrap_lifecycle_status: Option<String>,
    semantic_completion_fact_id: Option<String>,
    material_accepted_at: Option<String>,
    runner_session_created_at: Option<String>,
    runner_harness_key: Option<String>,
    runner_harness_version: Option<u16>,
    runner_harness_requested_at: Option<String>,
    runner_harness_applied_at: Option<String>,
    runner_launched_at: Option<String>,
}

#[derive(Clone, Debug)]
struct BootstrapAttemptRecord {
    id: String,
    transition_id: String,
    ordinal: i64,
    agent_session_id: String,
    agent_invocation_id: String,
    launched_at: Option<String>,
    lifecycle_status: Option<String>,
    lifecycle_observed_at: Option<String>,
    semantic_completion_fact_id: Option<String>,
    semantic_completed_at: Option<String>,
    retry_disposition: String,
    retry_reason: Option<String>,
    retry_attempt_id: Option<String>,
    accepted_at: Option<String>,
}

pub(crate) trait TransitionClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

struct SystemTransitionClock;
impl TransitionClock for SystemTransitionClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub(crate) struct SqliteBootstrapTransitionRepository {
    connection: Mutex<Connection>,
    clock: Arc<dyn TransitionClock>,
}

impl SqliteBootstrapTransitionRepository {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, TransitionError> {
        let connection = Connection::open(path).map_err(|error| {
            TransitionError::Unavailable(format!("open transition database: {error}"))
        })?;
        Self::new(connection)
    }

    pub(crate) fn new(connection: Connection) -> Result<Self, TransitionError> {
        Self::new_with_clock(connection, Arc::new(SystemTransitionClock))
    }

    fn new_with_clock(
        connection: Connection,
        clock: Arc<dyn TransitionClock>,
    ) -> Result<Self, TransitionError> {
        crate::storage::configure_sqlite_connection(&connection).map_err(|error| {
            TransitionError::Unavailable(format!("configure transition database: {error}"))
        })?;
        connection
            .execute_batch(POST_CONFIRMATION_SCHEMA)
            .map_err(|error| {
                TransitionError::Unavailable(format!("initialize transition schema: {error}"))
            })?;
        connection
            .execute_batch(POST_CONFIRMATION_ATTEMPT_SCHEMA)
            .map_err(|error| {
                TransitionError::Unavailable(format!(
                    "initialize bootstrap attempt schema: {error}"
                ))
            })?;
        for statement in POST_CONFIRMATION_RUNNER_HARNESS_SCHEMA.split(';') {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }
            match connection.execute(statement, []) {
                Ok(_) => {}
                Err(error) if error.to_string().contains("duplicate column name") => {}
                Err(error) => {
                    return Err(TransitionError::Unavailable(format!(
                        "initialize Epic Runner Harness binding schema: {error}"
                    )))
                }
            }
        }
        Ok(Self {
            connection: Mutex::new(connection),
            clock,
        })
    }

    fn snapshots(&self) -> Result<Vec<ConfirmedInitiationSnapshot>, TransitionError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare("SELECT initiation.id, initiation.epic_id, initiation.proposal_revision_id, snapshot.content_hash, snapshot.proposal_json FROM epic_initiations initiation JOIN epic_initiation_material_snapshots snapshot ON snapshot.id = initiation.material_snapshot_id ORDER BY initiation.recorded_at, initiation.id")
            .map_err(sql_unavailable("prepare confirmed initiation query"))?;
        let rows = statement
            .query_map([], |row| {
                let proposal_json: String = row.get(4)?;
                let proposal = serde_json::from_str(&proposal_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        proposal_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(ConfirmedInitiationSnapshot {
                    initiation_id: row.get(0)?,
                    epic_id: row.get(1)?,
                    proposal_revision_id: row.get(2)?,
                    material_snapshot_hash: row.get(3)?,
                    proposal_json,
                    proposal,
                })
            })
            .map_err(sql_unavailable("read confirmed initiations"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_unavailable("collect confirmed initiations"))?;
        Ok(rows)
    }

    fn snapshot(
        &self,
        initiation_id: &str,
    ) -> Result<ConfirmedInitiationSnapshot, TransitionError> {
        self.snapshots()?
            .into_iter()
            .find(|snapshot| snapshot.initiation_id == initiation_id)
            .ok_or(TransitionError::NotFound)
    }

    fn ensure_transition(
        &self,
        snapshot: &ConfirmedInitiationSnapshot,
        paths: &PreparedPaths,
    ) -> Result<TransitionRecord, TransitionError> {
        let now = self.timestamp();
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT OR IGNORE INTO epic_bootstrap_transitions (initiation_id,epic_id,proposal_revision_id,material_snapshot_hash,proposal_json,preparation_id,prepared_root,approved_plan_path,manifest_path,overview_path,runner_brief_path,bootstrap_session_id,bootstrap_invocation_id,runner_session_id,runner_invocation_id,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?16)",
                params![
                    snapshot.initiation_id,
                    snapshot.epic_id,
                    snapshot.proposal_revision_id,
                    snapshot.material_snapshot_hash,
                    snapshot.proposal_json,
                    paths.preparation_id,
                    paths.root,
                    paths.approved_plan,
                    paths.manifest,
                    paths.overview,
                    paths.runner_brief,
                    stable_id("epic-bootstrap-session", &snapshot.initiation_id),
                    stable_id("epic-bootstrap-invocation", &snapshot.initiation_id),
                    stable_id("epic-runner-session", &snapshot.initiation_id),
                    stable_id("epic-runner-invocation", &snapshot.initiation_id),
                    now,
                ],
            )
            .map_err(sql_unavailable("create post-confirmation transition"))?;
        let record = read_transition(&connection, &snapshot.initiation_id)?;
        if record.epic_id != snapshot.epic_id
            || record.proposal_revision_id != snapshot.proposal_revision_id
            || record.material_snapshot_hash != snapshot.material_snapshot_hash
            || record.proposal_json != snapshot.proposal_json
            || record.prepared_root != paths.root
            || record.approved_plan_path != paths.approved_plan
            || record.manifest_path != paths.manifest
            || record.overview_path != paths.overview
            || record.runner_brief_path != paths.runner_brief
        {
            return Err(TransitionError::IdentityMismatch(
                "persisted post-confirmation transition does not match the confirmed initiation snapshot or prepared paths".into(),
            ));
        }
        connection
            .execute(
                "INSERT OR IGNORE INTO epic_bootstrap_attempts (id,transition_id,ordinal,agent_session_id,agent_invocation_id,retry_disposition,created_at,updated_at) VALUES (?1,?2,0,?3,?4,'active',?5,?5)",
                params![
                    bootstrap_attempt_id(&snapshot.initiation_id, 0),
                    snapshot.initiation_id,
                    record.bootstrap_session_id,
                    record.bootstrap_invocation_id,
                    now,
                ],
            )
            .map_err(sql_unavailable("create initial bootstrap attempt"))?;
        Ok(record)
    }

    fn record_stage(&self, initiation_id: &str, column: &str) -> Result<(), TransitionError> {
        let allowed = [
            "prepared_at",
            "bootstrap_session_created_at",
            "bootstrap_launched_at",
            "material_accepted_at",
            "runner_session_created_at",
            "runner_harness_applied_at",
            "runner_launched_at",
        ];
        if !allowed.contains(&column) {
            return Err(TransitionError::Unavailable(
                "invalid durable transition stage".into(),
            ));
        }
        let now = self.timestamp();
        let connection = self.lock()?;
        connection
            .execute(
                &format!("UPDATE epic_bootstrap_transitions SET {column}=COALESCE({column},?2), updated_at=?2 WHERE initiation_id=?1"),
                params![initiation_id, now],
            )
            .map_err(sql_unavailable("record transition stage"))?;
        Ok(())
    }

    fn record_runner_harness_binding(
        &self,
        initiation_id: &str,
        key: &str,
        version: u16,
    ) -> Result<(), TransitionError> {
        let now = self.timestamp();
        let connection = self.lock()?;
        let changed = connection.execute(
            "UPDATE epic_bootstrap_transitions SET runner_harness_key=COALESCE(runner_harness_key,?2),runner_harness_version=COALESCE(runner_harness_version,?3),runner_harness_requested_at=COALESCE(runner_harness_requested_at,?4),updated_at=?4 WHERE initiation_id=?1 AND (runner_harness_key IS NULL OR (runner_harness_key=?2 AND runner_harness_version=?3))",
            params![initiation_id,key,version,now],
        ).map_err(sql_unavailable("record applied Epic Runner Harness binding"))?;
        if changed != 1 {
            return Err(TransitionError::IdentityMismatch(
                "Epic Runner Harness binding conflicts with the applied invocation configuration"
                    .into(),
            ));
        }
        Ok(())
    }

    fn record_lifecycle(
        &self,
        invocation_id: &str,
        status: &str,
        startup_recovery: bool,
    ) -> Result<Option<String>, TransitionError> {
        let now = self.timestamp();
        let connection = self.lock()?;
        let attempt: Option<(String, i64, bool, String, Option<String>)> = connection
            .query_row(
                "SELECT transition_id,ordinal,semantic_completion_fact_id IS NOT NULL,retry_disposition,retry_reason FROM epic_bootstrap_attempts WHERE agent_invocation_id=?1",
                params![invocation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(sql_unavailable("locate bootstrap attempt lifecycle"))?;
        if let Some((transition_id, ordinal, has_fact, prior_disposition, prior_reason)) = attempt {
            let (disposition, reason) = match status {
                _ if prior_disposition == "blocked"
                    && prior_reason.as_deref() == Some("terminal_without_retry_authority") =>
                {
                    (prior_disposition.as_str(), prior_reason.as_deref())
                }
                "completed" if has_fact => ("active", None),
                "completed" => ("blocked", Some("completed_without_semantic_fact")),
                "interrupted" if startup_recovery && ordinal + 1 < MAX_BOOTSTRAP_ATTEMPTS => {
                    ("retryable", Some("startup_interrupted"))
                }
                "interrupted" if startup_recovery => {
                    ("blocked", Some("startup_retry_limit_reached"))
                }
                "interrupted" | "failed" | "canceled" => {
                    ("blocked", Some("terminal_without_retry_authority"))
                }
                _ => ("active", None),
            };
            connection
                .execute(
                    "UPDATE epic_bootstrap_attempts SET lifecycle_status=?2,lifecycle_observed_at=COALESCE(lifecycle_observed_at,?3),retry_disposition=CASE WHEN retry_disposition IN ('accepted','retried') THEN retry_disposition ELSE ?4 END,retry_reason=CASE WHEN retry_disposition IN ('accepted','retried') THEN retry_reason ELSE ?5 END,updated_at=?3 WHERE agent_invocation_id=?1",
                    params![invocation_id, status, now, disposition, reason],
                )
                .map_err(sql_unavailable("record bootstrap attempt lifecycle"))?;
            return Ok(Some(transition_id));
        }
        let initiation_id: Option<String> = connection
            .query_row(
                "SELECT initiation_id FROM epic_bootstrap_transitions WHERE runner_invocation_id=?1",
                params![invocation_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_unavailable("locate runner lifecycle transition"))?;
        let Some(initiation_id) = initiation_id else {
            return Ok(None);
        };
        connection
            .execute(
                "UPDATE epic_bootstrap_transitions SET runner_lifecycle_status=?2,runner_lifecycle_observed_at=COALESCE(runner_lifecycle_observed_at,?3),updated_at=?3 WHERE initiation_id=?1",
                params![initiation_id, status, now],
            )
            .map_err(sql_unavailable("record lifecycle observation"))?;
        Ok(Some(initiation_id))
    }

    fn current_attempt(
        &self,
        initiation_id: &str,
    ) -> Result<BootstrapAttemptRecord, TransitionError> {
        let connection = self.lock()?;
        read_current_attempt(&connection, initiation_id)
    }

    fn attempts(
        &self,
        initiation_id: &str,
    ) -> Result<Vec<BootstrapAttemptRecord>, TransitionError> {
        let connection = self.lock()?;
        read_attempts(&connection, initiation_id)
    }

    fn ensure_retry_attempt(
        &self,
        previous: &BootstrapAttemptRecord,
    ) -> Result<BootstrapAttemptRecord, TransitionError> {
        if previous.retry_disposition != "retryable" {
            return Ok(previous.clone());
        }
        let now = self.timestamp();
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin bootstrap retry"))?;
        let refreshed = read_attempt(&transaction, &previous.id)?;
        if let Some(next_id) = &refreshed.retry_attempt_id {
            return read_attempt(&transaction, next_id);
        }
        let ordinal = refreshed.ordinal + 1;
        if refreshed.retry_disposition != "retryable" || ordinal >= MAX_BOOTSTRAP_ATTEMPTS {
            return Ok(refreshed);
        }
        let attempt_id = bootstrap_attempt_id(&refreshed.transition_id, ordinal);
        let invocation_id = stable_id(
            &format!("epic-bootstrap-invocation-attempt-{ordinal}"),
            &refreshed.transition_id,
        );
        transaction
            .execute(
                "INSERT OR IGNORE INTO epic_bootstrap_attempts (id,transition_id,ordinal,agent_session_id,agent_invocation_id,retry_disposition,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,'active',?6,?6)",
                params![attempt_id, refreshed.transition_id, ordinal, refreshed.agent_session_id, invocation_id, now],
            )
            .map_err(sql_unavailable("create bootstrap retry attempt"))?;
        transaction
            .execute(
                "UPDATE epic_bootstrap_attempts SET retry_disposition='retried',retry_attempt_id=?2,updated_at=?3 WHERE id=?1 AND retry_disposition='retryable'",
                params![refreshed.id, attempt_id, now],
            )
            .map_err(sql_unavailable("link bootstrap retry attempt"))?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit bootstrap retry attempt"))?;
        read_attempt(&connection, &attempt_id)
    }

    fn record_attempt_launched(&self, attempt_id: &str) -> Result<(), TransitionError> {
        let now = self.timestamp();
        let connection = self.lock()?;
        connection
            .execute(
                "UPDATE epic_bootstrap_attempts SET launched_at=COALESCE(launched_at,?2),updated_at=?2 WHERE id=?1",
                params![attempt_id, now],
            )
            .map_err(sql_unavailable("record bootstrap attempt launch"))?;
        Ok(())
    }

    fn accept_attempt(&self, attempt_id: &str) -> Result<bool, TransitionError> {
        let now = self.timestamp();
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin bootstrap material acceptance"))?;
        let attempt = read_attempt(&transaction, attempt_id)?;
        if attempt.lifecycle_status.as_deref() != Some("completed")
            || attempt.semantic_completion_fact_id.is_none()
        {
            return Ok(false);
        }
        let accepted_attempt: Option<String> = transaction
            .query_row(
                "SELECT id FROM epic_bootstrap_attempts WHERE transition_id=?1 AND accepted_at IS NOT NULL",
                params![attempt.transition_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_unavailable("read accepted bootstrap attempt"))?;
        if let Some(accepted_attempt) = accepted_attempt {
            if accepted_attempt != attempt.id {
                return Err(TransitionError::IdentityMismatch(
                    "a different bootstrap attempt already supplied accepted material".into(),
                ));
            }
            return Ok(true);
        }
        transaction
            .execute(
                "UPDATE epic_bootstrap_attempts SET retry_disposition='accepted',retry_reason=NULL,accepted_at=?2,updated_at=?2 WHERE id=?1",
                params![attempt.id, now],
            )
            .map_err(sql_unavailable("accept bootstrap attempt"))?;
        transaction
            .execute(
                "UPDATE epic_bootstrap_transitions SET material_accepted_at=COALESCE(material_accepted_at,?2),updated_at=?2 WHERE initiation_id=?1",
                params![attempt.transition_id, now],
            )
            .map_err(sql_unavailable("accept bootstrap material"))?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit bootstrap material acceptance"))?;
        Ok(true)
    }

    fn persist_completion(
        &self,
        invocation_id: &str,
        input: &BootstrapMaterialInput,
        inventory: &[MaterialInventoryItem],
    ) -> Result<(String, bool), TransitionError> {
        input.validate()?;
        let payload_json = serde_json::to_string(input)
            .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
        let payload_hash = sha256(payload_json.as_bytes());
        let inventory_json = serde_json::to_string(inventory)
            .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
        let now = self.timestamp();
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin semantic completion"))?;
        let attempt: Option<(String, String)> = transaction
            .query_row(
                "SELECT attempt.id,attempt.agent_session_id FROM epic_bootstrap_attempts attempt JOIN epic_bootstrap_transitions transition ON transition.initiation_id=attempt.transition_id WHERE attempt.agent_invocation_id=?1 AND transition.prepared_at IS NOT NULL AND transition.bootstrap_session_created_at IS NOT NULL AND attempt.retry_disposition IN ('active','blocked')",
                params![invocation_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sql_unavailable("authorize semantic completion"))?;
        let Some((attempt_id, session_id)) = attempt else {
            return Err(TransitionError::Forbidden);
        };
        let existing: Option<(String, String)> = transaction
            .query_row(
                "SELECT command.payload_hash,fact.id FROM epic_bootstrap_attempt_completion_commands command JOIN epic_bootstrap_attempt_completion_facts fact ON fact.command_id=command.id WHERE command.agent_invocation_id=?1",
                params![invocation_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sql_unavailable("read semantic completion replay"))?;
        if let Some((stored_hash, fact_id)) = existing {
            if stored_hash != payload_hash {
                return Err(TransitionError::IdempotencyConflict);
            }
            return Ok((fact_id, true));
        }
        let command_id = stable_id("epic-bootstrap-completion-command", invocation_id);
        let result_id = stable_id("epic-bootstrap-completion-result", invocation_id);
        let fact_id = stable_id("epic-bootstrap-completion-fact", invocation_id);
        transaction
            .execute(
                "INSERT INTO epic_bootstrap_attempt_completion_commands (id,attempt_id,agent_session_id,agent_invocation_id,payload_hash,payload_json,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![command_id, attempt_id, session_id, invocation_id, payload_hash, payload_json, now],
            )
            .map_err(sql_unavailable("persist semantic completion command"))?;
        transaction
            .execute(
                "INSERT INTO epic_bootstrap_attempt_completion_results (id,command_id,inventory_json,recorded_at) VALUES (?1,?2,?3,?4)",
                params![result_id, command_id, inventory_json, now],
            )
            .map_err(sql_unavailable("persist semantic completion result"))?;
        transaction
            .execute(
                "INSERT INTO epic_bootstrap_attempt_completion_facts (id,attempt_id,command_id,result_id,inventory_json,recorded_at) VALUES (?1,?2,?3,?4,?5,?6)",
                params![fact_id, attempt_id, command_id, result_id, inventory_json, now],
            )
            .map_err(sql_unavailable("persist semantic completion fact"))?;
        transaction
            .execute(
                "UPDATE epic_bootstrap_attempts SET semantic_completion_fact_id=?2,semantic_completed_at=?3,retry_disposition=CASE WHEN retry_reason='completed_without_semantic_fact' THEN 'active' ELSE retry_disposition END,retry_reason=CASE WHEN retry_reason='completed_without_semantic_fact' THEN NULL ELSE retry_reason END,updated_at=?3 WHERE id=?1",
                params![attempt_id, fact_id, now],
            )
            .map_err(sql_unavailable("record semantic completion"))?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit semantic completion"))?;
        Ok((fact_id, false))
    }

    fn completion_replay(
        &self,
        invocation_id: &str,
        input: &BootstrapMaterialInput,
    ) -> Result<Option<SemanticCompletionResult>, TransitionError> {
        let payload_json = serde_json::to_string(input)
            .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
        let payload_hash = sha256(payload_json.as_bytes());
        let connection = self.lock()?;
        let existing: Option<(String, String, String)> = connection
            .query_row(
                "SELECT command.payload_hash,fact.id,fact.inventory_json FROM epic_bootstrap_attempt_completion_commands command JOIN epic_bootstrap_attempt_completion_facts fact ON fact.command_id=command.id WHERE command.agent_invocation_id=?1",
                params![invocation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(sql_unavailable("read semantic completion replay"))?;
        let Some((stored_hash, fact_id, inventory_json)) = existing else {
            return Ok(None);
        };
        if stored_hash != payload_hash {
            return Err(TransitionError::IdempotencyConflict);
        }
        let inventory = serde_json::from_str(&inventory_json).map_err(|error| {
            TransitionError::Unavailable(format!("decode semantic completion replay: {error}"))
        })?;
        Ok(Some(SemanticCompletionResult {
            fact_id,
            inventory,
            idempotent_replay: true,
        }))
    }

    fn transition(&self, initiation_id: &str) -> Result<TransitionRecord, TransitionError> {
        let connection = self.lock()?;
        read_transition(&connection, initiation_id)
    }

    fn completion_inventory(
        &self,
        attempt_id: &str,
    ) -> Result<Vec<MaterialInventoryItem>, TransitionError> {
        let connection = self.lock()?;
        let json: String = connection
            .query_row(
                "SELECT inventory_json FROM epic_bootstrap_attempt_completion_facts WHERE attempt_id=?1",
                params![attempt_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_unavailable("read accepted material inventory"))?
            .ok_or(TransitionError::NotFound)?;
        serde_json::from_str(&json).map_err(|error| {
            TransitionError::Unavailable(format!("decode accepted material inventory: {error}"))
        })
    }

    fn query(&self) -> Result<BootstrapTransitionQueryV2, TransitionError> {
        let connection = self.lock()?;
        let mut transitions = {
            let mut statement = connection
                .prepare("SELECT initiation_id,epic_id,preparation_id,prepared_root,approved_plan_path,manifest_path,overview_path,runner_brief_path,bootstrap_session_id,bootstrap_invocation_id,prepared_at,bootstrap_session_created_at,bootstrap_launched_at,bootstrap_lifecycle_status,bootstrap_lifecycle_observed_at,semantic_completion_fact_id,semantic_completed_at,material_accepted_at,runner_session_id,runner_invocation_id,runner_session_created_at,runner_launched_at,runner_lifecycle_status,runner_lifecycle_observed_at FROM epic_bootstrap_transitions ORDER BY created_at, initiation_id")
                .map_err(sql_unavailable("prepare transition query"))?;
            let rows = statement
                .query_map([], |row| {
                    Ok(BootstrapTransitionStatusV2 {
                        initiation_id: row.get(0)?,
                        epic_id: row.get(1)?,
                        preparation_id: row.get(2)?,
                        prepared_root: row.get(3)?,
                        approved_plan_path: row.get(4)?,
                        manifest_path: row.get(5)?,
                        overview_path: row.get(6)?,
                        runner_brief_path: row.get(7)?,
                        bootstrap_session_id: row.get(8)?,
                        bootstrap_invocation_id: row.get(9)?,
                        prepared_at: row.get(10)?,
                        bootstrap_session_created_at: row.get(11)?,
                        bootstrap_launched_at: row.get(12)?,
                        bootstrap_lifecycle_status: row.get(13)?,
                        bootstrap_lifecycle_observed_at: row.get(14)?,
                        semantic_completion_fact_id: row.get(15)?,
                        semantic_completed_at: row.get(16)?,
                        material_accepted_at: row.get(17)?,
                        runner_session_id: row.get(18)?,
                        runner_invocation_id: row.get(19)?,
                        runner_session_created_at: row.get(20)?,
                        runner_launched_at: row.get(21)?,
                        runner_lifecycle_status: row.get(22)?,
                        runner_lifecycle_observed_at: row.get(23)?,
                        current_attempt_id: String::new(),
                        retry_state: String::new(),
                        blocked_reason: None,
                        accepted_attempt_id: None,
                        bootstrap_attempts: Vec::new(),
                    })
                })
                .map_err(sql_unavailable("read transition query"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_unavailable("collect transition query"))?;
            rows
        };
        for transition in &mut transitions {
            let attempts = read_attempts(&connection, &transition.initiation_id)?;
            let current = attempts.last().ok_or_else(|| {
                TransitionError::Unavailable("bootstrap transition has no attempt".into())
            })?;
            transition.bootstrap_session_id = current.agent_session_id.clone();
            transition.bootstrap_invocation_id = current.agent_invocation_id.clone();
            transition.bootstrap_launched_at = current.launched_at.clone();
            transition.bootstrap_lifecycle_status = current.lifecycle_status.clone();
            transition.bootstrap_lifecycle_observed_at = current.lifecycle_observed_at.clone();
            transition.semantic_completion_fact_id = current.semantic_completion_fact_id.clone();
            transition.semantic_completed_at = current.semantic_completed_at.clone();
            transition.current_attempt_id = current.id.clone();
            transition.retry_state = current.retry_disposition.clone();
            transition.blocked_reason = current.retry_reason.clone();
            transition.accepted_attempt_id = attempts
                .iter()
                .find(|attempt| attempt.accepted_at.is_some())
                .map(|attempt| attempt.id.clone());
            transition.bootstrap_attempts = attempts
                .into_iter()
                .map(BootstrapAttemptStatusV2::from)
                .collect();
        }
        Ok(BootstrapTransitionQueryV2 {
            contract: TRANSITION_QUERY_CONTRACT.into(),
            schema_version: 2,
            transitions,
        })
    }

    fn timestamp(&self) -> String {
        self.clock.now().to_rfc3339()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, TransitionError> {
        self.connection.lock().map_err(|_| {
            TransitionError::Unavailable("bootstrap transition database lock is poisoned".into())
        })
    }
}

pub(crate) trait BootstrapInvocationHandle: Send {
    fn injection(&self) -> &CodexMcpInjection;
    fn stop(self: Box<Self>);
}

pub(crate) trait BootstrapInvocationFactory: Send + Sync {
    fn start(
        &self,
        service: Arc<PostConfirmationTransitionService>,
        invocation_id: AgentInvocationId,
        enabled_tools: &[String],
        required: bool,
    ) -> Result<Box<dyn BootstrapInvocationHandle>, String>;
}

struct ProductionBootstrapInvocationFactory;
impl BootstrapInvocationFactory for ProductionBootstrapInvocationFactory {
    fn start(
        &self,
        service: Arc<PostConfirmationTransitionService>,
        invocation_id: AgentInvocationId,
        enabled_tools: &[String],
        required: bool,
    ) -> Result<Box<dyn BootstrapInvocationHandle>, String> {
        start_managed_bootstrap_invocation(
            service,
            invocation_id,
            enabled_tools,
            required,
            vec!["tauri://localhost".into()],
        )
        .map(|managed| Box::new(managed) as Box<dyn BootstrapInvocationHandle>)
        .map_err(|error| error.to_string())
    }
}

#[derive(Default)]
struct BootstrapInvocationRegistry {
    active: Mutex<HashMap<AgentInvocationId, Box<dyn BootstrapInvocationHandle>>>,
}

impl BootstrapInvocationRegistry {
    fn insert(&self, id: AgentInvocationId, handle: Box<dyn BootstrapInvocationHandle>) {
        let Ok(mut active) = self.active.lock() else {
            handle.stop();
            return;
        };
        if active.contains_key(&id) {
            handle.stop();
        } else {
            active.insert(id, handle);
        }
    }

    fn remove(&self, id: &AgentInvocationId) {
        if let Ok(mut active) = self.active.lock() {
            if let Some(handle) = active.remove(id) {
                handle.stop();
            }
        }
    }

    fn shutdown(&self) {
        if let Ok(mut active) = self.active.lock() {
            for (_, handle) in active.drain() {
                handle.stop();
            }
        }
    }
}

pub(crate) struct PostConfirmationTransitionService {
    repository: Arc<SqliteBootstrapTransitionRepository>,
    sessions: Arc<AgentSessionApplication>,
    material_root: PathBuf,
    factory: Arc<dyn BootstrapInvocationFactory>,
    registry: BootstrapInvocationRegistry,
    sprint_runners:
        Mutex<Option<Arc<super::sprint_runner_transition::SprintRunnerTransitionService>>>,
}

impl PostConfirmationTransitionService {
    pub(crate) fn new(
        repository: Arc<SqliteBootstrapTransitionRepository>,
        sessions: Arc<AgentSessionApplication>,
        material_root: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            repository,
            sessions,
            material_root,
            factory: Arc::new(ProductionBootstrapInvocationFactory),
            registry: BootstrapInvocationRegistry::default(),
            sprint_runners: Mutex::new(None),
        })
    }

    #[cfg(test)]
    fn with_factory(
        repository: Arc<SqliteBootstrapTransitionRepository>,
        sessions: Arc<AgentSessionApplication>,
        material_root: PathBuf,
        factory: Arc<dyn BootstrapInvocationFactory>,
    ) -> Arc<Self> {
        Arc::new(Self {
            repository,
            sessions,
            material_root,
            factory,
            registry: BootstrapInvocationRegistry::default(),
            sprint_runners: Mutex::new(None),
        })
    }

    pub(crate) fn on_initiation_persisted(
        self: &Arc<Self>,
        initiation_id: &str,
    ) -> Result<(), TransitionError> {
        let snapshot = self.repository.snapshot(initiation_id)?;
        let paths = PreparedPaths::derive(&self.material_root, &snapshot)?;
        self.repository.ensure_transition(&snapshot, &paths)?;
        self.reconcile(initiation_id)
    }

    pub(crate) fn reconcile_startup(self: &Arc<Self>) -> Result<usize, TransitionError> {
        let snapshots = self.repository.snapshots()?;
        let mut reconciled = 0;
        for snapshot in snapshots {
            let paths = PreparedPaths::derive(&self.material_root, &snapshot)?;
            self.repository.ensure_transition(&snapshot, &paths)?;
            self.observe_existing_terminals(&snapshot.initiation_id)?;
            self.reconcile(&snapshot.initiation_id)?;
            reconciled += 1;
        }
        Ok(reconciled)
    }

    pub(crate) fn on_agent_notification(
        self: &Arc<Self>,
        notification: &AgentSessionNotification,
    ) -> Result<(), TransitionError> {
        let AgentSessionNotification::InvocationTerminal { invocation, .. } = notification else {
            return Ok(());
        };
        let status = invocation_status(invocation.status);
        if let Some(initiation_id) =
            self.repository
                .record_lifecycle(invocation.id.as_str(), status, false)?
        {
            self.registry.remove(&invocation.id);
            if let Ok(slot) = self.sprint_runners.lock() {
                if let Some(service) = slot.as_ref() {
                    service.on_epic_runner_terminal(&invocation.id);
                }
            }
            self.reconcile(&initiation_id)?;
        }
        Ok(())
    }

    pub(crate) fn complete_bootstrap(
        self: &Arc<Self>,
        invocation_id: &AgentInvocationId,
        input: BootstrapMaterialInput,
    ) -> Result<SemanticCompletionResult, TransitionError> {
        input.validate()?;
        if let Some(replay) = self
            .repository
            .completion_replay(invocation_id.as_str(), &input)?
        {
            return Ok(replay);
        }
        let query = self.repository.query()?;
        let transition = query
            .transitions
            .iter()
            .find(|transition| transition.bootstrap_invocation_id == invocation_id.as_str())
            .ok_or(TransitionError::Forbidden)?;
        let record = self.repository.transition(&transition.initiation_id)?;
        let inventory = write_materials(&record, &input)?;
        let (fact_id, idempotent_replay) =
            self.repository
                .persist_completion(invocation_id.as_str(), &input, &inventory)?;
        self.reconcile(&record.initiation_id)?;
        Ok(SemanticCompletionResult {
            fact_id,
            inventory,
            idempotent_replay,
        })
    }

    pub(crate) fn query(&self) -> Result<BootstrapTransitionQueryV2, TransitionError> {
        self.repository.query()
    }

    pub(crate) fn shutdown(&self) {
        self.registry.shutdown();
        if let Ok(slot) = self.sprint_runners.lock() {
            if let Some(service) = slot.as_ref() {
                service.shutdown();
            }
        }
    }

    pub(crate) fn attach_sprint_runner_transition(
        &self,
        service: Arc<super::sprint_runner_transition::SprintRunnerTransitionService>,
    ) -> Result<(), TransitionError> {
        let mut slot = self.sprint_runners.lock().map_err(|_| {
            TransitionError::Unavailable(
                "Sprint Runner transition attachment is unavailable".into(),
            )
        })?;
        if slot.is_some() {
            return Err(TransitionError::Unavailable(
                "Sprint Runner transition is already attached".into(),
            ));
        }
        *slot = Some(service);
        Ok(())
    }

    pub(crate) fn persisted_initiation_observer(
        self: &Arc<Self>,
    ) -> Arc<dyn super::confirmation::PersistedInitiationObserver> {
        Arc::new(PersistedTransitionObserver(self.clone()))
    }

    fn reconcile(self: &Arc<Self>, initiation_id: &str) -> Result<(), TransitionError> {
        let mut record = self.repository.transition(initiation_id)?;
        let snapshot = ConfirmedInitiationSnapshot {
            initiation_id: record.initiation_id.clone(),
            epic_id: record.epic_id.clone(),
            proposal_revision_id: record.proposal_revision_id.clone(),
            material_snapshot_hash: record.material_snapshot_hash.clone(),
            proposal_json: record.proposal_json.clone(),
            proposal: record.proposal.clone(),
        };
        let paths = PreparedPaths {
            preparation_id: record.preparation_id.clone(),
            root: record.prepared_root.clone(),
            approved_plan: record.approved_plan_path.clone(),
            manifest: record.manifest_path.clone(),
            overview: record.overview_path.clone(),
            runner_brief: record.runner_brief_path.clone(),
        };
        prepare_inputs(&self.material_root, &snapshot, &paths)?;
        if record.prepared_at.is_none() {
            self.repository.record_stage(initiation_id, "prepared_at")?;
            record = self.repository.transition(initiation_id)?;
        }

        let discovery_root = conversation_harness::role_discovery_root(
            ConversationHarnessRole::EpicBootstrapGenerator,
        )
        .map_err(TransitionError::Unavailable)?;
        let bootstrap_harness =
            conversation_harness::profile(ConversationHarnessRole::EpicBootstrapGenerator)
                .map_err(TransitionError::Unavailable)?;
        let bootstrap_session_id = AgentSessionId::new(record.bootstrap_session_id.clone())
            .map_err(|error| TransitionError::IdentityMismatch(error.to_string()))?;
        if record.bootstrap_session_created_at.is_none() {
            self.sessions
                .create_application_session(CreateApplicationAgentSessionCommand {
                    session_id: bootstrap_session_id.clone(),
                    session: CreateAgentSessionCommand {
                        title: Some(format!("Epic Bootstrap Generator: {}", epic_name(&record))),
                        working_directory: Some(discovery_root.clone()),
                        requested_options: bootstrap_harness.runtime_options(),
                    },
                })
                .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
            self.repository
                .record_stage(initiation_id, "bootstrap_session_created_at")?;
            record = self.repository.transition(initiation_id)?;
        }

        let mut attempt = self.repository.current_attempt(initiation_id)?;
        if attempt.retry_disposition == "retryable" {
            attempt = self.repository.ensure_retry_attempt(&attempt)?;
        }

        if attempt.retry_disposition == "active" && attempt.launched_at.is_none() {
            let invocation_id = AgentInvocationId::new(attempt.agent_invocation_id.clone())
                .map_err(|error| TransitionError::IdentityMismatch(error.to_string()))?;
            match self
                .sessions
                .application_invocation_launch_evidence(&invocation_id, &bootstrap_session_id)
                .map_err(|error| TransitionError::Unavailable(error.to_string()))?
            {
                ApplicationInvocationLaunchEvidence::LaunchAccepted => {
                    self.repository.record_attempt_launched(&attempt.id)?;
                }
                ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {}
                ApplicationInvocationLaunchEvidence::NeverPersisted => {
                    let managed = self
                        .factory
                        .start(
                            self.clone(),
                            invocation_id.clone(),
                            &bootstrap_harness.mcp.enabled_tools,
                            bootstrap_harness.mcp.required,
                        )
                        .map_err(TransitionError::Unavailable)?;
                    let mut additional_args = bootstrap_harness.runtime_configuration_args();
                    additional_args.extend(managed.injection().configuration_args.clone());
                    let extension = RuntimeLaunchExtension {
                        additional_args,
                        environment: vec![managed.injection().environment.clone()],
                        initial_prompt_prefix: Some(bootstrap_harness.initial_prompt_prefix()),
                    };
                    let send = self
                        .sessions
                        .send_idempotent_application_message_with_launch_observation(
                            SendIdempotentApplicationAgentSessionMessageCommand {
                                invocation_id: invocation_id.clone(),
                                message: SendAgentSessionMessageCommand {
                                    session_id: Some(bootstrap_session_id.clone()),
                                    submitted_text: bootstrap_prompt(&record, &attempt),
                                    title: None,
                                    working_directory: Some(discovery_root),
                                    requested_options: Some(bootstrap_harness.runtime_options()),
                                },
                            },
                            Some(extension),
                        );
                    match send {
                        Ok(launch) => {
                            let terminal = self
                                .sessions
                                .load_session(&bootstrap_session_id)
                                .map_err(|error| TransitionError::Unavailable(error.to_string()))?
                                .invocations
                                .iter()
                                .find(|candidate| candidate.invocation.id == invocation_id)
                                .is_some_and(|candidate| candidate.invocation.status.is_terminal());
                            if terminal {
                                managed.stop();
                            } else {
                                self.registry.insert(invocation_id, managed);
                            }
                            if launch.launch_accepted {
                                self.repository.record_attempt_launched(&attempt.id)?;
                            }
                        }
                        Err(error) => {
                            managed.stop();
                            return Err(TransitionError::Unavailable(error.to_string()));
                        }
                    }
                }
            }
            attempt = self.repository.current_attempt(initiation_id)?;
        }

        if attempt.semantic_completion_fact_id.is_some()
            && attempt.lifecycle_status.as_deref() == Some("completed")
        {
            if self.repository.accept_attempt(&attempt.id)? {
                record = self.repository.transition(initiation_id)?;
                self.ensure_runner(&record, &attempt.id)?;
            }
        }
        Ok(())
    }

    fn ensure_runner(
        self: &Arc<Self>,
        record: &TransitionRecord,
        accepted_attempt_id: &str,
    ) -> Result<(), TransitionError> {
        let harness = conversation_harness::profile(ConversationHarnessRole::EpicRunner)
            .map_err(TransitionError::Unavailable)?;
        let discovery_root =
            conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunner)
                .map_err(TransitionError::Unavailable)?;
        let session_id = AgentSessionId::new(record.runner_session_id.clone())
            .map_err(|error| TransitionError::IdentityMismatch(error.to_string()))?;
        if record.runner_session_created_at.is_none() {
            self.sessions
                .create_application_session(CreateApplicationAgentSessionCommand {
                    session_id: session_id.clone(),
                    session: CreateAgentSessionCommand {
                        title: Some(format!("Epic Runner: {}", epic_name(record))),
                        working_directory: Some(discovery_root.clone()),
                        requested_options: harness.runtime_options(),
                    },
                })
                .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
            self.repository
                .record_stage(&record.initiation_id, "runner_session_created_at")?;
        }
        let refreshed = self.repository.transition(&record.initiation_id)?;
        if refreshed.runner_launched_at.is_none() {
            let invocation_id = AgentInvocationId::new(refreshed.runner_invocation_id.clone())
                .map_err(|error| TransitionError::IdentityMismatch(error.to_string()))?;
            let launch_accepted = match self
                .sessions
                .application_invocation_launch_evidence(&invocation_id, &session_id)
                .map_err(|error| TransitionError::Unavailable(error.to_string()))?
            {
                ApplicationInvocationLaunchEvidence::LaunchAccepted => true,
                ApplicationInvocationLaunchEvidence::PersistedNotAccepted => false,
                ApplicationInvocationLaunchEvidence::NeverPersisted => {
                    let sprint_runners = self
                        .sprint_runners
                        .lock()
                        .map_err(|_| {
                            TransitionError::Unavailable(
                                "Sprint Runner transition attachment is unavailable".into(),
                            )
                        })?
                        .clone()
                        .ok_or_else(|| {
                            TransitionError::Unavailable(
                                "Sprint Runner application action is unavailable".into(),
                            )
                        })?;
                    let injection = sprint_runners
                        .prepare_epic_runner_action(
                            invocation_id.clone(),
                            &harness.mcp.enabled_tools,
                            harness.mcp.required,
                        )
                        .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
                    self.repository.record_runner_harness_binding(
                        &record.initiation_id,
                        &harness.key,
                        harness.version,
                    )?;
                    let mut additional_args = harness.runtime_configuration_args();
                    additional_args.extend(injection.configuration_args);
                    let extension = RuntimeLaunchExtension {
                        additional_args,
                        environment: vec![injection.environment],
                        initial_prompt_prefix: Some(harness.initial_prompt_prefix()),
                    };
                    let launch = self
                        .sessions
                        .send_idempotent_application_message_with_launch_observation(
                            SendIdempotentApplicationAgentSessionMessageCommand {
                                invocation_id,
                                message: SendAgentSessionMessageCommand {
                                    session_id: Some(session_id),
                                    submitted_text: self
                                        .runner_prompt(&refreshed, accepted_attempt_id)?,
                                    title: None,
                                    working_directory: Some(discovery_root),
                                    requested_options: Some(harness.runtime_options()),
                                },
                            },
                            Some(extension),
                        )
                        .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
                    self.repository
                        .record_stage(&record.initiation_id, "runner_harness_applied_at")?;
                    launch.launch_accepted
                }
            };
            if launch_accepted {
                self.repository
                    .record_stage(&record.initiation_id, "runner_launched_at")?;
            }
        }
        Ok(())
    }

    fn runner_prompt(
        &self,
        record: &TransitionRecord,
        accepted_attempt_id: &str,
    ) -> Result<String, TransitionError> {
        let inventory = self.repository.completion_inventory(accepted_attempt_id)?;
        let inventory = serde_json::to_string_pretty(&inventory)
            .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
        Ok(format!(
            "Prepare to run the durably initiated Epic below. Do not create or start a product Sprint in this invocation.\n\nInitiation ID: {}\nEpic ID: {}\nApproved plan: {}\nTransition manifest: {}\nAccepted material inventory:\n{}\n\nThe approved proposal snapshot is:\n{}",
            record.initiation_id,
            record.epic_id,
            record.approved_plan_path,
            record.manifest_path,
            inventory,
            record.proposal_json,
        ))
    }

    fn observe_existing_terminals(&self, initiation_id: &str) -> Result<(), TransitionError> {
        let record = self.repository.transition(initiation_id)?;
        for attempt in self.repository.attempts(initiation_id)? {
            let session_id = AgentSessionId::new(attempt.agent_session_id.clone())
                .map_err(|error| TransitionError::IdentityMismatch(error.to_string()))?;
            let Ok(history) = self.sessions.load_session(&session_id) else {
                continue;
            };
            if let Some(found) = history
                .invocations
                .iter()
                .find(|candidate| candidate.invocation.id.as_str() == attempt.agent_invocation_id)
            {
                if found.invocation.status.is_terminal() {
                    self.repository.record_lifecycle(
                        &attempt.agent_invocation_id,
                        invocation_status(found.invocation.status),
                        true,
                    )?;
                }
            }
        }
        let session_id = AgentSessionId::new(record.runner_session_id.clone())
            .map_err(|error| TransitionError::IdentityMismatch(error.to_string()))?;
        if let Ok(history) = self.sessions.load_session(&session_id) {
            if let Some(found) = history
                .invocations
                .iter()
                .find(|candidate| candidate.invocation.id.as_str() == record.runner_invocation_id)
            {
                if found.invocation.status.is_terminal() {
                    self.repository.record_lifecycle(
                        &record.runner_invocation_id,
                        invocation_status(found.invocation.status),
                        true,
                    )?;
                }
            }
        }
        Ok(())
    }
}

struct PersistedTransitionObserver(Arc<PostConfirmationTransitionService>);

impl super::confirmation::PersistedInitiationObserver for PersistedTransitionObserver {
    fn on_persisted_initiation(
        &self,
        initiation: &super::domain::InitiateEpicResult,
    ) -> Result<(), String> {
        self.0
            .on_initiation_persisted(initiation.initiation_id.as_str())
            .map_err(|error| error.to_string())
    }
}

fn bootstrap_prompt(record: &TransitionRecord, attempt: &BootstrapAttemptRecord) -> String {
    format!(
        "Generate the bounded Epic bootstrap materials from the exact approved durable plan below. Submit both outputs once through complete_epic_bootstrap; do not write files directly and do not start the Epic Runner.\n\nInitiation ID: {}\nEpic ID: {}\nBootstrap attempt ID: {}\nBootstrap attempt ordinal: {}\nPrepared root: {}\nApproved plan input: {}\nTransition manifest: {}\nRequired output destinations (application-owned):\n- Epic overview: {}\n- Runner brief: {}\n\nNo additional accepted decisions, deferred decisions, or user-control policy were stored with this proposal; do not invent them.\n\nApproved proposal snapshot:\n{}",
        record.initiation_id,
        record.epic_id,
        attempt.id,
        attempt.ordinal,
        record.prepared_root,
        record.approved_plan_path,
        record.manifest_path,
        record.overview_path,
        record.runner_brief_path,
        record.proposal_json,
    )
}

fn epic_name(record: &TransitionRecord) -> String {
    record
        .proposal
        .suggested_epic_name
        .clone()
        .unwrap_or_else(|| record.epic_id.clone())
}

fn invocation_status(status: AgentInvocationStatus) -> &'static str {
    match status {
        AgentInvocationStatus::Pending => "pending",
        AgentInvocationStatus::Running => "running",
        AgentInvocationStatus::Completed => "completed",
        AgentInvocationStatus::Failed => "failed",
        AgentInvocationStatus::Canceled => "canceled",
        AgentInvocationStatus::Interrupted => "interrupted",
    }
}

#[derive(Clone)]
struct BootstrapCompletionMcp {
    service: Arc<PostConfirmationTransitionService>,
    invocation_id: AgentInvocationId,
    tool_router: ToolRouter<Self>,
}

impl BootstrapCompletionMcp {
    fn new(
        service: Arc<PostConfirmationTransitionService>,
        invocation_id: AgentInvocationId,
    ) -> Self {
        Self {
            service,
            invocation_id,
            tool_router: Self::tool_router(),
        }
    }

    fn error(code: &str, guidance: impl Into<String>) -> CallToolResult {
        CallToolResult::error(vec![ContentBlock::text(
            serde_json::json!({
                "code": code,
                "guidance": guidance.into(),
                "operationId": uuid::Uuid::new_v4().to_string(),
            })
            .to_string(),
        )])
    }
}

#[tool_router]
impl BootstrapCompletionMcp {
    #[tool(
        description = "Submit the two bounded semantic bootstrap materials exactly once. Input is ONLY {epicOverviewMarkdown: string, runnerBriefMarkdown: string}. The application derives Epic, session, invocation, paths, and replay authority, validates both values, writes them to exact prepared destinations, and returns the durable material fact. Do not send IDs or paths."
    )]
    fn complete_epic_bootstrap(
        &self,
        Parameters(input): Parameters<BootstrapMaterialInput>,
    ) -> CallToolResult {
        match self.service.complete_bootstrap(&self.invocation_id, input) {
            Ok(result) => CallToolResult::success(vec![ContentBlock::text(
                serde_json::json!({
                    "status": if result.idempotent_replay { "idempotent_replay" } else { "persisted" },
                    "semanticCompletionFactId": result.fact_id,
                    "inventory": result.inventory,
                    "guidance": "Semantic bootstrap completion is durable. Do not retry this successful submission; the application still requires the matching Agent Session lifecycle observation before material acceptance.",
                })
                .to_string(),
            )]),
            Err(TransitionError::Forbidden | TransitionError::NotFound) => Self::error(
                "forbidden",
                "This invocation is not the registered Bootstrap Generator invocation.",
            ),
            Err(TransitionError::InvalidMaterial(message)) => Self::error(
                "invalid_material",
                format!("{message}. Correct the bounded content and retry once."),
            ),
            Err(TransitionError::IdempotencyConflict) => Self::error(
                "idempotency_conflict",
                "This invocation already completed with different material semantics.",
            ),
            Err(TransitionError::IdentityMismatch(_) | TransitionError::Unavailable(_)) => {
                Self::error("internal_error", "The application could not record bootstrap completion.")
            }
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BootstrapCompletionMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Generate only the requested bounded content, then call complete_epic_bootstrap once. Tool exposure never supplies Epic, session, invocation, or path authority.",
        )
    }
}

struct ManagedBootstrapMcpServer {
    address: SocketAddr,
    cancellation: CancellationToken,
    join: Option<thread::JoinHandle<()>>,
}

impl ManagedBootstrapMcpServer {
    fn url(&self) -> String {
        format!("http://{}/mcp", self.address)
    }

    fn stop(mut self) {
        self.cancellation.cancel();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

struct ManagedBootstrapInvocation {
    server: ManagedBootstrapMcpServer,
    injection: CodexMcpInjection,
}

impl BootstrapInvocationHandle for ManagedBootstrapInvocation {
    fn injection(&self) -> &CodexMcpInjection {
        &self.injection
    }

    fn stop(self: Box<Self>) {
        self.server.stop();
    }
}

fn start_managed_bootstrap_invocation(
    service: Arc<PostConfirmationTransitionService>,
    invocation_id: AgentInvocationId,
    enabled_tools: &[String],
    required: bool,
    origins: Vec<String>,
) -> io::Result<ManagedBootstrapInvocation> {
    let bearer = uuid::Uuid::new_v4().simple().to_string();
    let server = start_bootstrap_server(service, invocation_id, bearer.clone(), origins)?;
    let injection = CodexMcpInjection::new_named(
        "epic_bootstrap",
        &server.url(),
        bearer,
        enabled_tools,
        required,
    );
    Ok(ManagedBootstrapInvocation { server, injection })
}

fn start_bootstrap_server(
    service: Arc<PostConfirmationTransitionService>,
    invocation_id: AgentInvocationId,
    bearer: String,
    origins: Vec<String>,
) -> io::Result<ManagedBootstrapMcpServer> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let cancellation = CancellationToken::new();
    let server_cancel = cancellation.clone();
    let join =
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("bootstrap MCP runtime");
            runtime.block_on(async move {
                let config =
                    rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default()
                        .with_allowed_hosts([format!("127.0.0.1:{}", address.port())])
                        .with_allowed_origins(origins.clone())
                        .with_cancellation_token(server_cancel.clone());
                let service_adapter: rmcp::transport::streamable_http_server::StreamableHttpService<
                BootstrapCompletionMcp,
                rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
            > = rmcp::transport::streamable_http_server::StreamableHttpService::new(
                move || Ok(BootstrapCompletionMcp::new(service.clone(), invocation_id.clone())),
                Default::default(),
                config,
            );
                let expected = Arc::new(bearer);
                let allowed_host = format!("127.0.0.1:{}", address.port());
                let allowed_origins = Arc::new(origins);
                let listener = tokio::net::TcpListener::from_std(listener)
                    .expect("async bootstrap MCP listener");
                loop {
                    let accepted = tokio::select! {
                        _ = server_cancel.cancelled() => break,
                        accepted = listener.accept() => accepted,
                    };
                    let Ok((stream, _)) = accepted else { continue };
                    let adapter = service_adapter.clone();
                    let expected = expected.clone();
                    let allowed_host = allowed_host.clone();
                    let allowed_origins = allowed_origins.clone();
                    tokio::spawn(async move {
                        let guard = service_fn(move |request| {
                            let adapter = adapter.clone();
                            let expected = expected.clone();
                            let allowed_host = allowed_host.clone();
                            let allowed_origins = allowed_origins.clone();
                            async move {
                                if let Some(status) = super::mcp::transport_denial(
                                    &expected,
                                    &allowed_host,
                                    &allowed_origins,
                                    &request,
                                ) {
                                    return Ok::<_, std::convert::Infallible>(
                                        Response::builder()
                                            .status(status)
                                            .body(Empty::<Bytes>::new())
                                            .expect("bootstrap MCP denial response")
                                            .map(axum::body::Body::new),
                                    );
                                }
                                let response = adapter
                                    .oneshot(request)
                                    .await
                                    .expect("bootstrap MCP response");
                                Ok::<_, std::convert::Infallible>(
                                    response.map(axum::body::Body::new),
                                )
                            }
                        });
                        let _ = http1::Builder::new()
                            .serve_connection(TokioIo::new(stream), guard)
                            .await;
                    });
                }
            });
        });
    Ok(ManagedBootstrapMcpServer {
        address,
        cancellation,
        join: Some(join),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent_sessions::{
            application::{AgentSessionNotifier, SystemAgentSessionProviders},
            domain::{AgentInvocation, AgentInvocationTerminalStatus, AgentRuntimeOptions},
            ports::{
                AgentRuntime, AgentRuntimeUpdateSink, RuntimeInvocationMode,
                RuntimeInvocationOutcome, RuntimeInvocationPreflight, RuntimeInvocationRequest,
                RuntimePortError, RuntimePortErrorKind, RuntimeUpdate,
            },
            repository::SqliteAgentSessionRepository,
        },
        orchestration::{
            application::OrchestrationApplication,
            accepted_candidate_authority::reconcile_accepted_candidate_authorities,
            accepted_integration::reconcile_accepted_integrations,
            conversation_harness::{self, ConversationHarnessRole},
            domain::{InitiateEpicCommand, ProposedSprint, SaveEpicPlanProposalCommand},
            execution_support::ProductExecutionSupportState,
            repository::{InitiatedSprintGitAuthorityWrite, SqliteOrchestrationRepository},
            work_unit_execution_harness::{
                WorkUnitExecutionHarnessService, WorkUnitHarnessRole,
            },
        },
    };
    use sha2::{Digest, Sha256};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Barrier, Weak,
    };

    #[derive(Default)]
    struct RecordedRuntime {
        requests: Mutex<Vec<RuntimeInvocationRequest>>,
        sinks: Mutex<HashMap<AgentInvocationId, Arc<dyn AgentRuntimeUpdateSink>>>,
        fail_next_launch: AtomicUsize,
        terminal_on_next_start: AtomicUsize,
    }

    impl RecordedRuntime {
        fn requests(&self) -> Vec<RuntimeInvocationRequest> {
            self.requests.lock().unwrap().clone()
        }

        fn fail_next_launch(&self) {
            self.fail_next_launch.store(1, Ordering::SeqCst);
        }

        /// Deliberately emits through the production AgentSession notifier before `start_invocation`
        /// returns. This exercises notifier re-entry rather than calling transition methods directly.
        fn terminal_on_next_start(&self) {
            self.terminal_on_next_start.store(1, Ordering::SeqCst);
        }

        fn finish(&self, invocation_id: &str, status: AgentInvocationTerminalStatus) {
            self.finish_result(invocation_id, status).unwrap();
        }

        fn finish_result(
            &self,
            invocation_id: &str,
            status: AgentInvocationTerminalStatus,
        ) -> Result<(), RuntimePortError> {
            let id = AgentInvocationId::new(invocation_id).unwrap();
            let sink = self.sinks.lock().unwrap().get(&id).cloned().unwrap();
            sink.emit_update(
                &id,
                RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                    status,
                    exit_code: Some(if status == AgentInvocationTerminalStatus::Completed {
                        0
                    } else {
                        1
                    }),
                    signal: None,
                    runtime_error: None,
                }),
            )
        }
    }

    impl AgentRuntime for RecordedRuntime {
        fn preflight_invocation(
            &self,
            _mode: RuntimeInvocationMode,
            requested_options: &AgentRuntimeOptions,
        ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            Ok(RuntimeInvocationPreflight {
                effective_options: requested_options.clone(),
            })
        }

        fn start_invocation(
            &self,
            request: RuntimeInvocationRequest,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.requests.lock().unwrap().push(request.clone());
            if self.fail_next_launch.swap(0, Ordering::SeqCst) == 1 {
                return Err(RuntimePortError::new(
                    RuntimePortErrorKind::LaunchFailed,
                    "induced transition launch failure",
                ));
            }
            self.sinks
                .lock()
                .unwrap()
                .insert(request.invocation_id.clone(), sink);
            if self.terminal_on_next_start.swap(0, Ordering::SeqCst) == 1 {
                self.finish_result(
                    request.invocation_id.as_str(),
                    AgentInvocationTerminalStatus::Completed,
                )?;
            }
            Ok(())
        }

        fn resume_invocation(
            &self,
            request: RuntimeInvocationRequest,
            _external_context_id: crate::agent_sessions::domain::ExternalRuntimeContextId,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.start_invocation(request, sink)
        }

        fn cancel_invocation(
            &self,
            _invocation_id: &AgentInvocationId,
        ) -> Result<(), RuntimePortError> {
            Err(RuntimePortError::new(
                RuntimePortErrorKind::NotActive,
                "recorded runtime cancellation is unavailable",
            ))
        }
    }

    #[derive(Default)]
    struct TransitionNotifier {
        service: Mutex<Option<Weak<PostConfirmationTransitionService>>>,
        sprint: Mutex<Option<Weak<crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService>>>,
    }

    impl TransitionNotifier {
        fn set(&self, service: &Arc<PostConfirmationTransitionService>) {
            *self.service.lock().unwrap() = Some(Arc::downgrade(service));
        }

        fn set_sprint(
            &self,
            service: &Arc<crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService>,
        ) {
            *self.sprint.lock().unwrap() = Some(Arc::downgrade(service));
        }
    }

    impl AgentSessionNotifier for TransitionNotifier {
        fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
            if let Some(service) = self
                .service
                .lock()
                .map_err(|_| "test notification registry unavailable".to_string())?
                .as_ref()
                .and_then(Weak::upgrade)
            {
                service
                    .on_agent_notification(&notification)
                    .map_err(|error| error.to_string())?;
            }
            if let Some(service) = self
                .sprint
                .lock()
                .map_err(|_| "test Sprint notification registry unavailable".to_string())?
                .as_ref()
                .and_then(Weak::upgrade)
            {
                service
                    .on_agent_notification(&notification)
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct LiveTransitionNotifier {
        service: Mutex<Option<Weak<PostConfirmationTransitionService>>>,
        terminals: Mutex<Vec<AgentInvocation>>,
        ready: std::sync::Condvar,
    }

    impl LiveTransitionNotifier {
        fn set(&self, service: &Arc<PostConfirmationTransitionService>) {
            *self.service.lock().unwrap() = Some(Arc::downgrade(service));
        }

        fn wait_for_terminals(&self, count: usize) -> Vec<AgentInvocation> {
            let terminals = self.terminals.lock().unwrap();
            let (terminals, wait) = self
                .ready
                .wait_timeout_while(terminals, std::time::Duration::from_secs(240), |items| {
                    items.len() < count
                })
                .unwrap();
            assert!(!wait.timed_out(), "installed Codex transition timed out");
            terminals.clone()
        }
    }

    impl AgentSessionNotifier for LiveTransitionNotifier {
        fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
            if let Some(service) = self
                .service
                .lock()
                .map_err(|_| "live transition registry unavailable".to_string())?
                .as_ref()
                .and_then(Weak::upgrade)
            {
                service
                    .on_agent_notification(&notification)
                    .map_err(|error| error.to_string())?;
            }
            if let AgentSessionNotification::InvocationTerminal { invocation, .. } = notification {
                self.terminals
                    .lock()
                    .map_err(|_| "live transition terminal registry unavailable".to_string())?
                    .push(invocation);
                self.ready.notify_all();
            }
            Ok(())
        }
    }

    struct DummyHandle {
        injection: CodexMcpInjection,
        stopped: Arc<AtomicUsize>,
    }

    impl BootstrapInvocationHandle for DummyHandle {
        fn injection(&self) -> &CodexMcpInjection {
            &self.injection
        }

        fn stop(self: Box<Self>) {
            self.stopped.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[derive(Default)]
    struct RecordedBootstrapFactory {
        starts: AtomicUsize,
        stops: Arc<AtomicUsize>,
    }

    impl BootstrapInvocationFactory for RecordedBootstrapFactory {
        fn start(
            &self,
            _service: Arc<PostConfirmationTransitionService>,
            _invocation_id: AgentInvocationId,
            enabled_tools: &[String],
            required: bool,
        ) -> Result<Box<dyn BootstrapInvocationHandle>, String> {
            self.starts.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(DummyHandle {
                injection: CodexMcpInjection::new_named(
                    "recorded_bootstrap",
                    "http://127.0.0.1:1/mcp",
                    "recorded-secret".into(),
                    enabled_tools,
                    required,
                ),
                stopped: self.stops.clone(),
            }))
        }
    }

    struct FailingBootstrapFactory;
    impl BootstrapInvocationFactory for FailingBootstrapFactory {
        fn start(
            &self,
            _service: Arc<PostConfirmationTransitionService>,
            _invocation_id: AgentInvocationId,
            _enabled_tools: &[String],
            _required: bool,
        ) -> Result<Box<dyn BootstrapInvocationHandle>, String> {
            Err("induced bootstrap listener failure".into())
        }
    }

    struct Fixture {
        _directory: tempfile::TempDir,
        database_path: PathBuf,
        material_root: PathBuf,
        initiation_id: String,
        service: Arc<PostConfirmationTransitionService>,
        sessions: Arc<AgentSessionApplication>,
        runtime: Arc<RecordedRuntime>,
        runtime_history: Vec<Arc<RecordedRuntime>>,
        notifier: Arc<TransitionNotifier>,
        factory: Arc<RecordedBootstrapFactory>,
    }

    impl Fixture {
        fn new() -> Self {
            let fixture = Self::unstarted();
            fixture
                .service
                .on_initiation_persisted(&fixture.initiation_id)
                .unwrap();
            fixture
        }

        fn unstarted() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database_path = directory.path().join("active.sqlite");
            drop(crate::storage::open_active_database(&database_path).unwrap());
            let now = Utc::now().to_rfc3339();
            let connection = Connection::open(&database_path).unwrap();
            crate::storage::configure_sqlite_connection(&connection).unwrap();
            connection.execute("INSERT INTO agent_sessions (id,title,availability,requested_options_json,created_at,updated_at) VALUES ('plan-builder-session','Plan Builder','available','{}',?1,?1)", params![now]).unwrap();
            drop(connection);

            let orchestration = SqliteOrchestrationRepository::open(&database_path).unwrap();
            let (draft, profile, association) = orchestration
                .bootstrap_managed_plan_builder("plan-builder-session")
                .unwrap();
            let saved = orchestration
                .save_epic_plan_proposal(SaveEpicPlanProposalCommand {
                    epic_planning_draft_id: draft.clone(),
                    capability_profile_id: profile,
                    agent_session_association_id: association,
                    agent_session_id: "plan-builder-session".into(),
                    actor_id: "managed-plan-builder".into(),
                    expected_revision: None,
                    proposal: PlanBuilderProposal {
                        suggested_epic_name: Some("Durable Bootstrap Epic".into()),
                        sprints: vec![
                            ProposedSprint {
                                title: "Foundation".into(),
                                intended_movement: "Establish the durable transition.".into(),
                                concern_summaries: vec!["Preserve explicit user control.".into()],
                            },
                            ProposedSprint {
                                title: "Integration".into(),
                                intended_movement: "Integrate later without starting now.".into(),
                                concern_summaries: vec![],
                            },
                        ],
                    },
                    idempotency_key: "fixture-proposal".into(),
                })
                .unwrap();
            let initiated = orchestration
                .initiate_epic(InitiateEpicCommand {
                    epic_planning_draft_id: draft,
                    expected_revision_token: saved.revision_token,
                    actor_id: "application-user".into(),
                    idempotency_key: "fixture-initiation".into(),
                })
                .unwrap();

            let agent_repository = Arc::new(
                SqliteAgentSessionRepository::new(Connection::open(&database_path).unwrap())
                    .unwrap(),
            );
            let runtime = Arc::new(RecordedRuntime::default());
            let notifier = Arc::new(TransitionNotifier::default());
            let sessions = Arc::new(AgentSessionApplication::new(
                agent_repository,
                runtime.clone(),
                notifier.clone(),
                Arc::new(SystemAgentSessionProviders),
                Arc::new(SystemAgentSessionProviders),
                Some("recorded-runtime".into()),
            ));
            let transition_repository =
                Arc::new(SqliteBootstrapTransitionRepository::open(&database_path).unwrap());
            let factory = Arc::new(RecordedBootstrapFactory::default());
            let material_root = directory.path().join("materials");
            let service = PostConfirmationTransitionService::with_factory(
                transition_repository,
                sessions.clone(),
                material_root.clone(),
                factory.clone(),
            );
            service
                .attach_sprint_runner_transition(
                    crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                        &database_path,
                        sessions.clone(),
                    )
                    .unwrap(),
                )
                .unwrap();
            notifier.set(&service);
            Self {
                _directory: directory,
                database_path,
                material_root,
                initiation_id: initiated.initiation_id.as_str().into(),
                service,
                sessions,
                runtime: runtime.clone(),
                runtime_history: vec![runtime],
                notifier,
                factory,
            }
        }

        fn status(&self) -> BootstrapTransitionStatusV2 {
            self.service.query().unwrap().transitions[0].clone()
        }

        fn prepare_work_slice_planner(&self) -> (
            Arc<crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService>,
            AgentInvocationId,
            String,
        ) {
            let bootstrap = self.status();
            let bootstrap_invocation = AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap();
            self.service
                .complete_bootstrap(
                    &bootstrap_invocation,
                    Self::materials(),
                )
                .unwrap();
            self.runtime
                .finish(&bootstrap.bootstrap_invocation_id, AgentInvocationTerminalStatus::Completed);
            let runner = self.status();
            let sprint_id: String = Connection::open(&self.database_path)
                .unwrap()
                .query_row(
                    "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            let service = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &self.database_path,
                self.sessions.clone(),
            )
            .unwrap();
            service
                .request_next_sprint_runner(
                    &AgentInvocationId::new(runner.runner_invocation_id).unwrap(),
                    crate::orchestration::sprint_runner_transition::SprintRunnerSelection {
                        sprint_id: sprint_id.clone(),
                    },
                )
                .unwrap();
            let control = AgentInvocationId::new("planning-control-http-invocation").unwrap();
            let control_harness = conversation_harness::profile(
                ConversationHarnessRole::SprintRunnerPlanningControl,
            )
            .unwrap();
            Connection::open(&self.database_path)
                .unwrap()
                .execute(
                    "UPDATE sprint_runner_transitions SET planning_control_invocation_id=?2,planning_control_harness_key=?3,planning_control_harness_version=?4,planning_control_harness_applied_at=?5,planning_control_launch_accepted_at=?5,planning_ready_at=?5 WHERE sprint_id=?1",
                    params![
                        sprint_id,
                        control.as_str(),
                        control_harness.key,
                        control_harness.version,
                        "2026-08-02T00:00:00Z"
                    ],
                )
                .unwrap();
            let repository_root = self._directory.path().join("http-sprint-repository");
            let worktree_root = repository_root.join("worktree");
            fs::create_dir_all(&worktree_root).unwrap();
            SqliteOrchestrationRepository::open(&self.database_path)
                .unwrap()
                .store_initiated_sprint_git_authority(InitiatedSprintGitAuthorityWrite {
                    sprint_id: sprint_id.clone(),
                    idempotency_key: "wsp-http-route-authority".into(),
                    repository_id: "wsp-http-repository".into(),
                    repository_root: repository_root.to_string_lossy().into_owned(),
                    repository_common_dir: repository_root.to_string_lossy().into_owned(),
                    worktree_id: "wsp-http-worktree".into(),
                    worktree_root: worktree_root.to_string_lossy().into_owned(),
                    baseline_object_id: "a".repeat(40),
                    current_object_id: "b".repeat(40),
                    runtime_instance_ref: "wsp-http-runtime".into(),
                    runtime_source_ref: "wsp-http-source".into(),
                    source_fingerprint: "c".repeat(64),
                })
                .unwrap();
            let status = service
                .request_work_slice_planner(
                    &control,
                    crate::orchestration::sprint_runner_transition::WorkSlicePlannerRequest {},
                )
                .unwrap();
            let planner_invocation = AgentInvocationId::new(
                status.work_slice_planner_invocation_id.unwrap(),
            )
            .unwrap();
            self.notifier.set_sprint(&service);
            (service, planner_invocation, sprint_id)
        }

        fn materials() -> BootstrapMaterialInput {
            BootstrapMaterialInput {
                epic_overview_markdown:
                    "# Durable Bootstrap Epic\n\nAdvance the approved Sprints in order.\n".into(),
                runner_brief_markdown:
                    "# Runner brief\n\nReach ready state without starting a Sprint.\n".into(),
            }
        }

        fn reopen_service(&mut self) {
            let repository =
                Arc::new(SqliteBootstrapTransitionRepository::open(&self.database_path).unwrap());
            let factory = Arc::new(RecordedBootstrapFactory::default());
            let service = PostConfirmationTransitionService::with_factory(
                repository,
                self.sessions.clone(),
                self.material_root.clone(),
                factory.clone(),
            );
            service.attach_sprint_runner_transition(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&self.database_path, self.sessions.clone()).unwrap()).unwrap();
            self.notifier.set(&service);
            service.reconcile_startup().unwrap();
            self.service = service;
            self.factory = factory;
        }

        fn restart_application(&mut self) -> usize {
            self.service.shutdown();
            let agent_repository = Arc::new(
                SqliteAgentSessionRepository::new(Connection::open(&self.database_path).unwrap())
                    .unwrap(),
            );
            let runtime = Arc::new(RecordedRuntime::default());
            let notifier = Arc::new(TransitionNotifier::default());
            let sessions = Arc::new(AgentSessionApplication::new(
                agent_repository,
                runtime.clone(),
                notifier.clone(),
                Arc::new(SystemAgentSessionProviders),
                Arc::new(SystemAgentSessionProviders),
                Some("recorded-runtime".into()),
            ));
            let reconciled = sessions.reconcile_startup().unwrap();
            let repository =
                Arc::new(SqliteBootstrapTransitionRepository::open(&self.database_path).unwrap());
            let factory = Arc::new(RecordedBootstrapFactory::default());
            let service = PostConfirmationTransitionService::with_factory(
                repository,
                sessions.clone(),
                self.material_root.clone(),
                factory.clone(),
            );
            service.attach_sprint_runner_transition(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&self.database_path, sessions.clone()).unwrap()).unwrap();
            notifier.set(&service);
            service.reconcile_startup().unwrap();
            self.service = service;
            self.sessions = sessions;
            self.runtime = runtime.clone();
            self.runtime_history.push(runtime);
            self.notifier = notifier;
            self.factory = factory;
            reconciled
        }

        fn all_requests(&self) -> Vec<RuntimeInvocationRequest> {
            self.runtime_history
                .iter()
                .flat_map(|runtime| runtime.requests())
                .collect()
        }
    }

    #[test]
    fn semantic_first_then_matching_lifecycle_launches_exactly_one_runner() {
        let mut fixture = Fixture::new();
        let initial = fixture.status();
        assert_eq!(fixture.runtime.requests().len(), 1);
        assert!(initial.prepared_at.is_some());
        assert!(initial.bootstrap_session_created_at.is_some());
        assert!(initial.bootstrap_launched_at.is_some());
        assert!(initial.semantic_completion_fact_id.is_none());
        assert!(initial.runner_launched_at.is_none());
        let bootstrap_request = &fixture.runtime.requests()[0];
        assert_eq!(
            bootstrap_request.options.sandbox,
            Some(crate::agent_sessions::domain::RuntimeSandboxMode::ReadOnly)
        );
        assert!(bootstrap_request
            .submitted_text
            .contains("complete_epic_bootstrap"));
        assert!(bootstrap_request
            .submitted_text
            .contains(&initial.approved_plan_path));
        assert_eq!(
            bootstrap_request
                .launch_extension
                .as_ref()
                .unwrap()
                .environment
                .len(),
            1
        );

        let completed = fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(initial.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        assert!(!completed.idempotent_replay);
        assert_eq!(completed.inventory.len(), 2);
        assert_eq!(
            fixture.runtime.requests().len(),
            1,
            "semantic-only must not launch Runner"
        );
        assert!(fixture.status().material_accepted_at.is_none());

        fixture.reopen_service();
        assert_eq!(
            fixture.runtime.requests().len(),
            1,
            "restart must not duplicate Bootstrap launch"
        );
        let status = fixture.status();
        fixture.runtime.finish(
            &status.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let accepted = fixture.status();
        assert!(accepted.bootstrap_lifecycle_observed_at.is_some());
        assert_eq!(
            accepted.bootstrap_lifecycle_status.as_deref(),
            Some("completed")
        );
        assert!(accepted.material_accepted_at.is_some());
        assert!(accepted.runner_session_created_at.is_some());
        assert!(accepted.runner_launched_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), 2);
        let runner_request = &fixture.runtime.requests()[1];
        assert_eq!(
            runner_request.options.sandbox,
            Some(crate::agent_sessions::domain::RuntimeSandboxMode::ReadOnly)
        );
        assert!(runner_request
            .submitted_text
            .contains("Do not create or start a product Sprint"));
        let runner_extension = runner_request.launch_extension.as_ref().unwrap();
        assert_eq!(runner_extension.environment.len(), 1);
        assert!(runner_extension
            .additional_args
            .iter()
            .any(|value| value.contains("request_next_sprint_runner")));

        fixture.service.reconcile_startup().unwrap();
        fixture.service.reconcile_startup().unwrap();
        assert_eq!(fixture.runtime.requests().len(), 2);
        let connection = Connection::open(&fixture.database_path).unwrap();
        for (table, expected) in [
            ("epic_bootstrap_transitions", 1),
            ("epic_bootstrap_attempts", 1),
            ("epic_bootstrap_attempt_completion_commands", 1),
            ("epic_bootstrap_attempt_completion_results", 1),
            ("epic_bootstrap_attempt_completion_facts", 1),
            ("agent_sessions", 3),
            ("agent_session_invocations", 2),
        ] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, expected, "unexpected durable count for {table}");
        }
    }

    #[test]
    fn launch_accepted_epic_runner_authorizes_one_ready_sprint_runner_without_downstream_effects() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &bootstrap.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let runner = fixture.status();
        assert!(runner.runner_launched_at.is_some());
        let sprint_id: String = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row(
                "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let service =
            crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &fixture.database_path,
                fixture.sessions.clone(),
            )
            .unwrap();
        let request = service
            .request_next_sprint_runner(
                &AgentInvocationId::new(runner.runner_invocation_id.clone()).unwrap(),
                crate::orchestration::sprint_runner_transition::SprintRunnerSelection {
                    sprint_id: sprint_id.clone(),
                },
            )
            .unwrap();
        assert!(request.pre_start_ready);
        assert!(!request.lifecycle_observed);
        assert!(!request.accepted);
        assert_eq!(fixture.runtime.requests().len(), 3);
        let replay = service
            .request_next_sprint_runner(
                &AgentInvocationId::new(runner.runner_invocation_id.clone()).unwrap(),
                crate::orchestration::sprint_runner_transition::SprintRunnerSelection {
                    sprint_id: sprint_id.clone(),
                },
            )
            .unwrap();
        assert_eq!(replay.request_id, request.request_id);
        assert_eq!(fixture.runtime.requests().len(), 3);
        assert!(matches!(
            service.request_next_sprint_runner(
                &AgentInvocationId::new("wrong-runner").unwrap(),
                crate::orchestration::sprint_runner_transition::SprintRunnerSelection { sprint_id: sprint_id.clone() },
            ),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        assert!(matches!(
            service.request_next_sprint_runner(
                &AgentInvocationId::new(bootstrap.runner_invocation_id.clone()).unwrap(),
                crate::orchestration::sprint_runner_transition::SprintRunnerSelection { sprint_id: "unknown-sprint".into() },
            ),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        let reopened =
            crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &fixture.database_path,
                fixture.sessions.clone(),
            )
            .unwrap();
        assert_eq!(reopened.reconcile_startup().unwrap(), 1);
        assert_eq!(fixture.runtime.requests().len(), 3);
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM sprint_runner_transitions",
                    [],
                    |row| row.get(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_sessions", [], |row| row.get(0))
                .unwrap(),
            4
        );
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('work_slice_planning_points','work_unit_executions')", [], |row| row.get(0)).unwrap(), 0);
        drop(connection);
        // Deterministic semantic state-machine: semantic outcome, matching terminal observation,
        // delivery, sole Epic authorization, same-session continuation, and reevaluation.
        fixture.runtime.finish(&runner.runner_invocation_id, AgentInvocationTerminalStatus::Completed);
        let outcome = crate::orchestration::sprint_runner_transition::PreStartOutcome { forecast_and_concerns: "forecast A".into(), material_uncertainty: "uncertainty A".into(), application_owned_prerequisite: "Epic authorization".into() };
        service.record_pre_start_outcome(&AgentInvocationId::new(request.sprint_runner_invocation_id.clone()).unwrap(), outcome.clone()).unwrap();
        assert!(matches!(service.record_pre_start_outcome(&AgentInvocationId::new(request.sprint_runner_invocation_id.clone()).unwrap(), crate::orchestration::sprint_runner_transition::PreStartOutcome { forecast_and_concerns: "different".into(), material_uncertainty: "uncertainty A".into(), application_owned_prerequisite: "Epic authorization".into() }), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        fixture.runtime.finish(&request.sprint_runner_invocation_id, AgentInvocationTerminalStatus::Completed);
        service.reconcile_startup().unwrap();
        let accepted=service.query().unwrap().transitions.into_iter().next().unwrap();
        assert!(accepted.accepted, "{accepted:?}");
        assert_eq!(fixture.runtime.requests().len(),4);
        let delivery=&fixture.runtime.requests()[3];assert!(delivery.submitted_text.contains("forecast A"));assert!(delivery.submitted_text.contains("uncertainty A"));assert!(delivery.submitted_text.contains("Epic authorization"));
        assert!(matches!(service.start_selected_sprint(&AgentInvocationId::new("wrong-epic-continuation").unwrap()),Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)));
        let barrier=Arc::new(Barrier::new(2));let start_calls=(0..2).map(|_|{let service=service.clone();let barrier=barrier.clone();let invocation=accepted.epic_continuation_invocation_id.clone().unwrap();std::thread::spawn(move||{barrier.wait();service.start_selected_sprint(&AgentInvocationId::new(invocation).unwrap())})}).collect::<Vec<_>>();let start_results=start_calls.into_iter().map(|call|call.join().unwrap()).collect::<Vec<_>>();assert!(start_results.iter().all(Result::is_ok),"{start_results:?}");
        let started=service.query().unwrap().transitions.into_iter().next().unwrap();assert_eq!(started.sprint_runner_session_id,request.sprint_runner_session_id);assert!(started.sprint_continuation_launch_accepted_at.is_some());assert_eq!(fixture.runtime.requests().len(),5);
        let reevaluation=crate::orchestration::sprint_runner_transition::StartedReevaluation{repository_branch_evaluation:"branch is clean".into(),started_forecast_and_concerns:"started concern".into()};service.record_started_reevaluation(&AgentInvocationId::new(started.sprint_continuation_invocation_id.clone().unwrap()).unwrap(),reevaluation.clone()).unwrap();assert!(matches!(service.record_started_reevaluation(&AgentInvocationId::new(started.sprint_continuation_invocation_id.unwrap()).unwrap(),crate::orchestration::sprint_runner_transition::StartedReevaluation{repository_branch_evaluation:"different branch".into(),started_forecast_and_concerns:"started concern".into()}),Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));let final_state=service.query().unwrap().transitions.into_iter().next().unwrap();assert!(final_state.planning_ready_at.is_none());assert!(final_state.downstream_not_started);
    }

    #[test]
    fn historical_epic_runner_v2_binding_is_not_relabelled_when_it_uses_its_existing_request_route() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &bootstrap.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let runner = fixture.status();
        let sprint_id: String = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row(
                "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        Connection::open(&fixture.database_path)
            .unwrap()
            .execute(
                "UPDATE epic_bootstrap_transitions SET runner_harness_version=2 WHERE initiation_id=?1",
                [&bootstrap.initiation_id],
            )
            .unwrap();
        let sprint = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        )
        .unwrap();
        sprint
            .request_next_sprint_runner(
                &AgentInvocationId::new(runner.runner_invocation_id).unwrap(),
                crate::orchestration::sprint_runner_transition::SprintRunnerSelection { sprint_id },
            )
            .unwrap();
        assert_eq!(
            Connection::open(&fixture.database_path)
                .unwrap()
                .query_row::<i64, _, _>(
                    "SELECT runner_harness_version FROM epic_bootstrap_transitions WHERE initiation_id=?1",
                    [&bootstrap.initiation_id],
                    |row| row.get(0),
                )
                .unwrap(),
            2,
        );
        assert_eq!(
            crate::orchestration::conversation_harness::profile(
                crate::orchestration::conversation_harness::ConversationHarnessRole::EpicRunner,
            )
            .unwrap()
            .version,
            3,
        );
    }

    #[test]
    fn reconciliation_drain_accepts_semantic_lifecycle_overlap_without_manual_replay() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture.service.complete_bootstrap(&AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(), Fixture::materials()).unwrap();
        fixture.runtime.finish(&bootstrap.bootstrap_invocation_id, AgentInvocationTerminalStatus::Completed);
        let runner = fixture.status();
        let sprint_id: String = Connection::open(&fixture.database_path).unwrap().query_row("SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1", [], |row| row.get(0)).unwrap();
        let sprint = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        fixture.notifier.set_sprint(&sprint);
        let request = sprint.request_next_sprint_runner(&AgentInvocationId::new(runner.runner_invocation_id.clone()).unwrap(), crate::orchestration::sprint_runner_transition::SprintRunnerSelection { sprint_id }).unwrap();
        // A continuation is legal for this overlap; only the Sprint semantic/lifecycle facts race.
        fixture.runtime.finish(&runner.runner_invocation_id, AgentInvocationTerminalStatus::Completed);
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let used = Arc::new(AtomicUsize::new(0));
        sprint.set_test_reconcile_snapshot_hook(Arc::new({
            let entered = entered.clone(); let release = release.clone(); let used = used.clone();
            move || if used.fetch_add(1, Ordering::SeqCst) == 0 { entered.wait(); release.wait(); }
        }));
        let outcome_service = sprint.clone();
        let outcome_id = request.sprint_runner_invocation_id.clone();
        let outcome = std::thread::spawn(move || outcome_service.record_pre_start_outcome(&AgentInvocationId::new(outcome_id).unwrap(), crate::orchestration::sprint_runner_transition::PreStartOutcome { forecast_and_concerns: "overlap forecast".into(), material_uncertainty: "overlap uncertainty".into(), application_owned_prerequisite: "overlap prerequisite".into() }));
        entered.wait();
        // This terminal travels through AgentSessionApplication's notifier while the elected
        // semantic pass still holds its stale snapshot. It must set a drain generation, not drop.
        fixture.runtime.finish(&request.sprint_runner_invocation_id, AgentInvocationTerminalStatus::Completed);
        release.wait();
        outcome.join().unwrap().unwrap();
        let accepted = sprint.query().unwrap().transitions.remove(0);
        assert!(accepted.accepted);
        assert!(accepted.epic_continuation_launch_accepted_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), 4);
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('work_slice_planning_points','work_unit_executions')", [], |row| row.get(0)).unwrap(), 0);
    }

    #[test]
    fn reconciliation_drain_unblocks_origin_terminal_overlap_without_manual_replay() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture.service.complete_bootstrap(&AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(), Fixture::materials()).unwrap();
        fixture.runtime.finish(&bootstrap.bootstrap_invocation_id, AgentInvocationTerminalStatus::Completed);
        let runner = fixture.status();
        let sprint_id: String = Connection::open(&fixture.database_path).unwrap().query_row("SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1", [], |row| row.get(0)).unwrap();
        let sprint = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        fixture.notifier.set_sprint(&sprint);
        let request = sprint.request_next_sprint_runner(&AgentInvocationId::new(runner.runner_invocation_id.clone()).unwrap(), crate::orchestration::sprint_runner_transition::SprintRunnerSelection { sprint_id }).unwrap();
        let input = crate::orchestration::sprint_runner_transition::PreStartOutcome { forecast_and_concerns: "origin overlap forecast".into(), material_uncertainty: "origin overlap uncertainty".into(), application_owned_prerequisite: "origin overlap prerequisite".into() };
        sprint.record_pre_start_outcome(&AgentInvocationId::new(request.sprint_runner_invocation_id.clone()).unwrap(), input.clone()).unwrap();
        fixture.runtime.finish(&request.sprint_runner_invocation_id, AgentInvocationTerminalStatus::Completed);
        assert!(sprint.query().unwrap().transitions[0].accepted);
        assert_eq!(fixture.runtime.requests().len(), 3);
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let used = Arc::new(AtomicUsize::new(0));
        sprint.set_test_origin_snapshot_hook(Arc::new({
            let entered = entered.clone(); let release = release.clone(); let used = used.clone();
            move || if used.fetch_add(1, Ordering::SeqCst) == 0 { entered.wait(); release.wait(); }
        }));
        // A duplicate semantic action elects the first reconciliation pass and pauses after it
        // snapshots origin-active. The origin terminal notifier must make that pass drain again.
        let replay_service = sprint.clone();
        let replay_id = request.sprint_runner_invocation_id.clone();
        let replay = std::thread::spawn(move || replay_service.record_pre_start_outcome(&AgentInvocationId::new(replay_id).unwrap(), input));
        entered.wait();
        fixture.runtime.finish(&runner.runner_invocation_id, AgentInvocationTerminalStatus::Completed);
        release.wait();
        replay.join().unwrap().unwrap();
        let launched = sprint.query().unwrap().transitions.remove(0);
        assert!(launched.epic_continuation_launch_accepted_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), 4);
        sprint.reconcile_startup().unwrap();
        assert_eq!(fixture.runtime.requests().len(), 4);
    }

    #[test]
    fn accepted_outcome_defers_epic_continuation_until_origin_terminal_notifier_and_restart_recovery() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &bootstrap.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let runner = fixture.status();
        let sprint_id: String = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row(
                "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let sprint = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        )
        .unwrap();
        fixture.notifier.set_sprint(&sprint);
        let request = sprint
            .request_next_sprint_runner(
                &AgentInvocationId::new(runner.runner_invocation_id.clone()).unwrap(),
                crate::orchestration::sprint_runner_transition::SprintRunnerSelection {
                    sprint_id: sprint_id.clone(),
                },
            )
            .unwrap();
        sprint
            .record_pre_start_outcome(
                &AgentInvocationId::new(request.sprint_runner_invocation_id.clone()).unwrap(),
                crate::orchestration::sprint_runner_transition::PreStartOutcome {
                    forecast_and_concerns: "deferred forecast".into(),
                    material_uncertainty: "deferred uncertainty".into(),
                    application_owned_prerequisite: "deferred prerequisite".into(),
                },
            )
            .unwrap();
        // The productive notifier observes the matching Sprint terminal while the original Epic
        // Runner is still active. Acceptance and intent persist, but no illegal second Session
        // invocation is attempted.
        fixture.runtime.finish(
            &request.sprint_runner_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let deferred = sprint.query().unwrap().transitions.remove(0);
        assert!(deferred.accepted);
        assert!(deferred.parent_continuation_delivery_requested_at.is_some());
        assert!(deferred.parent_continuation_delivery_persisted_at.is_none());
        assert!(deferred.epic_continuation_launch_accepted_at.is_none());
        assert_eq!(fixture.runtime.requests().len(), 3);

        // Production-equivalent reopen before the origin terminal retains only the durable
        // deferred state. The next terminal arrives through AgentSessionApplication's notifier.
        let reopened = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        )
        .unwrap();
        fixture.notifier.set_sprint(&reopened);
        reopened.reconcile_startup().unwrap();
        assert_eq!(fixture.runtime.requests().len(), 3);
        fixture.runtime.finish(
            &runner.runner_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let delivered = reopened.query().unwrap().transitions.remove(0);
        assert!(delivered.epic_continuation_launch_accepted_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), 4);
        let continuation = &fixture.runtime.requests()[3];
        assert!(continuation.submitted_text.contains("deferred forecast"));
        assert!(continuation.submitted_text.contains("deferred uncertainty"));
        assert!(continuation.submitted_text.contains("deferred prerequisite"));
        reopened.reconcile_startup().unwrap();
        let after_terminal = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        )
        .unwrap();
        after_terminal.reconcile_startup().unwrap();
        assert_eq!(fixture.runtime.requests().len(), 4);
        assert_eq!(
            Connection::open(&fixture.database_path)
                .unwrap()
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('work_slice_planning_points','work_unit_executions')",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            0,
        );
    }

    #[test]
    fn synchronous_agent_session_notifier_reentry_is_safe_for_each_sprint_transition_launch() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &bootstrap.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let runner = fixture.status();
        let sprint_id: String = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row(
                "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let sprint = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        )
        .unwrap();
        fixture.notifier.set_sprint(&sprint);

        // The fresh v2 pre-start invocation terminal is synchronously persisted and routed back
        // through AgentSessionApplication before its launch call returns. No semantic outcome yet.
        fixture.runtime.terminal_on_next_start();
        let request = sprint
            .request_next_sprint_runner(
                &AgentInvocationId::new(runner.runner_invocation_id.clone()).unwrap(),
                crate::orchestration::sprint_runner_transition::SprintRunnerSelection {
                    sprint_id: sprint_id.clone(),
                },
            )
            .unwrap();
        let outcome_only = sprint.query().unwrap().transitions.remove(0);
        assert!(outcome_only.pre_start_lifecycle_observed_at.is_some());
        assert!(!outcome_only.accepted);
        assert_eq!(fixture.runtime.requests().len(), 3);
        // The originating Epic invocation is terminal before its fresh continuation is sent.
        fixture.runtime.finish(
            &runner.runner_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );

        // The accepted-outcome delivery also completes synchronously through the real notifier.
        fixture.runtime.terminal_on_next_start();
        sprint
            .record_pre_start_outcome(
                &AgentInvocationId::new(request.sprint_runner_invocation_id.clone()).unwrap(),
                crate::orchestration::sprint_runner_transition::PreStartOutcome {
                    forecast_and_concerns: "notifier forecast".into(),
                    material_uncertainty: "notifier uncertainty".into(),
                    application_owned_prerequisite: "notifier prerequisite".into(),
                },
            )
            .unwrap();
        let accepted = sprint.query().unwrap().transitions.remove(0);
        assert!(accepted.accepted);
        let epic = accepted.epic_continuation_invocation_id.clone().unwrap();
        assert!(accepted.epic_continuation_launch_accepted_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), 4);

        // The same path is re-entered for the started Sprint continuation/reevaluation invocation.
        fixture.runtime.terminal_on_next_start();
        sprint
            .start_selected_sprint(&AgentInvocationId::new(epic.clone()).unwrap())
            .unwrap();
        let started = sprint.query().unwrap().transitions.remove(0);
        let continuation = started.sprint_continuation_invocation_id.clone().unwrap();
        assert!(started.sprint_start_persisted_at.is_some());
        assert!(started.sprint_continuation_launch_accepted_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), 5);

        // All three terminals were observed through the notifier and reopen/replay creates no
        // new message or invocation. Stable IDs remain the recorded values.
        let sprint_session = AgentSessionId::new(request.sprint_runner_session_id).unwrap();
        let history = fixture.sessions.load_session(&sprint_session).unwrap();
        for id in [&request.sprint_runner_invocation_id, &continuation] {
            assert!(history
                .invocations
                .iter()
                .find(|candidate| candidate.invocation.id.as_str() == id)
                .is_some_and(|candidate| candidate.invocation.status.is_terminal()));
        }
        let epic_session = AgentSessionId::new(runner.runner_session_id).unwrap();
        assert!(fixture
            .sessions
            .load_session(&epic_session)
            .unwrap()
            .invocations
            .iter()
            .find(|candidate| candidate.invocation.id.as_str() == epic)
            .is_some_and(|candidate| candidate.invocation.status.is_terminal()));
        let reopened = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        )
        .unwrap();
        reopened.reconcile_startup().unwrap();
        assert_eq!(fixture.runtime.requests().len(), 5);
    }

    #[test]
    fn v1_pre_start_upgrade_recovers_only_matching_v2_terminal_and_starts_once() {
        let fixture=Fixture::new();let bootstrap=fixture.status();fixture.service.complete_bootstrap(&AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),Fixture::materials()).unwrap();fixture.runtime.finish(&bootstrap.bootstrap_invocation_id,AgentInvocationTerminalStatus::Completed);let runner=fixture.status();let sprint_id:String=Connection::open(&fixture.database_path).unwrap().query_row("SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",[],|row|row.get(0)).unwrap();let service=crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path,fixture.sessions.clone()).unwrap();let initial=service.request_next_sprint_runner(&AgentInvocationId::new(runner.runner_invocation_id.clone()).unwrap(),crate::orchestration::sprint_runner_transition::SprintRunnerSelection{sprint_id:sprint_id.clone()}).unwrap();assert_eq!(fixture.runtime.requests().len(),3);
        // Simulate a persisted historical v1 record. Its identity is retained after terminal.
        Connection::open(&fixture.database_path).unwrap().execute("UPDATE sprint_runner_transitions SET sprint_runner_harness_version=1 WHERE sprint_id=?1",[&sprint_id]).unwrap();fixture.runtime.finish(&runner.runner_invocation_id,AgentInvocationTerminalStatus::Completed);fixture.runtime.finish(&initial.sprint_runner_invocation_id,AgentInvocationTerminalStatus::Completed);service.reconcile_startup().unwrap();let connection=Connection::open(&fixture.database_path).unwrap();let upgrade:String=connection.query_row("SELECT pre_start_upgrade_invocation_id FROM sprint_runner_transitions WHERE sprint_id=?1",[&sprint_id],|row|row.get(0)).unwrap();assert_eq!(connection.query_row::<u16,_,_>("SELECT sprint_runner_harness_version FROM sprint_runner_transitions WHERE sprint_id=?1",[&sprint_id],|row|row.get(0)).unwrap(),1);drop(connection);assert_eq!(fixture.runtime.requests().len(),4);
        service.record_pre_start_outcome(&AgentInvocationId::new(upgrade.clone()).unwrap(),crate::orchestration::sprint_runner_transition::PreStartOutcome{forecast_and_concerns:"v2 forecast".into(),material_uncertainty:"v2 uncertainty".into(),application_owned_prerequisite:"v2 prerequisite".into()}).unwrap();assert!(!service.query().unwrap().transitions[0].accepted);assert_eq!(fixture.runtime.requests().len(),4);
        // No transition notifier receives this terminal. Production-equivalent reopen must recover
        // the v2 invocation from durable history and deliver the correlated outcome exactly once.
        fixture.runtime.finish(&upgrade,AgentInvocationTerminalStatus::Completed);let reopened=crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path,fixture.sessions.clone()).unwrap();reopened.reconcile_startup().unwrap();let accepted=reopened.query().unwrap().transitions[0].clone();assert!(accepted.accepted);assert_eq!(fixture.runtime.requests().len(),5);let delivery=&fixture.runtime.requests()[4];assert!(delivery.submitted_text.contains("v2 forecast"));assert!(delivery.submitted_text.contains("v2 uncertainty"));assert!(delivery.submitted_text.contains("v2 prerequisite"));reopened.reconcile_startup().unwrap();assert_eq!(fixture.runtime.requests().len(),5);
        let continuation=accepted.epic_continuation_invocation_id.clone().unwrap();let harness=conversation_harness::profile(ConversationHarnessRole::EpicRunner).unwrap();Connection::open(&fixture.database_path).unwrap().execute("UPDATE sprint_runner_transitions SET epic_continuation_harness_version=?2 WHERE sprint_id=?1",params![&sprint_id,harness.version+1]).unwrap();assert!(matches!(reopened.start_selected_sprint(&AgentInvocationId::new(continuation.clone()).unwrap()),Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)));Connection::open(&fixture.database_path).unwrap().execute("UPDATE sprint_runner_transitions SET epic_continuation_harness_version=?2 WHERE sprint_id=?1",params![&sprint_id,harness.version]).unwrap();let barrier=Arc::new(Barrier::new(2));let starts=(0..2).map(|_|{let service=reopened.clone();let barrier=barrier.clone();let continuation=continuation.clone();std::thread::spawn(move||{barrier.wait();service.start_selected_sprint(&AgentInvocationId::new(continuation).unwrap())})}).collect::<Vec<_>>();assert!(starts.into_iter().all(|call|call.join().unwrap().is_ok()));assert_eq!(fixture.runtime.requests().len(),6);let conn=Connection::open(&fixture.database_path).unwrap();assert_eq!(conn.query_row::<i64,_,_>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('work_slice_planning_points','work_unit_executions')",[],|row|row.get(0)).unwrap(),0);
    }

    #[test]
    fn sprint_runner_launch_non_acceptance_stays_unready_and_does_not_relaunch_on_restart() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &bootstrap.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let sprint_id: String = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row(
                "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let service =
            crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &fixture.database_path,
                fixture.sessions.clone(),
            )
            .unwrap();
        fixture.runtime.fail_next_launch();
        let status = service
            .request_next_sprint_runner(
                &AgentInvocationId::new(bootstrap.runner_invocation_id).unwrap(),
                crate::orchestration::sprint_runner_transition::SprintRunnerSelection { sprint_id },
            )
            .unwrap();
        assert!(status.harness_applied_at.is_some());
        assert!(status.launch_accepted_at.is_none());
        assert!(!status.pre_start_ready);
        assert_eq!(fixture.runtime.requests().len(), 3);
        let reopened =
            crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &fixture.database_path,
                fixture.sessions.clone(),
            )
            .unwrap();
        assert_eq!(reopened.reconcile_startup().unwrap(), 1);
        assert_eq!(fixture.runtime.requests().len(), 3);
    }

    #[test]
    fn reopened_production_sequence_recovers_authorized_sprint_runner_once() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &bootstrap.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let runner = fixture.status();
        assert!(runner.runner_launched_at.is_some());

        let (sprint_id, later_sprint_id): (String, String) = {
            let connection = Connection::open(&fixture.database_path).unwrap();
            let mut statement = connection
                .prepare("SELECT id FROM initiated_sprints ORDER BY ordinal")
                .unwrap();
            let mut sprints = statement
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap();
            (
                sprints.next().unwrap().unwrap(),
                sprints.next().unwrap().unwrap(),
            )
        };
        let stable_id = |prefix: &str| {
            let mut hash = Sha256::new();
            hash.update(prefix.as_bytes());
            hash.update([0]);
            hash.update(sprint_id.as_bytes());
            format!("{prefix}-{:x}", hash.finalize())
        };
        let request_id = stable_id("sprint-runner-request");
        let session_id = stable_id("sprint-runner-session");
        let invocation_id = stable_id("sprint-runner-invocation");
        let epic_harness =
            conversation_harness::profile(ConversationHarnessRole::EpicRunner).unwrap();
        let sprint_harness =
            conversation_harness::profile(ConversationHarnessRole::SprintRunner).unwrap();
        let now = Utc::now().to_rfc3339();
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection.execute(
            "INSERT INTO sprint_runner_transitions (sprint_id,epic_id,request_id,epic_runner_session_id,epic_runner_invocation_id,epic_runner_harness_key,epic_runner_harness_version,sprint_runner_harness_key,sprint_runner_harness_version,sprint_runner_session_id,sprint_runner_invocation_id,requested_at,authorized_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12)",
            params![&sprint_id, runner.epic_id, &request_id, runner.runner_session_id, runner.runner_invocation_id, epic_harness.key, epic_harness.version, sprint_harness.key, sprint_harness.version, &session_id, &invocation_id, now],
        )
        .unwrap();
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM agent_sessions WHERE id=?1",
                    [&session_id],
                    |row| row.get(0),
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM agent_session_invocations WHERE id=?1",
                    [&invocation_id],
                    |row| row.get(0),
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1",
                    [&invocation_id],
                    |row| row.get(0),
                )
                .unwrap(),
            0
        );
        drop(connection);

        fixture.service.shutdown();
        let agent_repository = Arc::new(
            SqliteAgentSessionRepository::new(Connection::open(&fixture.database_path).unwrap())
                .unwrap(),
        );
        let runtime = Arc::new(RecordedRuntime::default());
        let notifier = Arc::new(TransitionNotifier::default());
        let sessions = Arc::new(AgentSessionApplication::new(
            agent_repository,
            runtime.clone(),
            notifier.clone(),
            Arc::new(SystemAgentSessionProviders),
            Arc::new(SystemAgentSessionProviders),
            Some("recorded-runtime".into()),
        ));
        sessions.reconcile_startup().unwrap();
        let transition_repository =
            Arc::new(SqliteBootstrapTransitionRepository::open(&fixture.database_path).unwrap());
        let bootstrap = PostConfirmationTransitionService::with_factory(
            transition_repository,
            sessions.clone(),
            fixture.material_root.clone(),
            Arc::new(RecordedBootstrapFactory::default()),
        );
        let sprint_runners =
            crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &fixture.database_path,
                sessions,
            )
            .unwrap();
        bootstrap
            .attach_sprint_runner_transition(sprint_runners.clone())
            .unwrap();
        notifier.set(&bootstrap);

        bootstrap.reconcile_startup().unwrap();
        assert_eq!(sprint_runners.reconcile_startup().unwrap(), 1);
        let recovered = sprint_runners.query().unwrap().transitions;
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].sprint_id, sprint_id);
        assert_eq!(recovered[0].sprint_runner_session_id, session_id);
        assert_eq!(recovered[0].sprint_runner_invocation_id, invocation_id);
        assert!(recovered[0].pre_start_ready);
        assert!(!recovered[0].lifecycle_observed);
        assert!(!recovered[0].accepted);
        assert_eq!(runtime.requests().len(), 1);
        let request = &runtime.requests()[0];
        assert_eq!(request.options, sprint_harness.runtime_options());
        assert!(request
            .submitted_text
            .contains("Sprint Runner for one application-authorized Sprint"));
        assert!(request
            .launch_extension
            .as_ref()
            .unwrap()
            .additional_args
            .iter()
            .any(|argument| argument == "approval_policy=\"never\""));

        bootstrap.reconcile_startup().unwrap();
        assert_eq!(sprint_runners.reconcile_startup().unwrap(), 1);
        assert_eq!(runtime.requests().len(), 1);
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM agent_sessions WHERE id=?1",
                    [&session_id],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM agent_session_invocations WHERE id=?1",
                    [&invocation_id],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1",
                    [&invocation_id],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<String, _, _>(
                    "SELECT sprint_runner_harness_key FROM sprint_runner_transitions WHERE sprint_id=?1",
                    [&sprint_id],
                    |row| row.get(0),
                )
                .unwrap(),
            sprint_harness.key
        );
        assert_eq!(
            connection
                .query_row::<u16, _, _>(
                    "SELECT sprint_runner_harness_version FROM sprint_runner_transitions WHERE sprint_id=?1",
                    [&sprint_id],
                    |row| row.get::<_, u16>(0),
                )
                .unwrap(),
            sprint_harness.version
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM sprint_runner_transitions WHERE sprint_id=?1",
                    [&later_sprint_id],
                    |row| row.get(0),
                )
                .unwrap(),
            0
        );
        // A pre-existing v1 record is never relabelled. Startup advances it through a fresh v2
        // invocation in the same Session and records that new applied Harness separately.
        connection.execute("UPDATE sprint_runner_transitions SET sprint_runner_harness_version=1 WHERE sprint_id=?1", [&sprint_id]).unwrap();
        drop(connection);
        runtime.finish(&invocation_id, AgentInvocationTerminalStatus::Completed);
        let upgraded = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, bootstrap.sessions.clone()).unwrap();
        assert_eq!(upgraded.reconcile_startup().unwrap(), 1);
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(connection.query_row::<u16,_,_>("SELECT sprint_runner_harness_version FROM sprint_runner_transitions WHERE sprint_id=?1",[&sprint_id],|row|row.get(0)).unwrap(),1);
        assert_eq!(connection.query_row::<u16,_,_>("SELECT pre_start_upgrade_harness_version FROM sprint_runner_transitions WHERE sprint_id=?1",[&sprint_id],|row|row.get(0)).unwrap(),sprint_harness.version);
        assert_eq!(connection.query_row::<String,_,_>("SELECT sprint_runner_session_id FROM sprint_runner_transitions WHERE sprint_id=?1",[&sprint_id],|row|row.get(0)).unwrap(),session_id);
        assert_eq!(runtime.requests().len(),2);
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('work_slice_planning_points','work_unit_executions')",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn concurrent_sprint_runner_requests_replay_one_durable_route_and_launch() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &bootstrap.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let runner = fixture.status();
        let sprint_id: String = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row(
                "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let service =
            crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &fixture.database_path,
                fixture.sessions.clone(),
            )
            .unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let calls = (0..2)
            .map(|_| {
                let service = service.clone();
                let barrier = barrier.clone();
                let sprint_id = sprint_id.clone();
                let runner = runner.runner_invocation_id.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    service.request_next_sprint_runner(
                        &AgentInvocationId::new(runner).unwrap(),
                        crate::orchestration::sprint_runner_transition::SprintRunnerSelection {
                            sprint_id,
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        let results = calls
            .into_iter()
            .map(|call| call.join().unwrap().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results[0].request_id, results[1].request_id);
        assert_eq!(
            results[0].sprint_runner_session_id,
            results[1].sprint_runner_session_id
        );
        assert_eq!(
            results[0].sprint_runner_invocation_id,
            results[1].sprint_runner_invocation_id
        );
        assert!(results.iter().all(|result| result.pre_start_ready), "{results:?}");
        assert_eq!(fixture.runtime.requests().len(), 3);
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM sprint_runner_transitions",
                    [],
                    |row| row.get(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_session_invocation_launch_acceptances WHERE invocation_id LIKE 'sprint-runner-invocation-%'", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('work_slice_planning_points','work_unit_executions')", [], |row| row.get(0)).unwrap(), 0);
    }

    #[test]
    fn bootstrap_launch_stage_requires_durable_launch_acceptance() {
        let bootstrap = Fixture::unstarted();
        bootstrap.runtime.fail_next_launch();
        bootstrap
            .service
            .on_initiation_persisted(&bootstrap.initiation_id)
            .unwrap();
        let failed_bootstrap = bootstrap.status();
        assert!(failed_bootstrap.bootstrap_launched_at.is_none());
        assert_eq!(
            failed_bootstrap.bootstrap_lifecycle_status.as_deref(),
            Some("failed")
        );
        assert_eq!(failed_bootstrap.retry_state, "blocked");
        assert!(failed_bootstrap.runner_session_created_at.is_none());
        assert!(failed_bootstrap.runner_launched_at.is_none());
    }

    #[test]
    fn runner_launch_stage_requires_durable_launch_acceptance() {
        let mut runner = Fixture::new();
        let bootstrap_status = runner.status();
        runner
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(bootstrap_status.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        runner.runtime.finish(
            &bootstrap_status.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let launched = runner.status();
        assert!(launched.runner_launched_at.is_some());
        let connection = Connection::open(&runner.database_path).unwrap();
        connection
            .execute(
                "DELETE FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1",
                params![launched.runner_invocation_id],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE epic_bootstrap_transitions SET runner_launched_at=NULL WHERE initiation_id=?1",
                params![runner.initiation_id],
            )
            .unwrap();
        drop(connection);

        runner.reopen_service();
        let unaccepted = runner.status();
        assert!(unaccepted.material_accepted_at.is_some());
        assert!(unaccepted.runner_session_created_at.is_some());
        assert!(unaccepted.runner_launched_at.is_none());
        assert_eq!(runner.runtime.requests().len(), 2);
        runner.service.reconcile_startup().unwrap();
        assert_eq!(runner.runtime.requests().len(), 2);
        assert!(runner.status().runner_launched_at.is_none());
    }

    #[test]
    fn lifecycle_first_waits_for_semantic_fact_and_duplicate_delivery_is_idempotent() {
        let fixture = Fixture::new();
        let initial = fixture.status();
        fixture.runtime.finish(
            &initial.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        assert_eq!(
            fixture.runtime.requests().len(),
            1,
            "lifecycle-only must not launch Runner"
        );
        let invocation = AgentInvocationId::new(initial.bootstrap_invocation_id).unwrap();
        let first = fixture
            .service
            .complete_bootstrap(&invocation, Fixture::materials())
            .unwrap();
        let replay = fixture
            .service
            .complete_bootstrap(&invocation, Fixture::materials())
            .unwrap();
        assert!(!first.idempotent_replay);
        assert!(replay.idempotent_replay);
        assert_eq!(first.fact_id, replay.fact_id);
        assert_eq!(fixture.runtime.requests().len(), 2);
        assert_eq!(fixture.factory.starts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn semantic_boundary_rejects_stale_invalid_conflicting_and_path_escape_calls() {
        let fixture = Fixture::new();
        let status = fixture.status();
        let foreign = AgentInvocationId::new("foreign-bootstrap-invocation").unwrap();
        assert_eq!(
            fixture
                .service
                .complete_bootstrap(&foreign, Fixture::materials()),
            Err(TransitionError::Forbidden)
        );
        let invocation = AgentInvocationId::new(status.bootstrap_invocation_id.clone()).unwrap();
        assert!(matches!(
            fixture.service.complete_bootstrap(
                &invocation,
                BootstrapMaterialInput {
                    epic_overview_markdown: "".into(),
                    runner_brief_markdown: "valid".into()
                }
            ),
            Err(TransitionError::InvalidMaterial(_))
        ));
        let first = fixture
            .service
            .complete_bootstrap(&invocation, Fixture::materials())
            .unwrap();
        let mut changed = Fixture::materials();
        changed.runner_brief_markdown.push_str("different");
        assert_eq!(
            fixture.service.complete_bootstrap(&invocation, changed),
            Err(TransitionError::IdempotencyConflict)
        );
        assert!(!first.idempotent_replay);

        let escaped_fixture = Fixture::new();
        let escaped_status = escaped_fixture.status();
        let escaped_invocation =
            AgentInvocationId::new(escaped_status.bootstrap_invocation_id).unwrap();
        let connection = Connection::open(&escaped_fixture.database_path).unwrap();
        let escaped = escaped_fixture.material_root.join("escaped.md");
        connection
            .execute(
                "UPDATE epic_bootstrap_transitions SET overview_path=?2 WHERE initiation_id=?1",
                params![escaped_fixture.initiation_id, escaped.to_string_lossy()],
            )
            .unwrap();
        drop(connection);
        assert!(matches!(
            escaped_fixture
                .service
                .complete_bootstrap(&escaped_invocation, Fixture::materials()),
            Err(TransitionError::IdentityMismatch(_))
        ));
        assert!(!escaped.exists());
    }

    #[test]
    fn restart_reverifies_prepared_bytes_and_fails_closed_on_identity_mismatch() {
        let fixture = Fixture::new();
        let status = fixture.status();
        fs::write(&status.approved_plan_path, b"tampered").unwrap();
        assert!(matches!(
            fixture.service.reconcile_startup(),
            Err(TransitionError::IdentityMismatch(message)) if message.contains("different bytes")
        ));
        assert_eq!(fixture.runtime.requests().len(), 1);
    }

    #[test]
    fn unsuccessful_terminal_lifecycle_never_accepts_material_or_launches_runner() {
        let fixture = Fixture::new();
        let status = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(status.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &status.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Failed,
        );
        let failed = fixture.status();
        assert_eq!(failed.bootstrap_lifecycle_status.as_deref(), Some("failed"));
        assert!(failed.material_accepted_at.is_none());
        assert!(failed.runner_launched_at.is_none());
        assert_eq!(fixture.runtime.requests().len(), 1);
    }

    #[test]
    fn recovery_reuses_session_and_invocation_effects_across_pre_and_post_ack_stages() {
        let mut fixture = Fixture::unstarted();
        let failing_repository =
            Arc::new(SqliteBootstrapTransitionRepository::open(&fixture.database_path).unwrap());
        let failing = PostConfirmationTransitionService::with_factory(
            failing_repository,
            fixture.sessions.clone(),
            fixture.material_root.clone(),
            Arc::new(FailingBootstrapFactory),
        );
        fixture.notifier.set(&failing);
        assert!(matches!(
            failing.on_initiation_persisted(&fixture.initiation_id),
            Err(TransitionError::Unavailable(message)) if message.contains("induced bootstrap listener failure")
        ));
        fixture.service = failing;
        let before_launch = fixture.status();
        assert!(before_launch.prepared_at.is_some());
        assert!(before_launch.bootstrap_session_created_at.is_some());
        assert!(before_launch.bootstrap_launched_at.is_none());
        assert!(fixture.runtime.requests().is_empty());

        fixture.reopen_service();
        assert_eq!(fixture.runtime.requests().len(), 1);
        let launched = fixture.status();
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection
            .execute(
                "UPDATE epic_bootstrap_attempts SET launched_at=NULL WHERE id=?1",
                params![launched.current_attempt_id],
            )
            .unwrap();
        drop(connection);
        fixture.reopen_service();
        assert_eq!(
            fixture.runtime.requests().len(),
            1,
            "send acceptance recovery must reuse the invocation"
        );
        assert!(fixture.status().bootstrap_launched_at.is_some());

        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(launched.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture
            .service
            .repository
            .record_lifecycle(&launched.bootstrap_invocation_id, "completed", false)
            .unwrap();
        let runner_harness =
            conversation_harness::profile(ConversationHarnessRole::EpicRunner).unwrap();
        let runner_root =
            conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunner).unwrap();
        fixture
            .sessions
            .create_application_session(CreateApplicationAgentSessionCommand {
                session_id: AgentSessionId::new(launched.runner_session_id.clone()).unwrap(),
                session: CreateAgentSessionCommand {
                    title: Some("Epic Runner: Durable Bootstrap Epic".into()),
                    working_directory: Some(runner_root),
                    requested_options: runner_harness.runtime_options(),
                },
            })
            .unwrap();
        fixture
            .service
            .repository
            .record_stage(&fixture.initiation_id, "runner_session_created_at")
            .unwrap();
        assert_eq!(fixture.runtime.requests().len(), 1);

        fixture.reopen_service();
        assert_eq!(
            fixture.runtime.requests().len(),
            2,
            "Runner created-before-launch must launch once"
        );
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection
            .execute(
                "UPDATE epic_bootstrap_transitions SET runner_launched_at=NULL WHERE initiation_id=?1",
                params![fixture.initiation_id],
            )
            .unwrap();
        drop(connection);
        fixture.reopen_service();
        assert_eq!(
            fixture.runtime.requests().len(),
            2,
            "Runner launch-ack recovery must reuse the invocation"
        );
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM agent_sessions WHERE id IN (?1,?2)",
                    params![launched.bootstrap_session_id, launched.runner_session_id],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM agent_session_invocations WHERE id IN (?1,?2)",
                    params![
                        launched.bootstrap_invocation_id,
                        launched.runner_invocation_id
                    ],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
    }

    #[test]
    fn post_persistence_failure_reconciles_fact_and_lifecycle_without_reapplying_effects() {
        let mut fixture = Fixture::new();
        let status = fixture.status();
        let approved = fs::read(&status.approved_plan_path).unwrap();
        fs::write(
            &status.approved_plan_path,
            b"induced post-persistence mismatch",
        )
        .unwrap();
        assert!(matches!(
            fixture.service.complete_bootstrap(
                &AgentInvocationId::new(status.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            ),
            Err(TransitionError::IdentityMismatch(_))
        ));
        let persisted = fixture.status();
        assert!(persisted.semantic_completion_fact_id.is_some());
        assert!(persisted.material_accepted_at.is_none());
        assert_eq!(fixture.runtime.requests().len(), 1);

        assert!(fixture
            .runtime
            .finish_result(
                &status.bootstrap_invocation_id,
                AgentInvocationTerminalStatus::Completed,
            )
            .is_err());
        assert_eq!(
            fixture.status().bootstrap_lifecycle_status.as_deref(),
            Some("completed")
        );
        fs::write(&status.approved_plan_path, approved).unwrap();
        fixture.reopen_service();
        assert!(fixture.status().material_accepted_at.is_some());
        assert!(fixture.status().runner_launched_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), 2);
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM epic_bootstrap_attempt_completion_facts",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM agent_session_invocations",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
    }

    #[test]
    fn real_startup_reconciliation_interrupts_and_retries_bootstrap_once() {
        let mut fixture = Fixture::new();
        let original = fixture.status();

        assert_eq!(fixture.restart_application(), 1);
        let retry = fixture.status();
        assert_eq!(retry.bootstrap_attempts.len(), 2);
        assert_eq!(
            retry.bootstrap_attempts[0].lifecycle_status.as_deref(),
            Some("interrupted")
        );
        assert_eq!(retry.bootstrap_attempts[0].retry_disposition, "retried");
        assert_eq!(retry.bootstrap_attempts[1].ordinal, 1);
        assert_ne!(
            retry.bootstrap_invocation_id,
            original.bootstrap_invocation_id
        );
        assert_eq!(fixture.all_requests().len(), 2);

        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(retry.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &retry.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let accepted = fixture.status();
        assert_eq!(accepted.accepted_attempt_id, Some(retry.current_attempt_id));
        assert!(accepted.runner_launched_at.is_some());
        assert_eq!(fixture.all_requests().len(), 3);
        assert_eq!(
            fixture
                .all_requests()
                .iter()
                .filter(|request| request.invocation_id.as_str() == accepted.runner_invocation_id)
                .count(),
            1
        );
    }

    #[test]
    fn semantic_fact_before_crash_is_preserved_but_retry_must_supply_its_own_fact() {
        let mut fixture = Fixture::new();
        let original = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(original.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();

        assert_eq!(fixture.restart_application(), 1);
        let retry = fixture.status();
        assert!(retry.bootstrap_attempts[0]
            .semantic_completion_fact_id
            .is_some());
        assert_eq!(
            retry.bootstrap_attempts[0].lifecycle_status.as_deref(),
            Some("interrupted")
        );
        assert!(retry.bootstrap_attempts[1]
            .semantic_completion_fact_id
            .is_none());
        assert!(retry.material_accepted_at.is_none());
        assert!(retry.runner_launched_at.is_none());

        let mut conflicting = Fixture::materials();
        conflicting.epic_overview_markdown.push_str("conflict\n");
        assert!(matches!(
            fixture.service.complete_bootstrap(
                &AgentInvocationId::new(retry.bootstrap_invocation_id.clone()).unwrap(),
                conflicting,
            ),
            Err(TransitionError::IdentityMismatch(_))
        ));

        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(retry.bootstrap_invocation_id.clone()).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.runtime.finish(
            &retry.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let accepted = fixture.status();
        assert_eq!(accepted.accepted_attempt_id, Some(retry.current_attempt_id));
        assert!(accepted.runner_launched_at.is_some());
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM epic_bootstrap_attempt_completion_facts",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            2
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM epic_bootstrap_attempts WHERE accepted_at IS NOT NULL",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn repeated_transition_reconciliation_at_retry_boundary_is_idempotent() {
        let mut fixture = Fixture::new();
        assert_eq!(fixture.restart_application(), 1);
        let retry = fixture.status();
        fixture.service.reconcile_startup().unwrap();
        fixture.service.reconcile_startup().unwrap();
        let repeated = fixture.status();
        assert_eq!(repeated.current_attempt_id, retry.current_attempt_id);
        assert_eq!(repeated.bootstrap_attempts.len(), 2);
        assert_eq!(fixture.all_requests().len(), 2);
    }

    #[test]
    fn ordinary_failed_and_canceled_attempts_are_blocked_without_auto_loop() {
        for terminal in [
            AgentInvocationTerminalStatus::Failed,
            AgentInvocationTerminalStatus::Canceled,
        ] {
            let mut fixture = Fixture::new();
            let initial = fixture.status();
            fixture
                .runtime
                .finish(&initial.bootstrap_invocation_id, terminal);
            fixture.service.reconcile_startup().unwrap();
            fixture.service.reconcile_startup().unwrap();
            assert_eq!(fixture.restart_application(), 0);
            let blocked = fixture.status();
            assert_eq!(blocked.retry_state, "blocked");
            assert_eq!(
                blocked.blocked_reason.as_deref(),
                Some("terminal_without_retry_authority")
            );
            assert_eq!(blocked.bootstrap_attempts.len(), 1);
            assert_eq!(fixture.all_requests().len(), 1);
        }
    }

    #[test]
    fn startup_interruption_retries_stop_at_the_durable_attempt_limit() {
        let mut fixture = Fixture::new();
        assert_eq!(fixture.restart_application(), 1);
        assert_eq!(fixture.restart_application(), 1);
        assert_eq!(fixture.restart_application(), 1);
        let blocked = fixture.status();
        assert_eq!(
            blocked.bootstrap_attempts.len(),
            MAX_BOOTSTRAP_ATTEMPTS as usize
        );
        assert_eq!(blocked.retry_state, "blocked");
        assert_eq!(
            blocked.blocked_reason.as_deref(),
            Some("startup_retry_limit_reached")
        );
        assert_eq!(
            fixture.all_requests().len(),
            MAX_BOOTSTRAP_ATTEMPTS as usize
        );
        fixture.service.reconcile_startup().unwrap();
        assert_eq!(
            fixture.status().bootstrap_attempts.len(),
            MAX_BOOTSTRAP_ATTEMPTS as usize
        );
        assert_eq!(
            fixture.all_requests().len(),
            MAX_BOOTSTRAP_ATTEMPTS as usize
        );
    }

    #[test]
    fn lifecycle_and_semantic_facts_never_mix_across_attempts() {
        let mut fixture = Fixture::new();
        let original = fixture.status();
        fixture
            .service
            .complete_bootstrap(
                &AgentInvocationId::new(original.bootstrap_invocation_id).unwrap(),
                Fixture::materials(),
            )
            .unwrap();
        fixture.restart_application();
        let retry = fixture.status();
        fixture.runtime.finish(
            &retry.bootstrap_invocation_id,
            AgentInvocationTerminalStatus::Completed,
        );
        let blocked = fixture.status();
        assert_eq!(blocked.retry_state, "blocked");
        assert_eq!(
            blocked.blocked_reason.as_deref(),
            Some("completed_without_semantic_fact")
        );
        assert!(blocked.material_accepted_at.is_none());
        assert!(blocked.runner_launched_at.is_none());
        assert_eq!(fixture.all_requests().len(), 2);
    }

    #[test]
    fn existing_output_link_is_rejected_before_read_or_acceptance() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("prepared");
        fs::create_dir(&root).unwrap();
        let target = root.join("epic-overview.md");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let outside = directory.path().join("outside-directory");
            fs::create_dir(&outside).unwrap();
            let output = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&target)
                .arg(&outside)
                .creation_flags(0x08000000)
                .output()
                .expect("create Windows output junction");
            assert!(output.status.success(), "mklink failed: {output:?}");
        }
        #[cfg(unix)]
        {
            let outside = directory.path().join("outside.md");
            fs::write(&outside, b"same bytes").unwrap();
            std::os::unix::fs::symlink(&outside, &target).expect("create output symlink");
        }

        let canonical_root = fs::canonicalize(&root).unwrap();
        assert!(matches!(
            write_exact_contained(&canonical_root, &target, b"same bytes"),
            Err(TransitionError::IdentityMismatch(message))
                if message.contains("symbolic link or reparse point")
        ));
    }

    #[test]
    #[ignore = "paid installed-Codex Bootstrap and Runner proof from isolated confirmed state"]
    fn installed_codex_bootstrap_and_runner_converge_without_starting_a_sprint() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        drop(crate::storage::open_active_database(&database_path).unwrap());
        let now = Utc::now().to_rfc3339();
        let connection = Connection::open(&database_path).unwrap();
        crate::storage::configure_sqlite_connection(&connection).unwrap();
        connection.execute("INSERT INTO agent_sessions (id,title,availability,requested_options_json,created_at,updated_at) VALUES ('live-plan-builder-session','Isolated confirmed test state','available','{}',?1,?1)", params![now]).unwrap();
        drop(connection);

        let orchestration = SqliteOrchestrationRepository::open(&database_path).unwrap();
        let (draft, profile, association) = orchestration
            .bootstrap_managed_plan_builder("live-plan-builder-session")
            .unwrap();
        let saved = orchestration
            .save_epic_plan_proposal(SaveEpicPlanProposalCommand {
                epic_planning_draft_id: draft.clone(),
                capability_profile_id: profile,
                agent_session_association_id: association,
                agent_session_id: "live-plan-builder-session".into(),
                actor_id: "managed-plan-builder".into(),
                expected_revision: None,
                proposal: PlanBuilderProposal {
                    suggested_epic_name: Some("Isolated live transition proof".into()),
                    sprints: vec![ProposedSprint {
                        title: "Future Sprint".into(),
                        intended_movement: "Remain planned and unstarted during this proof.".into(),
                        concern_summaries: vec![
                            "Preserve the production confirmation boundary.".into()
                        ],
                    }],
                },
                idempotency_key: "live-transition-proposal".into(),
            })
            .unwrap();
        let initiated = orchestration
            .initiate_epic(InitiateEpicCommand {
                epic_planning_draft_id: draft,
                expected_revision_token: saved.revision_token,
                actor_id: "application-user".into(),
                idempotency_key: "live-transition-initiation".into(),
            })
            .unwrap();

        let runtime = Arc::new(crate::runtime::codex::CodexCliRuntime::system(
            "codex", None,
        ));
        let notifier = Arc::new(LiveTransitionNotifier::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            Arc::new(SqliteAgentSessionRepository::open(&database_path).unwrap()),
            runtime.clone(),
            notifier.clone(),
            providers.clone(),
            providers,
            None,
        ));
        let service = PostConfirmationTransitionService::new(
            Arc::new(SqliteBootstrapTransitionRepository::open(&database_path).unwrap()),
            sessions,
            directory.path().join("materials"),
        );
        notifier.set(&service);
        service
            .on_initiation_persisted(initiated.initiation_id.as_str())
            .unwrap();
        let terminals = notifier.wait_for_terminals(2);
        assert_eq!(terminals.len(), 2);
        assert!(terminals
            .iter()
            .all(|item| item.status == AgentInvocationStatus::Completed));

        let status = service.query().unwrap().transitions.remove(0);
        assert!(status.prepared_at.is_some());
        assert!(status.bootstrap_launched_at.is_some());
        assert_eq!(
            status.bootstrap_lifecycle_status.as_deref(),
            Some("completed")
        );
        assert!(status.semantic_completion_fact_id.is_some());
        assert!(status.material_accepted_at.is_some());
        assert!(status.runner_session_created_at.is_some());
        assert!(status.runner_launched_at.is_some());
        assert_eq!(status.runner_lifecycle_status.as_deref(), Some("completed"));
        assert_eq!(status.bootstrap_attempts.len(), 1);
        assert_eq!(
            status.accepted_attempt_id.as_deref(),
            Some(status.current_attempt_id.as_str())
        );

        let connection = Connection::open(&database_path).unwrap();
        for (table, expected) in [
            ("epic_bootstrap_attempt_completion_commands", 1),
            ("epic_bootstrap_attempt_completion_results", 1),
            ("epic_bootstrap_attempt_completion_facts", 1),
            ("agent_session_invocation_launch_acceptances", 2),
            ("agent_session_invocations", 2),
            ("epic_initiations", 1),
            ("initiated_sprints", 1),
        ] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, expected, "unexpected live count for {table}");
        }
        let completed_mcp_calls = |invocation_id: &str, tool: &str| -> i64 {
            connection
                .query_row(
                    "SELECT COUNT(*) FROM agent_session_runtime_events WHERE invocation_id=?1 AND json_extract(raw_payload_json,'$.type')='item.completed' AND json_extract(raw_payload_json,'$.item.type')='mcp_tool_call' AND raw_payload_json LIKE ?2",
                    params![invocation_id, format!("%{tool}%")],
                    |row| row.get(0),
                )
                .unwrap()
        };
        assert_eq!(
            completed_mcp_calls(&status.bootstrap_invocation_id, "complete_epic_bootstrap"),
            1
        );
        assert_eq!(
            completed_mcp_calls(&status.runner_invocation_id, "mcp_tool_call"),
            0
        );
        let sprint_sessions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM agent_sessions WHERE title LIKE '%Sprint%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sprint_sessions, 0);
        let inventory: String = connection
            .query_row(
                "SELECT inventory_json FROM epic_bootstrap_attempt_completion_facts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let inventory: Vec<MaterialInventoryItem> = serde_json::from_str(&inventory).unwrap();
        assert_eq!(inventory.len(), 2);
        for item in &inventory {
            let bytes = fs::read(&item.path).unwrap();
            assert_eq!(item.sha256, sha256(&bytes));
            assert_eq!(item.size_bytes, bytes.len() as u64);
        }
        drop(connection);
        eprintln!(
            "live transition: 1 Bootstrap call, 1 accepted inventory, 1 Runner launch, 0 Sprint sessions"
        );
        service.shutdown();
        runtime.shutdown().unwrap();
    }

    #[tokio::test]
    async fn production_mcp_exposes_one_identity_free_semantic_action_and_persists_through_service()
    {
        let fixture = Fixture::new();
        let status = fixture.status();
        let invocation = AgentInvocationId::new(status.bootstrap_invocation_id.clone()).unwrap();
        let server = start_bootstrap_server(
            fixture.service.clone(),
            invocation,
            "bootstrap-bearer".into(),
            vec!["tauri://localhost".into()],
        )
        .unwrap();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        let initialize = serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "protocolVersion":"2025-11-25","capabilities":{},
                "clientInfo":{"name":"test","version":"1"}
            }
        })
        .to_string();
        assert_eq!(
            client
                .post(server.url())
                .header("content-type", "application/json")
                .body(initialize.clone())
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );
        let initialized = client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer bootstrap-bearer")
            .body(initialize)
            .send()
            .await
            .unwrap();
        let session = initialized
            .headers()
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let request = |id: u32, method: &str, params: serde_json::Value| {
            serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string()
        };
        client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer bootstrap-bearer")
            .header("mcp-session-id", &session)
            .body(
                serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"})
                    .to_string(),
            )
            .send()
            .await
            .unwrap();
        let listed = client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer bootstrap-bearer")
            .header("mcp-session-id", &session)
            .body(request(2, "tools/list", serde_json::json!({})))
            .send()
            .await
            .unwrap();
        let listed = mcp_response_json(listed).await;
        let tools = listed["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "complete_epic_bootstrap");
        let schema = &tools[0]["inputSchema"];
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            ["epicOverviewMarkdown", "runnerBriefMarkdown"]
        );
        assert!(!schema.to_string().contains("sessionId"));
        assert!(!schema.to_string().contains("path"));

        let arguments = serde_json::to_value(Fixture::materials()).unwrap();
        let completed = client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer bootstrap-bearer")
            .header("mcp-session-id", &session)
            .body(request(
                3,
                "tools/call",
                serde_json::json!({"name":"complete_epic_bootstrap","arguments":arguments}),
            ))
            .send()
            .await
            .unwrap();
        let completed = mcp_response_json(completed).await;
        assert_eq!(completed["result"]["isError"], false);
        assert!(completed.to_string().contains("semanticCompletionFactId"));
        assert!(fixture.status().semantic_completion_fact_id.is_some());
        assert!(fixture.status().runner_launched_at.is_none());

        let forbidden_shape = client.post(server.url()).header("content-type", "application/json").header("accept", "application/json, text/event-stream").header("authorization", "Bearer bootstrap-bearer").header("mcp-session-id", &session).body(request(4,"tools/call",serde_json::json!({"name":"complete_epic_bootstrap","arguments":{"epicOverviewMarkdown":"x","runnerBriefMarkdown":"y","agentSessionId":"forged"}}))).send().await.unwrap();
        let forbidden_shape = mcp_response_json(forbidden_shape).await;
        assert_eq!(forbidden_shape["result"]["isError"], true);
        server.stop();
    }

    #[tokio::test]
    async fn work_slice_planner_scoped_mcp_is_identity_free_and_transport_bound() {
        let fixture = Fixture::new();
        let (service, invocation, _sprint_id) = fixture.prepare_work_slice_planner();
        let server = crate::orchestration::sprint_runner_transition::start_work_slice_planner_test_server(
            service.clone(),
            invocation.clone(),
            "planner-bearer".into(),
            vec!["tauri://localhost".into()],
        )
        .unwrap();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        let initialize = || {
            serde_json::json!({
                "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                    "protocolVersion":"2025-11-25","capabilities":{},
                    "clientInfo":{"name":"test","version":"1"}
                }
            })
            .to_string()
        };
        assert_eq!(
            client
                .post(server.url())
                .header("content-type", "application/json")
                .header("accept", "application/json, text/event-stream")
                .header("authorization", "Bearer wrong")
                .body(initialize())
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .post(server.url())
                .header("content-type", "application/json")
                .header("accept", "application/json, text/event-stream")
                .header("authorization", "Bearer planner-bearer")
                .header("origin", "https://evil.example")
                .body(initialize())
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::FORBIDDEN
        );
        assert_eq!(
            client
                .post(server.url())
                .header("content-type", "application/json")
                .header("accept", "application/json, text/event-stream")
                .header("authorization", "Bearer planner-bearer")
                .header("host", "evil.example")
                .body(initialize())
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::FORBIDDEN
        );
        let initialized = client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer planner-bearer")
            .body(initialize())
            .send()
            .await
            .unwrap();
        let session = initialized
            .headers()
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let request = |id: u32, method: &str, params: serde_json::Value| {
            serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string()
        };
        client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer planner-bearer")
            .header("mcp-session-id", &session)
            .body(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string())
            .send()
            .await
            .unwrap();
        let listed = client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer planner-bearer")
            .header("mcp-session-id", &session)
            .body(request(2, "tools/list", serde_json::json!({})))
            .send()
            .await
            .unwrap();
        let listed = mcp_response_json(listed).await;
        let tools = listed["result"]["tools"].as_array().unwrap();
        assert_eq!(
            tools.iter().map(|tool| tool["name"].as_str().unwrap()).collect::<Vec<_>>(),
            [
                "complete_work_slice_planning",
                "read_current_planning_context",
                "request_work_slice_refinement",
                "submit_work_slice_proposal",
            ]
        );
        for tool in tools {
            let schema = &tool["inputSchema"];
            assert_ne!(schema["additionalProperties"], true);
            let mut property_names = schema["properties"]
                .as_object()
                .map(|properties| properties.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            property_names.sort();
            let expected_properties = match tool["name"].as_str().unwrap() {
                "read_current_planning_context" | "complete_work_slice_planning" => Vec::new(),
                "request_work_slice_refinement" => vec!["reason".to_string()],
                "submit_work_slice_proposal" => vec!["lanes".to_string(), "objective".to_string()],
                name => panic!("unexpected Work Slice Planner tool {name}"),
            };
            assert_eq!(property_names, expected_properties);
            let schema_text = schema.to_string().to_ascii_lowercase();
            for forbidden in ["session", "invocation", "sprint", "point", "route", "repository", "idempotency", "revision", "token", "acceptance", "authority"] {
                assert!(!schema_text.contains(forbidden), "Planner schema leaked {forbidden}: {schema_text}");
            }
        }
        let proposal_args = serde_json::json!({
            "objective":"Verify the scoped Planner exchange.",
            "lanes":[{"title":"Inspect","specification":"Inspect the exchange.","dependsOn":[]}]
        });
        let malformed = client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer planner-bearer")
            .header("mcp-session-id", &session)
            .body(request(3, "tools/call", serde_json::json!({
                "name":"submit_work_slice_proposal",
                "arguments": {"objective":"x","lanes":[],"revisionToken":"forged","route":"forged","accepted":true}
            })))
            .send()
            .await
            .unwrap();
        let malformed = mcp_response_json(malformed).await;
        assert_eq!(malformed["result"]["isError"], true);
        let submitted = client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer planner-bearer")
            .header("mcp-session-id", &session)
            .body(request(4, "tools/call", serde_json::json!({"name":"submit_work_slice_proposal","arguments":proposal_args.clone()})))
            .send()
            .await
            .unwrap();
        let submitted = mcp_response_json(submitted).await;
        assert_eq!(submitted["result"]["isError"], false);
        let submitted_text = submitted["result"]["content"][0]["text"].as_str().unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(submitted_text).unwrap(),
            serde_json::json!({"status":"proposal_validated","accepted":false,"materializationReady":false})
        );
        for forbidden in ["revision", "fingerprint", "command", "idempotency", "route", "token", "authority"] {
            assert!(!submitted_text.to_ascii_lowercase().contains(forbidden));
        }
        let replay = client
            .post(server.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer planner-bearer")
            .header("mcp-session-id", &session)
            .body(request(5, "tools/call", serde_json::json!({"name":"submit_work_slice_proposal","arguments":proposal_args})))
            .send()
            .await
            .unwrap();
        let replay = mcp_response_json(replay).await;
        let replay_text = replay["result"]["content"][0]["text"].as_str().unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(replay_text).unwrap(),
            serde_json::json!({"status":"proposal_replayed","accepted":false,"materializationReady":false})
        );
        let foreign = crate::orchestration::sprint_runner_transition::start_work_slice_planner_test_server(
            service,
            AgentInvocationId::new("foreign-planner-invocation").unwrap(),
            "foreign-bearer".into(),
            vec!["tauri://localhost".into()],
        )
        .unwrap();
        let foreign_initialized = client
            .post(foreign.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer foreign-bearer")
            .body(initialize())
            .send()
            .await
            .unwrap();
        let foreign_session = foreign_initialized
            .headers()
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        client
            .post(foreign.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer foreign-bearer")
            .header("mcp-session-id", &foreign_session)
            .body(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string())
            .send()
            .await
            .unwrap();
        let foreign_call = client
            .post(foreign.url())
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("authorization", "Bearer foreign-bearer")
            .header("mcp-session-id", &foreign_session)
            .body(request(6, "tools/call", serde_json::json!({"name":"read_current_planning_context","arguments":{}})))
            .send()
            .await
            .unwrap();
        let foreign_call = mcp_response_json(foreign_call).await;
        assert_eq!(foreign_call["result"]["isError"], false);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(foreign_call["result"]["content"][0]["text"].as_str().unwrap()).unwrap(),
            serde_json::json!({"status":"rejected","code":"forbidden"})
        );
        foreign.stop();
        server.stop();
    }

    async fn mcp_response_json(response: reqwest::Response) -> serde_json::Value {
        let text = response.text().await.unwrap();
        let json = text
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .find(|line| !line.trim().is_empty())
            .unwrap_or(&text);
        serde_json::from_str(json)
            .unwrap_or_else(|error| panic!("invalid MCP response {text:?}: {error}"))
    }

    #[test]
    fn work_slice_planning_request_launches_one_prepared_planner_and_marks_readiness() {
        let fixture = Fixture::new();
        let bootstrap = fixture.status();
        fixture.service.complete_bootstrap(
            &AgentInvocationId::new(bootstrap.bootstrap_invocation_id.clone()).unwrap(),
            Fixture::materials(),
        ).unwrap();
        fixture.runtime.finish(&bootstrap.bootstrap_invocation_id, AgentInvocationTerminalStatus::Completed);
        let runner = fixture.status();
        let sprint_id: String = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1", [], |row| row.get(0),
        ).unwrap();
        let service = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path, fixture.sessions.clone(),
        ).unwrap();
        let sprint = service.request_next_sprint_runner(
            &AgentInvocationId::new(runner.runner_invocation_id).unwrap(),
            crate::orchestration::sprint_runner_transition::SprintRunnerSelection { sprint_id: sprint_id.clone() },
        ).unwrap();
        let control = AgentInvocationId::new("planning-control-invocation").unwrap();
        let control_harness = conversation_harness::profile(ConversationHarnessRole::SprintRunnerPlanningControl).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE sprint_runner_transitions SET planning_control_invocation_id=?2,planning_control_harness_key=?3,planning_control_harness_version=?4,planning_control_harness_applied_at=?5,planning_control_launch_accepted_at=?5,planning_ready_at=?5 WHERE sprint_id=?1",
            params![sprint_id, control.as_str(), control_harness.key, control_harness.version, "2026-08-02T00:00:00Z"],
        ).unwrap();

        assert!(matches!(
            service.request_work_slice_planner(&control, crate::orchestration::sprint_runner_transition::WorkSlicePlannerRequest {}),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        let repository_root = fixture._directory.path().join("sprint-repository");
        let worktree_root = repository_root.join("worktree");
        fs::create_dir_all(&worktree_root).unwrap();
        let initial_authority = InitiatedSprintGitAuthorityWrite {
                sprint_id: sprint_id.clone(), idempotency_key: "wsp1-route-authority".into(),
                repository_id: "wsp1-repository".into(), repository_root: repository_root.to_string_lossy().into_owned(),
                repository_common_dir: repository_root.to_string_lossy().into_owned(), worktree_id: "wsp1-worktree".into(),
                worktree_root: worktree_root.to_string_lossy().into_owned(), baseline_object_id: "a".repeat(40),
                current_object_id: "b".repeat(40), runtime_instance_ref: "wsp1-runtime".into(),
                runtime_source_ref: "wsp1-source".into(), source_fingerprint: "c".repeat(64),
            };
        let authority_repository = SqliteOrchestrationRepository::open(&fixture.database_path).unwrap();
        authority_repository.store_initiated_sprint_git_authority(initial_authority.clone()).unwrap();

        assert!(serde_json::from_value::<crate::orchestration::sprint_runner_transition::WorkSlicePlannerRequest>(serde_json::json!({"sprintId":"forged"})).is_err());
        assert!(matches!(
            service.request_work_slice_planner(&AgentInvocationId::new("foreign-planning-control").unwrap(), crate::orchestration::sprint_runner_transition::WorkSlicePlannerRequest {}),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_planning_requests", [], |row| row.get(0)).unwrap(), 0);
        drop(connection);

        let launches_before = fixture.runtime.requests().len();
        let barrier = Arc::new(Barrier::new(2));
        let calls = (0..2).map(|_| {
            let service = service.clone();
            let barrier = barrier.clone();
            let control = control.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.request_work_slice_planner(&control, crate::orchestration::sprint_runner_transition::WorkSlicePlannerRequest {})
            })
        }).collect::<Vec<_>>();
        let results = calls.into_iter().map(|call| call.join().unwrap().unwrap()).collect::<Vec<_>>();
        assert_eq!(results[0].work_slice_planner_request_id, results[1].work_slice_planner_request_id);
        assert!(results[0].work_slice_planner_requested_at.is_some());
        assert!(results[0].work_slice_planner_authorized_at.is_some());
        assert!(
            results[0].work_slice_planner_requested_at.as_ref()
                <= results[0].work_slice_planner_authorized_at.as_ref()
        );
        assert_eq!(results[0].work_slice_planning_point_id, results[1].work_slice_planning_point_id);
        assert_eq!(results[0].work_slice_planner_session_id, results[1].work_slice_planner_session_id);
        assert_eq!(results[0].work_slice_planner_invocation_id, results[1].work_slice_planner_invocation_id);
        assert!(results[0].work_slice_planner_session_created_at.is_some());
        assert!(results[0].work_slice_planner_invocation_created_at.is_some());
        assert!(results[0].work_slice_planner_harness_applied_at.is_some());
        assert!(results[0].work_slice_planner_launch_requested_at.is_some());
        assert!(results[0].work_slice_planner_launch_accepted_at.is_some());
        assert!(results[0].work_slice_planner_ready_at.is_some());
        assert_eq!(results[0].work_slice_planner_provider_activation_observed_at, None);
        assert_eq!(results[0].work_slice_planner_lifecycle_observed_at, None);
        assert_eq!(results[0].work_slice_planner_repository_worktree_route, Some(worktree_root.to_string_lossy().into_owned()));
        let launches = fixture.runtime.requests();
        assert_eq!(launches.len(), launches_before + 1);
        let launch = &launches[launches_before];
        assert_eq!(launch.session_id.as_str(), results[0].work_slice_planner_session_id.as_deref().unwrap());
        assert_eq!(launch.invocation_id.as_str(), results[0].work_slice_planner_invocation_id.as_deref().unwrap());
        assert_eq!(launch.working_directory.as_deref(), Some(worktree_root.to_string_lossy().as_ref()));
        assert!(launch.submitted_text.contains("product_initial_prompt_prefix"));
        assert!(launch.submitted_text.contains("Submit only proposal-local lanes through the supplied actions"));
        assert!(!launch.submitted_text.contains("Review current Sprint reality and become ready to plan"));
        assert!(launch.submitted_text.contains("Do not accept a proposal"));
        assert!(launch.submitted_text.contains("create Work Units, Handler or Implementer Sessions"));
        assert!(launch.submitted_text.contains("settle the Sprint, or advance to a later planning point"));
        let extension = launch.launch_extension.as_ref().unwrap();
        assert_eq!(&extension.additional_args[..2], &["-c", "approval_policy=\"never\""]);
        assert!(extension.additional_args.iter().any(|value| value.contains("mcp_servers.work_slice_planner_")));
        assert_eq!(extension.environment.len(), 1);
        assert!(extension.environment[0].0.starts_with("CODEX_ORCHESTRATOR_MCP_"));

        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_planning_requests", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<String, _, _>("SELECT parent_sprint_runner_session_id FROM work_slice_planning_requests", [], |row| row.get(0)).unwrap(), sprint.sprint_runner_session_id);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_sessions WHERE id LIKE 'work-slice-planner-session-%'", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<String, _, _>("SELECT working_directory FROM agent_sessions WHERE id=?1", [&results[0].work_slice_planner_session_id.clone().unwrap()], |row| row.get(0)).unwrap(), worktree_root.to_string_lossy());
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_session_invocations WHERE id LIKE 'work-slice-planner-invocation-%' AND input_provenance='application' AND status='running'", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_units", [], |row| row.get(0)).unwrap(), 0);
        drop(connection);

        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE sprint_runner_transitions SET planning_control_harness_version=planning_control_harness_version+1 WHERE sprint_id=?1", [&sprint_id],
        ).unwrap();
        assert!(matches!(
            service.request_work_slice_planner(&control, crate::orchestration::sprint_runner_transition::WorkSlicePlannerRequest {}),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE sprint_runner_transitions SET planning_control_harness_version=?2 WHERE sprint_id=?1", params![sprint_id, control_harness.version],
        ).unwrap();

        let reopened = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        reopened.reconcile_startup().unwrap();
        let after_reopen = reopened.query().unwrap().transitions.into_iter().find(|status| status.sprint_id == sprint_id).unwrap();
        assert_eq!(after_reopen.work_slice_planning_point_id, results[0].work_slice_planning_point_id);
        assert_eq!(after_reopen.work_slice_planner_requested_at, results[0].work_slice_planner_requested_at);
        assert_eq!(after_reopen.work_slice_planner_authorized_at, results[0].work_slice_planner_authorized_at);
        assert_eq!(after_reopen.work_slice_planner_session_created_at, results[0].work_slice_planner_session_created_at);
        assert_eq!(after_reopen.work_slice_planner_invocation_created_at, results[0].work_slice_planner_invocation_created_at);
        assert_eq!(after_reopen.work_slice_planner_harness_applied_at, results[0].work_slice_planner_harness_applied_at);
        assert_eq!(after_reopen.work_slice_planner_launch_requested_at, results[0].work_slice_planner_launch_requested_at);
        assert_eq!(after_reopen.work_slice_planner_launch_accepted_at, results[0].work_slice_planner_launch_accepted_at);
        assert_eq!(after_reopen.work_slice_planner_ready_at, results[0].work_slice_planner_ready_at);
        assert_eq!(after_reopen.work_slice_planner_provider_activation_observed_at, None);
        assert_eq!(after_reopen.work_slice_planner_lifecycle_observed_at, None);
        assert_eq!(fixture.runtime.requests().len(), launches_before + 1);

        // Runtime acceptance can be durable before the Planner projection transaction. Reopen
        // records the missing product facts without starting a second process.
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_slice_planning_requests SET planner_launch_accepted_at=NULL,planner_ready_at=NULL WHERE sprint_id=?1",
            [&sprint_id],
        ).unwrap();
        let accepted_before_projection = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        let accepted_before_projection = accepted_before_projection.query().unwrap().transitions.into_iter().find(|status| status.sprint_id == sprint_id).unwrap();
        assert!(accepted_before_projection.work_slice_planner_launch_accepted_at.is_some());
        assert!(accepted_before_projection.work_slice_planner_ready_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), launches_before + 1);

        let planner_session = results[0].work_slice_planner_session_id.clone().unwrap();
        let planner_invocation = results[0].work_slice_planner_invocation_id.clone().unwrap();
        let snapshot: String = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT planner_harness_json FROM work_slice_planning_requests WHERE sprint_id=?1", [&sprint_id], |row| row.get(0),
        ).unwrap();

        // A Session-only partial effect reconstructs its reserved invocation and Harness facts.
        Connection::open(&fixture.database_path).unwrap().execute(
            "DELETE FROM agent_session_invocations WHERE id=?1", [&planner_invocation],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_slice_planning_requests SET planner_invocation_created_at=NULL,planner_harness_applied_at=NULL,planner_harness_json=NULL WHERE sprint_id=?1", [&sprint_id],
        ).unwrap();
        let session_only = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        assert!(session_only.query().unwrap().transitions.into_iter().find(|status| status.sprint_id == sprint_id).unwrap().work_slice_planner_harness_applied_at.is_some());

        // An invocation-without-Harness partial effect reconstructs only the missing Harness stage.
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_slice_planning_requests SET planner_harness_applied_at=NULL,planner_harness_json=NULL WHERE sprint_id=?1", [&sprint_id],
        ).unwrap();
        let invocation_only = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        assert!(invocation_only.query().unwrap().transitions.into_iter().find(|status| status.sprint_id == sprint_id).unwrap().work_slice_planner_harness_applied_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), launches_before + 2);

        Connection::open(&fixture.database_path).unwrap().execute("UPDATE agent_sessions SET working_directory='conflict' WHERE id=?1", [&planner_session]).unwrap();
        assert!(matches!(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Unavailable(_))));
        Connection::open(&fixture.database_path).unwrap().execute("UPDATE agent_sessions SET working_directory=?2 WHERE id=?1", params![planner_session, worktree_root.to_string_lossy()]).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute("UPDATE agent_session_invocations SET input_provenance='user' WHERE id=?1", [&planner_invocation]).unwrap();
        assert!(matches!(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Unavailable(_))));
        Connection::open(&fixture.database_path).unwrap().execute("UPDATE agent_session_invocations SET input_provenance='application' WHERE id=?1", [&planner_invocation]).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute("UPDATE work_slice_planning_requests SET planner_harness_json='{}' WHERE sprint_id=?1", [&sprint_id]).unwrap();
        assert!(matches!(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        Connection::open(&fixture.database_path).unwrap().execute("UPDATE work_slice_planning_requests SET planner_harness_json=?2 WHERE sprint_id=?1", params![sprint_id, snapshot]).unwrap();

        let mut ambiguous_authority = initial_authority.clone();
        ambiguous_authority.idempotency_key = "wsp1-route-authority-ambiguous".into();
        ambiguous_authority.runtime_instance_ref = "wsp1-runtime-ambiguous".into();
        ambiguous_authority.runtime_source_ref = "wsp1-source-ambiguous".into();
        authority_repository.store_initiated_sprint_git_authority(ambiguous_authority).unwrap();
        assert!(matches!(
            reopened.request_work_slice_planner(&control, crate::orchestration::sprint_runner_transition::WorkSlicePlannerRequest {}),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)
        ));

        Connection::open(&fixture.database_path).unwrap().execute(
            "DELETE FROM initiated_sprint_git_authorities WHERE sprint_id=?1", [&sprint_id],
        ).unwrap();
        let alternative_worktree = repository_root.join("alternative-worktree");
        fs::create_dir_all(&alternative_worktree).unwrap();
        let mut conflicting_authority = initial_authority;
        conflicting_authority.idempotency_key = "wsp1-route-authority-conflict".into();
        conflicting_authority.worktree_id = "wsp1-worktree-conflict".into();
        conflicting_authority.worktree_root = alternative_worktree.to_string_lossy().into_owned();
        conflicting_authority.runtime_instance_ref = "wsp1-runtime-conflict".into();
        conflicting_authority.runtime_source_ref = "wsp1-source-conflict".into();
        authority_repository.store_initiated_sprint_git_authority(conflicting_authority).unwrap();
        assert!(matches!(
            reopened.request_work_slice_planner(&control, crate::orchestration::sprint_runner_transition::WorkSlicePlannerRequest {}),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)
        ));

        // The application-owned Planner exchange is identity-free at the tool boundary but
        // remains bound to the exact current invocation, Harness, and durable revision.
        fixture.notifier.set_sprint(&reopened);
        let planner_invocation = AgentInvocationId::new(
            Connection::open(&fixture.database_path).unwrap().query_row(
                "SELECT planner_invocation_id FROM work_slice_planning_requests WHERE sprint_id=?1 AND is_current=1",
                [&sprint_id],
                |row| row.get::<_, String>(0),
            ).unwrap(),
        ).unwrap();
        assert_eq!(
            reopened.read_work_slice_planning_context(&planner_invocation).unwrap()["hasCurrentRevision"],
            false,
        );
        assert!(matches!(
            reopened.read_work_slice_planning_context(&AgentInvocationId::new("foreign-planner").unwrap()),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_slice_planning_requests SET planner_harness_version=planner_harness_version+1 WHERE sprint_id=?1",
            [&sprint_id],
        ).unwrap();
        assert!(matches!(
            reopened.read_work_slice_planning_context(&planner_invocation),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        let planner_harness = conversation_harness::profile(ConversationHarnessRole::WorkSlicePlanner).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_slice_planning_requests SET planner_harness_version=?2 WHERE sprint_id=?1",
            params![sprint_id, planner_harness.version],
        ).unwrap();

        let lane = |title: &str, depends_on: Vec<&str>| crate::orchestration::sprint_runner_transition::WorkSliceLane {
            title: title.into(), specification: format!("Bounded specification for {title}."),
            depends_on: depends_on.into_iter().map(str::to_owned).collect(),
        };
        let proposal = crate::orchestration::sprint_runner_transition::WorkSliceProposal {
            objective: "Harden the current planning exchange.".into(),
            lanes: vec![lane("Inspect", vec![]), lane("Verify", vec!["Inspect"])],
        };
        let invalid = |lanes| crate::orchestration::sprint_runner_transition::WorkSliceProposal {
            objective: "Invalid proposal".into(), lanes,
        };
        for candidate in [
            invalid(vec![]),
            invalid(vec![lane("Duplicate", vec![]), lane("Duplicate", vec![])]),
            invalid(vec![lane("Missing", vec!["Unknown"])]),
            invalid(vec![lane("Cycle A", vec!["Cycle B"]), lane("Cycle B", vec!["Cycle A"])]),
        ] {
            assert!(matches!(
                reopened.submit_work_slice_proposal(&planner_invocation, candidate),
                Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Invalid)
            ));
        }
        let mut oversized = proposal.clone();
        oversized.objective = "x".repeat(20_001);
        assert!(matches!(
            reopened.submit_work_slice_proposal(&planner_invocation, oversized),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Invalid)
        ));
        assert!(matches!(
            reopened.complete_work_slice_planning(&planner_invocation, crate::orchestration::sprint_runner_transition::WorkSliceCompletion {}),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        let assert_public_result = |result: &serde_json::Value, status: &str| {
            assert_eq!(
                result,
                &serde_json::json!({
                    "status": status,
                    "accepted": false,
                    "materializationReady": false,
                })
            );
            let payload = result.to_string();
            for forbidden in ["revision", "fingerprint", "command", "idempotency", "route", "token", "authority"] {
                assert!(!payload.to_ascii_lowercase().contains(forbidden), "Planner payload leaked {forbidden}: {payload}");
            }
        };
        let barrier = Arc::new(Barrier::new(2));
        let exact_calls = (0..2).map(|_| {
            let service = reopened.clone();
            let barrier = barrier.clone();
            let invocation = planner_invocation.clone();
            let proposal = proposal.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.submit_work_slice_proposal(&invocation, proposal)
            })
        }).collect::<Vec<_>>();
        let exact_results = exact_calls.into_iter().map(|call| call.join().unwrap().unwrap()).collect::<Vec<_>>();
        assert_eq!(exact_results.iter().filter(|result| result["status"] == "proposal_validated").count(), 1);
        assert_eq!(exact_results.iter().filter(|result| result["status"] == "proposal_replayed").count(), 1);
        assert_public_result(exact_results.iter().find(|result| result["status"] == "proposal_validated").unwrap(), "proposal_validated");
        assert_public_result(exact_results.iter().find(|result| result["status"] == "proposal_replayed").unwrap(), "proposal_replayed");
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_proposal_revisions", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_proposal_revisions WHERE is_current=1", [], |row| row.get(0)).unwrap(), 1);
        drop(connection);
        let mut divergent = proposal.clone();
        divergent.objective = "Divergent objective.".into();
        reopened.request_work_slice_refinement(
            &planner_invocation,
            crate::orchestration::sprint_runner_transition::WorkSliceRefinement { reason: "Verify one boundary more carefully.".into() },
        ).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let refinement_calls = (0..2).map(|_| {
            let service = reopened.clone();
            let barrier = barrier.clone();
            let invocation = planner_invocation.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.request_work_slice_refinement(
                    &invocation,
                    crate::orchestration::sprint_runner_transition::WorkSliceRefinement { reason: "Verify one boundary more carefully.".into() },
                )
            })
        }).collect::<Vec<_>>();
        assert!(refinement_calls.into_iter().all(|call| call.join().unwrap().is_ok()));
        assert!(matches!(
            reopened.request_work_slice_refinement(
                &planner_invocation,
                crate::orchestration::sprint_runner_transition::WorkSliceRefinement { reason: "A different refinement.".into() },
            ),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)
        ));
        assert!(matches!(
            reopened.complete_work_slice_planning(&planner_invocation, crate::orchestration::sprint_runner_transition::WorkSliceCompletion {}),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        let mut alternate = proposal.clone();
        alternate.objective = "Alternate divergent objective.".into();
        let barrier = Arc::new(Barrier::new(2));
        let divergent_calls = [divergent.clone(), alternate.clone()].into_iter().map(|candidate| {
            let service = reopened.clone();
            let barrier = barrier.clone();
            let invocation = planner_invocation.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let result = service.submit_work_slice_proposal(&invocation, candidate.clone());
                (candidate, result)
            })
        }).collect::<Vec<_>>();
        let divergent_results = divergent_calls.into_iter().map(|call| call.join().unwrap()).collect::<Vec<_>>();
        assert_eq!(divergent_results.iter().filter(|(_, result)| result.is_ok()).count(), 1);
        assert_eq!(divergent_results.iter().filter(|(_, result)| matches!(result, Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict))).count(), 1);
        let successor = divergent_results.into_iter().find_map(|(candidate, result)| result.ok().map(|result| (candidate, result))).unwrap();
        assert_public_result(&successor.1, "proposal_validated");
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_planning_episodes", [], |row| row.get(0)).unwrap(), 1);
        let revisions = connection.prepare(
            "SELECT revision_id,revision_number,is_current,parent_revision_id,refinement_requested_at,semantic_completed_at,accepted_at FROM work_slice_proposal_revisions ORDER BY revision_number",
        ).unwrap().query_map([], |row| Ok((row.get::<_, String>(0)?,row.get::<_, i64>(1)?,row.get::<_, i64>(2)?,row.get::<_, Option<String>>(3)?,row.get::<_, Option<String>>(4)?,row.get::<_, Option<String>>(5)?,row.get::<_, Option<String>>(6)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(revisions.len(), 2);
        assert_eq!(revisions[0].1, 1);
        assert_eq!(revisions[0].2, 0);
        assert!(revisions[0].3.is_none());
        assert!(revisions[0].4.is_some());
        assert!(revisions[0].5.is_none());
        assert_eq!(revisions[1].1, 2);
        assert_eq!(revisions[1].2, 1);
        assert_eq!(revisions[1].3.as_deref(), Some(revisions[0].0.as_str()));
        drop(connection);
        reopened.complete_work_slice_planning(&planner_invocation, crate::orchestration::sprint_runner_transition::WorkSliceCompletion {}).unwrap();
        let before_terminal = reopened.query().unwrap().transitions.into_iter().find(|item| item.sprint_id == sprint_id).unwrap();
        assert!(before_terminal.work_slice_semantic_completed_at.is_some());
        assert!(before_terminal.work_slice_terminal_lifecycle_observed_at.is_none());
        assert!(before_terminal.work_slice_application_accepted_at.is_none());
        assert!(before_terminal.work_slice_materialization_ready_at.is_none());
        let launches_before_acceptance = fixture.runtime.requests().len();
        fixture.runtime.finish(planner_invocation.as_str(), AgentInvocationTerminalStatus::Completed);
        let accepted = reopened.query().unwrap().transitions.into_iter().find(|item| item.sprint_id == sprint_id).unwrap();
        assert!(accepted.work_slice_terminal_lifecycle_observed_at.is_some());
        assert!(accepted.work_slice_application_accepted_at.is_some());
        assert!(accepted.work_slice_materialization_ready_at.is_some());
        assert_eq!(fixture.runtime.requests().len(), launches_before_acceptance);
        let accepted_at = accepted.work_slice_application_accepted_at.clone();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_slice_proposal_revisions SET materialization_ready_at=NULL WHERE is_current=1",
            [],
        ).unwrap();
        let repaired = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        let repaired = repaired.query().unwrap().transitions.into_iter().find(|item| item.sprint_id == sprint_id).unwrap();
        assert!(repaired.work_slice_application_accepted_at.is_some());
        assert!(repaired.work_slice_materialization_ready_at.is_some());
        assert_eq!(repaired.work_slice_application_accepted_at, accepted_at);
        assert_eq!(fixture.runtime.requests().len(), launches_before_acceptance);
        let connection = Connection::open(&fixture.database_path).unwrap();
        let materialization: (String,String,String,String,String,Option<String>,Option<String>,Option<String>,Option<String>) = connection.query_row(
            "SELECT materialization_id,planning_point_id,accepted_revision_id,epic_id,sprint_id,attempt_recorded_at,work_units_created_at,relationships_completed_at,settled_at FROM work_unit_materializations", [],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?)),
        ).unwrap();
        assert!(materialization.5.is_some());
        assert!(materialization.6.is_some());
        assert!(materialization.7.is_some());
        assert!(materialization.8.is_some());
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_units WHERE materialization_id=?1", [&materialization.0], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_relationships WHERE materialization_id=?1", [&materialization.0], |row| row.get(0)).unwrap(), 7);
        let unit_ids = connection.prepare("SELECT work_unit_id FROM work_units WHERE materialization_id=?1 ORDER BY lane_ordinal").unwrap().query_map([&materialization.0], |row| row.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        let relationship_ids = connection.prepare("SELECT relationship_id FROM work_unit_relationships WHERE materialization_id=?1 ORDER BY relationship_id").unwrap().query_map([&materialization.0], |row| row.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        connection.execute("DELETE FROM work_unit_relationships WHERE relationship_id=?1", [&relationship_ids[0]]).unwrap();
        connection.execute("UPDATE work_unit_materializations SET relationships_completed_at=NULL,settled_at=NULL WHERE materialization_id=?1", [&materialization.0]).unwrap();
        drop(connection);
        let replayed = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        let connection = Connection::open(&fixture.database_path).unwrap();
        let repaired_unit_ids = connection.prepare("SELECT work_unit_id FROM work_units WHERE materialization_id=?1 ORDER BY lane_ordinal").unwrap().query_map([&materialization.0], |row| row.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        let repaired_relationship_ids = connection.prepare("SELECT relationship_id FROM work_unit_relationships WHERE materialization_id=?1 ORDER BY relationship_id").unwrap().query_map([&materialization.0], |row| row.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        assert_eq!(repaired_unit_ids, unit_ids);
        assert_eq!(repaired_relationship_ids, relationship_ids);
        assert_eq!(connection.query_row::<Option<String>, _, _>("SELECT settled_at FROM work_unit_materializations WHERE materialization_id=?1", [&materialization.0], |row| row.get(0)).unwrap().is_some(), true);
        drop(connection);
        let native = serde_json::to_value(SqliteOrchestrationRepository::open(&fixture.database_path).unwrap().native_query().unwrap()).unwrap();
        assert_eq!(native["workUnitMaterializations"].as_array().unwrap().len(), 1);
        assert_eq!(native["workUnits"].as_array().unwrap().len(), 2);
        assert_eq!(native["workUnitRelationships"].as_array().unwrap().len(), 7);

        // This is the actual settled-materialization coordinator path: the root gets exactly one
        // Handler attempt, grant, worktree, Session, prepared invocation, launch acceptance, and
        // ready fact; the dependent remains blocked even after the root is ready.
        let handler_repository_root = fixture._directory.path().join("handler-repository");
        let handler_sprint_root = fixture._directory.path().join("handler-sprint-worktree");
        fs::create_dir_all(&handler_repository_root).unwrap();
        for arguments in [&["init"][..], &["config", "user.email", "handler@example.test"][..], &["config", "user.name", "Handler Test"][..]] {
            assert!(std::process::Command::new("git").args(arguments).current_dir(&handler_repository_root).status().unwrap().success());
        }
        fs::write(handler_repository_root.join("README.md"), "handler fixture\n").unwrap();
        assert!(std::process::Command::new("git").args(["add", "README.md"]).current_dir(&handler_repository_root).status().unwrap().success());
        assert!(std::process::Command::new("git").args(["commit", "-m", "handler fixture"]).current_dir(&handler_repository_root).status().unwrap().success());
        let handler_initial = String::from_utf8(std::process::Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&handler_repository_root).output().unwrap().stdout).unwrap().trim().to_owned();
        assert!(std::process::Command::new("git").args(["worktree", "add", "-b", "handler-sprint", handler_sprint_root.to_string_lossy().as_ref(), &handler_initial]).current_dir(&handler_repository_root).status().unwrap().success());
        fs::write(handler_sprint_root.join("README.md"), "handler sprint fixture\n").unwrap();
        assert!(std::process::Command::new("git").args(["add", "README.md"]).current_dir(&handler_sprint_root).status().unwrap().success());
        assert!(std::process::Command::new("git").args(["commit", "-m", "handler sprint fixture"]).current_dir(&handler_sprint_root).status().unwrap().success());
        let handler_head = String::from_utf8(std::process::Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&handler_sprint_root).output().unwrap().stdout).unwrap().trim().to_owned();
        let handler_repository_root = handler_repository_root.canonicalize().unwrap();
        let handler_sprint_root = handler_sprint_root.canonicalize().unwrap();
        let handler_common = handler_repository_root.join(".git").canonicalize().unwrap();
        let handler_repository = Arc::new(SqliteOrchestrationRepository::open(&fixture.database_path).unwrap());
        let handler_orchestration = Arc::new(OrchestrationApplication::new(handler_repository.clone()));
        let handler_support = ProductExecutionSupportState::new(
            &fixture.database_path,
            fixture._directory.path().join("handler-workspaces"),
            handler_repository,
        ).unwrap();
        let handler = Arc::new(WorkUnitExecutionHarnessService::new(
            handler_support.service(), fixture.sessions.clone(), handler_orchestration.clone(),
        ));
        let handler_runner = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path, fixture.sessions.clone(),
        ).unwrap();
        fixture.notifier.set_sprint(&handler_runner);
        let handler_launches_before = fixture.runtime.requests().len();
        Connection::open(&fixture.database_path).unwrap().execute("DELETE FROM initiated_sprint_git_authorities WHERE sprint_id=?1", [&sprint_id]).unwrap();
        handler_runner.attach_work_unit_handler_activation(handler.clone()).unwrap();
        let blocked_without_authority: (String,Option<String>,Option<String>,Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT blocked_reason,execution_support_granted_at,handler_session_created_at,handler_invocation_prepared_at FROM work_unit_handler_activations WHERE blocked_reason='initiated_sprint_git_authority_missing' LIMIT 1", [],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
        ).unwrap();
        assert_eq!(blocked_without_authority.0, "initiated_sprint_git_authority_missing");
        assert!(blocked_without_authority.1.is_none() && blocked_without_authority.2.is_none() && blocked_without_authority.3.is_none());
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before);
        SqliteOrchestrationRepository::open(&fixture.database_path).unwrap().store_initiated_sprint_git_authority(InitiatedSprintGitAuthorityWrite {
            sprint_id: sprint_id.clone(), idempotency_key: "handler-authority".into(),
            repository_id: "handler-repository".into(), repository_root: handler_repository_root.to_string_lossy().into_owned(),
            repository_common_dir: handler_common.to_string_lossy().into_owned(), worktree_id: "handler-sprint-worktree".into(),
            worktree_root: handler_sprint_root.to_string_lossy().into_owned(), baseline_object_id: handler_initial,
            current_object_id: handler_head, runtime_instance_ref: "handler-runtime".into(), runtime_source_ref: "handler-source".into(), source_fingerprint: "d".repeat(64),
        }).unwrap();
        handler_runner.attach_work_unit_handler_activation(handler.clone()).unwrap();
        let connection = Connection::open(&fixture.database_path).unwrap();
        let activations = connection.prepare("SELECT work_unit_id,attempt_id,handler_session_id,handler_invocation_id,eligibility_state,blocked_reason,handler_harness_revision_id,handler_harness_configuration_digest,handler_harness_repository_commit_ref,authorized_at,attempt_created_at,execution_support_granted_at,isolated_worktree_ready_at,handler_session_created_at,handler_invocation_prepared_at,handler_harness_bound_at,launch_requested_at,launch_accepted_at,handler_ready_at,provider_activation_observed_at FROM work_unit_handler_activations ORDER BY work_unit_id").unwrap().query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,Option<String>>(5)?,row.get::<_,Option<String>>(6)?,row.get::<_,Option<String>>(7)?,row.get::<_,Option<String>>(8)?,row.get::<_,Option<String>>(9)?,row.get::<_,Option<String>>(10)?,row.get::<_,Option<String>>(11)?,row.get::<_,Option<String>>(12)?,row.get::<_,Option<String>>(13)?,row.get::<_,Option<String>>(14)?,row.get::<_,Option<String>>(15)?,row.get::<_,Option<String>>(16)?,row.get::<_,Option<String>>(17)?,row.get::<_,Option<String>>(18)?,row.get::<_,Option<String>>(19)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        assert_eq!(activations.len(), 2);
        let root = activations.iter().find(|row| row.4 == "eligible").unwrap();
        assert!(root.0.starts_with("work-unit-"));
        assert!(root.1.starts_with("work-unit-handler-attempt-"));
        assert!(root.2.starts_with("work-unit-handler-session-"));
        assert!(root.3.starts_with("work-unit-handler-invocation-"));
        for timestamp in [&root.9,&root.10,&root.11,&root.12,&root.13,&root.14,&root.15,&root.16,&root.17,&root.18] { assert!(timestamp.is_some()); }
        assert!(root.6.as_deref().is_some_and(|value| value.starts_with("harness-revision-")));
        assert!(root.7.as_deref().is_some_and(|value| value.len() == 64));
        assert!(root.8.as_deref().is_some_and(|value| value.contains("harness-revision-commit/v1")));
        let dependent = activations.iter().find(|row| row.4 == "blocked").unwrap();
        assert_eq!(dependent.5.as_deref(), Some("prerequisite_satisfaction_not_authoritative"));
        for timestamp in [&dependent.9,&dependent.10,&dependent.11,&dependent.12,&dependent.13,&dependent.14,&dependent.15,&dependent.16,&dependent.17,&dependent.18,&dependent.19] { assert!(timestamp.is_none()); }
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM agent_sessions WHERE id LIKE 'work-unit-handler-session-%'", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM agent_session_invocations WHERE id LIKE 'work-unit-handler-invocation-%' AND input_provenance='application'", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM execution_support_attempt_authorizations WHERE role_kind='implementer'", [], |row| row.get(0)).unwrap(), 0);
        drop(connection);
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 1);
        fixture.runtime.finish(&root.3, AgentInvocationTerminalStatus::Completed);
        let continuation: (String,String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT attempt_id,handler_session_id,original_handler_invocation_id,action_invocation_id,
                    blocked_reason,authorized_at,invocation_prepared_at,harness_bound_at,launch_accepted_at,action_ready_at
             FROM work_unit_handler_action_continuations WHERE work_unit_id=?1",
            [&root.0], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?)),
        ).unwrap();
        assert_eq!(continuation.0, root.1);
        assert_eq!(continuation.1, root.2);
        assert_eq!(continuation.2, root.3);
        assert!(continuation.3.starts_with("work-unit-handler-action-invocation-"));
        assert!(continuation.4.is_none());
        for timestamp in [&continuation.5,&continuation.6,&continuation.7,&continuation.8,&continuation.9] { assert!(timestamp.is_some()); }
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 2);
        assert!(matches!(handler_runner.request_work_unit_implementer_from_authenticated_continuation(&root.3), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)));
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_implementer_activations", [], |row| row.get(0)).unwrap(), 0);
        // Every rejected caller fails before it can record an Implementer request or grant.
        // In particular, a terminal continuation cannot rely on an old action_ready_at fact.
        let (action_digest, action_launch_requested): (String, String) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT action_harness_configuration_digest,launch_requested_at FROM work_unit_handler_action_continuations WHERE work_unit_id=?1",
            [&root.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        let rejected = |invocation: &str| {
            assert!(matches!(
                handler_runner.request_work_unit_implementer_from_authenticated_continuation(invocation),
                Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
            ));
            assert_eq!(
                Connection::open(&fixture.database_path).unwrap().query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_implementer_activations",
                    [],
                    |row| row.get(0),
                ).unwrap(),
                0
            );
        };
        rejected("foreign-handler-action-invocation");
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET original_handler_invocation_id='foreign-original-handler' WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET original_handler_invocation_id=?2 WHERE work_unit_id=?1",
            params![root.0, continuation.2],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET handler_session_id='foreign-handler-session' WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET handler_session_id=?2 WHERE work_unit_id=?1",
            params![root.0, continuation.1],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET action_harness_configuration_digest='forged' WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET action_harness_configuration_digest=?2 WHERE work_unit_id=?1",
            params![root.0, action_digest],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET blocked_reason='stale_action_block' WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET blocked_reason=NULL WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET failure_reason='stale_action_failure' WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET failure_reason=NULL WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET launch_requested_at=NULL WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET launch_requested_at=?2 WHERE work_unit_id=?1",
            params![root.0, action_launch_requested],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET action_ready_at=NULL WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_action_continuations SET action_ready_at=?2 WHERE work_unit_id=?1",
            params![root.0, continuation.9],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET status='completed',completed_at=?2 WHERE id=?1",
            params![continuation.3, chrono::Utc::now().to_rfc3339()],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET status='running',completed_at=NULL WHERE id=?1",
            [&continuation.3],
        ).unwrap();
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_activations SET eligibility_state='blocked',blocked_reason='test_dependency_ineligible' WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        rejected(&continuation.3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_activations SET eligibility_state='eligible',blocked_reason=NULL WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        let upstream_before: (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT epic_continuation_invocation_id,epic_continuation_launch_accepted_at,
                    sprint_continuation_invocation_id,sprint_continuation_launch_accepted_at,
                    planning_ready_at
             FROM sprint_runner_transitions WHERE sprint_id=?1",
            [&sprint_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).unwrap();
        let materialization_before: (i64, Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT COUNT(*),MAX(settled_at) FROM work_unit_materializations WHERE materialization_id=?1",
            [&materialization.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        let injection = handler_runner.prepared_handler_action_injection(&continuation.3).unwrap();
        let endpoint = injection.configuration_args.iter().find_map(|argument| argument.strip_prefix("mcp_servers.").and_then(|value| value.split_once(".url=\"")).map(|(_, value)| value.trim_end_matches('"').to_owned())).unwrap();
        let bearer = injection.environment.1.clone();
        tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().unwrap().block_on(async {
            let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(2)).build().unwrap();
            let initialize = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}).to_string();
            assert_eq!(client.post(&endpoint).header("content-type","application/json").header("accept","application/json, text/event-stream").header("authorization","Bearer wrong").body(initialize.clone()).send().await.unwrap().status(), reqwest::StatusCode::UNAUTHORIZED);
            assert_eq!(client.post(&endpoint).header("content-type","application/json").header("accept","application/json, text/event-stream").header("authorization",format!("Bearer {bearer}")).header("origin","https://evil.example").body(initialize.clone()).send().await.unwrap().status(), reqwest::StatusCode::FORBIDDEN);
            assert_eq!(client.post(&endpoint).header("content-type","application/json").header("accept","application/json, text/event-stream").header("authorization",format!("Bearer {bearer}")).header("host","evil.example").body(initialize.clone()).send().await.unwrap().status(), reqwest::StatusCode::FORBIDDEN);
            let initialized = client.post(&endpoint).header("content-type","application/json").header("accept","application/json, text/event-stream").header("authorization",format!("Bearer {bearer}")).body(initialize).send().await.unwrap();
            let session = initialized.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_owned();
            client.post(&endpoint).header("content-type","application/json").header("accept","application/json, text/event-stream").header("authorization",format!("Bearer {bearer}")).header("mcp-session-id",&session).body(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string()).send().await.unwrap();
            let listed = client.post(&endpoint).header("content-type","application/json").header("accept","application/json, text/event-stream").header("authorization",format!("Bearer {bearer}")).header("mcp-session-id",&session).body(serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}).to_string()).send().await.unwrap();
            let listed_text = listed.text().await.unwrap();
            let listed_json = listed_text.lines().filter_map(|line| line.strip_prefix("data: ")).find(|line| !line.trim().is_empty()).unwrap_or(&listed_text);
            let listed_result: serde_json::Value = serde_json::from_str(listed_json).unwrap();
            assert_eq!(listed_result["result"]["tools"].as_array().unwrap().len(), 1);
            assert_eq!(listed_result["result"]["tools"][0]["name"], "request_work_unit_implementer");
            let response = client.post(&endpoint).header("content-type","application/json").header("accept","application/json, text/event-stream").header("authorization",format!("Bearer {bearer}")).header("mcp-session-id",&session).body(serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"request_work_unit_implementer","arguments":{}}}).to_string()).send().await.unwrap();
            let text = response.text().await.unwrap();
            let json = text.lines().filter_map(|line| line.strip_prefix("data: ")).find(|line| !line.trim().is_empty()).unwrap_or(&text);
            let result: serde_json::Value = serde_json::from_str(json).unwrap();
            assert_eq!(result["result"]["isError"], false);
            assert_eq!(serde_json::from_str::<serde_json::Value>(result["result"]["content"][0]["text"].as_str().unwrap()).unwrap()["status"], "implementer_request_recorded");
        });
        handler_runner.request_work_unit_implementer_from_authenticated_continuation(&continuation.3).unwrap();
        let implementer: (String,String,String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT attempt_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,
                    implementer_harness_configuration_digest,authorized_at,execution_support_granted_at,
                    isolated_worktree_ready_at,implementer_harness_bound_at,launch_accepted_at,implementer_ready_at
             FROM work_unit_implementer_activations WHERE work_unit_id=?1",
            [&root.0], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?,row.get(10)?)),
        ).unwrap();
        assert_eq!(implementer.0, root.1);
        assert!(implementer.1.starts_with("work-unit-implementer-session-"));
        assert!(implementer.2.starts_with("work-unit-implementer-invocation-"));
        assert!(implementer.3.starts_with("harness-revision-"));
        assert_eq!(implementer.4.len(), 64);
        for timestamp in [&implementer.5,&implementer.6,&implementer.7,&implementer.8,&implementer.9,&implementer.10] { assert!(timestamp.is_some()); }
        let grants = Connection::open(&fixture.database_path).unwrap().prepare("SELECT role_id,capability_ref,workspace_id FROM execution_support_grants WHERE attempt_id=?1 ORDER BY role_id").unwrap().query_map([&root.1], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        assert_eq!(grants.len(), 2);
        assert_eq!(grants[0].0, "work_unit_handler");
        assert_eq!(grants[1].0, "work_unit_implementer");
        assert_ne!(grants[0].1, grants[1].1);
        assert_eq!(grants[0].2, grants[1].2);
        // These are the persisted/prepared application launch records, not catalog-only
        // profiles: both Handler invocations remain read-only while the distinct Implementer
        // capability receives the only workspace-write runtime.  All three are pinned to the
        // one execution-support workspace and carry no caller-provided model or approval
        // override.
        let launches = fixture.runtime.requests();
        let original_handler_launch = launches
            .iter()
            .find(|launch| launch.invocation_id.as_str() == root.3)
            .unwrap();
        let action_handler_launch = launches
            .iter()
            .find(|launch| launch.invocation_id.as_str() == continuation.3)
            .unwrap();
        let implementer_launch = launches
            .iter()
            .find(|launch| launch.invocation_id.as_str() == implementer.2)
            .unwrap();
        let shared_working_directory = original_handler_launch.working_directory.as_deref().unwrap();
        assert_eq!(
            action_handler_launch.working_directory.as_deref(),
            Some(shared_working_directory)
        );
        assert_eq!(
            implementer_launch.working_directory.as_deref(),
            Some(shared_working_directory)
        );
        assert_eq!(original_handler_launch.options.model, None);
        assert_eq!(action_handler_launch.options.model, None);
        assert_eq!(implementer_launch.options.model, None);
        assert_eq!(
            original_handler_launch.options.sandbox,
            Some(crate::agent_sessions::domain::RuntimeSandboxMode::ReadOnly)
        );
        assert_eq!(
            action_handler_launch.options.sandbox,
            Some(crate::agent_sessions::domain::RuntimeSandboxMode::ReadOnly)
        );
        assert_eq!(
            implementer_launch.options.sandbox,
            Some(crate::agent_sessions::domain::RuntimeSandboxMode::WorkspaceWrite)
        );
        for handler_launch in [original_handler_launch, action_handler_launch] {
            let extension = handler_launch.launch_extension.as_ref().unwrap();
            assert_eq!(
                &extension.additional_args[..2],
                &["-c", "approval_policy=\"never\""]
            );
            assert_eq!(
                extension.initial_prompt_prefix.as_ref().unwrap().source,
                "work_unit_handler"
            );
        }
        let implementer_extension = implementer_launch.launch_extension.as_ref().unwrap();
        assert_eq!(
            implementer_extension.additional_args,
            ["-c", "approval_policy=\"never\""]
        );
        assert!(implementer_extension.environment.is_empty());
        assert_eq!(
            implementer_extension.initial_prompt_prefix.as_ref().unwrap().source,
            "work_unit_implementer"
        );
        let persisted_directories = Connection::open(&fixture.database_path)
            .unwrap()
            .prepare("SELECT id,working_directory FROM agent_sessions WHERE id IN (?1,?2)")
            .unwrap()
            .query_map([&continuation.1, &implementer.1], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(persisted_directories.len(), 2);
        assert!(persisted_directories
            .iter()
            .all(|(_, directory)| directory == shared_working_directory));
        // The valid action creates only its own continuation/Implementer boundary for the eligible
        // root. The dependency remains blocked and has no action continuation or Implementer.
        let action_continuations = Connection::open(&fixture.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_handler_action_continuations", [], |row| row.get(0),
        ).unwrap();
        let implementer_activations = Connection::open(&fixture.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_implementer_activations", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(action_continuations, 1);
        assert_eq!(implementer_activations, 1);
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_handler_action_continuations WHERE work_unit_id=?1",
            [&dependent.0], |row| row.get(0),
        ).unwrap(), 0);
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_implementer_activations WHERE work_unit_id=?1",
            [&dependent.0], |row| row.get(0),
        ).unwrap(), 0);
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<String, _, _>(
            "SELECT eligibility_state FROM work_unit_handler_activations WHERE work_unit_id=?1",
            [&dependent.0], |row| row.get(0),
        ).unwrap(), "blocked");
        let upstream_after: (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT epic_continuation_invocation_id,epic_continuation_launch_accepted_at,
                    sprint_continuation_invocation_id,sprint_continuation_launch_accepted_at,
                    planning_ready_at
             FROM sprint_runner_transitions WHERE sprint_id=?1",
            [&sprint_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).unwrap();
        let materialization_after: (i64, Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT COUNT(*),MAX(settled_at) FROM work_unit_materializations WHERE materialization_id=?1",
            [&materialization.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(upstream_after, upstream_before);
        assert_eq!(materialization_after, materialization_before);
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN (
                'work_unit_implementation_outputs','work_unit_implementation_feedback',
                'work_unit_handler_acceptances','work_unit_handler_returns',
                'work_unit_integrations',
                'work_unit_handoffs','work_unit_executions')",
            [], |row| row.get(0),
        ).unwrap(), 0);
        let projected = serde_json::to_value(
            SqliteOrchestrationRepository::open(&fixture.database_path).unwrap().native_query().unwrap(),
        ).unwrap();
        let projected_unit = projected["workUnits"].as_array().unwrap().iter()
            .find(|unit| unit["workUnitId"] == root.0).unwrap();
        assert_eq!(projected_unit["handlerActivation"]["attemptId"], root.1);
        assert_eq!(projected_unit["actionContinuation"]["originalHandlerInvocationId"], root.3);
        assert_eq!(projected_unit["actionContinuation"]["actionInvocationId"], continuation.3);
        assert_eq!(projected_unit["implementerActivation"]["attemptId"], root.1);
        assert_eq!(projected_unit["implementerActivation"]["handlerActionInvocationId"], continuation.3);
        assert!(projected_unit["implementerActivation"]["launchAcceptedAt"].is_string());
        assert!(projected_unit["implementerActivation"]["implementerReadyAt"].is_string());
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        let concurrent_requests = (0..2).map(|_| {
            let service = handler_runner.clone();
            let invocation = continuation.3.clone();
            std::thread::spawn(move || service.request_work_unit_implementer_from_authenticated_continuation(&invocation))
        }).collect::<Vec<_>>();
        assert!(concurrent_requests.into_iter().all(|request| request.join().unwrap().is_ok()));
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_implementer_activations WHERE work_unit_id=?1", [&root.0], |row| row.get(0)).unwrap(), 1);
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_implementer_activations
             SET implementer_harness_bound_at=NULL,launch_accepted_at=NULL,implementer_ready_at=NULL
             WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        let partial_implementer = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        partial_implementer.attach_work_unit_handler_activation(handler.clone()).unwrap();
        let recovered_implementer: (String,String,String,String,Option<String>,Option<String>,Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT attempt_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,
                    implementer_harness_bound_at,launch_accepted_at,implementer_ready_at
             FROM work_unit_implementer_activations WHERE work_unit_id=?1",
            [&root.0], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?)),
        ).unwrap();
        assert_eq!(recovered_implementer.0, implementer.0);
        assert_eq!(recovered_implementer.1, implementer.1);
        assert_eq!(recovered_implementer.2, implementer.2);
        assert_eq!(recovered_implementer.3, implementer.3);
        for timestamp in [&recovered_implementer.4,&recovered_implementer.5,&recovered_implementer.6] { assert!(timestamp.is_some()); }
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "INSERT INTO agent_session_runtime_events (id,invocation_id,sequence,source,raw_payload_json,normalized_json,recorded_at) VALUES (?1,?2,0,'runtime','{}',?3,?4)",
            params!["implementer-provider-activity", implementer.2, r#"{"kind":"processing_started","text":null,"externalContextId":null,"usage":null,"details":null}"#, chrono::Utc::now().to_rfc3339()],
        ).unwrap();
        let observed_implementer = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        observed_implementer.attach_work_unit_handler_activation(handler.clone()).unwrap();
        let observation: (Option<String>,Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT provider_activation_observed_at,implementer_ready_at FROM work_unit_implementer_activations WHERE work_unit_id=?1",
            [&root.0], |row| Ok((row.get(0)?,row.get(1)?)),
        ).unwrap();
        assert!(observation.0.is_some() && observation.1.is_some());
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        // Once the authenticated action has recorded the Implementer request it may terminate.
        // Reopen drains only that persisted request, does not recreate the terminal action MCP
        // server, and does not make the public action callable from a terminal invocation.
        fixture.runtime.finish(&continuation.3, AgentInvocationTerminalStatus::Completed);
        assert!(handler_runner.prepared_handler_action_injection(&continuation.3).is_none());
        let terminal_action_reopen = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path, fixture.sessions.clone(),
        ).unwrap();
        terminal_action_reopen.attach_work_unit_handler_activation(handler.clone()).unwrap();
        assert!(terminal_action_reopen.prepared_handler_action_injection(&continuation.3).is_none());
        assert!(matches!(
            terminal_action_reopen.request_work_unit_implementer_from_authenticated_continuation(&continuation.3),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET status='running',completed_at=NULL WHERE id=?1",
            [&continuation.3],
        ).unwrap();
        // Publish a legitimate newer Handler revision B after this activation pinned A. Reopen
        // must keep loading A, rather than consulting the now-newer current revision.
        let working_copy = handler_orchestration.load_harness_working_copy("work_unit_handler").unwrap().unwrap();
        let mut newer_configuration = working_copy.configuration.clone();
        newer_configuration.prompt_prefix.content.push_str("\nRevision B is for future activations only.");
        let saved = handler_orchestration.save_harness_working_copy(
            crate::orchestration::conversation_harness_working_copy::SaveHarnessWorkingCopyCommand {
                harness_key: "work_unit_handler".into(), configuration: newer_configuration,
                expected_current_revision: working_copy.draft_revision,
                editor: crate::orchestration::conversation_harness_working_copy::HarnessWorkingCopyEditor {
                    kind: crate::orchestration::conversation_harness_working_copy::HarnessEditorKind::ApplicationUser,
                    reference: "handler-revision-test".into(),
                }, idempotency_key: "handler-revision-b-working-copy".into(),
            },
        ).unwrap();
        let saved = match saved {
            crate::orchestration::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::Stored(copy)
            | crate::orchestration::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::IdempotentReplay(copy) => copy,
        };
        let action_revision: String = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT action_harness_revision_id FROM work_unit_handler_action_continuations WHERE work_unit_id=?1",
            [&root.0], |row| row.get(0),
        ).unwrap();
        let revision_b = handler_orchestration.create_harness_revision(
            crate::orchestration::conversation_harness_revision::CreateHarnessRevisionCommand {
                harness_key: "work_unit_handler".into(), expected_source_draft_revision: saved.draft_revision,
                expected_predecessor_revision_id: Some(action_revision), idempotency_key: "handler-revision-b".into(),
                creation_provenance: crate::orchestration::conversation_harness_revision::HarnessRevisionCreationProvenance {
                    kind: crate::orchestration::conversation_harness_revision::HarnessRevisionProvenanceKind::ApplicationUser,
                    reference: "handler-revision-test".into(),
                },
            },
        ).unwrap();
        let revision_b = match revision_b {
            crate::orchestration::conversation_harness_revision::CreateHarnessRevisionResult::Published(revision)
            | crate::orchestration::conversation_harness_revision::CreateHarnessRevisionResult::IdempotentReplay(revision) => revision,
        };
        assert_ne!(root.6.as_deref(), Some(revision_b.revision_id.as_str()));
        // Persist provider activity without routing a notification, then recover it solely by
        // reopening/reconciling the correlated invocation observation seam.
        Connection::open(&fixture.database_path).unwrap().execute(
            "INSERT INTO agent_session_runtime_events (id,invocation_id,sequence,source,raw_payload_json,normalized_json,recorded_at) VALUES (?1,?2,0,'runtime','{}',?3,?4)",
            params!["handler-provider-activity", root.3, r#"{"kind":"processing_started","text":null,"externalContextId":null,"usage":null,"details":null}"#, chrono::Utc::now().to_rfc3339()],
        ).unwrap();
        let replayed_handlers = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        replayed_handlers.attach_work_unit_handler_activation(handler.clone()).unwrap();
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<String,_,_>("SELECT handler_harness_revision_id FROM work_unit_handler_activations WHERE work_unit_id=?1", [&root.0], |row| row.get(0)).unwrap(), root.6.clone().unwrap());
        assert!(Connection::open(&fixture.database_path).unwrap().query_row::<Option<String>,_,_>("SELECT provider_activation_observed_at FROM work_unit_handler_activations WHERE work_unit_id=?1", [&root.0], |row| row.get(0)).unwrap().is_some());
        // Recover durable partial stages without replacing any identity or starting another
        // runtime process: an existing prepared invocation is rebound, then launch evidence
        // projects acceptance/readiness again from the same invocation.
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_activations SET handler_harness_bound_at=NULL,launch_accepted_at=NULL,handler_ready_at=NULL WHERE work_unit_id=?1", [&root.0],
        ).unwrap();
        let partial_recovery = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        partial_recovery.attach_work_unit_handler_activation(handler.clone()).unwrap();
        let recovered: (String,String,String,Option<String>,Option<String>,Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT attempt_id,handler_session_id,handler_invocation_id,handler_harness_bound_at,launch_accepted_at,handler_ready_at FROM work_unit_handler_activations WHERE work_unit_id=?1", [&root.0],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?)),
        ).unwrap();
        assert_eq!(recovered.0, root.1); assert_eq!(recovered.1, root.2); assert_eq!(recovered.2, root.3);
        assert!(recovered.3.is_some() && recovered.4.is_some() && recovered.5.is_some());
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        // Missing immutable evidence fails closed and never falls forward to newly published B.
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_activations SET handler_harness_revision_id='harness-revision-00000000-0000-0000-0000-000000000000' WHERE work_unit_id=?1", [&root.0],
        ).unwrap();
        let missing_pinned = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        assert!(missing_pinned.attach_work_unit_handler_activation(handler.clone()).is_err());
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE work_unit_handler_activations SET handler_harness_revision_id=?2 WHERE work_unit_id=?1", params![root.0, root.6],
        ).unwrap();
        let concurrent_handler_services = (0..2).map(|_| {
            let path = fixture.database_path.clone();
            let sessions = fixture.sessions.clone();
            let handler_repository = Arc::new(SqliteOrchestrationRepository::open(&path).unwrap());
            let handler_orchestration = Arc::new(OrchestrationApplication::new(handler_repository.clone()));
            let support = ProductExecutionSupportState::new(&path, fixture._directory.path().join("handler-workspaces"), handler_repository).unwrap();
            let handler = Arc::new(WorkUnitExecutionHarnessService::new(support.service(), sessions.clone(), handler_orchestration));
            std::thread::spawn(move || {
                let service = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(path, sessions).unwrap();
                service.attach_work_unit_handler_activation(handler)
            })
        }).collect::<Vec<_>>();
        assert!(concurrent_handler_services.into_iter().all(|call| call.join().unwrap().is_ok()));
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        let concurrent = (0..2).map(|_| { let service = replayed.clone(); let path = fixture.database_path.clone(); let sessions = fixture.sessions.clone(); std::thread::spawn(move || { drop(service); crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(path, sessions) }) }).collect::<Vec<_>>();
        assert!(concurrent.into_iter().all(|call| call.join().unwrap().is_ok()));
        // A retryable pre-terminal failure retains the prepared identity. Restoring the durable
        // Session invariant lets the same row become launch-accepted and ready exactly once.
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection.execute(
            "DELETE FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1",
            [&implementer.2],
        ).unwrap();
        connection.execute(
            "UPDATE agent_session_invocations
             SET status='pending',effective_options_json=NULL,started_at=NULL,completed_at=NULL,
                 exit_code=NULL,signal=NULL,runtime_error_json=NULL
             WHERE id=?1",
            [&implementer.2],
        ).unwrap();
        connection.execute(
            "UPDATE work_unit_implementer_activations
             SET implementer_harness_bound_at=NULL,launch_requested_at=NULL,launch_accepted_at=NULL,
                 provider_activation_observed_at=NULL,implementer_ready_at=NULL,failure_reason=NULL
             WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        connection.execute(
            "UPDATE agent_sessions SET working_directory='retryable-conflict' WHERE id=?1",
            [&implementer.1],
        ).unwrap();
        drop(connection);
        assert!(matches!(
            handler_runner.request_work_unit_implementer_from_authenticated_continuation(&continuation.3),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Unavailable(_))
        ));
        let retryable_failure: (String, String, String, String, Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT attempt_id,implementer_session_id,implementer_invocation_id,failure_reason,implementer_ready_at
             FROM work_unit_implementer_activations WHERE work_unit_id=?1",
            [&root.0],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).unwrap();
        assert_eq!(retryable_failure.0, implementer.0);
        assert_eq!(retryable_failure.1, implementer.1);
        assert_eq!(retryable_failure.2, implementer.2);
        assert_eq!(retryable_failure.3, "implementer_session_creation_failed");
        assert!(retryable_failure.4.is_none());
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<String, _, _>(
            "SELECT status FROM agent_session_invocations WHERE id=?1", [&implementer.2], |row| row.get(0),
        ).unwrap(), "pending");
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 3);
        Connection::open(&fixture.database_path).unwrap().execute(
            "UPDATE agent_sessions SET working_directory=?2 WHERE id=?1",
            params![implementer.1, shared_working_directory],
        ).unwrap();
        handler_runner.request_work_unit_implementer_from_authenticated_continuation(&continuation.3).unwrap();
        let recovered_retryable: (String, String, String, Option<String>, Option<String>, Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT attempt_id,implementer_session_id,implementer_invocation_id,
                    failure_reason,launch_accepted_at,implementer_ready_at
             FROM work_unit_implementer_activations WHERE work_unit_id=?1",
            [&root.0],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        ).unwrap();
        assert_eq!(recovered_retryable.0, retryable_failure.0);
        assert_eq!(recovered_retryable.1, retryable_failure.1);
        assert_eq!(recovered_retryable.2, retryable_failure.2);
        assert!(recovered_retryable.3.is_none());
        assert!(recovered_retryable.4.is_some() && recovered_retryable.5.is_some());
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 4);
        // A failed start terminalizes this exact persisted Implementer invocation. Reopen and
        // replay preserve the row and all correlations, keep readiness absent, and never launch
        // a replacement or retry the terminal process.
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection.execute(
            "DELETE FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1",
            [&implementer.2],
        ).unwrap();
        connection.execute(
            "UPDATE agent_session_invocations
             SET status='pending',effective_options_json=NULL,started_at=NULL,completed_at=NULL,
                 exit_code=NULL,signal=NULL,runtime_error_json=NULL
             WHERE id=?1",
            [&implementer.2],
        ).unwrap();
        connection.execute(
            "UPDATE work_unit_implementer_activations
             SET launch_requested_at=NULL,launch_accepted_at=NULL,provider_activation_observed_at=NULL,
                 implementer_ready_at=NULL,failure_reason=NULL
             WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        drop(connection);
        fixture.runtime.fail_next_launch();
        assert!(matches!(
            handler_runner.request_work_unit_implementer_from_authenticated_continuation(&continuation.3),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Unavailable(_))
        ));
        let terminal_failure: (String, String, String, String, String, Option<String>, Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT attempt_id,implementer_session_id,implementer_invocation_id,
                    implementer_harness_revision_id, failure_reason,launch_accepted_at,implementer_ready_at
             FROM work_unit_implementer_activations WHERE work_unit_id=?1",
            [&root.0],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
        ).unwrap();
        assert_eq!(terminal_failure.0, implementer.0);
        assert_eq!(terminal_failure.1, implementer.1);
        assert_eq!(terminal_failure.2, implementer.2);
        assert_eq!(terminal_failure.3, implementer.3);
        assert_eq!(terminal_failure.4, "implementer_launch_not_accepted");
        assert!(terminal_failure.5.is_none() && terminal_failure.6.is_none());
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<String, _, _>(
            "SELECT status FROM agent_session_invocations WHERE id=?1", [&implementer.2], |row| row.get(0),
        ).unwrap(), "failed");
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 5);
        let terminal_reopen = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        assert!(terminal_reopen.attach_work_unit_handler_activation(handler.clone()).is_err());
        assert!(matches!(
            terminal_reopen.request_work_unit_implementer_from_authenticated_continuation(&continuation.3),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Unavailable(_))
        ));
        let replayed_terminal_failure: (String, String, String, String, Option<String>, Option<String>) = Connection::open(&fixture.database_path).unwrap().query_row(
            "SELECT attempt_id,implementer_session_id,implementer_invocation_id,
                    implementer_harness_revision_id,launch_accepted_at,implementer_ready_at
             FROM work_unit_implementer_activations WHERE work_unit_id=?1",
            [&root.0],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        ).unwrap();
        assert_eq!(replayed_terminal_failure.0, terminal_failure.0);
        assert_eq!(replayed_terminal_failure.1, terminal_failure.1);
        assert_eq!(replayed_terminal_failure.2, terminal_failure.2);
        assert_eq!(replayed_terminal_failure.3, terminal_failure.3);
        assert!(replayed_terminal_failure.4.is_none() && replayed_terminal_failure.5.is_none());
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 5);

        // A terminal failure of the same-Session Handler action continuation is also durable and
        // factual. Reopen keeps the pinned action identity, projects the failure reason, and does
        // not launch a replacement or retry the terminal invocation.
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection.execute(
            "DELETE FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1",
            [&continuation.3],
        ).unwrap();
        connection.execute(
            "UPDATE agent_session_invocations
             SET status='pending',effective_options_json=NULL,started_at=NULL,completed_at=NULL,
                 exit_code=NULL,signal=NULL,runtime_error_json=NULL
             WHERE id=?1",
            [&continuation.3],
        ).unwrap();
        connection.execute(
            "UPDATE work_unit_handler_action_continuations
             SET launch_requested_at=NULL,launch_accepted_at=NULL,
                 provider_activation_observed_at=NULL,action_ready_at=NULL,failure_reason=NULL
             WHERE work_unit_id=?1",
            [&root.0],
        ).unwrap();
        drop(connection);
        fixture.runtime.fail_next_launch();
        let failed_action = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path, fixture.sessions.clone(),
        ).unwrap();
        assert!(failed_action.attach_work_unit_handler_activation(handler.clone()).is_err());
        let action_terminal_failure: (String, String, String, Option<String>, Option<String>) =
            Connection::open(&fixture.database_path).unwrap().query_row(
                "SELECT action_invocation_id,failure_reason,
                        (SELECT status FROM agent_session_invocations WHERE id=action_invocation_id),
                        launch_accepted_at,action_ready_at
                 FROM work_unit_handler_action_continuations WHERE work_unit_id=?1",
                [&root.0],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            ).unwrap();
        assert_eq!(action_terminal_failure.0, continuation.3);
        assert_eq!(action_terminal_failure.1, "handler_action_launch_not_accepted");
        assert_eq!(action_terminal_failure.2, "failed");
        assert!(action_terminal_failure.3.is_none() && action_terminal_failure.4.is_none());
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 6);
        let failed_action_reopen = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path, fixture.sessions.clone(),
        ).unwrap();
        assert!(failed_action_reopen.attach_work_unit_handler_activation(handler.clone()).is_err());
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<String, _, _>(
            "SELECT failure_reason FROM work_unit_handler_action_continuations WHERE work_unit_id=?1",
            [&root.0], |row| row.get(0),
        ).unwrap(), action_terminal_failure.1);
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 6);
        let failed_action_projection = serde_json::to_value(
            SqliteOrchestrationRepository::open(&fixture.database_path).unwrap().native_query().unwrap(),
        ).unwrap();
        let projected_failed_action = failed_action_projection["workUnits"].as_array().unwrap().iter()
            .find(|unit| unit["workUnitId"] == root.0).unwrap();
        assert_eq!(
            projected_failed_action["actionContinuation"]["failureReason"],
            "handler_action_launch_not_accepted",
        );
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection.execute("UPDATE work_slice_proposal_revisions SET is_current=0 WHERE revision_id=?1", [&materialization.2]).unwrap();
        drop(connection);
        assert!(matches!(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(&fixture.database_path, fixture.sessions.clone()), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)));
        assert_eq!(fixture.runtime.requests().len(), handler_launches_before + 6);
    }

    #[test]
    fn accepted_review_terminal_dispatch_drains_newly_eligible_dependent_in_same_activation() {
        let fixture = Fixture::new();
        let (service, planner, sprint_id) = fixture.prepare_work_slice_planner();
        let lane = |title: &str, depends_on: Vec<&str>| {
            crate::orchestration::sprint_runner_transition::WorkSliceLane {
                title: title.into(),
                specification: format!("Drain fixture for {title}."),
                depends_on: depends_on.into_iter().map(str::to_owned).collect(),
            }
        };
        service
            .submit_work_slice_proposal(
                &planner,
                crate::orchestration::sprint_runner_transition::WorkSliceProposal {
                    objective: "Prove bounded Handler graph draining.".into(),
                    lanes: vec![
                        lane("root-a", vec![]),
                        lane("root-b", vec![]),
                        lane("middle", vec!["root-a"]),
                        lane("leaf", vec!["middle"]),
                    ],
                },
            )
            .unwrap();
        service
            .complete_work_slice_planning(
                &planner,
                crate::orchestration::sprint_runner_transition::WorkSliceCompletion {},
            )
            .unwrap();
        fixture
            .runtime
            .finish(planner.as_str(), AgentInvocationTerminalStatus::Completed);

        let repository_root = fixture._directory.path().join("drain-repository");
        let sprint_root = fixture._directory.path().join("drain-sprint-worktree");
        fs::create_dir_all(&repository_root).unwrap();
        let git = |root: &Path, arguments: &[&str]| {
            let output = std::process::Command::new("git")
                .args(arguments)
                .current_dir(root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        };
        git(&repository_root, &["init"]);
        git(&repository_root, &["config", "user.email", "drain@example.test"]);
        git(&repository_root, &["config", "user.name", "Drain Test"]);
        fs::write(repository_root.join("README.md"), "drain base\n").unwrap();
        git(&repository_root, &["add", "README.md"]);
        git(&repository_root, &["commit", "-m", "drain base"]);
        let initial = git(&repository_root, &["rev-parse", "HEAD"]);
        git(
            &repository_root,
            &[
                "worktree",
                "add",
                "-b",
                "drain-sprint",
                sprint_root.to_string_lossy().as_ref(),
                &initial,
            ],
        );
        fs::write(sprint_root.join("README.md"), "drain sprint\n").unwrap();
        git(&sprint_root, &["add", "README.md"]);
        git(&sprint_root, &["commit", "-m", "drain sprint"]);
        let current = git(&sprint_root, &["rev-parse", "HEAD"]);
        let repository_root = repository_root.canonicalize().unwrap();
        let sprint_root = sprint_root.canonicalize().unwrap();
        // The planning helper installs a deliberately non-Git route solely to authorize the
        // Planner fixture. Replace it before exercising the actual Handler workspace boundary.
        Connection::open(&fixture.database_path)
            .unwrap()
            .execute(
                "DELETE FROM initiated_sprint_git_authorities WHERE sprint_id=?1",
                [&sprint_id],
            )
            .unwrap();
        let authority = match SqliteOrchestrationRepository::open(&fixture.database_path)
            .unwrap()
            .store_initiated_sprint_git_authority(InitiatedSprintGitAuthorityWrite {
                sprint_id: sprint_id.clone(),
                idempotency_key: "handler-drain-authority".into(),
                repository_id: "handler-drain-repository".into(),
                repository_root: repository_root.to_string_lossy().into_owned(),
                repository_common_dir: repository_root
                    .join(".git")
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                worktree_id: "handler-drain-sprint-worktree".into(),
                worktree_root: sprint_root.to_string_lossy().into_owned(),
                baseline_object_id: initial.clone(),
                current_object_id: current,
                runtime_instance_ref: "handler-drain-runtime".into(),
                runtime_source_ref: "handler-drain-source".into(),
                source_fingerprint: "d".repeat(64),
            })
            .unwrap()
        {
            crate::orchestration::repository::StoreInitiatedSprintGitAuthorityResult::Stored {
                authority_id,
            }
            | crate::orchestration::repository::StoreInitiatedSprintGitAuthorityResult::IdempotentReplay {
                authority_id,
            } => authority_id,
        };
        let repository = Arc::new(SqliteOrchestrationRepository::open(&fixture.database_path).unwrap());
        let handler = Arc::new(WorkUnitExecutionHarnessService::new(
            ProductExecutionSupportState::new(
                &fixture.database_path,
                fixture._directory.path().join("drain-workspaces"),
                repository.clone(),
            )
            .unwrap()
            .service(),
            fixture.sessions.clone(),
            Arc::new(OrchestrationApplication::new(repository)),
        ));
        let materialization: (String, String) = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row(
                "SELECT materialization_id,accepted_revision_id FROM work_unit_materializations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let units = Connection::open(&fixture.database_path)
            .unwrap()
            .prepare("SELECT work_unit_id,lane_title FROM work_units WHERE materialization_id=?1")
            .unwrap()
            .query_map([&materialization.0], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect::<Result<HashMap<_, _>, _>>()
            .unwrap();
        let root_a = units
            .iter()
            .find_map(|(id, title)| (title == "root-a").then_some(id.clone()))
            .unwrap();
        let middle = units
            .iter()
            .find_map(|(id, title)| (title == "middle").then_some(id.clone()))
            .unwrap();
        let higher_continuations_before = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM sprint_runner_transitions
                 WHERE epic_continuation_invocation_id IS NOT NULL
                    OR sprint_continuation_invocation_id IS NOT NULL
                    OR planning_control_invocation_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let post_pass_calls = Arc::new(AtomicUsize::new(0));
        service.set_test_work_unit_handler_post_pass_hook(Arc::new({
            let database_path = fixture.database_path.clone();
            let materialization_id = materialization.0.clone();
            let authority = authority.clone();
            let initial = initial.clone();
            let root_a = root_a.clone();
            let middle = middle.clone();
            let post_pass_calls = post_pass_calls.clone();
            move || {
                assert_eq!(post_pass_calls.fetch_add(1, Ordering::SeqCst), 0);
                let connection = Connection::open(&database_path).unwrap();
                connection.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
                let integration = format!("drain-integration-{root_a}");
                connection.execute(
                    "INSERT INTO accepted_work_unit_integrations (integration_id,work_unit_id,candidate_id,authority_id,target_ref_name,pre_object_id,pre_version,candidate_commit_id,candidate_tree_id,baseline_object_id,intent_fingerprint,intent_recorded_at,authorization_recorded_at,stage,integration_commit_id,integration_tree_id,object_created_at,ref_advanced_at,runtime_advanced_at,db_advanced_at,settled_at,notification_intent_recorded_at) VALUES (?1,?2,?3,?4,'refs/heads/drain',?5,1,?5,?6,?5,?7,'t','t','settled',?5,?6,'t','t','t','t','t','t')",
                    params![integration, root_a, format!("drain-candidate-{root_a}"), authority, initial, "e".repeat(40), format!("drain-intent-{root_a}")],
                ).unwrap();
                connection.execute(
                    "INSERT INTO work_unit_settlements(settlement_id,work_unit_id,integration_id,settled_at) VALUES(?1,?2,?3,'t')",
                    params![format!("drain-settlement-{root_a}"), root_a, integration],
                ).unwrap();
                let edge: String = connection.query_row(
                    "SELECT relationship_id FROM work_unit_relationships WHERE materialization_id=?1 AND relationship_kind='depends_on' AND from_id=?2 AND to_id=?3",
                    params![materialization_id, middle, root_a],
                    |row| row.get(0),
                ).unwrap();
                connection.execute(
                    "INSERT INTO work_unit_prerequisite_contributions(contribution_id,prerequisite_work_unit_id,dependent_work_unit_id,integration_id,relationship_id,recorded_at) VALUES(?1,?2,?3,?4,?5,'t')",
                    params![format!("drain-contribution-{root_a}"), root_a, middle, integration, edge],
                ).unwrap();
                connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
            }
        }));
        fixture.notifier.set_sprint(&service);
        service.attach_reporting_test_harness(handler.clone());
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        connection.execute(
            "INSERT INTO work_unit_handler_reviews (
                attempt_id,work_unit_id,reporting_invocation_id,handler_session_id,
                original_handler_invocation_id,action_handler_invocation_id,review_invocation_id,
                review_harness_revision_id,review_harness_configuration_digest,
                review_harness_repository_commit_ref,delivery_requested_at,launch_accepted_at,
                delivered_payload_json,delivered_payload_fingerprint,semantic_judgment_variant,
                semantic_judgment_fingerprint,semantic_judgment_at,lifecycle_observed_at,
                lifecycle_status
             ) VALUES ('accepted-review-terminal-attempt',?1,'accepted-review-reporting',
                       'accepted-review-session','accepted-review-handler',
                       'accepted-review-action','accepted-review-terminal-drain',
                       'accepted-review-revision','accepted-review-digest',
                       'accepted-review-commit','t','t','{}','accepted-review-delivery',
                       'accept','accepted-review-judgment','t','t','completed')",
            [&root_a],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_handler_decisions (
                review_invocation_id,attempt_id,work_unit_id,decision_variant,
                decision_fingerprint,decision_recorded_at,implementation_accepted_at
             ) VALUES ('accepted-review-terminal-drain','accepted-review-terminal-attempt',?1,
                       'accepted','accepted-review-terminal-decision','t','t')",
            [&root_a],
        ).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        drop(connection);
        service
            .reconcile_handler_review_terminal_movement_for_test(
                "accepted-review-terminal-drain",
            )
            .unwrap();

        let materialization: (String, String) = Connection::open(&fixture.database_path)
            .unwrap()
            .query_row(
                "SELECT materialization_id,accepted_revision_id FROM work_unit_materializations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let units = Connection::open(&fixture.database_path)
            .unwrap()
            .prepare("SELECT work_unit_id,lane_title FROM work_units WHERE materialization_id=?1")
            .unwrap()
            .query_map([&materialization.0], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect::<Result<HashMap<_, _>, _>>()
            .unwrap();
        let root_a = units
            .iter()
            .find_map(|(id, title)| (title == "root-a").then_some(id.clone()))
            .unwrap();
        let root_b = units
            .iter()
            .find_map(|(id, title)| (title == "root-b").then_some(id.clone()))
            .unwrap();
        let middle = units
            .iter()
            .find_map(|(id, title)| (title == "middle").then_some(id.clone()))
            .unwrap();
        let leaf = units
            .iter()
            .find_map(|(id, title)| (title == "leaf").then_some(id.clone()))
            .unwrap();
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_handler_activations WHERE eligibility_state='eligible'",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            3
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_handler_activations WHERE work_unit_id IN (?1,?2) AND handler_ready_at IS NOT NULL",
                    params![root_a, root_b],
                    |row| row.get(0),
                )
                .unwrap(),
            2
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_handler_activations WHERE work_unit_id=?1 AND eligibility_state='blocked'",
                    [&leaf],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(post_pass_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_handler_activations WHERE work_unit_id=?1 AND eligibility_state='eligible' AND launch_accepted_at IS NOT NULL AND handler_ready_at IS NOT NULL",
                    [&middle],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<String, _, _>(
                    "SELECT execution_state FROM work_unit_execution_states WHERE work_unit_id=?1",
                    [&middle],
                    |row| row.get(0),
                )
                .unwrap(),
            "active"
        );
        for unit in [&leaf] {
            assert_eq!(
                connection
                    .query_row::<String, _, _>(
                        "SELECT execution_state FROM work_unit_execution_states WHERE work_unit_id=?1",
                        [unit],
                        |row| row.get(0),
                    )
                    .unwrap(),
                "waiting_on_prerequisites",
                "{unit}"
            );
        }
        for table in [
            "work_slice_execution_graph_completions",
            "work_slice_execution_settlements",
            "work_slice_planning_point_execution_settlements",
        ] {
            assert_eq!(
                connection
                    .query_row::<i64, _, _>(
                        &format!("SELECT COUNT(*) FROM {table}"),
                        [],
                        |row| row.get(0),
                    )
                    .unwrap(),
                0,
                "{table}"
            );
        }
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM sprint_runner_transitions
                     WHERE epic_continuation_invocation_id IS NOT NULL
                        OR sprint_continuation_invocation_id IS NOT NULL
                        OR planning_control_invocation_id IS NOT NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            higher_continuations_before
        );
        drop(connection);

        // The first root contribution became durable inside the prior Handler reconciliation
        // post-pass. The second durable generation below remains the missed-notification reopen
        // proof; neither boundary duplicates the existing root or middle Handler activations.
        let seed_accepted_generation = |unit: &str, dependent: &str| {
            let connection = Connection::open(&fixture.database_path).unwrap();
            connection.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
            let integration = format!("drain-integration-{unit}");
            connection.execute(
                "INSERT INTO accepted_work_unit_integrations (integration_id,work_unit_id,candidate_id,authority_id,target_ref_name,pre_object_id,pre_version,candidate_commit_id,candidate_tree_id,baseline_object_id,intent_fingerprint,intent_recorded_at,authorization_recorded_at,stage,integration_commit_id,integration_tree_id,object_created_at,ref_advanced_at,runtime_advanced_at,db_advanced_at,settled_at,notification_intent_recorded_at) VALUES (?1,?2,?3,?4,'refs/heads/drain',?5,1,?5,?6,?5,?7,'t','t','settled',?5,?6,'t','t','t','t','t','t')",
                params![integration, unit, format!("drain-candidate-{unit}"), authority, initial, "e".repeat(40), format!("drain-intent-{unit}")],
            ).unwrap();
            connection.execute(
                "INSERT INTO work_unit_settlements(settlement_id,work_unit_id,integration_id,settled_at) VALUES(?1,?2,?3,'t')",
                params![format!("drain-settlement-{unit}"), unit, integration],
            ).unwrap();
            let edge: String = connection.query_row(
                "SELECT relationship_id FROM work_unit_relationships WHERE materialization_id=?1 AND relationship_kind='depends_on' AND from_id=?2 AND to_id=?3",
                params![materialization.0, dependent, unit],
                |row| row.get(0),
            ).unwrap();
            connection.execute(
                "INSERT INTO work_unit_prerequisite_contributions(contribution_id,prerequisite_work_unit_id,dependent_work_unit_id,integration_id,relationship_id,recorded_at) VALUES(?1,?2,?3,?4,?5,'t')",
                params![format!("drain-contribution-{unit}"), unit, dependent, integration, edge],
            ).unwrap();
            connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        };
        // Simulate a crash after the root-b activation persisted but before its final readiness
        // projection. Reopen must adopt that partial effect and keep it out of attention.
        Connection::open(&fixture.database_path)
            .unwrap()
            .execute(
                "UPDATE work_unit_handler_activations SET handler_ready_at=NULL WHERE work_unit_id=?1",
                [&root_b],
            )
            .unwrap();
        let reopened = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        )
        .unwrap();
        fixture.notifier.set_sprint(&reopened);
        reopened
            .attach_work_unit_handler_activation(handler.clone())
            .unwrap();
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_activations WHERE work_unit_id IN (?1,?2) AND handler_ready_at IS NOT NULL", params![root_a, root_b], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_activations WHERE work_unit_id=?1 AND eligibility_state='eligible' AND handler_ready_at IS NOT NULL", [&middle], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_execution_attentions", [], |row| row.get(0)).unwrap(), 0);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_activations WHERE work_unit_id IN (?1,?2)", params![root_a, root_b], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<String, _, _>("SELECT execution_state FROM work_unit_execution_states WHERE work_unit_id=?1", [&middle], |row| row.get(0)).unwrap(), "active");
        assert_eq!(connection.query_row::<String, _, _>("SELECT execution_state FROM work_unit_execution_states WHERE work_unit_id=?1", [&leaf], |row| row.get(0)).unwrap(), "waiting_on_prerequisites");
        drop(connection);

        seed_accepted_generation(&middle, &leaf);
        // No callback is delivered for the second accepted generation. A fresh service instance
        // must discover the durable contribution and activate the leaf without Sprint/Epic work.
        let missed_notification_reopen = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        )
        .unwrap();
        fixture.notifier.set_sprint(&missed_notification_reopen);
        missed_notification_reopen
            .attach_work_unit_handler_activation(handler)
            .unwrap();
        let connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_activations WHERE eligibility_state='eligible' AND handler_ready_at IS NOT NULL", [], |row| row.get(0)).unwrap(), 4);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(DISTINCT attempt_id) FROM work_unit_handler_activations", [], |row| row.get(0)).unwrap(), 4);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM accepted_work_unit_integrations WHERE stage='settled'", [], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_settlements", [], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_prerequisite_contributions", [], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_activations WHERE work_unit_id=?1", [&leaf], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<String, _, _>("SELECT execution_state FROM work_unit_execution_states WHERE work_unit_id=?1", [&leaf], |row| row.get(0)).unwrap(), "active");
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('sprint_settlements','epic_settlements')", [], |row| row.get(0)).unwrap(), 0);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_execution_settlements", [], |row| row.get(0)).unwrap(), 0);
        assert_eq!(connection.query_row::<String, _, _>("SELECT accepted_revision_id FROM work_unit_execution_states WHERE work_unit_id=?1", [&leaf], |row| row.get(0)).unwrap(), materialization.1);
    }

    fn terminal_authority_fingerprint(parts: &[&str]) -> String {
        let mut hash = Sha256::new();
        for part in parts {
            hash.update((part.len() as u64).to_be_bytes());
            hash.update(part.as_bytes());
        }
        format!("{:x}", hash.finalize())
    }

    fn terminal_authority_fingerprint_bytes(prefix: &str, value: &[u8]) -> String {
        let hex = value.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let mut hash = Sha256::new();
        hash.update(prefix.as_bytes());
        hash.update([0]);
        hash.update(hex.as_bytes());
        format!("{prefix}-{:x}", hash.finalize())
    }

    fn terminal_authority_projection_id(prefix: &str, value: &str) -> String {
        let mut hash = Sha256::new();
        hash.update(prefix.as_bytes());
        hash.update([0]);
        hash.update(value.as_bytes());
        format!("{prefix}-{:x}", hash.finalize())
    }

    fn terminal_authority_base64(value: &[u8]) -> String {
        const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut encoded = String::new();
        for chunk in value.chunks(3) {
            let first = chunk[0];
            let second = *chunk.get(1).unwrap_or(&0);
            let third = *chunk.get(2).unwrap_or(&0);
            encoded.push(TABLE[(first >> 2) as usize] as char);
            encoded.push(TABLE[(((first & 0b0000_0011) << 4) | (second >> 4)) as usize] as char);
            encoded.push(if chunk.len() > 1 { TABLE[(((second & 0b0000_1111) << 2) | (third >> 6)) as usize] as char } else { '=' });
            encoded.push(if chunk.len() > 2 { TABLE[(third & 0b0011_1111) as usize] as char } else { '=' });
        }
        encoded
    }

    fn terminal_authority_git(root: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "NUL")
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    /// The frontend fixture is deliberately an execution-graph fixture, not a transcript of the
    /// private Handler/Implementer histories required to reach it. This projection preserves every
    /// public graph field it consumes and maps only generated identities and timestamps to its
    /// stable fixture vocabulary.
    fn normalized_terminal_execution_projection(query: &serde_json::Value) -> serde_json::Value {
        let materializations = query["workUnitMaterializations"].as_array().unwrap();
        assert_eq!(materializations.len(), 1);
        let materialization = &materializations[0];
        let materialization_id = materialization["materializationId"].as_str().unwrap();
        let planning_point_id = materialization["planningPointId"].as_str().unwrap();
        let accepted_revision_id = materialization["acceptedRevisionId"].as_str().unwrap();
        let sprint_id = materialization["sprintId"].as_str().unwrap();
        let work_slice_id = materialization["workSliceId"].as_str().unwrap();
        let mut unit_ids = HashMap::new();
        for unit in query["workUnits"].as_array().unwrap() {
            let fixture_id = match unit["laneTitle"].as_str().unwrap() {
                "Root A" => "execution-root-a",
                "Root B" => "execution-root-b",
                "Middle" => "execution-middle",
                "Leaf" => "execution-leaf",
                title => panic!("unexpected terminal fixture lane {title}"),
            };
            assert!(unit_ids.insert(unit["workUnitId"].as_str().unwrap(), fixture_id).is_none());
        }
        assert_eq!(unit_ids.len(), 4);
        let normalize = |value: &str| -> String {
            if value == materialization_id { "execution-materialization-fixture".into() }
            else if value == planning_point_id { "execution-planning-point-fixture".into() }
            else if value == accepted_revision_id { "execution-accepted-revision-fixture".into() }
            else if value == sprint_id { "sprint-fixture".into() }
            else if value == work_slice_id { "execution-work-slice-fixture".into() }
            else { unit_ids.get(value).unwrap_or_else(|| panic!("foreign execution identity {value}")).to_string() }
        };
        let sort = |values: Vec<serde_json::Value>| {
            let mut values = values;
            values.sort_by_key(|value| serde_json::to_string(value).unwrap());
            values
        };
        let work_units = sort(query["workUnits"].as_array().unwrap().iter().map(|unit| serde_json::json!({
            "workUnitId": normalize(unit["workUnitId"].as_str().unwrap()),
            "materializationId": normalize(unit["materializationId"].as_str().unwrap()),
            "workSliceId": normalize(unit["workSliceId"].as_str().unwrap()),
            "acceptedRevisionId": normalize(unit["acceptedRevisionId"].as_str().unwrap()),
            "laneOrdinal": unit["laneOrdinal"],
            "laneTitle": unit["laneTitle"],
            "specification": unit["specification"],
        })).collect());
        let relationships = sort(query["workUnitRelationships"].as_array().unwrap().iter().map(|relationship| {
            let kind = relationship["relationshipKind"].as_str().unwrap();
            let from = normalize(relationship["fromId"].as_str().unwrap());
            let to = normalize(relationship["toId"].as_str().unwrap());
            let relationship_id = match kind {
                "planning_point" => "execution-point".into(),
                "sprint" => "execution-sprint".into(),
                "lane" | "order" => format!("execution-{kind}-{}", to.strip_prefix("execution-").unwrap()),
                "depends_on" => format!("execution-dep-{}", from.strip_prefix("execution-").unwrap()),
                _ => panic!("unexpected execution relationship {kind}"),
            };
            serde_json::json!({
                "relationshipId": relationship_id,
                "materializationId": normalize(relationship["materializationId"].as_str().unwrap()),
                "relationshipKind": kind,
                "fromId": from,
                "toId": to,
                "ordinal": relationship.get("ordinal").cloned().unwrap_or(serde_json::Value::Null),
            })
        }).collect());
        let activation_intents = sort(query["dependencyActivationIntents"].as_array().unwrap().iter().map(|intent| serde_json::json!({
            "workUnitId": normalize(intent["workUnitId"].as_str().unwrap()),
            "materializationId": normalize(intent["materializationId"].as_str().unwrap()),
            "acceptedRevisionId": normalize(intent["acceptedRevisionId"].as_str().unwrap()),
            "eligibilityState": intent["eligibilityState"],
            "eligibilityRecordedAt": "<timestamp>",
            "activationIntendedAt": "<timestamp>",
        })).collect());
        let states = sort(query["workUnitExecutionStates"].as_array().unwrap().iter().map(|state| serde_json::json!({
            "workUnitId": normalize(state["workUnitId"].as_str().unwrap()),
            "materializationId": normalize(state["materializationId"].as_str().unwrap()),
            "acceptedRevisionId": normalize(state["acceptedRevisionId"].as_str().unwrap()),
            "state": state["state"],
            "recordedAt": "<timestamp>",
        })).collect());
        serde_json::json!({
            "workUnitMaterializations": [{
                "materializationId": "execution-materialization-fixture",
                "planningPointId": "execution-planning-point-fixture",
                "acceptedRevisionId": "execution-accepted-revision-fixture",
                "epicId": "<epic-id>",
                "sprintId": "sprint-fixture",
                "workSliceId": "execution-work-slice-fixture",
                "authorizationRecordedAt": "<timestamp>",
                "attemptRecordedAt": "<timestamp>",
                "workUnitsCreatedAt": "<timestamp>",
                "relationshipsCompletedAt": "<timestamp>",
                "settledAt": "<timestamp>",
            }],
            "workUnits": work_units,
            "workUnitRelationships": relationships,
            "dependencyActivationIntents": activation_intents,
            "workUnitExecutionStates": states,
            "workSliceExecutionGraphCompletions": [{
                "materializationId": "execution-materialization-fixture",
                "acceptedRevisionId": "execution-accepted-revision-fixture",
                "completedAt": "<timestamp>",
            }],
            "workSliceExecutionSettlements": [{
                "materializationId": "execution-materialization-fixture",
                "graphCompletionMaterializationId": "execution-materialization-fixture",
                "settledAt": "<timestamp>",
            }],
            "workSlicePlanningPointExecutionSettlements": [{
                "planningPointId": "execution-planning-point-fixture",
                "materializationId": "execution-materialization-fixture",
                "workSliceExecutionMaterializationId": "execution-materialization-fixture",
                "settledAt": "<timestamp>",
            }],
            "workSliceExecutionAttentions": [],
        })
    }

    /// Creates the private accepted-Handler lineage that the real retained-candidate and
    /// accepted-integration reconcilers consume. Product code, rather than this helper, owns
    /// candidate pinning, target advancement, settlement, and prerequisite contribution rows.
    fn record_terminal_authority_candidate(
        fixture: &Fixture,
        repository_root: &Path,
        baseline: &str,
        authority_id: &str,
        work_unit_id: &str,
        ordinal: usize,
    ) -> PathBuf {
        let connection = Connection::open(&fixture.database_path).unwrap();
        let (materialization_id, sprint_id, epic_id, provenance_id): (String, String, String, String) = connection
            .query_row(
                "SELECT m.materialization_id,m.sprint_id,m.epic_id,e.provenance_id
                   FROM work_unit_materializations m
                   JOIN epic_initiations e ON e.epic_id=m.epic_id
                  WHERE m.materialization_id=(SELECT materialization_id FROM work_units WHERE work_unit_id=?1)",
                [work_unit_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        let attempt_root = fixture._directory.path().join(format!("terminal-attempt-{ordinal}"));
        terminal_authority_git(
            repository_root,
            &["worktree", "add", "-b", &format!("terminal-candidate-{ordinal}"), attempt_root.to_string_lossy().as_ref(), baseline],
        );
        let content = format!("terminal candidate {ordinal}\n");
        let filename = format!("terminal-{ordinal}.txt");
        fs::write(attempt_root.join(&filename), &content).unwrap();
        terminal_authority_git(&attempt_root, &["add", &filename]);
        terminal_authority_git(&attempt_root, &["commit", "-m", &format!("terminal candidate {ordinal}")]);
        let candidate = terminal_authority_git(&attempt_root, &["rev-parse", "HEAD"]);

        let suffix = ordinal.to_string();
        let attempt_id = format!("terminal-attempt-{suffix}");
        let reporting = terminal_authority_projection_id(
            "work-unit-implementer-reporting-invocation",
            &attempt_id,
        );
        let review = terminal_authority_projection_id(
            "work-unit-handler-review-invocation",
            &attempt_id,
        );
        let document = format!("terminal-document-{suffix}");
        let artifact = format!("terminal-artifact-{suffix}");
        let capture = format!("terminal-capture-{suffix}");
        let evidence_ref = format!("terminal-evidence-{suffix}");
        let payload = serde_json::to_vec(&serde_json::json!({
            "files": [{
                "changedFileReferenceId": evidence_ref,
                "content": {"encoding": "base64", "bytesBase64": terminal_authority_base64(content.as_bytes())}
            }]
        }))
        .unwrap();
        let payload_value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        let manifest = serde_json::json!([{
            "evidenceRef": evidence_ref,
            "displayName": filename,
            "changeKind": "added"
        }])
        .to_string();
        let contents = serde_json::json!([{
            "evidenceRef": evidence_ref,
            "contentFingerprint": terminal_authority_fingerprint_bytes(
                "implementer-evidence-content",
                &serde_json::to_vec(&payload_value["files"][0]).unwrap(),
            )
        }])
        .to_string();
        let comparison = terminal_authority_fingerprint_bytes("implementer-evidence-comparison", &payload);
        let review_payload = format!(
            r#"{{"summary":{},"validationStatement":"Local Git candidate captured.","changedFiles":[{{"evidenceRef":{},"displayName":{},"changeKind":"added"}}],"comparisonFingerprint":{},"evidenceContentFingerprints":[{{"evidenceRef":{},"contentFingerprint":{}}}]}}"#,
            serde_json::to_string(&format!("Accepted terminal candidate {ordinal}.")).unwrap(),
            serde_json::to_string(&evidence_ref).unwrap(),
            serde_json::to_string(&filename).unwrap(),
            serde_json::to_string(&comparison).unwrap(),
            serde_json::to_string(&evidence_ref).unwrap(),
            serde_json::to_string(&terminal_authority_fingerprint_bytes(
                "implementer-evidence-content",
                &serde_json::to_vec(&payload_value["files"][0]).unwrap(),
            )).unwrap(),
        );
        let delivery_fingerprint = terminal_authority_projection_id(
            "work-unit-handler-review-delivery",
            &review_payload,
        );
        let outcome_payload = format!(
            r#"{{"outcome":"review_pending","summary":{},"validationStatement":"Local Git candidate captured."}}"#,
            serde_json::to_string(&format!("Accepted terminal candidate {ordinal}.")).unwrap(),
        );
        let outcome_fingerprint = terminal_authority_projection_id("implementer-outcome", &outcome_payload);
        let repository_route = repository_root.to_string_lossy().to_string();
        let attempt_route = attempt_root.to_string_lossy().to_string();
        let capture_fingerprint = terminal_authority_fingerprint(&[
            &capture, &format!("terminal-capture-key-{suffix}"), &epic_id, &sprint_id, &provenance_id, "terminal-repository",
            &repository_route, &format!("terminal-attempt-worktree-{suffix}"), &attempt_route, baseline, &candidate,
        ]);
        let now = "2026-08-05T00:00:00Z";
        connection.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        connection.execute(
            "INSERT INTO work_unit_handler_activations
               (work_unit_id,materialization_id,sprint_id,attempt_id,handler_session_id,
                handler_invocation_id,handler_harness_key,handler_harness_version,
                eligibility_state,requested_at)
             VALUES(?1,?2,?3,?4,?5,?6,'terminal-handler',1,'eligible',?7)",
            params![work_unit_id, materialization_id, sprint_id, attempt_id, format!("terminal-handler-session-{suffix}"), format!("terminal-handler-invocation-{suffix}"), now],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_handler_action_continuations
               (work_unit_id,attempt_id,handler_session_id,original_handler_invocation_id,
                action_invocation_id,action_harness_revision_id,action_harness_configuration_digest,
                action_harness_repository_commit_ref,requested_at)
             VALUES(?1,?2,?3,?4,?5,'terminal-action-revision','terminal-action-digest',
                    'terminal-action-commit',?6)",
            params![work_unit_id, attempt_id, format!("terminal-handler-session-{suffix}"), format!("terminal-handler-invocation-{suffix}"), format!("terminal-handler-action-{suffix}"), now],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_implementer_activations
               (work_unit_id,handler_attempt_id,handler_invocation_id,attempt_id,
                implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,
                implementer_harness_configuration_digest,implementer_harness_repository_commit_ref,
                requested_at,authorized_at,execution_support_granted_at,isolated_worktree_ready_at,
                implementer_session_created_at,implementer_invocation_prepared_at,
                implementer_harness_bound_at,launch_requested_at,launch_accepted_at,implementer_ready_at)
             VALUES(?1,?2,?3,?2,?4,?5,'terminal-implementer-revision',
                    'terminal-implementer-digest','terminal-implementer-commit',?6,?6,?6,?6,?6,?6,?6,?6,?6,?6)",
            params![work_unit_id, attempt_id, format!("terminal-handler-action-{suffix}"), format!("terminal-implementer-session-{suffix}"), format!("terminal-implementer-invocation-{suffix}"), now],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_handler_reviews
               (work_unit_id,attempt_id,reporting_invocation_id,handler_session_id,
                original_handler_invocation_id,action_handler_invocation_id,
                review_invocation_id,review_harness_revision_id,
                review_harness_configuration_digest,review_harness_repository_commit_ref,
                delivery_requested_at,delivery_persisted_at,harness_bound_at,launch_requested_at,
                launch_accepted_at,review_ready_at,delivered_payload_json,delivered_payload_fingerprint,
                semantic_judgment_variant,semantic_judgment_fingerprint,semantic_judgment_at,
                lifecycle_observed_at,lifecycle_status)
             VALUES(?1,?2,?3,?4,?5,?6,?7,'terminal-review-revision',
                    'terminal-review-digest','terminal-review-commit',?8,?8,?8,?8,?8,?8,?9,?10,'accept',?11,?8,?8,'completed')",
            params![work_unit_id, attempt_id, reporting, format!("terminal-handler-session-{suffix}"), format!("terminal-handler-invocation-{suffix}"), format!("terminal-handler-action-{suffix}"), review, now, review_payload, delivery_fingerprint, format!("terminal-review-judgment-{suffix}")],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_handler_decisions
               (work_unit_id,attempt_id,review_invocation_id,decision_variant,decision_fingerprint,
                decision_recorded_at,implementation_accepted_at)
             VALUES(?1,?2,?3,'accepted',?4,?5,?5)",
            params![work_unit_id, attempt_id, review, format!("terminal-decision-{suffix}"), now],
        ).unwrap();
        connection.execute(
            "INSERT INTO execution_support_grants
               (attempt_id,capability_ref,epic_id,sprint_id,work_unit_id,repository_id,role_id,
                workspace_id,workspace_fingerprint,correlation_fingerprint,recorded_at)
             VALUES(?1,?2,?3,?4,?5,'terminal-repository','work_unit_implementer',
                    ?6,'terminal-workspace-fingerprint','terminal-correlation-fingerprint',?7)",
            params![attempt_id, format!("terminal-capability-{suffix}"), epic_id, sprint_id, work_unit_id, format!("terminal-attempt-worktree-{suffix}"), now],
        ).unwrap();
        connection.execute(
            "INSERT INTO execution_support_attempt_authorizations
               (attempt_id,work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id,
                authorization_fingerprint,recorded_at)
             VALUES(?1,?2,'work_unit_implementer',?3,?4,?5,?6)",
            params![attempt_id, work_unit_id, authority_id, baseline, format!("terminal-authorization-{suffix}"), now],
        ).unwrap();
        connection.execute(
            "INSERT INTO file_review_documents
               (document_ref_id,epic_id,sprint_id,provenance_id,opaque_reference,title,
                idempotency_key,payload_fingerprint,recorded_at)
             VALUES(?1,?2,?3,?4,?5,'Terminal evidence',?6,?7,?8)",
            params![document, epic_id, sprint_id, provenance_id, format!("terminal-opaque-{suffix}"), format!("terminal-document-key-{suffix}"), format!("terminal-document-fingerprint-{suffix}"), now],
        ).unwrap();
        connection.execute(
            "INSERT INTO file_review_changed_files
               (document_ref_id,changed_file_reference_id,display_name,change_kind,ordinal)
             VALUES(?1,?2,?3,'added',0)",
            params![document, evidence_ref, filename],
        ).unwrap();
        connection.execute(
            "INSERT INTO file_review_git_capture_authorizations
               (capture_authorization_id,idempotency_key,payload_fingerprint,epic_id,sprint_id,
                provenance_id,repository_id,repository_root,worktree_id,worktree_root,
                baseline_object_id,current_object_id,recorded_at)
             VALUES(?1,?2,?3,?4,?5,?6,'terminal-repository',?7,?8,?9,?10,?11,?12)",
            params![capture, format!("terminal-capture-key-{suffix}"), capture_fingerprint, epic_id, sprint_id, provenance_id, repository_route, format!("terminal-attempt-worktree-{suffix}"), attempt_route, baseline, candidate, now],
        ).unwrap();
        connection.execute(
            "INSERT INTO stored_file_review_artifacts
               (artifact_id,document_ref_id,contract_version,payload,payload_bytes,provenance_id)
             VALUES(?1,?2,'stored-file-review-artifact/v1',?3,?4,?5)",
            params![artifact, document, payload, payload.len() as i64, provenance_id],
        ).unwrap();
        connection.execute(
            "INSERT INTO file_review_git_capture_documents
               (capture_authorization_id,document_ref_id,artifact_id,linkage_fingerprint,recorded_at)
             VALUES(?1,?2,?3,?4,?5)",
            params![capture, document, artifact, format!("terminal-linkage-{suffix}"), now],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_implementer_outcomes
               (work_unit_id,attempt_id,attempt_ordinal,implementer_session_id,implementer_invocation_id,
                reporting_invocation_id,reporting_harness_revision_id,
                reporting_harness_configuration_digest,reporting_harness_repository_commit_ref,
                reporting_requested_at,reporting_prepared_at,reporting_harness_bound_at,
                reporting_launch_requested_at,reporting_launch_accepted_at,reporting_ready_at,
                submitted_summary,outcome_variant,submitted_validation_statement,semantic_payload_json,
                submission_fingerprint,submitted_at,validation_at,validation_result,
                evidence_manifest_json,comparison_fingerprint,
                evidence_content_fingerprints_json,file_review_capture_authorization_id,
                evidence_ready_at,semantic_completed_at,semantic_completion_invocation_id,
                lifecycle_observed_at,lifecycle_status,application_accepted_at,handler_review_ready_at)
             VALUES(?1,?2,0,?3,?4,?5,'terminal-reporting-revision','terminal-reporting-digest',
                    'terminal-reporting-commit',?6,?6,?6,?6,?6,?6,?7,'review_pending',?8,?9,?10,?6,?6,'valid',?11,?12,?13,?14,?6,?6,?5,?6,'completed',?6,?6)",
            params![work_unit_id, attempt_id, format!("terminal-implementer-session-{suffix}"), format!("terminal-implementer-invocation-{suffix}"), reporting, now, format!("Accepted terminal candidate {ordinal}."), "Local Git candidate captured.", outcome_payload, outcome_fingerprint, manifest, comparison, contents, capture],
        ).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        attempt_root
    }

    #[test]
    fn terminal_authority_fixture_converges_product_materialization_and_real_git_gateway() {
        let fixture = Fixture::new();
        let (service, planner, sprint_id) = fixture.prepare_work_slice_planner();
        service.submit_work_slice_proposal(&planner, crate::orchestration::sprint_runner_transition::WorkSliceProposal {
            objective: "Converge the terminal authority fixture.".into(),
            lanes: vec![
                crate::orchestration::sprint_runner_transition::WorkSliceLane { title: "Root A".into(), specification: "Canonical execution fixture responsibility: Root A.".into(), depends_on: vec![] },
                crate::orchestration::sprint_runner_transition::WorkSliceLane { title: "Root B".into(), specification: "Canonical execution fixture responsibility: Root B.".into(), depends_on: vec![] },
                crate::orchestration::sprint_runner_transition::WorkSliceLane { title: "Middle".into(), specification: "Canonical execution fixture responsibility: Middle.".into(), depends_on: vec!["Root A".into()] },
                crate::orchestration::sprint_runner_transition::WorkSliceLane { title: "Leaf".into(), specification: "Canonical execution fixture responsibility: Leaf.".into(), depends_on: vec!["Middle".into()] },
            ],
        }).unwrap();
        service.complete_work_slice_planning(&planner, crate::orchestration::sprint_runner_transition::WorkSliceCompletion {}).unwrap();
        fixture.runtime.finish(planner.as_str(), AgentInvocationTerminalStatus::Completed);

        let repository_root = fixture._directory.path().join("terminal-authority-repository");
        fs::create_dir_all(&repository_root).unwrap();
        terminal_authority_git(&repository_root, &["init", "-b", "main"]);
        terminal_authority_git(&repository_root, &["config", "user.email", "terminal@example.test"]);
        terminal_authority_git(&repository_root, &["config", "user.name", "Terminal Test"]);
        fs::write(repository_root.join("README.md"), "terminal base\n").unwrap();
        terminal_authority_git(&repository_root, &["add", "README.md"]);
        terminal_authority_git(&repository_root, &["commit", "-m", "terminal base"]);
        let baseline = terminal_authority_git(&repository_root, &["rev-parse", "HEAD"]);
        let sprint_root = fixture._directory.path().join("terminal-authority-sprint");
        terminal_authority_git(&repository_root, &["worktree", "add", "-b", "terminal-sprint", sprint_root.to_string_lossy().as_ref(), &baseline]);
        fs::write(sprint_root.join("SPRINT.md"), "terminal sprint\n").unwrap();
        terminal_authority_git(&sprint_root, &["add", "SPRINT.md"]);
        terminal_authority_git(&sprint_root, &["commit", "-m", "terminal sprint"]);
        let current = terminal_authority_git(&sprint_root, &["rev-parse", "HEAD"]);
        let repository_root = repository_root.canonicalize().unwrap();
        let sprint_root = sprint_root.canonicalize().unwrap();
        Connection::open(&fixture.database_path).unwrap().execute("DELETE FROM initiated_sprint_git_authorities WHERE sprint_id=?1", [&sprint_id]).unwrap();
        let authority_id = match SqliteOrchestrationRepository::open(&fixture.database_path).unwrap().store_initiated_sprint_git_authority(InitiatedSprintGitAuthorityWrite {
            sprint_id: sprint_id.clone(), idempotency_key: "terminal-authority".into(), repository_id: "terminal-repository".into(),
            repository_root: repository_root.to_string_lossy().into_owned(), repository_common_dir: repository_root.join(".git").canonicalize().unwrap().to_string_lossy().into_owned(),
            worktree_id: "terminal-sprint-worktree".into(), worktree_root: sprint_root.to_string_lossy().into_owned(),
            baseline_object_id: baseline.clone(), current_object_id: current, runtime_instance_ref: "terminal-runtime".into(), runtime_source_ref: "terminal-source".into(), source_fingerprint: "f".repeat(64),
        }).unwrap() {
            crate::orchestration::repository::StoreInitiatedSprintGitAuthorityResult::Stored { authority_id }
            | crate::orchestration::repository::StoreInitiatedSprintGitAuthorityResult::IdempotentReplay { authority_id } => authority_id,
        };
        let units = Connection::open(&fixture.database_path).unwrap().prepare("SELECT work_unit_id FROM work_units ORDER BY lane_ordinal").unwrap().query_map([], |row| row.get::<_, String>(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(units.len(), 4);
        let attempts = units.iter().enumerate().map(|(ordinal, unit)| record_terminal_authority_candidate(&fixture, &repository_root, &baseline, &authority_id, unit, ordinal)).collect::<Vec<_>>();
        let mut connection = Connection::open(&fixture.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_decisions d JOIN work_unit_handler_reviews r ON r.review_invocation_id=d.review_invocation_id JOIN work_unit_implementer_outcomes o ON o.work_unit_id=d.work_unit_id AND o.reporting_invocation_id=r.reporting_invocation_id JOIN work_unit_handler_activations h ON h.work_unit_id=d.work_unit_id AND h.attempt_id=o.attempt_id JOIN work_unit_materializations m ON m.materialization_id=h.materialization_id JOIN initiated_sprint_git_authorities a ON a.sprint_id=m.sprint_id JOIN execution_support_attempt_authorizations x ON x.attempt_id=o.attempt_id AND x.work_unit_id=d.work_unit_id AND x.role_kind='work_unit_implementer' AND x.sprint_git_authority_id=a.authority_id JOIN execution_support_grants g ON g.attempt_id=o.attempt_id AND g.role_id='work_unit_implementer' JOIN file_review_git_capture_authorizations c ON c.capture_authorization_id=o.file_review_capture_authorization_id AND c.worktree_id=g.workspace_id AND c.repository_id=a.repository_id AND c.baseline_object_id=x.baseline_object_id JOIN file_review_git_capture_documents l ON l.capture_authorization_id=c.capture_authorization_id WHERE d.decision_variant='accepted' AND d.implementation_accepted_at IS NOT NULL AND r.lifecycle_status='completed' AND r.semantic_judgment_variant='accept' AND o.evidence_ready_at IS NOT NULL AND o.application_accepted_at IS NOT NULL", [], |row| row.get(0)).unwrap(), 4);
        reconcile_accepted_candidate_authorities(&mut connection).unwrap();
        for attempt in attempts {
            terminal_authority_git(&repository_root, &["worktree", "remove", "--force", attempt.to_string_lossy().as_ref()]);
        }
        reconcile_accepted_integrations(&mut connection).unwrap();
        reconcile_accepted_integrations(&mut connection).unwrap();
        let terminal_review: String = connection.query_row(
            "SELECT review_invocation_id FROM work_unit_handler_reviews ORDER BY review_invocation_id LIMIT 1",
            [],
            |row| row.get(0),
        ).unwrap();
        drop(connection);

        // This is the productive completed-review movement. It drains the authoritative graph
        // and both settlement layers, then records the Sprint decision and upward result before
        // this call returns; the service is deliberately not reopened.
        service
            .reconcile_handler_review_terminal_movement_for_test(&terminal_review)
            .unwrap();

        let connection = Connection::open(&fixture.database_path).unwrap();

        let candidate_status = connection.prepare("SELECT candidate_id,pinned_at,attention_reason FROM accepted_handler_candidates ORDER BY candidate_id").unwrap().query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        let candidate_attentions = connection.prepare("SELECT candidate_id,attention_reason FROM accepted_candidate_authority_attentions ORDER BY candidate_id").unwrap().query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(candidate_status.iter().filter(|(_, pinned, attention)| pinned.is_some() && attention.is_none()).count(), 4, "{candidate_status:?}; {candidate_attentions:?}");
        for table in ["accepted_work_unit_integrations", "accepted_work_unit_integration_evidence", "work_unit_settlements"] {
            assert_eq!(connection.query_row::<i64, _, _>(&format!("SELECT COUNT(*) FROM {table} WHERE 1"), [], |row| row.get(0)).unwrap(), 4, "{table}");
        }
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_prerequisite_contributions", [], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_prerequisite_contributions p JOIN work_unit_relationships e ON e.relationship_id=p.relationship_id WHERE e.relationship_kind='depends_on' AND e.to_id=p.prerequisite_work_unit_id AND e.from_id=p.dependent_work_unit_id", [], |row| row.get(0)).unwrap(), 2);
        for table in ["work_slice_execution_graph_completions", "work_slice_execution_settlements", "work_slice_planning_point_execution_settlements"] {
            assert_eq!(connection.query_row::<i64, _, _>(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0)).unwrap(), 1, "{table}");
        }
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_slice_execution_attentions", [], |row| row.get(0)).unwrap(), 0);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_decisions WHERE sprint_id=?1 AND decision_state='settled'", [&sprint_id], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_continuation_current_decisions current JOIN sprint_continuation_decisions decision ON decision.decision_id=current.decision_id WHERE current.sprint_id=?1 AND current.decision_state='settled' AND decision.decision_state='settled'", [&sprint_id], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM sprint_upward_results result JOIN sprint_continuation_decisions decision ON decision.decision_id=result.decision_id WHERE result.sprint_id=?1 AND result.result_kind='settled' AND decision.decision_state='settled'", [&sprint_id], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('sprint_settlements','epic_settlements')", [], |row| row.get(0)).unwrap(), 0);
        drop(connection);

        let native = serde_json::to_value(SqliteOrchestrationRepository::open(&fixture.database_path).unwrap().native_query().unwrap()).unwrap();
        let canonical: serde_json::Value = serde_json::from_str(include_str!("fixtures/orchestration-native-query-v2/valid-execution-graph.json")).unwrap();
        assert_eq!(
            normalized_terminal_execution_projection(&native),
            normalized_terminal_execution_projection(&canonical),
            "the Rust-owned frontend execution fixture must exactly match the public terminal projection",
        );
        let serialized = serde_json::to_string(&native).unwrap();
        assert!(!serialized.contains("terminal-attempt-worktree"));
        assert!(!serialized.contains("refs/codex/orchestrator/accepted"));
        assert!(!serialized.contains(repository_root.to_string_lossy().as_ref()));
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ReportingFacts {
        summary: Option<String>,
        validation: Option<String>,
        payload: Option<String>,
        fingerprint: Option<String>,
        submitted_at: Option<String>,
        validation_result: Option<String>,
        evidence_manifest: Option<String>,
        comparison_fingerprint: Option<String>,
        evidence_contents: Option<String>,
        evidence_ready_at: Option<String>,
        semantic_completed_at: Option<String>,
        semantic_invocation: Option<String>,
        lifecycle_status: Option<String>,
        application_accepted_at: Option<String>,
        handler_review_ready_at: Option<String>,
    }

    struct ReportingFixture {
        base: Fixture,
        transition: Arc<crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService>,
        handler: Arc<WorkUnitExecutionHarnessService>,
        work_unit_id: String,
        attempt_id: String,
        session_id: String,
        implementer_invocation_id: String,
        reporting_invocation_id: String,
        handler_session_id: String,
        handler_invocation_id: String,
        handler_action_invocation_id: String,
        authority_id: String,
        working_directory: PathBuf,
        expected_identities: (String, String, String, String, String),
    }

    impl ReportingFixture {
        fn new() -> Self {
            let base = Fixture::unstarted();
            let transition = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &base.database_path,
                base.sessions.clone(),
            ).unwrap();
            let sprint_id: String = Connection::open(&base.database_path).unwrap().query_row(
                "SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",
                [],
                |row| row.get(0),
            ).unwrap();

            let repository_root = base._directory.path().join("reporting-repository");
            let sprint_root = base._directory.path().join("reporting-sprint-worktree");
            fs::create_dir_all(&repository_root).unwrap();
            let git = |root: &Path, arguments: &[&str]| {
                let output = std::process::Command::new("git")
                    .args(arguments)
                    .current_dir(root)
                    .output()
                    .unwrap();
                assert!(output.status.success(), "{output:?}");
                String::from_utf8(output.stdout).unwrap().trim().to_owned()
            };
            git(&repository_root, &["init"]);
            git(&repository_root, &["config", "user.email", "reporting@example.test"]);
            git(&repository_root, &["config", "user.name", "Reporting Test"]);
            fs::write(repository_root.join("README.md"), "base\n").unwrap();
            git(&repository_root, &["add", "README.md"]);
            git(&repository_root, &["commit", "-m", "reporting base"]);
            let initial = git(&repository_root, &["rev-parse", "HEAD"]);
            git(
                &repository_root,
                &[
                    "worktree",
                    "add",
                    "-b",
                    "reporting-sprint",
                    sprint_root.to_string_lossy().as_ref(),
                    &initial,
                ],
            );
            fs::write(sprint_root.join("README.md"), "sprint baseline\n").unwrap();
            git(&sprint_root, &["add", "README.md"]);
            git(&sprint_root, &["commit", "-m", "reporting sprint baseline"]);
            let current = git(&sprint_root, &["rev-parse", "HEAD"]);
            let repository_root = repository_root.canonicalize().unwrap();
            let sprint_root = sprint_root.canonicalize().unwrap();
            let repository = Arc::new(SqliteOrchestrationRepository::open(&base.database_path).unwrap());
            let authority_id = match repository.store_initiated_sprint_git_authority(
                InitiatedSprintGitAuthorityWrite {
                    sprint_id,
                    idempotency_key: "implementer-reporting-test-authority".into(),
                    repository_id: "implementer-reporting-repository".into(),
                    repository_root: repository_root.to_string_lossy().into_owned(),
                    repository_common_dir: repository_root.join(".git").canonicalize().unwrap().to_string_lossy().into_owned(),
                    worktree_id: "implementer-reporting-sprint-worktree".into(),
                    worktree_root: sprint_root.to_string_lossy().into_owned(),
                    baseline_object_id: initial,
                    current_object_id: current,
                    runtime_instance_ref: "implementer-reporting-runtime".into(),
                    runtime_source_ref: "implementer-reporting-source".into(),
                    source_fingerprint: "e".repeat(64),
                },
            ).unwrap() {
                crate::orchestration::repository::StoreInitiatedSprintGitAuthorityResult::Stored { authority_id }
                | crate::orchestration::repository::StoreInitiatedSprintGitAuthorityResult::IdempotentReplay { authority_id } => authority_id,
            };
            let orchestration = Arc::new(OrchestrationApplication::new(repository.clone()));
            let support = ProductExecutionSupportState::new(
                &base.database_path,
                base._directory.path().join("reporting-workspaces"),
                repository,
            ).unwrap();
            let handler = Arc::new(WorkUnitExecutionHarnessService::new(
                support.service(),
                base.sessions.clone(),
                orchestration,
            ));
            let work_unit_id = "implementer-reporting-work-unit".to_string();
            let attempt_id = "implementer-reporting-attempt".to_string();
            handler.authorize_implementer_attempt(&attempt_id, &work_unit_id, &authority_id).unwrap();
            let original_revision = handler.current_implementer_revision().unwrap();
            let reporting_revision = handler.current_implementer_reporting_revision().unwrap();
            let original_package = handler.construct_for_pinned_profile(
                &attempt_id,
                WorkUnitHarnessRole::Implementer,
                original_revision.profile.clone(),
            ).unwrap();
            let reporting_package = handler.construct_for_pinned_profile(
                &attempt_id,
                WorkUnitHarnessRole::Implementer,
                reporting_revision.profile.clone(),
            ).unwrap();
            assert_eq!(original_package.working_directory(), reporting_package.working_directory());
            let working_directory = PathBuf::from(original_package.working_directory());
            let session_id = "implementer-reporting-session".to_string();
            let implementer_invocation_id = "implementer-original-invocation".to_string();
            let reporting_invocation_id = {
                let mut hash = Sha256::new();
                hash.update(b"work-unit-implementer-reporting-invocation");
                hash.update([0]);
                hash.update(attempt_id.as_bytes());
                format!("work-unit-implementer-reporting-invocation-{:x}", hash.finalize())
            };
            let original_runtime = original_package.runtime_launch_configuration();
            base.sessions.create_application_session(CreateApplicationAgentSessionCommand {
                session_id: AgentSessionId::new(session_id.clone()).unwrap(),
                session: CreateAgentSessionCommand {
                    title: Some("Implementer reporting test".into()),
                    working_directory: Some(working_directory.to_string_lossy().into_owned()),
                    requested_options: original_runtime.requested_options.clone(),
                },
            }).unwrap();
            base.sessions.send_idempotent_application_message_with_launch_observation(
                SendIdempotentApplicationAgentSessionMessageCommand {
                    invocation_id: AgentInvocationId::new(implementer_invocation_id.clone()).unwrap(),
                    message: SendAgentSessionMessageCommand {
                        session_id: Some(AgentSessionId::new(session_id.clone()).unwrap()),
                        submitted_text: "Implement the bounded Work Unit.".into(),
                        title: None,
                        working_directory: Some(working_directory.to_string_lossy().into_owned()),
                        requested_options: Some(original_runtime.requested_options),
                    },
                },
                Some(original_runtime.extension),
            ).unwrap();
            base.runtime.finish(&implementer_invocation_id, AgentInvocationTerminalStatus::Completed);
            let reporting_runtime = reporting_package.runtime_launch_configuration();
            let reporting_launch = base.sessions.send_idempotent_application_message_with_launch_observation(
                SendIdempotentApplicationAgentSessionMessageCommand {
                    invocation_id: AgentInvocationId::new(reporting_invocation_id.clone()).unwrap(),
                    message: SendAgentSessionMessageCommand {
                        session_id: Some(AgentSessionId::new(session_id.clone()).unwrap()),
                        submitted_text: "Report the completed implementation.".into(),
                        title: None,
                        working_directory: Some(working_directory.to_string_lossy().into_owned()),
                        requested_options: Some(reporting_runtime.requested_options),
                    },
                },
                Some(reporting_runtime.extension),
            ).unwrap();
            assert!(reporting_launch.launch_accepted);

            let now = "2026-08-04T00:00:00Z";
            let connection = Connection::open(&base.database_path).unwrap();
            connection.pragma_update(None, "foreign_keys", false).unwrap();
            connection.execute(
                "INSERT INTO work_unit_implementer_activations (
                    work_unit_id,handler_attempt_id,handler_invocation_id,attempt_id,
                    implementer_session_id,implementer_invocation_id,
                    implementer_harness_revision_id,implementer_harness_configuration_digest,
                    implementer_harness_repository_commit_ref,requested_at,authorized_at,
                    execution_support_granted_at,isolated_worktree_ready_at,
                    implementer_session_created_at,implementer_invocation_prepared_at,
                    implementer_harness_bound_at,launch_requested_at,launch_accepted_at,
                    implementer_ready_at
                 ) VALUES (?1,'reporting-handler-attempt','reporting-handler-action',?2,?3,?4,?5,?6,?7,?8,?8,?8,?8,?8,?8,?8,?8,?8,?8)",
                params![
                    work_unit_id,
                    attempt_id,
                    session_id,
                    implementer_invocation_id,
                    original_revision.revision_id,
                    original_revision.configuration_digest,
                    original_revision.repository_commit_ref,
                    now,
                ],
            ).unwrap();
            connection.execute(
                "INSERT INTO work_unit_implementer_outcomes (
                    work_unit_id,attempt_id,attempt_ordinal,implementer_session_id,implementer_invocation_id,
                    reporting_invocation_id,reporting_harness_revision_id,
                    reporting_harness_configuration_digest,reporting_harness_repository_commit_ref,
                    reporting_requested_at,reporting_prepared_at,reporting_harness_bound_at,
                    reporting_launch_requested_at,reporting_launch_accepted_at,reporting_ready_at
                 ) VALUES (?1,?2,0,?3,?4,?5,?6,?7,?8,?9,?9,?9,?9,?9,?9)",
                params![
                    work_unit_id,
                    attempt_id,
                    session_id,
                    implementer_invocation_id,
                    reporting_invocation_id,
                    reporting_revision.revision_id,
                    reporting_revision.configuration_digest,
                    reporting_revision.repository_commit_ref,
                    now,
                ],
            ).unwrap();
            connection.pragma_update(None, "foreign_keys", true).unwrap();
            transition.attach_reporting_test_harness(handler.clone());
            let expected_identities = (
                work_unit_id.clone(),
                attempt_id.clone(),
                session_id.clone(),
                implementer_invocation_id.clone(),
                reporting_invocation_id.clone(),
            );
            Self {
                base,
                transition,
                handler,
                work_unit_id,
                attempt_id,
                session_id,
                implementer_invocation_id,
                reporting_invocation_id,
                handler_session_id: "implementer-review-handler-session".into(),
                handler_invocation_id: "implementer-review-handler-original".into(),
                handler_action_invocation_id: "implementer-review-handler-action".into(),
                authority_id,
                working_directory,
                expected_identities,
            }
        }

        fn invocation(&self) -> AgentInvocationId {
            AgentInvocationId::new(self.reporting_invocation_id.clone()).unwrap()
        }

        fn claims(&self) -> crate::orchestration::sprint_runner_transition::ImplementationOutcomeClaims {
            crate::orchestration::sprint_runner_transition::ImplementationOutcomeClaims {
                outcome: crate::orchestration::sprint_runner_transition::ImplementationOutcomeVariant::ReviewPending,
                summary: "Implemented the reporting boundary.".into(),
                validation_statement: "Focused deterministic proof passed.".into(),
            }
        }

        fn facts(&self) -> ReportingFacts {
            Connection::open(&self.base.database_path).unwrap().query_row(
                "SELECT submitted_summary,submitted_validation_statement,semantic_payload_json,
                        submission_fingerprint,submitted_at,validation_result,evidence_manifest_json,
                        comparison_fingerprint,evidence_content_fingerprints_json,evidence_ready_at,
                        semantic_completed_at,semantic_completion_invocation_id,lifecycle_status,
                        application_accepted_at,handler_review_ready_at
                 FROM work_unit_implementer_outcomes WHERE work_unit_id=?1",
                [&self.work_unit_id],
                |row| Ok(ReportingFacts {
                    summary: row.get(0)?, validation: row.get(1)?, payload: row.get(2)?,
                    fingerprint: row.get(3)?, submitted_at: row.get(4)?, validation_result: row.get(5)?,
                    evidence_manifest: row.get(6)?, comparison_fingerprint: row.get(7)?,
                    evidence_contents: row.get(8)?, evidence_ready_at: row.get(9)?,
                    semantic_completed_at: row.get(10)?, semantic_invocation: row.get(11)?,
                    lifecycle_status: row.get(12)?, application_accepted_at: row.get(13)?,
                    handler_review_ready_at: row.get(14)?,
                }),
            ).unwrap()
        }

        fn identities(&self) -> (String, String, String, String, String) {
            Connection::open(&self.base.database_path).unwrap().query_row(
                "SELECT work_unit_id,attempt_id,implementer_session_id,implementer_invocation_id,
                        reporting_invocation_id FROM work_unit_implementer_outcomes",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            ).unwrap()
        }

        fn write_evidence(&self, content: &str) {
            fs::write(self.working_directory.join("README.md"), content).unwrap();
            for arguments in [["add", "README.md"].as_slice(), ["commit", "-m", "reporting evidence"].as_slice()] {
                let output = std::process::Command::new("git")
                    .args(arguments)
                    .current_dir(&self.working_directory)
                    .output()
                    .unwrap();
                assert!(output.status.success(), "{output:?}");
            }
        }

        fn assert_pinned_evidence_available(&self) {
            let (revision, digest, commit): (String, String, String) = Connection::open(&self.base.database_path).unwrap().query_row(
                "SELECT reporting_harness_revision_id,reporting_harness_configuration_digest,
                        reporting_harness_repository_commit_ref
                 FROM work_unit_implementer_outcomes WHERE work_unit_id=?1",
                [&self.work_unit_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).unwrap();
            let pinned = self.handler.load_pinned_implementer_revision(&revision, &digest, &commit).unwrap();
            let package = self.handler.construct_for_pinned_profile(
                &self.attempt_id,
                WorkUnitHarnessRole::Implementer,
                pinned.profile,
            ).unwrap();
            package.bind_correlated_invocation(
                AgentSessionId::new(self.session_id.clone()).unwrap(),
                self.invocation(),
            ).unwrap();
            let manifest = package.changed_file_manifest().unwrap();
            assert!(!manifest.is_empty(), "expected a changed-file manifest");
            assert!(!package.comparison().unwrap().is_empty(), "expected File Review comparison bytes");
            for entry in manifest {
                assert!(!package.evidence_content(&entry.evidence_ref).unwrap().is_empty(), "missing evidence content for {}", entry.display_name);
            }
        }

        fn enable_notifications(&self) {
            self.base.notifier.set_sprint(&self.transition);
        }

        fn finish(&self, status: AgentInvocationTerminalStatus) {
            self.base.runtime.finish(&self.reporting_invocation_id, status);
        }

        fn reopened(&self) -> Arc<crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService> {
            let service = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
                &self.base.database_path,
                self.base.sessions.clone(),
            ).unwrap();
            service.attach_reporting_test_harness(self.handler.clone());
            service
        }

        fn enable_handler_review_route(&self) {
            self.handler.authorize_handler_attempt(&self.attempt_id, &self.work_unit_id, &self.authority_id).unwrap();
            let original = self.handler.current_handler_revision().unwrap();
            let action = self.handler.current_handler_action_revision().unwrap();
            let original_package = self.handler.construct_for_pinned_profile(&self.attempt_id, WorkUnitHarnessRole::Handler, original.profile.clone()).unwrap();
            let action_package = self.handler.construct_for_pinned_profile(&self.attempt_id, WorkUnitHarnessRole::Handler, action.profile.clone()).unwrap();
            assert_eq!(original_package.working_directory(), action_package.working_directory());
            let session = AgentSessionId::new(self.handler_session_id.clone()).unwrap();
            let runtime = original_package.runtime_launch_configuration();
            self.base.sessions.create_application_session(CreateApplicationAgentSessionCommand {
                session_id: session.clone(),
                session: CreateAgentSessionCommand { title: Some("Handler review test".into()), working_directory: Some(original_package.working_directory().into()), requested_options: runtime.requested_options.clone() },
            }).unwrap();
            self.base.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand {
                invocation_id: AgentInvocationId::new(self.handler_invocation_id.clone()).unwrap(),
                message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: "Original Handler.".into(), title: None, working_directory: Some(original_package.working_directory().into()), requested_options: Some(runtime.requested_options) },
            }, Some(runtime.extension)).unwrap();
            self.base.runtime.finish(&self.handler_invocation_id, AgentInvocationTerminalStatus::Completed);
            let action_runtime = action_package.runtime_launch_configuration();
            self.base.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand {
                invocation_id: AgentInvocationId::new(self.handler_action_invocation_id.clone()).unwrap(),
                message: SendAgentSessionMessageCommand { session_id: Some(session), submitted_text: "Handler action.".into(), title: None, working_directory: Some(action_package.working_directory().into()), requested_options: Some(action_runtime.requested_options) },
            }, Some(action_runtime.extension)).unwrap();
            self.base.runtime.finish(&self.handler_action_invocation_id, AgentInvocationTerminalStatus::Completed);
            let connection = Connection::open(&self.base.database_path).unwrap();
            connection.pragma_update(None, "foreign_keys", false).unwrap();
            let now = "2026-08-04T00:00:00Z";
            connection.execute("INSERT OR IGNORE INTO work_units (work_unit_id,materialization_id,work_slice_id,accepted_revision_id,lane_ordinal,lane_title,specification) VALUES (?1,'reporting-materialization','reporting-slice','reporting-revision',0,'Reporting','Handler review test')", [&self.work_unit_id]).unwrap();
            connection.execute("INSERT INTO work_unit_handler_activations (work_unit_id,materialization_id,sprint_id,attempt_id,handler_session_id,handler_invocation_id,handler_harness_key,handler_harness_version,handler_harness_revision_id,handler_harness_configuration_digest,handler_harness_repository_commit_ref,eligibility_state,requested_at,authorized_at,attempt_created_at,execution_support_granted_at,isolated_worktree_ready_at,handler_session_created_at,handler_invocation_prepared_at,handler_harness_bound_at,launch_requested_at,launch_accepted_at,provider_activation_observed_at,handler_ready_at) VALUES (?1,'reporting-materialization','reporting-sprint',?2,?3,?4,?5,?6,?7,?8,?9,'eligible',?10,?10,?10,?10,?10,?10,?10,?10,?10,?10,?10,?10)", params![self.work_unit_id,self.attempt_id,self.handler_session_id,self.handler_invocation_id,original.profile.key,original.profile.version,original.revision_id,original.configuration_digest,original.repository_commit_ref,now]).unwrap();
            connection.execute("INSERT INTO work_unit_handler_action_continuations (work_unit_id,attempt_id,handler_session_id,original_handler_invocation_id,action_invocation_id,action_harness_revision_id,action_harness_configuration_digest,action_harness_repository_commit_ref,requested_at,authorized_at,invocation_prepared_at,harness_bound_at,launch_requested_at,launch_accepted_at,provider_activation_observed_at,action_ready_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,?9,?9,?9,?9,?9,?9)", params![self.work_unit_id,self.attempt_id,self.handler_session_id,self.handler_invocation_id,self.handler_action_invocation_id,action.revision_id,action.configuration_digest,action.repository_commit_ref,now]).unwrap();
            connection.pragma_update(None, "foreign_keys", true).unwrap();
        }

        fn ready_review(&self) -> String {
            self.enable_handler_review_route();
            self.transition.submit_implementation_outcome(&self.invocation(), self.claims()).unwrap();
            self.write_evidence("review evidence\n");
            self.transition.complete_implementation_outcome(&self.invocation()).unwrap();
            self.enable_notifications();
            self.finish(AgentInvocationTerminalStatus::Completed);
            Connection::open(&self.base.database_path).unwrap().query_row("SELECT review_invocation_id FROM work_unit_handler_reviews WHERE work_unit_id=?1", [&self.work_unit_id], |row| row.get(0)).unwrap()
        }

        fn assert_no_submission_evidence_or_completion(&self) {
            let facts = self.facts();
            assert!(facts.summary.is_none() && facts.validation.is_none());
            assert!(facts.payload.is_none() && facts.fingerprint.is_none() && facts.submitted_at.is_none());
            assert!(facts.validation_result.is_none());
            assert!(facts.evidence_manifest.is_none() && facts.comparison_fingerprint.is_none());
            assert!(facts.evidence_contents.is_none() && facts.evidence_ready_at.is_none());
            assert!(facts.semantic_completed_at.is_none() && facts.semantic_invocation.is_none());
            assert!(facts.application_accepted_at.is_none() && facts.handler_review_ready_at.is_none());
        }

        fn return_one_retry(&self) -> (String, String, String, String) {
            let review = self.ready_review();
            self.transition.record_handler_review_judgment_for_test(
                &review,
                "return",
                Some(crate::orchestration::sprint_runner_transition::HandlerReviewReturnReason {
                    code: "review_failed".into(),
                    explanation: "evidence requires correction".into(),
                }),
            ).unwrap();
            self.base.runtime.finish(&review, AgentInvocationTerminalStatus::Completed);
            Connection::open(&self.base.database_path).unwrap().query_row(
                "SELECT retry_attempt_id,implementer_session_id,implementer_invocation_id,private_ref_name
                 FROM work_unit_retry_attempts WHERE work_unit_id=?1",
                [&self.work_unit_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            ).unwrap()
        }

        fn retry_count(&self) -> i64 {
            Connection::open(&self.base.database_path).unwrap().query_row(
                "SELECT COUNT(*) FROM work_unit_retry_attempts WHERE work_unit_id=?1 AND ordinal=1",
                [&self.work_unit_id],
                |row| row.get(0),
            ).unwrap()
        }
    }

    #[test]
    fn no_progress_disposition_persists_one_handback_without_receiver_effects() {
        let no_progress = ReportingFixture::new();
        let review = no_progress.ready_review();
        let disposition = crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition {
            code: "blocked_by_missing_input".into(), explanation: "a bounded input is unavailable".into(),
            classification: crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::Blocked,
            meaningful_progress: false,
        };
        no_progress.transition.record_handler_incomplete_disposition_for_test(&review, disposition.clone()).unwrap();
        no_progress.transition.record_handler_incomplete_disposition_for_test(&review, disposition).unwrap();
        no_progress.base.runtime.finish(&review, AgentInvocationTerminalStatus::Completed);
        no_progress.transition.reconcile_handler_reviews_for_test().unwrap();
        no_progress.transition.reconcile_handler_reviews_for_test().unwrap();
        let connection = Connection::open(&no_progress.base.database_path).unwrap();
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_handler_incomplete_dispositions WHERE meaningful_progress=0 AND next_attempt_authorized_at IS NULL", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_no_progress_handbacks WHERE sprint_runner_receiver_activated_at IS NULL AND sprint_runner_receiver_decision_at IS NULL", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_retry_attempts", [], |row| row.get(0)).unwrap(), 0);
    }

    #[test]
    fn direct_sprint_result_receipt_reopens_concurrently_and_preserves_its_exact_attention() {
        let fixture = ReportingFixture::new();
        let (sprint, epic):(String,String)=Connection::open(&fixture.base.database_path).unwrap().query_row("SELECT id,epic_id FROM initiated_sprints ORDER BY ordinal LIMIT 1",[],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();
        let harness=conversation_harness::profile(ConversationHarnessRole::EpicRunner).unwrap(); let session=AgentSessionId::new("direct-result-epic-session").unwrap();
        fixture.base.sessions.create_application_session(CreateApplicationAgentSessionCommand{session_id:session.clone(),session:CreateAgentSessionCommand{title:Some("Direct result Epic Runner".into()),working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunner).unwrap()),requested_options:harness.runtime_options()}}).unwrap();
        fixture.base.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:AgentInvocationId::new("direct-result-original").unwrap(),message:SendAgentSessionMessageCommand{session_id:Some(session.clone()),submitted_text:"Original Epic Runner work.".into(),title:None,working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunner).unwrap()),requested_options:Some(harness.runtime_options())}},None).unwrap(); fixture.base.runtime.finish("direct-result-original",AgentInvocationTerminalStatus::Completed);
        let connection=Connection::open(&fixture.base.database_path).unwrap();
        connection.execute("INSERT INTO sprint_runner_transitions (sprint_id,epic_id,request_id,epic_runner_session_id,epic_runner_invocation_id,epic_runner_harness_key,epic_runner_harness_version,sprint_runner_harness_key,sprint_runner_harness_version,sprint_runner_session_id,sprint_runner_invocation_id,requested_at,authorized_at) VALUES (?1,?2,'direct-result-request',?3,'direct-result-original','epic_runner',3,'sprint_runner',2,'direct-result-sprint-session','direct-result-sprint-invocation','2026-08-05T00:00:00Z','2026-08-05T00:00:00Z')",params![sprint,epic,session.as_str()]).unwrap();
        connection.execute("INSERT INTO sprint_continuation_decisions VALUES('direct-decision',?1,1,'attention','structured_human_or_external_attention',0,'direct-input','2026-08-05T00:00:00Z')",[&sprint]).unwrap();
        connection.execute("INSERT INTO sprint_continuation_current_decisions VALUES(?1,'direct-decision','attention','2026-08-05T00:00:00Z')",[&sprint]).unwrap();
        connection.execute("INSERT INTO sprint_continuation_attentions VALUES('direct-decision','direct-attention','exact-result-attention','direct-attention-fingerprint',NULL,'2026-08-05T00:00:00Z')",[]).unwrap();
        connection.execute("INSERT INTO sprint_upward_results VALUES('direct-result','direct-decision',?1,'attention','direct-chronology','2026-08-05T00:00:00Z')",[&sprint]).unwrap(); drop(connection);
        let first=fixture.reopened(); let second=fixture.reopened(); let barrier=Arc::new(Barrier::new(2)); let results=[first.clone(),second.clone()].into_iter().map(|service|{let barrier=barrier.clone();std::thread::spawn(move||{barrier.wait();service.reconcile_sprint_result_receivers_for_test()})}).collect::<Vec<_>>().into_iter().map(|call|call.join().unwrap()).collect::<Vec<_>>(); assert!(results.iter().all(Result::is_ok),"{results:?}");
        let connection=Connection::open(&fixture.base.database_path).unwrap(); let receiver:(String,Option<String>,Option<String>,Option<String>,i64)=connection.query_row("SELECT reassessment_invocation_id,delivery_persisted_at,harness_bound_at,launch_accepted_at,(SELECT COUNT(*) FROM epic_runner_sprint_result_receivers WHERE result_id='direct-result') FROM epic_runner_sprint_result_receivers WHERE result_id='direct-result'",[],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))).unwrap(); assert!(receiver.1.is_some()&&receiver.2.is_some()&&receiver.3.is_some());assert_eq!(receiver.4,1); drop(connection);
        let context=first.sprint_result_reassessment_context_for_test(&receiver.0).unwrap(); assert_eq!(context.pointer("/sprintResult/structuredAttention").and_then(|value|value.as_str()),Some("exact-result-attention"));
        fixture.base.runtime.finish(&receiver.0,AgentInvocationTerminalStatus::Completed); let invocation=fixture.base.sessions.load_session(&session).unwrap().invocations.into_iter().find(|entry|entry.invocation.id.as_str()==receiver.0).unwrap().invocation; first.on_agent_notification(&AgentSessionNotification::InvocationTerminal{session_id:session.clone(),invocation}).unwrap();
        let lifecycle:(Option<String>,Option<String>)=Connection::open(&fixture.base.database_path).unwrap().query_row("SELECT provider_activation_observed_at,reassessment_lifecycle_observed_at FROM epic_runner_sprint_result_receivers WHERE result_id='direct-result'",[],|row|Ok((row.get(0)?,row.get(1)?))).unwrap(); assert!(lifecycle.0.is_some()&&lifecycle.1.is_some());
        let disposition=crate::orchestration::sprint_runner_transition::EpicEscalationReassessmentDisposition{movement_kind:"consider_other_epic_work".into(),rationale:"retain the exact unresolved concern for later consideration".into(),considered_intent:Some("consider only a later bounded Epic movement".into()),downstream_request:None,human_external_attention:None}; first.record_sprint_result_disposition_for_test(&receiver.0,disposition.clone()).unwrap(); second.record_sprint_result_disposition_for_test(&receiver.0,disposition).unwrap();
        let divergent=crate::orchestration::sprint_runner_transition::EpicEscalationReassessmentDisposition{movement_kind:"consider_other_epic_work".into(),rationale:"different".into(),considered_intent:Some("different movement".into()),downstream_request:None,human_external_attention:None}; assert!(matches!(fixture.reopened().record_sprint_result_disposition_for_test(&receiver.0,divergent),Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        let facts:(i64,Option<String>)=Connection::open(&fixture.base.database_path).unwrap().query_row("SELECT (SELECT COUNT(*) FROM epic_runner_sprint_result_dispositions WHERE result_id='direct-result'),semantic_reassessment_recorded_at FROM epic_runner_sprint_result_receivers WHERE result_id='direct-result'",[],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();assert_eq!(facts.0,1);assert!(facts.1.is_some());
    }

    #[test]
    fn no_progress_handback_replays_one_agent_dependency_without_higher_effects() {
        let fixture = ReportingFixture::new(); let review = fixture.ready_review();
        let sprint: String = Connection::open(&fixture.base.database_path).unwrap().query_row("SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1",[],|row|row.get(0)).unwrap();
        let harness = conversation_harness::profile(ConversationHarnessRole::SprintRunnerHandbackReassessment).unwrap(); let root = conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunnerHandbackReassessment).unwrap(); let session = AgentSessionId::new("dependency-sprint-runner-session").unwrap();
        fixture.base.sessions.create_application_session(CreateApplicationAgentSessionCommand { session_id:session.clone(), session:CreateAgentSessionCommand { title:Some("Dependency Sprint Runner".into()), working_directory:Some(root), requested_options:harness.runtime_options() }}).unwrap();
        let connection=Connection::open(&fixture.base.database_path).unwrap(); connection.execute("UPDATE work_unit_handler_activations SET sprint_id=?2 WHERE work_unit_id=?1",params![fixture.work_unit_id,sprint]).unwrap(); connection.execute("INSERT INTO sprint_runner_transitions (sprint_id,epic_id,request_id,epic_runner_session_id,epic_runner_invocation_id,epic_runner_harness_key,epic_runner_harness_version,sprint_runner_harness_key,sprint_runner_harness_version,sprint_runner_session_id,sprint_runner_invocation_id,requested_at,authorized_at) VALUES (?1,'dependency-epic','dependency-request','dependency-epic-session','dependency-epic-invocation','epic_runner',3,'sprint_runner',2,?2,'dependency-original-runner','2026-08-04T00:00:00Z','2026-08-04T00:00:00Z')",params![sprint,session.as_str()]).unwrap(); drop(connection);
        fixture.transition.record_handler_incomplete_disposition_for_test(&review,crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition { code:"dependency".into(), explanation:"the exact concern awaits an agent result".into(), classification:crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::Blocked, meaningful_progress:false }).unwrap(); fixture.base.runtime.finish(&review,AgentInvocationTerminalStatus::Completed); fixture.transition.reconcile_handler_reviews_for_test().unwrap();
        let (handback,invocation):(String,String)=Connection::open(&fixture.base.database_path).unwrap().query_row("SELECT h.handback_id,d.reassessment_invocation_id FROM work_unit_no_progress_handbacks h JOIN sprint_runner_handback_deliveries d ON d.handback_id=h.handback_id WHERE h.work_unit_id=?1",[&fixture.work_unit_id],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();
        let wait=crate::orchestration::sprint_runner_transition::SprintHandbackDisposition { movement_kind:"wait_for_agent_dependency".into(), rationale:"the exact concern awaits its agent route".into(), eligible_work_summary:None, dependency_owner:Some("bounded Work Unit Handler".into()), dependency_owner_classification:Some(crate::orchestration::sprint_runner_transition::AgentAchievableDependencyOwner::WorkUnitHandler), enabling_result:Some("persisted Handler result".into()), resumption_path:Some("reconcile this Handback".into()), local_exhaustion_summary:None };
        fixture.transition.record_handback_disposition_for_test(&invocation,wait.clone()).unwrap(); fixture.transition.record_handback_disposition_for_test(&invocation,wait).unwrap(); let reopened=fixture.reopened(); reopened.record_handback_disposition_for_test(&invocation,crate::orchestration::sprint_runner_transition::SprintHandbackDisposition { movement_kind:"wait_for_agent_dependency".into(), rationale:"the exact concern awaits its agent route".into(), eligible_work_summary:None, dependency_owner:Some("bounded Work Unit Handler".into()), dependency_owner_classification:Some(crate::orchestration::sprint_runner_transition::AgentAchievableDependencyOwner::WorkUnitHandler), enabling_result:Some("persisted Handler result".into()), resumption_path:Some("reconcile this Handback".into()), local_exhaustion_summary:None }).unwrap();
        let connection=Connection::open(&fixture.base.database_path).unwrap(); let details:String=connection.query_row("SELECT details_json FROM sprint_runner_handback_dispositions WHERE handback_id=?1 AND preserves_handback=1",[&handback],|row|row.get(0)).unwrap(); assert!(details.contains("work_unit_handler")&&details.contains("bounded Work Unit Handler")&&details.contains("persisted Handler result")&&details.contains("reconcile this Handback")); assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_handback_escalations WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap(),0);
        drop(connection); let invalid=crate::orchestration::sprint_runner_transition::SprintHandbackDisposition { movement_kind:"wait_for_agent_dependency".into(), rationale:"human gate".into(), eligible_work_summary:None, dependency_owner:Some("human approval".into()), dependency_owner_classification:None, enabling_result:Some("approval".into()), resumption_path:Some("resume".into()), local_exhaustion_summary:None }; assert!(matches!(reopened.record_handback_disposition_for_test(&invocation,invalid),Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Invalid)));
        let connection=Connection::open(&fixture.base.database_path).unwrap(); assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_handback_dispositions WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap(),1); assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_handback_escalations WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap(),0); assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_transitions WHERE epic_continuation_invocation_id IS NOT NULL OR sprint_start_persisted_at IS NOT NULL OR planning_control_invocation_id IS NOT NULL",[],|row|row.get(0)).unwrap(),0);
    }

    #[test]
    fn no_progress_handback_delivers_one_epic_receiver_without_higher_effects() {
        let fixture = ReportingFixture::new();
        let review = fixture.ready_review();
        let sprint_id: String = Connection::open(&fixture.base.database_path).unwrap().query_row("SELECT id FROM initiated_sprints ORDER BY ordinal LIMIT 1", [], |row| row.get(0)).unwrap();
        let runner_session = AgentSessionId::new("handback-sprint-runner-session").unwrap();
        let handback_harness = conversation_harness::profile(ConversationHarnessRole::SprintRunnerHandbackReassessment).unwrap();
        let handback_root = conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunnerHandbackReassessment).unwrap();
        fixture.base.sessions.create_application_session(CreateApplicationAgentSessionCommand { session_id: runner_session.clone(), session: CreateAgentSessionCommand { title: Some("Handback Sprint Runner".into()), working_directory: Some(handback_root), requested_options: handback_harness.runtime_options() } }).unwrap();
        let epic_harness=conversation_harness::profile(ConversationHarnessRole::EpicRunner).unwrap();let epic_session=AgentSessionId::new("handback-epic-runner-session").unwrap();fixture.base.sessions.create_application_session(CreateApplicationAgentSessionCommand{session_id:epic_session.clone(),session:CreateAgentSessionCommand{title:Some("Handback Epic Runner".into()),working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunner).unwrap()),requested_options:epic_harness.runtime_options()}}).unwrap();fixture.base.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:AgentInvocationId::new("handback-epic-invocation").unwrap(),message:SendAgentSessionMessageCommand{session_id:Some(epic_session.clone()),submitted_text:"Original Epic Runner work.".into(),title:None,working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunner).unwrap()),requested_options:Some(epic_harness.runtime_options())}},None).unwrap();fixture.base.runtime.finish("handback-epic-invocation",AgentInvocationTerminalStatus::Completed);
        fixture.base.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: AgentInvocationId::new("handback-active-runner").unwrap(), message: SendAgentSessionMessageCommand { session_id: Some(runner_session.clone()), submitted_text: "Current Sprint Runner work.".into(), title: None, working_directory: Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunnerHandbackReassessment).unwrap()), requested_options: Some(handback_harness.runtime_options()) } }, None).unwrap();
        let now = "2026-08-04T00:00:00Z";
        let connection = Connection::open(&fixture.base.database_path).unwrap();
        connection.execute("UPDATE work_unit_handler_activations SET sprint_id=?2 WHERE work_unit_id=?1", params![fixture.work_unit_id,sprint_id]).unwrap();
        connection.execute("INSERT INTO sprint_runner_transitions (sprint_id,epic_id,request_id,epic_runner_session_id,epic_runner_invocation_id,epic_runner_harness_key,epic_runner_harness_version,sprint_runner_harness_key,sprint_runner_harness_version,sprint_runner_session_id,sprint_runner_invocation_id,requested_at,authorized_at) VALUES (?1,'handback-epic','handback-request',?2,'handback-epic-invocation','epic_runner',3,'sprint_runner',2,?3,'handback-original-runner',?4,?4)",params![sprint_id,epic_session.as_str(),runner_session.as_str(),now]).unwrap();
        drop(connection);
        fixture.transition.record_handler_incomplete_disposition_for_test(&review, crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition { code: "no_progress".into(), explanation: "the bounded concern did not progress".into(), classification: crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::FunctionalObjectiveNotSatisfied, meaningful_progress: false }).unwrap();
        fixture.base.runtime.finish(&review, AgentInvocationTerminalStatus::Completed);
        fixture.transition.reconcile_handler_reviews_for_test().unwrap();
        let connection = Connection::open(&fixture.base.database_path).unwrap();
        let (handback, invocation, persisted): (String,String,Option<String>) = connection.query_row("SELECT h.handback_id,d.reassessment_invocation_id,d.delivery_persisted_at FROM work_unit_no_progress_handbacks h JOIN sprint_runner_handback_deliveries d ON d.handback_id=h.handback_id WHERE h.work_unit_id=?1",[&fixture.work_unit_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert!(persisted.is_none(), "an active Runner leaves the exact delivery visibly pending");
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_handback_deliveries WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap(),1);
        for table in [
            "work_slice_execution_graph_completions",
            "work_slice_execution_settlements",
            "work_slice_planning_point_execution_settlements",
        ] {
            assert_eq!(
                connection
                    .query_row::<i64, _, _>(
                        &format!("SELECT COUNT(*) FROM {table}"),
                        [],
                        |row| row.get(0),
                    )
                    .unwrap(),
                0,
                "{table}"
            );
        }
        drop(connection);
        fixture.base.runtime.finish("handback-active-runner", AgentInvocationTerminalStatus::Completed);
        fixture.base.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: AgentInvocationId::new(invocation.clone()).unwrap(), message: SendAgentSessionMessageCommand { session_id: Some(runner_session), submitted_text: "The application delivered one exact no-progress Work Unit concern. Read only the supplied reassessment context, record one truthful next movement, then stop. Continuing eligible work does not settle the concern; do not contact an Epic Runner or declare Sprint/Epic blockage.".into(), title: None, working_directory: Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunnerHandbackReassessment).unwrap()), requested_options: Some(handback_harness.runtime_options()) } }).unwrap();
        let reopened = fixture.reopened();
        let concurrent = fixture.reopened(); let barrier = Arc::new(Barrier::new(2));
        let calls = [reopened.clone(), concurrent].into_iter().map(|service| { let barrier = barrier.clone(); std::thread::spawn(move || { barrier.wait(); service.reconcile_no_progress_handbacks_for_test() }) }).collect::<Vec<_>>();
        let results = calls.into_iter().map(|call| call.join().unwrap()).collect::<Vec<_>>();
        assert!(results.iter().all(Result::is_ok), "{results:?}");
        let recovered: (Option<String>,Option<String>,Option<String>,i64) = Connection::open(&fixture.base.database_path).unwrap().query_row("SELECT d.harness_bound_at,d.launch_requested_at,d.launch_accepted_at,(SELECT COUNT(*) FROM sprint_runner_handback_deliveries WHERE handback_id=d.handback_id) FROM sprint_runner_handback_deliveries d WHERE d.handback_id=?1",[&handback],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
        assert!(recovered.0.is_some() && recovered.1.is_some() && recovered.2.is_some() && recovered.3 == 1);
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_transitions WHERE epic_continuation_invocation_id IS NOT NULL OR epic_start_semantic_authorization_recorded_at IS NOT NULL OR sprint_start_persisted_at IS NOT NULL",[],|row|row.get(0)).unwrap(),0);
        let exhaustion = crate::orchestration::sprint_runner_transition::SprintHandbackDisposition { movement_kind: "local_exhaustion_escalate".into(), rationale: "the bounded Sprint route is locally exhausted".into(), eligible_work_summary: None, dependency_owner: None, dependency_owner_classification: None, enabling_result: None, resumption_path: None, local_exhaustion_summary: Some("no application-owned Sprint movement remains".into()) };
        reopened.record_handback_disposition_for_test(&invocation, exhaustion.clone()).unwrap();
        reopened.record_handback_disposition_for_test(&invocation, exhaustion).unwrap();
        let connection = Connection::open(&fixture.base.database_path).unwrap();
        let facts: (String,Option<String>,i64,i64) = connection.query_row("SELECT d.movement_kind,r.semantic_reassessment_fact_id,(SELECT COUNT(*) FROM sprint_runner_handback_escalations e WHERE e.handback_id=d.handback_id AND e.escalation_intent_id IS NOT NULL AND e.delivery_request_id IS NOT NULL),d.preserves_handback FROM sprint_runner_handback_dispositions d JOIN sprint_runner_handback_deliveries r ON r.handback_id=d.handback_id WHERE d.handback_id=?1",[&handback],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
        assert_eq!(facts.0,"local_exhaustion_escalate"); assert!(facts.1.is_some()); assert_eq!((facts.2,facts.3),(1,1));
        let receiver:(String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>)=connection.query_row("SELECT governing_runner_session_id,governing_runner_invocation_id,reassessment_invocation_id,delivery_persisted_at,harness_bound_at,launch_accepted_at,provider_activation_observed_at,semantic_reassessment_recorded_at FROM epic_runner_escalation_receivers WHERE handback_id=?1",[&handback],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?))).unwrap();assert_eq!(receiver.0,epic_session.as_str());assert_eq!(receiver.1,"handback-epic-invocation");assert!(receiver.3.is_some()&&receiver.4.is_some()&&receiver.5.is_some());assert_eq!((receiver.6,receiver.7),(None,None));assert!(connection.query_row::<Option<String>,_,_>("SELECT delivery_persisted_at FROM sprint_runner_handback_escalations WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap().is_some());
        connection.execute("UPDATE epic_runner_escalation_receivers SET launch_accepted_at=NULL WHERE handback_id=?1",[&handback]).unwrap();drop(connection);let reopened_receiver=fixture.reopened();let concurrent_receiver=fixture.reopened();let barrier=Arc::new(Barrier::new(2));let results=[reopened_receiver.clone(),concurrent_receiver.clone()].into_iter().map(|service|{let barrier=barrier.clone();std::thread::spawn(move||{barrier.wait();service.reconcile_epic_escalation_receivers_for_test()})}).collect::<Vec<_>>().into_iter().map(|call|call.join().unwrap()).collect::<Vec<_>>();assert!(results.iter().all(Result::is_ok),"{results:?}");let connection=Connection::open(&fixture.base.database_path).unwrap();assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM epic_runner_escalation_receivers WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap(),1);assert!(connection.query_row::<Option<String>,_,_>("SELECT launch_accepted_at FROM epic_runner_escalation_receivers WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap().is_some());connection.execute("UPDATE sprint_runner_transitions SET epic_runner_invocation_id='divergent-epic-runner' WHERE sprint_id=?1",[&sprint_id]).unwrap();connection.execute("UPDATE epic_runner_escalation_receivers SET launch_accepted_at=NULL WHERE handback_id=?1",[&handback]).unwrap();drop(connection);assert!(matches!(reopened_receiver.reconcile_epic_escalation_receivers_for_test(),Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));let connection=Connection::open(&fixture.base.database_path).unwrap();connection.execute("UPDATE sprint_runner_transitions SET epic_runner_invocation_id='handback-epic-invocation' WHERE sprint_id=?1",[&sprint_id]).unwrap();connection.execute("UPDATE epic_runner_escalation_receivers SET launch_accepted_at=COALESCE(launch_accepted_at,'2026-08-05T00:00:00Z') WHERE handback_id=?1",[&handback]).unwrap();drop(connection);fixture.base.runtime.finish(&receiver.2,AgentInvocationTerminalStatus::Completed);reopened_receiver.on_epic_runner_terminal(&AgentInvocationId::new(receiver.2.clone()).unwrap());reopened_receiver.reconcile_epic_escalation_receivers_for_test().unwrap();let connection=Connection::open(&fixture.base.database_path).unwrap();let lifecycle:(Option<String>,Option<String>,Option<String>)=connection.query_row("SELECT provider_activation_observed_at,reassessment_lifecycle_observed_at,semantic_reassessment_recorded_at FROM epic_runner_escalation_receivers WHERE handback_id=?1",[&handback],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();assert!(lifecycle.0.is_some()&&lifecycle.1.is_some());assert_eq!(lifecycle.2,None);
        connection.execute("UPDATE epic_runner_escalation_receivers SET launch_accepted_at=NULL WHERE handback_id=?1",[&handback]).unwrap();drop(connection);fixture.base.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:AgentInvocationId::new("handback-epic-active").unwrap(),message:SendAgentSessionMessageCommand{session_id:Some(epic_session.clone()),submitted_text:"Other Epic Runner work.".into(),title:None,working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunner).unwrap()),requested_options:Some(epic_harness.runtime_options())}},None).unwrap();reopened_receiver.reconcile_epic_escalation_receivers_for_test().unwrap();let pending=Connection::open(&fixture.base.database_path).unwrap().query_row::<Option<String>,_,_>("SELECT launch_accepted_at FROM epic_runner_escalation_receivers WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap();assert_eq!(pending,None);fixture.base.runtime.finish("handback-epic-active",AgentInvocationTerminalStatus::Completed);let active=fixture.base.sessions.load_session(&epic_session).unwrap().invocations.into_iter().find(|entry|entry.invocation.id.as_str()=="handback-epic-active").unwrap().invocation;reopened_receiver.on_agent_notification(&AgentSessionNotification::InvocationTerminal{session_id:epic_session.clone(),invocation:active}).unwrap();let connection=Connection::open(&fixture.base.database_path).unwrap();assert!(connection.query_row::<Option<String>,_,_>("SELECT launch_accepted_at FROM epic_runner_escalation_receivers WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap().is_some());
        let before: i64 = connection.query_row("SELECT COUNT(*) FROM sprint_runner_handback_dispositions WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap();
        drop(connection);
        let divergent = crate::orchestration::sprint_runner_transition::SprintHandbackDisposition { movement_kind: "continue_eligible_work".into(), rationale: "different movement".into(), eligible_work_summary: Some("another unit".into()), dependency_owner: None, dependency_owner_classification: None, enabling_result: None, resumption_path: None, local_exhaustion_summary: None };
        assert!(matches!(reopened.record_handback_disposition_for_test(&invocation, divergent), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        let connection = Connection::open(&fixture.base.database_path).unwrap();
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_handback_dispositions WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap(),before);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_handback_escalations WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap(),1);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_transitions WHERE epic_continuation_invocation_id IS NOT NULL OR epic_start_semantic_authorization_recorded_at IS NOT NULL OR sprint_start_persisted_at IS NOT NULL OR sprint_continuation_invocation_id IS NOT NULL OR planning_control_invocation_id IS NOT NULL",[],|row|row.get(0)).unwrap(),0);
        let settlement_before: (Option<String>,Option<String>) = connection.query_row("SELECT d.settlement_ready_at,m.settled_at FROM work_unit_handler_decisions d JOIN work_units u ON u.work_unit_id=d.work_unit_id LEFT JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id WHERE d.work_unit_id=?1",[&fixture.work_unit_id],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();
        assert_eq!(settlement_before,(None,None));
        // No Sprint/Epic final-blockage table exists in the authoritative schema or native DTO;
        // this route deliberately records no substitute final-state effect.
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('sprint_final_blockages','epic_final_blockages','epic_runner_handback_receivers','sprint_epic_settlements')",[],|row|row.get(0)).unwrap(),0);
        drop(connection);
        let semantic_context=reopened.epic_escalation_reassessment_context_for_test(&receiver.2).unwrap();
        assert!(semantic_context.get("acceptedEpicPlan").is_some()&&semantic_context.get("currentSprintState").is_some());
        assert!(!semantic_context.to_string().contains("handback-epic-runner-session"));
        let return_context=crate::orchestration::sprint_runner_transition::EpicEscalationReassessmentDisposition { movement_kind:"return_context_to_sprint_runner".into(),rationale:"the unresolved concern needs an explicit bounded decision from the Sprint Runner".into(),considered_intent:None,downstream_request:Some(crate::orchestration::sprint_runner_transition::EpicEscalationDownstreamRequest { target:crate::orchestration::sprint_runner_transition::EpicEscalationDownstreamTarget::SprintRunner,dependency:None,request:"return the missing dependency decision without clearing this concern".into(),resumption_path:"reassess this exact escalation after the Sprint Runner response".into() }),human_external_attention:None };
        let barrier=Arc::new(Barrier::new(2)); let concurrent=[reopened.clone(),reopened_receiver.clone()].into_iter().map(|service|{let barrier=barrier.clone();let invocation=receiver.2.clone();let disposition=return_context.clone();std::thread::spawn(move||{barrier.wait();service.record_epic_escalation_disposition_for_test(&invocation,disposition)})}).collect::<Vec<_>>(); assert!(concurrent.into_iter().all(|call|call.join().unwrap().is_ok()));
        reopened.record_epic_escalation_disposition_for_test(&receiver.2,return_context).unwrap();
        let divergent=crate::orchestration::sprint_runner_transition::EpicEscalationReassessmentDisposition { movement_kind:"consider_other_epic_work".into(),rationale:"different disposition".into(),considered_intent:Some("consider a separate safe Epic work area only".into()),downstream_request:None,human_external_attention:None }; assert!(matches!(fixture.reopened().record_epic_escalation_disposition_for_test(&receiver.2,divergent),Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        let connection=Connection::open(&fixture.base.database_path).unwrap();
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM epic_runner_escalation_dispositions WHERE handback_id=?1 AND preserves_handback=1",[&handback],|row|row.get(0)).unwrap(),1);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM epic_runner_escalation_downstream_requests WHERE handback_id=?1 AND request_kind='sprint_runner'",[&handback],|row|row.get(0)).unwrap(),1);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM epic_runner_escalation_attentions WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap(),0);
        assert!(connection.query_row::<Option<String>,_,_>("SELECT semantic_reassessment_recorded_at FROM epic_runner_escalation_receivers WHERE handback_id=?1",[&handback],|row|row.get(0)).unwrap().is_some());
        connection.execute_batch("PRAGMA foreign_keys=OFF").unwrap(); let attention_handback="attention-handback"; let attention_invocation="attention-reassessment"; let context_json=r#"{"classification":"blocked","reason":"external policy is missing","evidence":"bounded evidence"}"#; let details_json=r#"{"movementKind":"local_exhaustion_escalate","rationale":"local exhaustion","localExhaustionSummary":"no local route"}"#;
        connection.execute("INSERT INTO work_unit_no_progress_handbacks (handback_id,work_unit_id,source_attempt_id,source_review_invocation_id,decision_fingerprint,classification,context_json,context_fingerprint,persisted_at,delivery_intended_at) VALUES (?1,?2,'attention-attempt','attention-review','attention-decision','blocked',?3,'attention-context','now','now')",params![attention_handback,&fixture.work_unit_id,context_json]).unwrap();
        connection.execute("INSERT INTO sprint_runner_handback_dispositions (handback_id,disposition_id,movement_kind,details_json,disposition_fingerprint,selected_at,preserves_handback) VALUES (?1,'attention-disposition','local_exhaustion_escalate',?2,'attention-fingerprint','now',1)",params![attention_handback,details_json]).unwrap();
        connection.execute("INSERT INTO sprint_runner_handback_escalations (handback_id,escalation_intent_id,delivery_request_id,requested_at,delivery_requested_at,delivery_persisted_at) VALUES (?1,'attention-intent','attention-delivery','now','now','now')",[attention_handback]).unwrap();
        connection.execute("INSERT INTO epic_runner_escalation_receivers (handback_id,escalation_intent_id,delivery_request_id,sprint_id,epic_id,governing_runner_session_id,governing_runner_invocation_id,reassessment_invocation_id,delivery_fact_id,delivery_requested_at,delivery_persisted_at,harness_key,harness_version,harness_bound_at,launch_requested_at,launch_accepted_at,correlation_fingerprint) VALUES (?1,'attention-intent','attention-delivery',?2,'handback-epic','attention-session','attention-governing',?3,'attention-fact','now','now','epic_runner_escalation_reassessment',2,'now','now','now','attention-correlation')",params![attention_handback,&sprint_id,attention_invocation]).unwrap(); drop(connection);
        let attention=crate::orchestration::sprint_runner_transition::EpicEscalationReassessmentDisposition { movement_kind:"human_or_external_attention".into(),rationale:"the unresolved concern needs policy authority".into(),considered_intent:None,downstream_request:None,human_external_attention:Some(crate::orchestration::sprint_runner_transition::EpicEscalationAttention { reason:"external policy is missing".into(),authority_needed:"designated policy authority".into(),evidence_context:"bounded evidence".into(),resumption_path:"resume this exact reassessment after the policy decision".into() }) };
        let attention_service=fixture.reopened(); attention_service.record_epic_escalation_disposition_for_test(attention_invocation,attention.clone()).unwrap(); attention_service.record_epic_escalation_disposition_for_test(attention_invocation,attention).unwrap();
        let connection=Connection::open(&fixture.base.database_path).unwrap(); let attention_json:String=connection.query_row("SELECT attention_json FROM epic_runner_escalation_attentions WHERE handback_id=?1",[attention_handback],|row|row.get(0)).unwrap(); assert!(attention_json.contains("external policy is missing")&&attention_json.contains("designated policy authority")&&attention_json.contains("bounded evidence")&&attention_json.contains("resume this exact reassessment")); assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM epic_runner_escalation_downstream_requests WHERE handback_id=?1",[attention_handback],|row|row.get(0)).unwrap(),0); assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sprint_runner_transitions WHERE epic_continuation_invocation_id IS NOT NULL OR sprint_start_persisted_at IS NOT NULL",[],|row|row.get(0)).unwrap(),0);
        reopened.shutdown(); reopened_receiver.shutdown(); concurrent_receiver.shutdown(); fixture.transition.shutdown();
    }

    #[test]
    fn implementer_reporting_exact_retries_replay_without_replacement_and_divergence_conflicts() {
        let fixture = ReportingFixture::new();
        let barrier = Arc::new(Barrier::new(2));
        let calls = (0..2).map(|_| {
            let service = fixture.transition.clone();
            let invocation = fixture.invocation();
            let claims = fixture.claims();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.submit_implementation_outcome(&invocation, claims)
            })
        }).collect::<Vec<_>>();
        assert!(calls.into_iter().all(|call| call.join().unwrap().is_ok()));
        let recorded = fixture.facts();
        fixture.transition.submit_implementation_outcome(&fixture.invocation(), fixture.claims()).unwrap();
        assert_eq!(fixture.facts(), recorded);
        let mut divergent = fixture.claims();
        divergent.summary = "A different implementation claim.".into();
        assert!(matches!(
            fixture.transition.submit_implementation_outcome(&fixture.invocation(), divergent),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)
        ));
        assert_eq!(fixture.facts(), recorded);
        assert_eq!(fixture.identities(), fixture.expected_identities);
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_implementer_outcomes", [], |row| row.get(0),
        ).unwrap(), 1);
    }

    #[test]
    fn implementer_reporting_requires_pinned_file_evidence_then_completed_lifecycle_for_review_readiness() {
        let fixture = ReportingFixture::new();
        fixture.transition.submit_implementation_outcome(&fixture.invocation(), fixture.claims()).unwrap();
        assert!(matches!(
            fixture.transition.complete_implementation_outcome(&fixture.invocation()),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        let without_evidence = fixture.facts();
        assert!(without_evidence.evidence_ready_at.is_none());
        assert!(without_evidence.semantic_completed_at.is_none());

        fixture.write_evidence("implemented reporting boundary\n");
        fixture.assert_pinned_evidence_available();
        let barrier = Arc::new(Barrier::new(2));
        let calls = (0..2).map(|_| {
            let service = fixture.transition.clone();
            let invocation = fixture.invocation();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.complete_implementation_outcome(&invocation)
            })
        }).collect::<Vec<_>>();
        let results = calls.into_iter().map(|call| call.join().unwrap()).collect::<Vec<_>>();
        assert!(results.iter().any(Result::is_ok), "{results:?}");
        fixture.transition.complete_implementation_outcome(&fixture.invocation()).unwrap();
        let completed_semantic = fixture.facts();
        assert!(completed_semantic.evidence_manifest.as_deref().is_some_and(|value| value.contains("README.md")));
        assert!(completed_semantic.comparison_fingerprint.is_some());
        assert!(completed_semantic.evidence_contents.is_some());
        assert!(completed_semantic.evidence_ready_at.is_some());
        assert!(completed_semantic.semantic_completed_at.is_some());
        assert_eq!(completed_semantic.semantic_invocation.as_deref(), Some(fixture.reporting_invocation_id.as_str()));
        assert!(completed_semantic.application_accepted_at.is_none());
        assert!(completed_semantic.handler_review_ready_at.is_none());
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<String, _, _>(
            "SELECT role_id FROM execution_support_grants WHERE attempt_id=?1",
            [&fixture.attempt_id],
            |row| row.get(0),
        ).unwrap(), "work_unit_implementer");

        fixture.enable_notifications();
        fixture.finish(AgentInvocationTerminalStatus::Completed);
        let accepted = fixture.facts();
        assert_eq!(accepted.lifecycle_status.as_deref(), Some("completed"));
        assert!(accepted.application_accepted_at.is_some());
        assert!(accepted.handler_review_ready_at.is_some());
        assert_eq!(fixture.identities(), fixture.expected_identities);
    }

    #[test]
    fn implementer_reporting_rejects_wrong_foreign_stale_and_terminal_calls_without_semantic_facts() {
        let fixture = ReportingFixture::new();
        for invocation in [
            AgentInvocationId::new(fixture.implementer_invocation_id.clone()).unwrap(),
            AgentInvocationId::new("foreign-reporting-invocation").unwrap(),
        ] {
            assert!(matches!(
                fixture.transition.submit_implementation_outcome(&invocation, fixture.claims()),
                Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
            ));
            assert!(matches!(
                fixture.transition.complete_implementation_outcome(&invocation),
                Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
            ));
        }
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE work_unit_implementer_outcomes SET reporting_ready_at=NULL WHERE work_unit_id=?1",
            [&fixture.work_unit_id],
        ).unwrap();
        assert!(matches!(
            fixture.transition.submit_implementation_outcome(&fixture.invocation(), fixture.claims()),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE work_unit_implementer_outcomes SET reporting_ready_at='2026-08-04T00:00:00Z',reporting_harness_configuration_digest='stale' WHERE work_unit_id=?1",
            [&fixture.work_unit_id],
        ).unwrap();
        assert!(matches!(
            fixture.transition.submit_implementation_outcome(&fixture.invocation(), fixture.claims()),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)
        ));
        fixture.assert_no_submission_evidence_or_completion();

        let terminal = ReportingFixture::new();
        terminal.enable_notifications();
        terminal.finish(AgentInvocationTerminalStatus::Failed);
        assert!(matches!(
            terminal.transition.submit_implementation_outcome(&terminal.invocation(), terminal.claims()),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        assert!(matches!(
            terminal.transition.complete_implementation_outcome(&terminal.invocation()),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)
        ));
        terminal.assert_no_submission_evidence_or_completion();

        for status in [AgentInvocationTerminalStatus::Failed, AgentInvocationTerminalStatus::Canceled] {
            let lifecycle = ReportingFixture::new();
            lifecycle.transition.submit_implementation_outcome(&lifecycle.invocation(), lifecycle.claims()).unwrap();
            lifecycle.write_evidence("non-completed lifecycle evidence\n");
            lifecycle.transition.complete_implementation_outcome(&lifecycle.invocation()).unwrap();
            lifecycle.enable_notifications();
            lifecycle.finish(status);
            let facts = lifecycle.facts();
            assert!(facts.evidence_ready_at.is_some() && facts.semantic_completed_at.is_some());
            assert_ne!(facts.lifecycle_status.as_deref(), Some("completed"));
            assert!(facts.application_accepted_at.is_none());
            assert!(facts.handler_review_ready_at.is_none());
        }
    }

    #[test]
    fn implementer_reporting_reopen_reconciles_exact_identities_and_drift_blocks_acceptance() {
        let fixture = ReportingFixture::new();
        fixture.transition.submit_implementation_outcome(&fixture.invocation(), fixture.claims()).unwrap();
        fixture.write_evidence("reopen evidence\n");
        fixture.assert_pinned_evidence_available();
        fixture.transition.complete_implementation_outcome(&fixture.invocation()).unwrap();
        fixture.finish(AgentInvocationTerminalStatus::Completed);
        assert!(fixture.facts().lifecycle_status.is_none());
        let reopened = fixture.reopened();
        let barrier = Arc::new(Barrier::new(2));
        let calls = (0..2).map(|_| {
            let service = reopened.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.reconcile_reporting_for_test()
            })
        }).collect::<Vec<_>>();
        assert!(calls.into_iter().all(|call| call.join().unwrap().is_ok()));
        let accepted = fixture.facts();
        assert_eq!(accepted.lifecycle_status.as_deref(), Some("completed"));
        assert!(accepted.application_accepted_at.is_some());
        assert!(accepted.handler_review_ready_at.is_some());
        assert_eq!(fixture.identities(), fixture.expected_identities);

        let evidence_drift = ReportingFixture::new();
        evidence_drift.transition.submit_implementation_outcome(&evidence_drift.invocation(), evidence_drift.claims()).unwrap();
        evidence_drift.write_evidence("captured evidence\n");
        evidence_drift.transition.complete_implementation_outcome(&evidence_drift.invocation()).unwrap();
        evidence_drift.finish(AgentInvocationTerminalStatus::Completed);
        evidence_drift.write_evidence("changed after capture\n");
        assert!(matches!(
            evidence_drift.reopened().reconcile_reporting_for_test(),
            Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)
        ));
        assert!(evidence_drift.facts().application_accepted_at.is_none());
        assert!(evidence_drift.facts().handler_review_ready_at.is_none());

        for column in ["semantic_payload_json", "submission_fingerprint"] {
            let payload_drift = ReportingFixture::new();
            payload_drift.transition.submit_implementation_outcome(&payload_drift.invocation(), payload_drift.claims()).unwrap();
            payload_drift.write_evidence("payload evidence\n");
            payload_drift.transition.complete_implementation_outcome(&payload_drift.invocation()).unwrap();
            payload_drift.finish(AgentInvocationTerminalStatus::Completed);
            let value = if column == "semantic_payload_json" {
                r#"{"summary":"Implemented the reporting boundary.","outcome":"review_pending","validationStatement":"Focused deterministic proof passed."}"#
            } else {
                "drifted-fingerprint"
            };
            Connection::open(&payload_drift.base.database_path).unwrap().execute(
                &format!("UPDATE work_unit_implementer_outcomes SET {column}=?1 WHERE work_unit_id=?2"),
                params![value, payload_drift.work_unit_id],
            ).unwrap();
            assert!(matches!(
                payload_drift.reopened().reconcile_reporting_for_test(),
                Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)
            ));
            assert!(payload_drift.facts().application_accepted_at.is_none());
            assert!(payload_drift.facts().handler_review_ready_at.is_none());
        }
    }

    #[test]
    fn reserved_implementer_reporting_row_is_reopen_and_startup_safe() {
        let fixture = Fixture::unstarted();
        let service = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        ).unwrap();
        drop(service);
        let connection = Connection::open(&fixture.database_path).unwrap();
        connection.pragma_update(None, "foreign_keys", false).unwrap();
        connection.execute(
            "INSERT INTO work_unit_implementer_outcomes (
                work_unit_id,attempt_id,attempt_ordinal,implementer_session_id,implementer_invocation_id,
                reporting_invocation_id,reporting_harness_revision_id,
                reporting_harness_configuration_digest,reporting_harness_repository_commit_ref,
                reporting_requested_at
             ) VALUES ('reserved-work-unit','reserved-attempt',0,'reserved-session','reserved-original',
                       'reserved-reporting','reserved-revision','reserved-digest','reserved-commit',
                       '2026-08-04T00:00:00Z')",
            [],
        ).unwrap();
        connection.pragma_update(None, "foreign_keys", true).unwrap();
        drop(connection);
        let reopened = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.database_path,
            fixture.sessions.clone(),
        ).unwrap();
        assert_eq!(reopened.reconcile_startup().unwrap(), 0);
        assert_eq!(Connection::open(&fixture.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_implementer_outcomes
             WHERE reporting_ready_at IS NULL AND submitted_at IS NULL AND application_accepted_at IS NULL",
            [],
            |row| row.get(0),
        ).unwrap(), 1);
    }

    #[test]
    fn handler_review_uses_one_exact_read_only_boundary_and_finalizes_only_completed_judgments() {
        let accepted = ReportingFixture::new();
        let review = accepted.ready_review();
        let review_facts:(String,String,String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>) = Connection::open(&accepted.base.database_path).unwrap().query_row(
            "SELECT handler_session_id,review_invocation_id,review_harness_revision_id,review_harness_configuration_digest,review_harness_repository_commit_ref,delivery_persisted_at,harness_bound_at,launch_requested_at,launch_accepted_at,review_ready_at FROM work_unit_handler_reviews WHERE work_unit_id=?1", [&accepted.work_unit_id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?)),
        ).unwrap();
        assert_eq!(review_facts.0, accepted.handler_session_id);
        assert_eq!(review_facts.1, review);
        assert!(review_facts.5.is_some() && review_facts.6.is_some() && review_facts.7.is_some() && review_facts.8.is_some() && review_facts.9.is_some());
        let pinned = accepted.handler.load_pinned_handler_revision(&review_facts.2, &review_facts.3, &review_facts.4).unwrap();
        assert_eq!(pinned.profile.runtime_options().sandbox, Some(crate::agent_sessions::domain::RuntimeSandboxMode::ReadOnly));
        assert!(pinned.profile.runtime_configuration_args().iter().any(|value| value == "approval_policy=\"never\""));
        assert_eq!(pinned.profile.mcp.enabled_tools, ["read_handler_review_evidence", "accept_implementation_outcome", "return_implementation_outcome"]);
        let evidence: serde_json::Value = serde_json::from_str(&accepted.transition.handler_review_evidence_for_test(&review).unwrap()).unwrap();
        assert_eq!(evidence["summary"], "Implemented the reporting boundary.");
        assert!(!evidence["changedFiles"].is_null() && !evidence["evidenceContentFingerprints"].is_null());
        let launches_before_reopen = accepted.base.runtime.requests().len();
        Connection::open(&accepted.base.database_path).unwrap().execute("UPDATE work_unit_handler_reviews SET delivery_persisted_at=NULL,harness_bound_at=NULL,launch_requested_at=NULL,launch_accepted_at=NULL,review_ready_at=NULL WHERE work_unit_id=?1", [&accepted.work_unit_id]).unwrap();
        let reopened = accepted.reopened();
        reopened.reconcile_handler_reviews_for_test().unwrap();
        assert_eq!(accepted.base.runtime.requests().len(), launches_before_reopen);
        assert_eq!(Connection::open(&accepted.base.database_path).unwrap().query_row::<String,_,_>("SELECT review_invocation_id FROM work_unit_handler_reviews WHERE work_unit_id=?1", [&accepted.work_unit_id], |row| row.get(0)).unwrap(), review);
        assert_eq!(Connection::open(&accepted.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_handler_reviews", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(Connection::open(&accepted.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_handler_reviews WHERE work_unit_id=?1 AND delivery_persisted_at IS NOT NULL AND harness_bound_at IS NOT NULL AND launch_requested_at IS NOT NULL AND launch_accepted_at IS NOT NULL AND review_ready_at IS NOT NULL", [&accepted.work_unit_id], |row| row.get(0)).unwrap(), 1);
        let connection = Connection::open(&accepted.base.database_path).unwrap();
        let (delivered,delivery_fingerprint):(String,String) = connection.query_row("SELECT delivered_payload_json,delivered_payload_fingerprint FROM work_unit_handler_reviews WHERE work_unit_id=?1", [&accepted.work_unit_id], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
        connection.execute("UPDATE work_unit_handler_reviews SET delivered_payload_json='{}' WHERE work_unit_id=?1", [&accepted.work_unit_id]).unwrap();
        assert!(matches!(reopened.handler_review_evidence_for_test(&review), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        connection.execute("UPDATE work_unit_handler_reviews SET delivered_payload_json=?1 WHERE work_unit_id=?2", params![delivered,accepted.work_unit_id]).unwrap();
        let comparison:String = connection.query_row("SELECT comparison_fingerprint FROM work_unit_implementer_outcomes WHERE work_unit_id=?1", [&accepted.work_unit_id], |row| row.get(0)).unwrap();
        connection.execute("UPDATE work_unit_implementer_outcomes SET comparison_fingerprint='drifted-comparison' WHERE work_unit_id=?1", [&accepted.work_unit_id]).unwrap();
        assert!(matches!(reopened.handler_review_evidence_for_test(&review), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        connection.execute("UPDATE work_unit_implementer_outcomes SET comparison_fingerprint=?1 WHERE work_unit_id=?2", params![comparison,accepted.work_unit_id]).unwrap();
        connection.execute("UPDATE work_unit_handler_reviews SET delivered_payload_fingerprint='drifted-delivery' WHERE work_unit_id=?1", [&accepted.work_unit_id]).unwrap();
        assert!(matches!(reopened.handler_review_evidence_for_test(&review), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        connection.execute("UPDATE work_unit_handler_reviews SET delivered_payload_fingerprint=?1 WHERE work_unit_id=?2", params![delivery_fingerprint,accepted.work_unit_id]).unwrap();
        for invocation in [&accepted.handler_invocation_id, &accepted.handler_action_invocation_id, &accepted.implementer_invocation_id, &accepted.reporting_invocation_id, "foreign-review"] {
            assert!(matches!(reopened.handler_review_evidence_for_test(invocation), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)));
        }
        reopened.record_handler_review_judgment_for_test(&review, "accept", None).unwrap();
        reopened.record_handler_review_judgment_for_test(&review, "accept", None).unwrap();
        assert_eq!(Connection::open(&accepted.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_handler_decisions", [], |row| row.get(0)).unwrap(), 0);
        assert!(matches!(reopened.record_handler_review_judgment_for_test(&review, "return", Some(crate::orchestration::sprint_runner_transition::HandlerReviewReturnReason { code: "review_failed".into(), explanation: "evidence requires correction".into() })), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        accepted.base.runtime.finish(&review, AgentInvocationTerminalStatus::Completed);
        let decision:(String,Option<String>) = Connection::open(&accepted.base.database_path).unwrap().query_row("SELECT decision_variant,settlement_ready_at FROM work_unit_handler_decisions WHERE work_unit_id=?1", [&accepted.work_unit_id], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
        assert_eq!(decision, ("accepted".into(), None));
        assert!(matches!(reopened.handler_review_evidence_for_test(&review), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Forbidden)));

        let returned = ReportingFixture::new();
        let returned_review = returned.ready_review();
        let reason = crate::orchestration::sprint_runner_transition::HandlerReviewReturnReason { code: "review_failed".into(), explanation: "evidence requires correction".into() };
        returned.transition.record_handler_review_judgment_for_test(&returned_review, "return", Some(reason.clone())).unwrap();
        returned.transition.record_handler_review_judgment_for_test(&returned_review, "return", Some(reason.clone())).unwrap();
        assert!(matches!(returned.transition.record_handler_review_judgment_for_test(&returned_review, "accept", None), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        returned.base.runtime.finish(&returned_review, AgentInvocationTerminalStatus::Completed);
        returned.transition.reconcile_handler_reviews_for_test().unwrap();
        let return_decision:(String,String,Option<String>,Option<String>) = Connection::open(&returned.base.database_path).unwrap().query_row("SELECT decision_variant,return_reason_json,settlement_ready_at,retry_required_at FROM work_unit_handler_decisions WHERE work_unit_id=?1", [&returned.work_unit_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
        assert_eq!(return_decision.0, "returned");
        assert_eq!(serde_json::from_str::<serde_json::Value>(&return_decision.1).unwrap()["code"], "review_failed");
        assert!(return_decision.2.is_none());
        assert!(return_decision.3.is_some());
        assert_eq!(Connection::open(&returned.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_implementer_activations WHERE work_unit_id=?1", [&returned.work_unit_id], |row| row.get(0)).unwrap(), 1);
        let retry: (i64,String,String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,String) = Connection::open(&returned.base.database_path).unwrap().query_row(
            "SELECT ordinal,origin_attempt_id,retry_attempt_id,implementer_session_id,implementer_invocation_id,candidate_pinned_at,launch_accepted_at,retry_ready_at,failure_reason,handoff_json FROM work_unit_retry_attempts WHERE work_unit_id=?1",
            [&returned.work_unit_id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?)),
        ).unwrap();
        assert_eq!(retry.0, 1);
        assert_eq!(retry.1, returned.attempt_id);
        assert_ne!(retry.2, returned.attempt_id);
        assert_ne!(retry.4, returned.implementer_invocation_id);
        assert!(retry.5.is_some() && retry.6.is_some() && retry.7.is_some());
        assert!(retry.8.is_none());
        let handoff: serde_json::Value = serde_json::from_str(&retry.9).unwrap();
        assert_eq!(handoff["handlerReturnReason"]["code"], "review_failed");
        assert!(handoff.get("privateRef").is_none() && handoff.get("candidateCommitId").is_none());
        let retry_before_reopen = (retry.2.clone(), retry.3.clone(), retry.4.clone());
        let launches_before_reopen = returned.base.runtime.requests().len();
        let reopened_retry = returned.reopened();
        reopened_retry.reconcile_handler_reviews_for_test().unwrap();
        assert_eq!(returned.base.runtime.requests().len(), launches_before_reopen);
        let retry_after_reopen: (String,String,String,String,String,String) = Connection::open(&returned.base.database_path).unwrap().query_row(
            "SELECT retry_attempt_id,implementer_session_id,implementer_invocation_id,private_ref_name,candidate_commit_id,sprint_current_object_id FROM work_unit_retry_attempts WHERE work_unit_id=?1", [&returned.work_unit_id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?)),
        ).unwrap();
        assert_eq!((retry_after_reopen.0.clone(), retry_after_reopen.1.clone(), retry_after_reopen.2.clone()), retry_before_reopen);
        Connection::open(&returned.base.database_path).unwrap().execute(
            "UPDATE work_unit_retry_attempts SET sprint_baseline_object_id='tampered-baseline' WHERE work_unit_id=?1",
            [&returned.work_unit_id],
        ).unwrap();
        assert!(matches!(reopened_retry.reconcile_handler_reviews_for_test(), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        assert_eq!(Connection::open(&returned.base.database_path).unwrap().query_row::<String,_,_>(
            "SELECT failure_reason FROM work_unit_retry_attempts WHERE work_unit_id=?1", [&returned.work_unit_id], |row| row.get(0),
        ).unwrap(), "retry_immutable_lineage_mismatch");
        Connection::open(&returned.base.database_path).unwrap().execute(
            "UPDATE work_unit_retry_attempts SET sprint_baseline_object_id=(SELECT baseline_object_id FROM initiated_sprint_git_authorities WHERE authority_id=?2),failure_reason=NULL WHERE work_unit_id=?1",
            params![returned.work_unit_id, returned.authority_id],
        ).unwrap();
        reopened_retry.reconcile_handler_reviews_for_test().unwrap();
        let authority_root: String = Connection::open(&returned.base.database_path).unwrap().query_row(
            "SELECT repository_root FROM initiated_sprint_git_authorities WHERE authority_id=?1", [&returned.authority_id], |row| row.get(0),
        ).unwrap();
        let retarget = std::process::Command::new("git").args(["update-ref", &retry_after_reopen.3, &retry_after_reopen.5])
            .current_dir(&authority_root).output().unwrap();
        assert!(retarget.status.success());
        assert!(matches!(reopened_retry.reconcile_handler_reviews_for_test(), Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict)));
        assert_eq!(Connection::open(&returned.base.database_path).unwrap().query_row::<String,_,_>(
            "SELECT failure_reason FROM work_unit_retry_attempts WHERE work_unit_id=?1", [&returned.work_unit_id], |row| row.get(0),
        ).unwrap(), "retry_private_ref_pin_failed");
        let restore = std::process::Command::new("git").args(["update-ref", &retry_after_reopen.3, &retry_after_reopen.4])
            .current_dir(&authority_root).output().unwrap();
        assert!(restore.status.success());
        reopened_retry.reconcile_handler_reviews_for_test().unwrap();
        let recovered: (String,String,String,Option<String>,Option<String>) = Connection::open(&returned.base.database_path).unwrap().query_row(
            "SELECT retry_attempt_id,implementer_session_id,implementer_invocation_id,failure_reason,retry_ready_at FROM work_unit_retry_attempts WHERE work_unit_id=?1", [&returned.work_unit_id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
        ).unwrap();
        assert_eq!((recovered.0,recovered.1,recovered.2), retry_before_reopen);
        assert!(recovered.3.is_none() && recovered.4.is_some());
        assert_eq!(Connection::open(&returned.base.database_path).unwrap().query_row::<i64,_,_>(
            "SELECT COUNT(*) FROM work_unit_retry_attempts WHERE work_unit_id=?1 AND ordinal=1", [&returned.work_unit_id], |row| row.get(0),
        ).unwrap(), 1);
        // Once retry lineage is durable, source-candidate drift is a bounded semantic failure.
        // Reopen does not silently leave the prior ready projection in place or allocate a new
        // retry identity; restoring the exact source lets the same retry recover.
        let source_drift = returned.working_directory.join("retry-source-drift.txt");
        fs::write(&source_drift, "untracked retry source drift\n").unwrap();
        assert!(reopened_retry.reconcile_handler_reviews_for_test().is_err());
        assert_eq!(Connection::open(&returned.base.database_path).unwrap().query_row::<String,_,_>(
            "SELECT failure_reason FROM work_unit_retry_attempts WHERE work_unit_id=?1", [&returned.work_unit_id], |row| row.get(0),
        ).unwrap(), "retry_evidence_revalidation_failed");
        fs::remove_file(source_drift).unwrap();
        reopened_retry.reconcile_handler_reviews_for_test().unwrap();
        assert_eq!(Connection::open(&returned.base.database_path).unwrap().query_row::<i64,_,_>(
            "SELECT COUNT(*) FROM work_unit_retry_attempts WHERE work_unit_id=?1 AND ordinal=1", [&returned.work_unit_id], |row| row.get(0),
        ).unwrap(), 1);
        // Policy B: a runtime start error terminally fails this exact ordinal-1 invocation.  The
        // retry row remains factual and unready; reopen only observes it and never relaunches.
        let connection = Connection::open(&returned.base.database_path).unwrap();
        connection.execute("DELETE FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1", [&retry_before_reopen.2]).unwrap();
        connection.execute("UPDATE agent_session_invocations SET status='pending',effective_options_json=NULL,started_at=NULL,completed_at=NULL,exit_code=NULL,signal=NULL,runtime_error_json=NULL WHERE id=?1", [&retry_before_reopen.2]).unwrap();
        connection.execute("UPDATE work_unit_retry_attempts SET launch_accepted_at=NULL,provider_activation_observed_at=NULL,retry_ready_at=NULL,failure_reason=NULL WHERE work_unit_id=?1", [&returned.work_unit_id]).unwrap();
        drop(connection);
        let launches_before_terminal_failure = returned.base.runtime.requests().len();
        returned.base.runtime.fail_next_launch();
        assert!(reopened_retry.reconcile_handler_reviews_for_test().is_err());
        let terminal_failure: (String,String,String,String,Option<String>,Option<String>,Option<String>,String) = Connection::open(&returned.base.database_path).unwrap().query_row(
            "SELECT retry_attempt_id,implementer_session_id,implementer_invocation_id,failure_reason,launch_accepted_at,provider_activation_observed_at,retry_ready_at,(SELECT status FROM agent_session_invocations WHERE id=implementer_invocation_id) FROM work_unit_retry_attempts WHERE work_unit_id=?1", [&returned.work_unit_id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?)),
        ).unwrap();
        assert_eq!((terminal_failure.0.clone(),terminal_failure.1.clone(),terminal_failure.2.clone()), retry_before_reopen);
        assert_eq!(terminal_failure.3, "retry_terminal_launch_failed");
        assert!(terminal_failure.4.is_none() && terminal_failure.5.is_none() && terminal_failure.6.is_none());
        assert_eq!(terminal_failure.7, "failed");
        let terminal_reopen = returned.reopened();
        terminal_reopen.reconcile_handler_reviews_for_test().unwrap();
        assert_eq!(returned.base.runtime.requests().len(), launches_before_terminal_failure + 1);
        assert_eq!(Connection::open(&returned.base.database_path).unwrap().query_row::<String,_,_>("SELECT failure_reason FROM work_unit_retry_attempts WHERE work_unit_id=?1", [&returned.work_unit_id], |row| row.get(0)).unwrap(), "retry_terminal_launch_failed");

        let without_judgment = ReportingFixture::new();
        let without_judgment_review = without_judgment.ready_review();
        without_judgment.base.runtime.finish(&without_judgment_review, AgentInvocationTerminalStatus::Completed);
        assert_eq!(Connection::open(&without_judgment.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_handler_decisions", [], |row| row.get(0)).unwrap(), 0);

        for status in [AgentInvocationTerminalStatus::Failed, AgentInvocationTerminalStatus::Canceled, AgentInvocationTerminalStatus::Interrupted] {
            let terminal = ReportingFixture::new();
            let terminal_review = terminal.ready_review();
            terminal.transition.record_handler_review_judgment_for_test(&terminal_review, "accept", None).unwrap();
            terminal.base.runtime.finish(&terminal_review, status);
            assert_eq!(Connection::open(&terminal.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_handler_decisions", [], |row| row.get(0)).unwrap(), 0);
        }
    }

    #[test]
    fn returned_retry_two_independent_services_converge_on_one_ordinal_one_launch() {
        let fixture = ReportingFixture::new();
        let review = fixture.ready_review();
        fixture.transition.record_handler_incomplete_disposition_for_test(
            &review,
            crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition {
                code: "review_failed".into(), explanation: "evidence requires correction".into(),
                classification: crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::RefinementNeeded,
                meaningful_progress: true,
            },
        ).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET status='completed',completed_at=?2 WHERE id=?1",
            params![review, "2026-08-04T00:00:01Z"],
        ).unwrap();

        let first = fixture.reopened();
        let second = fixture.reopened();
        let barrier = Arc::new(Barrier::new(2));
        let drains = [first, second].map(|service| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.reconcile_handler_reviews_for_test()
            })
        });
        assert!(drains.into_iter().all(|drain| drain.join().unwrap().is_ok()));

        let (attempt, session, invocation, revision, private_ref, pinned, accepted, ready):
            (String, String, String, String, String, Option<String>, Option<String>, Option<String>) =
            Connection::open(&fixture.base.database_path).unwrap().query_row(
                "SELECT retry_attempt_id,implementer_session_id,implementer_invocation_id,
                        implementer_harness_revision_id,private_ref_name,candidate_pinned_at,
                        launch_accepted_at,retry_ready_at
                 FROM work_unit_retry_attempts WHERE work_unit_id=?1 AND ordinal=1",
                [&fixture.work_unit_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
            ).unwrap();
        assert!(pinned.is_some() && accepted.is_some() && ready.is_some());
        let connection = Connection::open(&fixture.base.database_path).unwrap();
        assert_eq!(fixture.retry_count(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM execution_support_grants WHERE attempt_id=?1", [&attempt], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_sessions WHERE id=?1", [&session], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_session_invocations WHERE id=?1 AND session_id=?2 AND input_provenance='application'", params![invocation, session], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1", [&invocation], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_retry_attempts WHERE implementer_harness_revision_id=?1", [&revision], |row| row.get(0)).unwrap(), 1);
        drop(connection);
        let authority_root: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT repository_root FROM initiated_sprint_git_authorities WHERE authority_id=?1",
            [&fixture.authority_id],
            |row| row.get(0),
        ).unwrap();
        let target = std::process::Command::new("git")
            .args(["rev-parse", "--verify", &format!("{private_ref}^{{commit}}")])
            .current_dir(authority_root)
            .output().unwrap();
        assert!(target.status.success());
        assert_eq!(fixture.base.runtime.requests().iter().filter(|request| request.invocation_id.as_str() == invocation).count(), 1);
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_handler_reviews WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| row.get(0),
        ).unwrap(), 1);
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_implementer_outcomes WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| row.get(0),
        ).unwrap(), 1);
    }

    #[test]
    fn meaningful_progress_replays_one_productive_ordinal_two_attempt() {
        let fixture = ReportingFixture::new();
        let first_review = fixture.ready_review();
        fixture.transition.record_handler_incomplete_disposition_for_test(
            &first_review,
            crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition {
                code: "needs_refinement".into(), explanation: "the first correction remains bounded".into(),
                classification: crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::RefinementNeeded,
                meaningful_progress: true,
            },
        ).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET status='completed',completed_at=?2 WHERE id=?1",
            params![first_review, "2026-08-04T00:00:01Z"],
        ).unwrap();
        fixture.transition.reconcile_handler_reviews_for_test().unwrap();

        let (first_attempt, first_implementer, revision, digest, commit): (String, String, String, String, String) =
            Connection::open(&fixture.base.database_path).unwrap().query_row(
                "SELECT retry_attempt_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref FROM work_unit_retry_attempts WHERE work_unit_id=?1 AND ordinal=1",
                [&fixture.work_unit_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            ).unwrap();
        let pinned = fixture.handler.load_pinned_implementer_revision(&revision, &digest, &commit).unwrap();
        let package = fixture.handler.construct_for_pinned_profile(&first_attempt, WorkUnitHarnessRole::Implementer, pinned.profile).unwrap();
        let workspace = PathBuf::from(package.working_directory());
        fs::write(workspace.join("README.md"), "ordinal-one evidence\n").unwrap();
        for arguments in [["add", "README.md"].as_slice(), ["commit", "-m", "ordinal one evidence"].as_slice()] {
            let output = std::process::Command::new("git").args(arguments).current_dir(&workspace).output().unwrap();
            assert!(output.status.success(), "{output:?}");
        }
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET status='completed',completed_at=?2 WHERE id=?1",
            params![first_implementer, "2026-08-04T00:00:02Z"],
        ).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE work_unit_implementer_activations SET implementer_ready_at=NULL WHERE work_unit_id=?1",
            [&fixture.work_unit_id],
        ).unwrap();
        fixture.transition.prepare_later_attempt_reporting_for_test().unwrap();

        let reporting: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT reporting_invocation_id FROM work_unit_implementer_outcomes WHERE attempt_id=?1", [&first_attempt], |row| row.get(0),
        ).unwrap();
        let reporting = AgentInvocationId::new(reporting).unwrap();
        fixture.transition.submit_implementation_outcome(&reporting, fixture.claims()).unwrap();
        fixture.transition.complete_implementation_outcome(&reporting).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET status='completed',completed_at=?2 WHERE id=?1",
            params![reporting.as_str(), "2026-08-04T00:00:03Z"],
        ).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE work_unit_implementer_activations SET implementer_ready_at=?2 WHERE work_unit_id=?1",
            params![fixture.work_unit_id, "2026-08-04T00:00:00Z"],
        ).unwrap();
        fixture.transition.reconcile_later_attempt_for_test().unwrap();

        let second_review: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT review_invocation_id FROM work_unit_handler_reviews WHERE attempt_id=?1", [&first_attempt], |row| row.get(0),
        ).unwrap();
        fixture.transition.record_handler_incomplete_disposition_for_test(
            &second_review,
            crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition {
                code: "needs_another_refinement".into(), explanation: "the second correction remains bounded".into(),
                classification: crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::RefinementNeeded,
                meaningful_progress: true,
            },
        ).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET status='completed',completed_at=?2 WHERE id=?1",
            params![second_review, "2026-08-04T00:00:04Z"],
        ).unwrap();
        fixture.transition.reconcile_handler_reviews_for_test().unwrap();

        let connection = Connection::open(&fixture.base.database_path).unwrap();
        let second: (String, String, String, String, String, Option<String>, Option<String>) = connection.query_row(
            "SELECT retry_attempt_id,implementer_session_id,implementer_invocation_id,origin_attempt_id,candidate_commit_id,launch_accepted_at,retry_ready_at FROM work_unit_retry_attempts WHERE work_unit_id=?1 AND ordinal=2",
            [&fixture.work_unit_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
        ).unwrap();
        assert_eq!(second.3, first_attempt);
        assert!(second.5.is_some() && second.6.is_some());
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_implementer_outcomes WHERE work_unit_id=?1 AND attempt_ordinal IN (0,1)", [&fixture.work_unit_id], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_incomplete_dispositions WHERE work_unit_id=?1 AND meaningful_progress=1 AND next_attempt_authorized_at IS NOT NULL", [&fixture.work_unit_id], |row| row.get(0)).unwrap(), 2);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_no_progress_handbacks WHERE work_unit_id=?1", [&fixture.work_unit_id], |row| row.get(0)).unwrap(), 0);
        drop(connection);

        fixture.reopened().reconcile_handler_reviews_for_test().unwrap();
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<(String, String, String, String, String), _, _>(
            "SELECT retry_attempt_id,implementer_session_id,implementer_invocation_id,origin_attempt_id,candidate_commit_id FROM work_unit_retry_attempts WHERE work_unit_id=?1 AND ordinal=2",
            [&fixture.work_unit_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).unwrap(), (second.0, second.1, second.2, second.3, second.4));
    }

    #[test]
    fn populated_legacy_attempt_history_and_retry_migrate_without_overwriting_ordinal_zero() {
        let fixture = ReportingFixture::new();
        let connection = Connection::open(&fixture.base.database_path).unwrap();
        connection.pragma_update(None, "foreign_keys", false).unwrap();
        connection.execute_batch(r#"
DROP TABLE work_unit_handler_decisions;
DROP TABLE work_unit_handler_reviews;
DROP TABLE work_unit_implementer_outcomes;
DROP TABLE work_unit_retry_attempts;
CREATE TABLE work_unit_implementer_outcomes (
  work_unit_id TEXT PRIMARY KEY, attempt_id TEXT, attempt_ordinal INTEGER, implementer_session_id TEXT, implementer_invocation_id TEXT,
  reporting_invocation_id TEXT, reporting_harness_revision_id TEXT, reporting_harness_configuration_digest TEXT, reporting_harness_repository_commit_ref TEXT,
  reporting_requested_at TEXT, reporting_prepared_at TEXT, reporting_harness_bound_at TEXT, reporting_launch_requested_at TEXT, reporting_launch_accepted_at TEXT,
  reporting_ready_at TEXT, submitted_summary TEXT, outcome_variant TEXT, submitted_validation_statement TEXT, semantic_payload_json TEXT,
  submission_fingerprint TEXT, submitted_at TEXT, validation_at TEXT, validation_result TEXT, evidence_manifest_json TEXT, comparison_fingerprint TEXT,
  evidence_content_fingerprints_json TEXT, evidence_ready_at TEXT, semantic_completed_at TEXT, semantic_completion_invocation_id TEXT,
  lifecycle_observed_at TEXT, lifecycle_status TEXT, application_accepted_at TEXT, handler_review_ready_at TEXT, failure_reason TEXT
);
CREATE TABLE work_unit_handler_reviews (
  work_unit_id TEXT PRIMARY KEY, attempt_id TEXT, reporting_invocation_id TEXT, handler_session_id TEXT, original_handler_invocation_id TEXT,
  action_handler_invocation_id TEXT, review_invocation_id TEXT, review_harness_revision_id TEXT, review_harness_configuration_digest TEXT,
  review_harness_repository_commit_ref TEXT, delivery_requested_at TEXT, delivery_persisted_at TEXT, harness_bound_at TEXT, launch_requested_at TEXT,
  launch_accepted_at TEXT, review_ready_at TEXT, delivered_payload_json TEXT, delivered_payload_fingerprint TEXT, semantic_judgment_variant TEXT,
  semantic_return_reason_json TEXT, semantic_judgment_fingerprint TEXT, semantic_judgment_at TEXT, lifecycle_observed_at TEXT, lifecycle_status TEXT,
  conflict_at TEXT, conflict_reason TEXT
);
CREATE TABLE work_unit_handler_decisions (
  review_invocation_id TEXT PRIMARY KEY, work_unit_id TEXT, decision_variant TEXT, decision_fingerprint TEXT, return_reason_json TEXT,
  decision_recorded_at TEXT, implementation_accepted_at TEXT, implementation_returned_at TEXT, retry_required_at TEXT, settlement_ready_at TEXT
);
CREATE TABLE work_unit_retry_attempts (
  work_unit_id TEXT PRIMARY KEY, ordinal INTEGER NOT NULL CHECK (ordinal=1), origin_attempt_id TEXT, review_invocation_id TEXT,
  decision_fingerprint TEXT, sprint_git_authority_id TEXT, sprint_baseline_object_id TEXT, sprint_current_object_id TEXT, retry_attempt_id TEXT,
  implementer_session_id TEXT, implementer_invocation_id TEXT, implementer_harness_revision_id TEXT, implementer_harness_configuration_digest TEXT,
  implementer_harness_repository_commit_ref TEXT, capture_intent_id TEXT, capture_fingerprint TEXT, handoff_json TEXT, handoff_fingerprint TEXT,
  candidate_commit_id TEXT, candidate_tree_id TEXT, private_ref_name TEXT, capture_requested_at TEXT, candidate_pinned_at TEXT, authorized_at TEXT,
  execution_support_granted_at TEXT, isolated_worktree_ready_at TEXT, implementer_session_created_at TEXT, implementer_invocation_prepared_at TEXT,
  implementer_harness_bound_at TEXT, launch_requested_at TEXT, launch_accepted_at TEXT, provider_activation_observed_at TEXT, retry_ready_at TEXT,
  failure_reason TEXT
);
"#).unwrap();
        let time = "2026-08-04T00:00:00Z";
        let outcome = ("legacy-attempt-0", "legacy-session-0", "legacy-original-0", "legacy-reporting-0");
        connection.execute(
            "INSERT INTO work_unit_implementer_outcomes VALUES (?1,?2,0,?3,?4,?5,'reporting-revision','reporting-digest','reporting-commit',?6,?6,?6,?6,?6,?6,'legacy summary','review_pending','legacy validation','{}','submission',?6,?6,'valid','[]','comparison','[]',?6,?6,?5,?6,'completed',?6,?6,NULL)",
            params![fixture.work_unit_id, outcome.0, outcome.1, outcome.2, outcome.3, time],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_handler_reviews VALUES (?1,?2,?3,'legacy-handler-session','legacy-handler-original','legacy-handler-action','legacy-review','review-revision','review-digest','review-commit',?4,?4,?4,?4,?4,?4,'{\"summary\":\"legacy summary\",\"validationStatement\":\"legacy validation\",\"changedFiles\":[],\"comparisonFingerprint\":\"comparison\",\"evidenceContentFingerprints\":[]}','delivery','return','{\"code\":\"review_failed\",\"explanation\":\"legacy return\"}','judgment',?4,?4,'completed',NULL,NULL)",
            params![fixture.work_unit_id, outcome.0, outcome.3, time],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_handler_decisions VALUES ('legacy-review',?1,'returned','legacy-decision','{\"code\":\"review_failed\",\"explanation\":\"legacy return\"}',?2,NULL,?2,?2,NULL)",
            params![fixture.work_unit_id, time],
        ).unwrap();
        connection.execute(
            "INSERT INTO work_unit_retry_attempts VALUES (?1,1,?2,'legacy-review','legacy-decision','legacy-authority','baseline','current','legacy-retry-1','legacy-retry-session','legacy-retry-invocation','retry-revision','retry-digest','retry-commit','capture','capture-fingerprint','{}','handoff-fingerprint','candidate','tree','refs/private',?3,?3,?3,?3,?3,?3,?3,?3,?3,?3,?3,?3,NULL)",
            params![fixture.work_unit_id, outcome.0, time],
        ).unwrap();
        connection.pragma_update(None, "foreign_keys", true).unwrap();
        drop(connection);

        let reopened = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.base.database_path,
            fixture.base.sessions.clone(),
        ).unwrap();
        let connection = Connection::open(&fixture.base.database_path).unwrap();
        assert_eq!(connection.query_row::<(String, i64, String, String), _, _>(
            "SELECT attempt_id,attempt_ordinal,submitted_summary,reporting_invocation_id FROM work_unit_implementer_outcomes WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).unwrap(), (outcome.0.into(), 0, "legacy summary".into(), outcome.3.into()));
        assert_eq!(connection.query_row::<(String, String), _, _>(
            "SELECT attempt_id,review_invocation_id FROM work_unit_handler_reviews WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap(), (outcome.0.into(), "legacy-review".into()));
        assert_eq!(connection.query_row::<(String, String, String), _, _>(
            "SELECT attempt_id,review_invocation_id,decision_variant FROM work_unit_handler_decisions WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap(), (outcome.0.into(), "legacy-review".into(), "returned".into()));
        assert_eq!(connection.query_row::<(i64, String, String), _, _>(
            "SELECT ordinal,origin_attempt_id,retry_attempt_id FROM work_unit_retry_attempts WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap(), (1, outcome.0.into(), "legacy-retry-1".into()));
        let retry_schema: String = connection.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='work_unit_retry_attempts'", [], |row| row.get(0),
        ).unwrap();
        assert!(retry_schema.contains("retry_attempt_id TEXT PRIMARY KEY"));
        assert!(retry_schema.contains("CHECK (ordinal >= 0)"));
        assert!(!retry_schema.contains("CHECK (ordinal=1)"));
        drop(connection);
        drop(reopened);
        let reopened_again = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open(
            &fixture.base.database_path,
            fixture.base.sessions.clone(),
        ).unwrap();
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM work_unit_implementer_outcomes WHERE attempt_id='legacy-attempt-0'", [], |row| row.get(0),
        ).unwrap(), 1);
        drop(reopened_again);
    }

    #[test]
    fn returned_retry_adopts_each_exact_effect_after_its_intent_crash_window() {
        let fixture = ReportingFixture::new();
        let (attempt, session, invocation, private_ref) = fixture.return_one_retry();
        let launches = fixture.base.runtime.requests().len();
        let reopened = fixture.reopened();
        let stage = |fixture: &ReportingFixture, columns: &str| {
            let assignments = columns.replace(',', "=NULL,");
            Connection::open(&fixture.base.database_path).unwrap().execute(
                &format!("UPDATE work_unit_retry_attempts SET {assignments}=NULL,failure_reason=NULL WHERE work_unit_id=?1"),
                [&fixture.work_unit_id],
            ).unwrap();
            reopened.reconcile_handler_reviews_for_test().unwrap();
            assert_eq!(fixture.retry_count(), 1);
            assert_eq!(fixture.base.runtime.requests().len(), launches);
        };

        let authority_root: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT repository_root FROM initiated_sprint_git_authorities WHERE authority_id=?1",
            [&fixture.authority_id], |row| row.get(0),
        ).unwrap();
        assert!(std::process::Command::new("git").args(["update-ref", "-d", &private_ref])
            .current_dir(&authority_root).output().unwrap().status.success());
        stage(&fixture, "candidate_pinned_at");
        stage(&fixture, "candidate_pinned_at");
        stage(&fixture, "execution_support_granted_at,isolated_worktree_ready_at");
        stage(&fixture, "implementer_session_created_at");
        stage(&fixture, "implementer_invocation_prepared_at");
        stage(&fixture, "implementer_harness_bound_at");
        stage(&fixture, "launch_accepted_at,retry_ready_at");

        let connection = Connection::open(&fixture.base.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM execution_support_grants WHERE attempt_id=?1", [&attempt], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_sessions WHERE id=?1", [&session], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_session_invocations WHERE id=?1", [&invocation], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_session_invocation_launch_acceptances WHERE invocation_id=?1", [&invocation], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_retry_attempts WHERE ordinal=2", [], |row| row.get(0)).unwrap(), 0);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_reviews WHERE work_unit_id=?1", [&fixture.work_unit_id], |row| row.get(0)).unwrap(), 1);
    }

    #[test]
    fn returned_retry_divergent_effects_fail_closed_without_replacement() {
        let fixture = ReportingFixture::new();
        let (attempt, session, invocation, private_ref) = fixture.return_one_retry();
        let launches = fixture.base.runtime.requests().len();
        let reopened = fixture.reopened();
        let retry_identity = (attempt.clone(), session.clone(), invocation.clone(), private_ref.clone());
        let failure = |expected: &str| {
            assert!(reopened.reconcile_handler_reviews_for_test().is_err());
            assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<String, _, _>(
                "SELECT failure_reason FROM work_unit_retry_attempts WHERE work_unit_id=?1",
                [&fixture.work_unit_id], |row| row.get(0),
            ).unwrap(), expected);
            assert_eq!(fixture.retry_count(), 1);
            assert_eq!(fixture.base.runtime.requests().len(), launches);
        };
        let recover = || {
            reopened.reconcile_handler_reviews_for_test().unwrap();
            assert_eq!(fixture.retry_count(), 1);
            assert_eq!(fixture.base.runtime.requests().len(), launches);
            let identity: (String,String,String,String) = Connection::open(&fixture.base.database_path).unwrap().query_row(
                "SELECT retry_attempt_id,implementer_session_id,implementer_invocation_id,private_ref_name
                 FROM work_unit_retry_attempts WHERE work_unit_id=?1",
                [&fixture.work_unit_id],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
            ).unwrap();
            assert_eq!(identity, retry_identity);
        };

        let authority_root: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT repository_root FROM initiated_sprint_git_authorities WHERE authority_id=?1",
            [&fixture.authority_id], |row| row.get(0),
        ).unwrap();
        let seed: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT candidate_commit_id FROM work_unit_retry_attempts WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| row.get(0),
        ).unwrap();
        let foreign_target: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT sprint_current_object_id FROM work_unit_retry_attempts WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| row.get(0),
        ).unwrap();
        assert!(std::process::Command::new("git").args(["update-ref", &private_ref, &foreign_target])
            .current_dir(&authority_root).output().unwrap().status.success());
        failure("retry_private_ref_pin_failed");
        assert!(std::process::Command::new("git").args(["update-ref", &private_ref, &seed])
            .current_dir(&authority_root).output().unwrap().status.success());
        recover();

        let retry_worktree: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT working_directory FROM agent_sessions WHERE id=?1", [&session], |row| row.get(0),
        ).unwrap();
        let worktree_drift = PathBuf::from(&retry_worktree).join("retry-worktree-drift.txt");
        fs::write(&worktree_drift, "untracked divergence\n").unwrap();
        failure("retry_workspace_validation_failed");
        fs::remove_file(&worktree_drift).unwrap();
        recover();

        let baseline: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT sprint_baseline_object_id FROM work_unit_retry_attempts WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| row.get(0),
        ).unwrap();
        assert_ne!(baseline, seed);
        assert!(std::process::Command::new("git").args(["checkout", "--detach", &baseline])
            .current_dir(&retry_worktree).output().unwrap().status.success());
        failure("retry_workspace_validation_failed");
        assert!(std::process::Command::new("git").args(["checkout", "--detach", &seed])
            .current_dir(&retry_worktree).output().unwrap().status.success());
        recover();

        let original_common: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT repository_common_dir FROM initiated_sprint_git_authorities WHERE authority_id=?1",
            [&fixture.authority_id], |row| row.get(0),
        ).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE initiated_sprint_git_authorities SET repository_common_dir='C:\\foreign-common' WHERE authority_id=?1",
            [&fixture.authority_id],
        ).unwrap();
        failure("retry_evidence_revalidation_failed");
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE initiated_sprint_git_authorities SET repository_common_dir=?2 WHERE authority_id=?1",
            params![fixture.authority_id, original_common],
        ).unwrap();
        recover();

        let original_root: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT repository_root FROM initiated_sprint_git_authorities WHERE authority_id=?1",
            [&fixture.authority_id], |row| row.get(0),
        ).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE initiated_sprint_git_authorities SET repository_root=?2 WHERE authority_id=?1",
            params![fixture.authority_id, retry_worktree],
        ).unwrap();
        failure("retry_evidence_revalidation_failed");
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE initiated_sprint_git_authorities SET repository_root=?2 WHERE authority_id=?1",
            params![fixture.authority_id, original_root],
        ).unwrap();
        recover();

        let original_baseline: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT baseline_object_id FROM execution_support_attempt_authorizations
             WHERE attempt_id=?1 AND role_kind='work_unit_implementer'",
            [&attempt], |row| row.get(0),
        ).unwrap();
        let foreign_baseline = String::from_utf8(
            std::process::Command::new("git")
                .args(["commit-tree", "4b825dc642cb6eb9a060e54bf8d69288fbee4904", "-m", "foreign retry baseline"])
                .current_dir(&authority_root)
                .env("GIT_AUTHOR_NAME", "Codex test")
                .env("GIT_AUTHOR_EMAIL", "codex-test@example.invalid")
                .env("GIT_COMMITTER_NAME", "Codex test")
                .env("GIT_COMMITTER_EMAIL", "codex-test@example.invalid")
                .output().unwrap().stdout,
        ).unwrap().trim().to_owned();
        assert_eq!(foreign_baseline.len(), 40);
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE execution_support_attempt_authorizations SET baseline_object_id=?2
             WHERE attempt_id=?1 AND role_kind='work_unit_implementer'",
            params![attempt, foreign_baseline],
        ).unwrap();
        failure("retry_execution_authorization_failed");
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE execution_support_attempt_authorizations SET baseline_object_id=?2
             WHERE attempt_id=?1 AND role_kind='work_unit_implementer'",
            params![attempt, original_baseline],
        ).unwrap();
        recover();

        let (manifest, comparison, contents): (String,String,String) = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT evidence_manifest_json,comparison_fingerprint,evidence_content_fingerprints_json
             FROM work_unit_implementer_outcomes WHERE work_unit_id=?1",
            [&fixture.work_unit_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        ).unwrap();
        for (column, value, expected) in [
            ("evidence_manifest_json", "[]", "retry_evidence_revalidation_failed"),
            ("evidence_content_fingerprints_json", "[]", "retry_evidence_revalidation_failed"),
        ] {
            Connection::open(&fixture.base.database_path).unwrap().execute(
                &format!("UPDATE work_unit_implementer_outcomes SET {column}=?1 WHERE work_unit_id=?2"),
                params![value, fixture.work_unit_id],
            ).unwrap();
            failure(expected);
            Connection::open(&fixture.base.database_path).unwrap().execute(
                "UPDATE work_unit_implementer_outcomes SET evidence_manifest_json=?1,comparison_fingerprint=?2,evidence_content_fingerprints_json=?3 WHERE work_unit_id=?4",
                params![manifest, comparison, contents, fixture.work_unit_id],
            ).unwrap();
            recover();
        }

        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_sessions SET working_directory='divergent-session-route' WHERE id=?1",
            [&session],
        ).unwrap();
        failure("retry_session_creation_failed");
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_sessions SET working_directory=?2 WHERE id=?1",
            params![session, retry_worktree],
        ).unwrap();
        recover();

        let submitted_text: String = Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT submitted_text FROM agent_session_invocations WHERE id=?1", [&invocation], |row| row.get(0),
        ).unwrap();
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET submitted_text='divergent prepared invocation' WHERE id=?1",
            [&invocation],
        ).unwrap();
        failure("retry_invocation_preparation_failed");
        Connection::open(&fixture.base.database_path).unwrap().execute(
            "UPDATE agent_session_invocations SET submitted_text=?2 WHERE id=?1",
            params![invocation, submitted_text],
        ).unwrap();
        recover();

        let connection = Connection::open(&fixture.base.database_path).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM execution_support_grants WHERE attempt_id=?1", [&attempt], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_sessions WHERE id=?1", [&session], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_session_invocations WHERE id=?1", [&invocation], |row| row.get(0)).unwrap(), 1);
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_retry_attempts WHERE ordinal=2", [], |row| row.get(0)).unwrap(), 0);
    }

    #[test]
    fn incomplete_disposition_two_services_reopen_reuses_one_functional_no_progress_handback() {
        let fixture = ReportingFixture::new();
        let review = fixture.ready_review();
        let reopened = fixture.reopened();
        let disposition = crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition {
            code: "objective_not_satisfied".into(),
            explanation: "the functional objective remains unsatisfied".into(),
            classification: crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::FunctionalObjectiveNotSatisfied,
            meaningful_progress: false,
        };
        let barrier = Arc::new(Barrier::new(2));
        let calls = [fixture.transition.clone(), reopened.clone()].into_iter().map(|service| {
            let invocation = review.clone();
            let disposition = disposition.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || { barrier.wait(); service.record_handler_incomplete_disposition_for_test(&invocation, disposition) })
        }).collect::<Vec<_>>();
        assert!(calls.into_iter().all(|call| call.join().unwrap().is_ok()));
        fixture.base.runtime.finish(&review, AgentInvocationTerminalStatus::Completed);
        reopened.reconcile_handler_reviews_for_test().unwrap();
        let durable = |fixture: &ReportingFixture| Connection::open(&fixture.base.database_path).unwrap().query_row(
            "SELECT d.decision_fingerprint,h.handback_id,h.context_fingerprint FROM work_unit_handler_incomplete_dispositions d JOIN work_unit_no_progress_handbacks h ON h.source_attempt_id=d.attempt_id WHERE d.work_unit_id=?1 AND d.classification='functional_objective_not_satisfied' AND d.meaningful_progress=0 AND d.next_attempt_authorized_at IS NULL AND h.source_review_invocation_id=d.review_invocation_id AND h.sprint_runner_receiver_activated_at IS NULL AND h.sprint_runner_receiver_decision_at IS NULL",
            [&fixture.work_unit_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        ).unwrap();
        let before = durable(&fixture);
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_handler_incomplete_dispositions", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_no_progress_handbacks", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_retry_attempts", [], |row| row.get(0)).unwrap(), 0);
        let reopened_again = fixture.reopened();
        reopened_again.reconcile_handler_reviews_for_test().unwrap();
        assert_eq!(durable(&fixture), before);
        assert_eq!(Connection::open(&fixture.base.database_path).unwrap().query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_no_progress_handbacks", [], |row| row.get(0)).unwrap(), 1);
    }

    #[test]
    fn divergent_incomplete_disposition_replay_conflicts_without_contradictory_effects() {
        let fixture = ReportingFixture::new();
        let review = fixture.ready_review();
        let reopened = fixture.reopened();
        let barrier = Arc::new(Barrier::new(2));
        let functional = {
            let service = fixture.transition.clone(); let invocation = review.clone(); let barrier = barrier.clone();
            std::thread::spawn(move || { barrier.wait(); service.record_handler_incomplete_disposition_for_test(&invocation, crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition { code: "objective_not_satisfied".into(), explanation: "the functional objective remains unsatisfied".into(), classification: crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::FunctionalObjectiveNotSatisfied, meaningful_progress: true }) })
        };
        let blocked = {
            let service = reopened.clone(); let invocation = review.clone(); let barrier = barrier.clone();
            std::thread::spawn(move || { barrier.wait(); service.record_handler_incomplete_disposition_for_test(&invocation, crate::orchestration::sprint_runner_transition::HandlerReviewIncompleteDisposition { code: "blocked_input".into(), explanation: "a bounded input is unavailable".into(), classification: crate::orchestration::sprint_runner_transition::IncompleteAttemptClassification::Blocked, meaningful_progress: false }) })
        };
        let results = [functional.join().unwrap(), blocked.join().unwrap()];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| matches!(result, Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict))).count(), 1);
        fixture.base.runtime.finish(&review, AgentInvocationTerminalStatus::Completed);
        reopened.reconcile_handler_reviews_for_test().unwrap();
        let connection = Connection::open(&fixture.base.database_path).unwrap();
        let facts: (i64,i64,i64,i64) = connection.query_row("SELECT (SELECT COUNT(*) FROM work_unit_handler_incomplete_dispositions), (SELECT COUNT(*) FROM work_unit_handler_incomplete_dispositions WHERE next_attempt_authorized_at IS NOT NULL), (SELECT COUNT(*) FROM work_unit_no_progress_handbacks), (SELECT COUNT(*) FROM work_unit_handler_reviews WHERE conflict_reason='divergent_review_judgment')", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
        assert_eq!(facts.0, 1);
        assert_eq!(facts.1 + facts.2, 1);
        assert_eq!(facts.3, 1);
        assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_retry_attempts", [], |row| row.get(0)).unwrap(), facts.1);
    }

    #[test]
    fn handler_review_concurrent_replays_and_divergent_race_converge_without_downstream_effects() {
        let downstream_effects = |fixture: &ReportingFixture| {
            Connection::open(&fixture.base.database_path)
                .unwrap()
                .query_row(
                    "SELECT
                       (SELECT COUNT(*) FROM work_unit_implementer_activations WHERE work_unit_id=?1),
                       (SELECT COUNT(*) FROM work_unit_handler_activations WHERE work_unit_id=?1),
                       (SELECT COUNT(*) FROM work_unit_handler_action_continuations WHERE work_unit_id=?1),
                       (SELECT COUNT(*) FROM work_unit_materializations),
                       (SELECT COUNT(*) FROM work_units),
                       (SELECT COUNT(*) FROM work_unit_relationships),
                       (SELECT COUNT(*) FROM work_slice_planning_requests),
                       (SELECT COUNT(*) FROM work_slice_planning_episodes),
                       (SELECT COUNT(*) FROM work_slice_proposal_revisions),
                       (SELECT COUNT(*) FROM sprint_runner_transitions WHERE parent_continuation_delivery_requested_at IS NOT NULL OR epic_continuation_invocation_id IS NOT NULL OR sprint_continuation_invocation_id IS NOT NULL)",
                    [&fixture.work_unit_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, i64>(5)?, row.get::<_, i64>(6)?, row.get::<_, i64>(7)?, row.get::<_, i64>(8)?, row.get::<_, i64>(9)?)),
                )
                .unwrap()
        };

        let replay = ReportingFixture::new();
        let replay_review = replay.ready_review();
        let replay_effects = downstream_effects(&replay);
        let barrier = Arc::new(Barrier::new(2));
        let calls = (0..2).map(|_| {
            let service = replay.transition.clone();
            let invocation = replay_review.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.record_handler_review_judgment_for_test(&invocation, "accept", None)
            })
        }).collect::<Vec<_>>();
        let results = calls.into_iter().map(|call| call.join().unwrap()).collect::<Vec<_>>();
        assert!(results.iter().all(Result::is_ok));
        let replay_connection = Connection::open(&replay.base.database_path).unwrap();
        assert_eq!(replay_connection.query_row::<String, _, _>("SELECT semantic_judgment_variant FROM work_unit_handler_reviews WHERE work_unit_id=?1", [&replay.work_unit_id], |row| row.get(0)).unwrap(), "accept");
        assert_eq!(replay_connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_reviews WHERE work_unit_id=?1 AND conflict_at IS NOT NULL", [&replay.work_unit_id], |row| row.get(0)).unwrap(), 0);
        drop(replay_connection);
        replay.base.runtime.finish(&replay_review, AgentInvocationTerminalStatus::Completed);
        assert_eq!(Connection::open(&replay.base.database_path).unwrap().query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_decisions WHERE work_unit_id=?1 AND decision_variant='accepted'", [&replay.work_unit_id], |row| row.get(0)).unwrap(), 1);
        assert_eq!(downstream_effects(&replay), replay_effects);

        let race = ReportingFixture::new();
        let race_review = race.ready_review();
        let race_effects = downstream_effects(&race);
        let barrier = Arc::new(Barrier::new(2));
        let accept = {
            let service = race.transition.clone();
            let invocation = race_review.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.record_handler_review_judgment_for_test(&invocation, "accept", None)
            })
        };
        let returned = {
            let service = race.transition.clone();
            let invocation = race_review.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                service.record_handler_review_judgment_for_test(&invocation, "return", Some(crate::orchestration::sprint_runner_transition::HandlerReviewReturnReason {
                    code: "review_failed".into(),
                    explanation: "evidence requires correction".into(),
                }))
            })
        };
        let results = [accept.join().unwrap(), returned.join().unwrap()];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| matches!(result, Err(crate::orchestration::sprint_runner_transition::SprintRunnerTransitionError::Conflict))).count(), 1);
        let race_connection = Connection::open(&race.base.database_path).unwrap();
        let judgment: String = race_connection.query_row("SELECT semantic_judgment_variant FROM work_unit_handler_reviews WHERE work_unit_id=?1", [&race.work_unit_id], |row| row.get(0)).unwrap();
        assert!(judgment == "accept" || judgment == "return");
        assert_eq!(race_connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_reviews WHERE work_unit_id=?1 AND conflict_reason='divergent_review_judgment'", [&race.work_unit_id], |row| row.get(0)).unwrap(), 1);
        drop(race_connection);
        race.base.runtime.finish(&race_review, AgentInvocationTerminalStatus::Completed);
        assert_eq!(Connection::open(&race.base.database_path).unwrap().query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_handler_decisions WHERE work_unit_id=?1", [&race.work_unit_id], |row| row.get(0)).unwrap(), 1);
        assert_eq!(downstream_effects(&race), race_effects);
    }
}

fn read_transition(
    connection: &Connection,
    initiation_id: &str,
) -> Result<TransitionRecord, TransitionError> {
    connection
        .query_row(
            "SELECT initiation_id,epic_id,proposal_revision_id,material_snapshot_hash,proposal_json,preparation_id,prepared_root,approved_plan_path,manifest_path,overview_path,runner_brief_path,bootstrap_session_id,bootstrap_invocation_id,runner_session_id,runner_invocation_id,prepared_at,bootstrap_session_created_at,bootstrap_launched_at,bootstrap_lifecycle_status,semantic_completion_fact_id,material_accepted_at,runner_session_created_at,runner_harness_key,runner_harness_version,runner_harness_requested_at,runner_harness_applied_at,runner_launched_at FROM epic_bootstrap_transitions WHERE initiation_id=?1",
            params![initiation_id],
            |row| {
                let proposal_json: String = row.get(4)?;
                let proposal = serde_json::from_str(&proposal_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(proposal_json.len(), rusqlite::types::Type::Text, Box::new(error))
                })?;
                Ok(TransitionRecord {
                    initiation_id: row.get(0)?, epic_id: row.get(1)?, proposal_revision_id: row.get(2)?, material_snapshot_hash: row.get(3)?,
                    proposal_json, proposal, preparation_id: row.get(5)?, prepared_root: row.get(6)?, approved_plan_path: row.get(7)?,
                    manifest_path: row.get(8)?, overview_path: row.get(9)?, runner_brief_path: row.get(10)?, bootstrap_session_id: row.get(11)?,
                    bootstrap_invocation_id: row.get(12)?, runner_session_id: row.get(13)?, runner_invocation_id: row.get(14)?, prepared_at: row.get(15)?,
                    bootstrap_session_created_at: row.get(16)?, bootstrap_launched_at: row.get(17)?, bootstrap_lifecycle_status: row.get(18)?,
                    semantic_completion_fact_id: row.get(19)?, material_accepted_at: row.get(20)?, runner_session_created_at: row.get(21)?, runner_harness_key: row.get(22)?, runner_harness_version: row.get(23)?, runner_harness_requested_at: row.get(24)?, runner_harness_applied_at: row.get(25)?, runner_launched_at: row.get(26)?,
                })
            },
        )
        .optional()
        .map_err(sql_unavailable("read bootstrap transition"))?
        .ok_or(TransitionError::NotFound)
}

fn read_attempt(
    connection: &Connection,
    attempt_id: &str,
) -> Result<BootstrapAttemptRecord, TransitionError> {
    connection
        .query_row(
            "SELECT id,transition_id,ordinal,agent_session_id,agent_invocation_id,launched_at,lifecycle_status,lifecycle_observed_at,semantic_completion_fact_id,semantic_completed_at,retry_disposition,retry_reason,retry_attempt_id,accepted_at FROM epic_bootstrap_attempts WHERE id=?1",
            params![attempt_id],
            map_attempt,
        )
        .optional()
        .map_err(sql_unavailable("read bootstrap attempt"))?
        .ok_or(TransitionError::NotFound)
}

fn read_current_attempt(
    connection: &Connection,
    transition_id: &str,
) -> Result<BootstrapAttemptRecord, TransitionError> {
    connection
        .query_row(
            "SELECT id,transition_id,ordinal,agent_session_id,agent_invocation_id,launched_at,lifecycle_status,lifecycle_observed_at,semantic_completion_fact_id,semantic_completed_at,retry_disposition,retry_reason,retry_attempt_id,accepted_at FROM epic_bootstrap_attempts WHERE transition_id=?1 ORDER BY ordinal DESC LIMIT 1",
            params![transition_id],
            map_attempt,
        )
        .optional()
        .map_err(sql_unavailable("read current bootstrap attempt"))?
        .ok_or(TransitionError::NotFound)
}

fn read_attempts(
    connection: &Connection,
    transition_id: &str,
) -> Result<Vec<BootstrapAttemptRecord>, TransitionError> {
    let mut statement = connection
        .prepare("SELECT id,transition_id,ordinal,agent_session_id,agent_invocation_id,launched_at,lifecycle_status,lifecycle_observed_at,semantic_completion_fact_id,semantic_completed_at,retry_disposition,retry_reason,retry_attempt_id,accepted_at FROM epic_bootstrap_attempts WHERE transition_id=?1 ORDER BY ordinal")
        .map_err(sql_unavailable("prepare bootstrap attempts"))?;
    let attempts = statement
        .query_map(params![transition_id], map_attempt)
        .map_err(sql_unavailable("read bootstrap attempts"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_unavailable("collect bootstrap attempts"))?;
    Ok(attempts)
}

fn map_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<BootstrapAttemptRecord> {
    Ok(BootstrapAttemptRecord {
        id: row.get(0)?,
        transition_id: row.get(1)?,
        ordinal: row.get(2)?,
        agent_session_id: row.get(3)?,
        agent_invocation_id: row.get(4)?,
        launched_at: row.get(5)?,
        lifecycle_status: row.get(6)?,
        lifecycle_observed_at: row.get(7)?,
        semantic_completion_fact_id: row.get(8)?,
        semantic_completed_at: row.get(9)?,
        retry_disposition: row.get(10)?,
        retry_reason: row.get(11)?,
        retry_attempt_id: row.get(12)?,
        accepted_at: row.get(13)?,
    })
}

fn sql_unavailable(context: &'static str) -> impl FnOnce(rusqlite::Error) -> TransitionError {
    move |error| TransitionError::Unavailable(format!("{context}: {error}"))
}

#[derive(Clone, Debug)]
struct PreparedPaths {
    preparation_id: String,
    root: String,
    approved_plan: String,
    manifest: String,
    overview: String,
    runner_brief: String,
}

impl PreparedPaths {
    fn derive(
        base: &Path,
        snapshot: &ConfirmedInitiationSnapshot,
    ) -> Result<Self, TransitionError> {
        if !safe_segment(&snapshot.epic_id) {
            return Err(TransitionError::IdentityMismatch(
                "Epic identity is not a safe path segment".into(),
            ));
        }
        let root = base.join("epics").join(&snapshot.epic_id);
        Ok(Self {
            preparation_id: stable_id("epic-bootstrap-preparation", &snapshot.initiation_id),
            approved_plan: path_text(&root.join("approved-plan.json"))?,
            manifest: path_text(&root.join("transition-manifest.json"))?,
            overview: path_text(&root.join("epic-overview.md"))?,
            runner_brief: path_text(&root.join("runner-brief.md"))?,
            root: path_text(&root)?,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApprovedPlanInput<'a> {
    contract: &'static str,
    initiation_id: &'a str,
    epic_id: &'a str,
    proposal_revision_id: &'a str,
    material_snapshot_hash: &'a str,
    proposal: &'a PlanBuilderProposal,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransitionManifestInput<'a> {
    contract: &'static str,
    preparation_id: &'a str,
    prepared_root: &'a str,
    approved_plan_path: &'a str,
    overview_path: &'a str,
    runner_brief_path: &'a str,
    semantic_action: &'static str,
}

fn prepare_inputs(
    base: &Path,
    snapshot: &ConfirmedInitiationSnapshot,
    paths: &PreparedPaths,
) -> Result<(), TransitionError> {
    fs::create_dir_all(base).map_err(fs_unavailable("create orchestration material root"))?;
    let base =
        fs::canonicalize(base).map_err(fs_unavailable("resolve orchestration material root"))?;
    let root = PathBuf::from(&paths.root);
    fs::create_dir_all(&root).map_err(fs_unavailable("create prepared Epic root"))?;
    let canonical_root =
        fs::canonicalize(&root).map_err(fs_unavailable("resolve prepared Epic root"))?;
    if !canonical_root.starts_with(&base) || canonical_root == base {
        return Err(TransitionError::IdentityMismatch(
            "prepared Epic root escapes the application material root".into(),
        ));
    }
    let approved = json_bytes(&ApprovedPlanInput {
        contract: "epic-approved-plan-input/v1",
        initiation_id: &snapshot.initiation_id,
        epic_id: &snapshot.epic_id,
        proposal_revision_id: &snapshot.proposal_revision_id,
        material_snapshot_hash: &snapshot.material_snapshot_hash,
        proposal: &snapshot.proposal,
    })?;
    let manifest = json_bytes(&TransitionManifestInput {
        contract: "epic-bootstrap-transition-manifest/v1",
        preparation_id: &paths.preparation_id,
        prepared_root: &paths.root,
        approved_plan_path: &paths.approved_plan,
        overview_path: &paths.overview,
        runner_brief_path: &paths.runner_brief,
        semantic_action: "complete_epic_bootstrap",
    })?;
    write_exact_contained(&canonical_root, Path::new(&paths.approved_plan), &approved)?;
    write_exact_contained(&canonical_root, Path::new(&paths.manifest), &manifest)?;
    Ok(())
}

fn write_materials(
    record: &TransitionRecord,
    input: &BootstrapMaterialInput,
) -> Result<Vec<MaterialInventoryItem>, TransitionError> {
    input.validate()?;
    let root = fs::canonicalize(&record.prepared_root)
        .map_err(fs_unavailable("resolve prepared Epic root"))?;
    let outputs = [
        (
            "epic_overview",
            &record.overview_path,
            input.epic_overview_markdown.as_bytes(),
        ),
        (
            "runner_brief",
            &record.runner_brief_path,
            input.runner_brief_markdown.as_bytes(),
        ),
    ];
    let mut inventory = Vec::new();
    for (kind, path, bytes) in outputs {
        let target = Path::new(path);
        write_exact_contained(&root, target, bytes)?;
        inventory.push(MaterialInventoryItem {
            kind: kind.into(),
            path: path.clone(),
            sha256: sha256(bytes),
            size_bytes: bytes.len() as u64,
        });
    }
    Ok(inventory)
}

fn write_exact_contained(root: &Path, target: &Path, bytes: &[u8]) -> Result<(), TransitionError> {
    let parent = target.parent().ok_or_else(|| {
        TransitionError::IdentityMismatch("material destination has no parent".into())
    })?;
    let canonical_parent =
        fs::canonicalize(parent).map_err(fs_unavailable("resolve material destination"))?;
    if canonical_parent != root
        || target
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(TransitionError::IdentityMismatch(
            "material destination escapes the prepared Epic root".into(),
        ));
    }
    reject_link_or_reparse(target)?;
    if target.exists() {
        let existing =
            fs::read(target).map_err(fs_unavailable("read existing prepared material"))?;
        if existing != bytes {
            return Err(TransitionError::IdentityMismatch(format!(
                "prepared material already exists with different bytes: {}",
                target.display()
            )));
        }
        return Ok(());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(fs_unavailable("create prepared material"))?;
    file.write_all(bytes)
        .map_err(fs_unavailable("write prepared material"))?;
    file.sync_all()
        .map_err(fs_unavailable("sync prepared material"))?;
    Ok(())
}

fn reject_link_or_reparse(path: &Path) -> Result<(), TransitionError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(fs_unavailable("inspect prepared material target")(error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(TransitionError::IdentityMismatch(format!(
            "prepared material target is a symbolic link or reparse point: {}",
            path.display()
        )));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(TransitionError::IdentityMismatch(format!(
                "prepared material target is a symbolic link or reparse point: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn fs_unavailable(context: &'static str) -> impl FnOnce(std::io::Error) -> TransitionError {
    move |error| TransitionError::Unavailable(format!("{context}: {error}"))
}

fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>, TransitionError> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| TransitionError::Unavailable(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn path_text(path: &Path) -> Result<String, TransitionError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| TransitionError::IdentityMismatch("prepared path is not valid UTF-8".into()))
}

fn safe_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn stable_id(prefix: &str, source: &str) -> String {
    format!("{prefix}-{}", sha256(source.as_bytes()))
}

fn bootstrap_attempt_id(initiation_id: &str, ordinal: i64) -> String {
    format!("epic-bootstrap-attempt-{ordinal}-{initiation_id}")
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BootstrapTransitionQueryV2 {
    pub(crate) contract: String,
    pub(crate) schema_version: u16,
    pub(crate) transitions: Vec<BootstrapTransitionStatusV2>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BootstrapTransitionStatusV2 {
    pub(crate) initiation_id: String,
    pub(crate) epic_id: String,
    pub(crate) preparation_id: String,
    pub(crate) prepared_root: String,
    pub(crate) approved_plan_path: String,
    pub(crate) manifest_path: String,
    pub(crate) overview_path: String,
    pub(crate) runner_brief_path: String,
    pub(crate) bootstrap_session_id: String,
    pub(crate) bootstrap_invocation_id: String,
    pub(crate) prepared_at: Option<String>,
    pub(crate) bootstrap_session_created_at: Option<String>,
    pub(crate) bootstrap_launched_at: Option<String>,
    pub(crate) bootstrap_lifecycle_status: Option<String>,
    pub(crate) bootstrap_lifecycle_observed_at: Option<String>,
    pub(crate) semantic_completion_fact_id: Option<String>,
    pub(crate) semantic_completed_at: Option<String>,
    pub(crate) material_accepted_at: Option<String>,
    pub(crate) runner_session_id: String,
    pub(crate) runner_invocation_id: String,
    pub(crate) runner_session_created_at: Option<String>,
    pub(crate) runner_launched_at: Option<String>,
    pub(crate) runner_lifecycle_status: Option<String>,
    pub(crate) runner_lifecycle_observed_at: Option<String>,
    pub(crate) current_attempt_id: String,
    pub(crate) retry_state: String,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) accepted_attempt_id: Option<String>,
    pub(crate) bootstrap_attempts: Vec<BootstrapAttemptStatusV2>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BootstrapAttemptStatusV2 {
    pub(crate) attempt_id: String,
    pub(crate) ordinal: i64,
    pub(crate) agent_session_id: String,
    pub(crate) agent_invocation_id: String,
    pub(crate) launched_at: Option<String>,
    pub(crate) lifecycle_status: Option<String>,
    pub(crate) lifecycle_observed_at: Option<String>,
    pub(crate) semantic_completion_fact_id: Option<String>,
    pub(crate) semantic_completed_at: Option<String>,
    pub(crate) retry_disposition: String,
    pub(crate) retry_reason: Option<String>,
    pub(crate) retry_attempt_id: Option<String>,
    pub(crate) accepted_at: Option<String>,
}

impl From<BootstrapAttemptRecord> for BootstrapAttemptStatusV2 {
    fn from(attempt: BootstrapAttemptRecord) -> Self {
        Self {
            attempt_id: attempt.id,
            ordinal: attempt.ordinal,
            agent_session_id: attempt.agent_session_id,
            agent_invocation_id: attempt.agent_invocation_id,
            launched_at: attempt.launched_at,
            lifecycle_status: attempt.lifecycle_status,
            lifecycle_observed_at: attempt.lifecycle_observed_at,
            semantic_completion_fact_id: attempt.semantic_completion_fact_id,
            semantic_completed_at: attempt.semantic_completed_at,
            retry_disposition: attempt.retry_disposition,
            retry_reason: attempt.retry_reason,
            retry_attempt_id: attempt.retry_attempt_id,
            accepted_at: attempt.accepted_at,
        }
    }
}
