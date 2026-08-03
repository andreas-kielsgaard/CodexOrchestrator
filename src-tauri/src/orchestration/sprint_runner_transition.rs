//! The first downstream boundary: one Epic Runner semantic request creates one ready Sprint Runner.

use super::conversation_harness::{self, ConversationHarnessRole};
use super::work_unit_execution_harness::{WorkUnitExecutionHarnessService, WorkUnitHarnessRole};
use super::mcp::CodexMcpInjection;
use super::repository::{InitiatedSprintGitAuthority, InitiatedSprintGitAuthorityError, SqliteOrchestrationRepository};
use crate::agent_sessions::{
    application::{
        AgentSessionApplication, ApplicationInvocationLaunchEvidence, CreateAgentSessionCommand,
        CreateApplicationAgentSessionCommand, SendAgentSessionMessageCommand,
        SendIdempotentApplicationAgentSessionMessageCommand,
        AgentSessionNotification,
    },
    domain::{AgentInvocationId, AgentSessionId, AgentInvocationStatus},
    ports::RuntimeLaunchExtension,
};
use bytes::Bytes;
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
use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

pub(crate) const SPRINT_RUNNER_QUERY_CONTRACT: &str = "sprint-runner-transition-query/v1";

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sprint_runner_transitions (
  sprint_id TEXT PRIMARY KEY,
  epic_id TEXT NOT NULL,
  request_id TEXT NOT NULL UNIQUE,
  epic_runner_session_id TEXT NOT NULL,
  epic_runner_invocation_id TEXT NOT NULL UNIQUE,
  epic_runner_harness_key TEXT NOT NULL,
  epic_runner_harness_version INTEGER NOT NULL,
  sprint_runner_harness_key TEXT NOT NULL,
  sprint_runner_harness_version INTEGER NOT NULL,
  sprint_runner_session_id TEXT NOT NULL UNIQUE,
  sprint_runner_invocation_id TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL,
  authorized_at TEXT NOT NULL,
  session_created_at TEXT,
  harness_applied_at TEXT,
  launch_accepted_at TEXT,
  pre_start_semantic_outcome_recorded_at TEXT,
  pre_start_outcome_fact_id TEXT UNIQUE,
  pre_start_outcome_invocation_id TEXT,
  pre_start_forecast TEXT,
  pre_start_material_uncertainty TEXT,
  pre_start_prerequisite TEXT,
  pre_start_upgrade_invocation_id TEXT UNIQUE,
  pre_start_upgrade_harness_key TEXT,
  pre_start_upgrade_harness_version INTEGER,
  pre_start_upgrade_harness_applied_at TEXT,
  pre_start_upgrade_launch_accepted_at TEXT,
  pre_start_lifecycle_status TEXT,
  pre_start_lifecycle_invocation_id TEXT,
  pre_start_lifecycle_observed_at TEXT,
  pre_start_outcome_accepted_at TEXT,
  parent_continuation_delivery_requested_at TEXT,
  parent_continuation_delivery_persisted_at TEXT,
  parent_continuation_delivery_fact_id TEXT UNIQUE,
  parent_continuation_delivered_outcome_fact_id TEXT,
  epic_continuation_invocation_id TEXT UNIQUE,
  epic_continuation_harness_key TEXT,
  epic_continuation_harness_version INTEGER,
  epic_continuation_harness_applied_at TEXT,
  epic_continuation_launch_accepted_at TEXT,
  provider_receiver_activation_observed_at TEXT,
  epic_start_semantic_authorization_requested_at TEXT,
  epic_start_semantic_authorization_recorded_at TEXT,
  sprint_start_authorized_at TEXT,
  sprint_start_persisted_at TEXT,
  sprint_continuation_invocation_id TEXT UNIQUE,
  sprint_continuation_harness_key TEXT,
  sprint_continuation_harness_version INTEGER,
  sprint_continuation_harness_applied_at TEXT,
  sprint_continuation_launch_accepted_at TEXT,
  repository_branch_reevaluation_fact_id TEXT UNIQUE,
  repository_branch_reevaluation_recorded_at TEXT,
  repository_branch_evaluation TEXT,
  started_forecast_and_concerns TEXT,
  started_reevaluation_lifecycle_status TEXT,
  started_reevaluation_lifecycle_invocation_id TEXT,
  started_reevaluation_lifecycle_observed_at TEXT,
  planning_control_delivery_requested_at TEXT,
  planning_control_delivery_persisted_at TEXT,
  planning_control_invocation_id TEXT UNIQUE,
  planning_control_harness_key TEXT,
  planning_control_harness_version INTEGER,
  planning_control_harness_applied_at TEXT,
  planning_control_launch_accepted_at TEXT,
  planning_ready_at TEXT,
  FOREIGN KEY (sprint_id) REFERENCES initiated_sprints(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS work_slice_planning_requests (
  planning_point_id TEXT PRIMARY KEY,
  sprint_id TEXT NOT NULL,
  planning_episode INTEGER NOT NULL,
  is_current INTEGER NOT NULL CHECK (is_current IN (0, 1)),
  request_fact_id TEXT NOT NULL UNIQUE,
  parent_sprint_runner_session_id TEXT NOT NULL,
  parent_planning_control_invocation_id TEXT NOT NULL UNIQUE,
  authority_id TEXT NOT NULL,
  authority_epic_id TEXT NOT NULL,
  authority_provenance_id TEXT NOT NULL,
  authority_repository_id TEXT NOT NULL,
  authority_worktree_id TEXT NOT NULL,
  authority_baseline_object_id TEXT NOT NULL,
  authority_current_object_id TEXT NOT NULL,
  authority_source_fingerprint TEXT NOT NULL,
  repository_worktree_route TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  authorized_at TEXT NOT NULL,
  planner_harness_key TEXT NOT NULL,
  planner_harness_version INTEGER NOT NULL,
  planner_session_id TEXT NOT NULL UNIQUE,
  planner_invocation_id TEXT NOT NULL UNIQUE,
  planner_session_created_at TEXT,
  planner_invocation_created_at TEXT,
  planner_harness_applied_at TEXT,
  planner_harness_json TEXT CHECK (planner_harness_json IS NULL OR json_valid(planner_harness_json)),
  planner_launch_requested_at TEXT,
  planner_launch_accepted_at TEXT,
  planner_ready_at TEXT,
  planner_provider_activation_observed_at TEXT,
  planner_lifecycle_observed_at TEXT,
  UNIQUE (sprint_id, planning_episode),
  FOREIGN KEY (sprint_id) REFERENCES sprint_runner_transitions(sprint_id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS work_slice_planning_requests_one_current
  ON work_slice_planning_requests(sprint_id) WHERE is_current = 1;

-- Proposal facts deliberately remain separate from the Planner-ready request.  Nothing in
-- these tables is a Work Unit or a request to create one.
CREATE TABLE IF NOT EXISTS work_slice_planning_episodes (
  planning_point_id TEXT PRIMARY KEY REFERENCES work_slice_planning_requests(planning_point_id) ON DELETE RESTRICT,
  sprint_id TEXT NOT NULL,
  authority_id TEXT NOT NULL,
  planner_session_id TEXT NOT NULL,
  planner_invocation_id TEXT NOT NULL,
  harness_json TEXT NOT NULL CHECK (json_valid(harness_json)),
  repository_worktree_route TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS work_slice_proposal_revisions (
  revision_id TEXT PRIMARY KEY,
  planning_point_id TEXT NOT NULL REFERENCES work_slice_planning_episodes(planning_point_id) ON DELETE RESTRICT,
  revision_number INTEGER NOT NULL,
  parent_revision_id TEXT,
  is_current INTEGER NOT NULL CHECK (is_current IN (0,1)),
  idempotency_key TEXT NOT NULL,
  content_fingerprint TEXT NOT NULL,
  proposal_json TEXT NOT NULL CHECK (json_valid(proposal_json)),
  submitted_at TEXT NOT NULL,
  validation_at TEXT,
  validation_result TEXT,
  refinement_requested_at TEXT,
  refinement_reason TEXT,
  semantic_completed_at TEXT,
  semantic_completion_invocation_id TEXT,
  lifecycle_observed_at TEXT,
  lifecycle_status TEXT,
  accepted_at TEXT,
  materialization_ready_at TEXT,
  UNIQUE(planning_point_id, revision_number),
  UNIQUE(planning_point_id, idempotency_key)
);
CREATE UNIQUE INDEX IF NOT EXISTS work_slice_proposal_revisions_one_current
 ON work_slice_proposal_revisions(planning_point_id) WHERE is_current=1;

-- Materialization is a product-owned, durable responsibility projection.  It has no execution
-- semantics: neither these facts nor their repair can create a Session, invocation, or worktree.
CREATE TABLE IF NOT EXISTS work_unit_materializations (
  materialization_id TEXT PRIMARY KEY,
  planning_point_id TEXT NOT NULL UNIQUE REFERENCES work_slice_planning_episodes(planning_point_id) ON DELETE RESTRICT,
  accepted_revision_id TEXT NOT NULL UNIQUE REFERENCES work_slice_proposal_revisions(revision_id) ON DELETE RESTRICT,
  epic_id TEXT NOT NULL REFERENCES epic_initiations(epic_id) ON DELETE RESTRICT,
  sprint_id TEXT NOT NULL REFERENCES initiated_sprints(id) ON DELETE RESTRICT,
  work_slice_id TEXT NOT NULL UNIQUE,
  authorization_recorded_at TEXT NOT NULL,
  attempt_recorded_at TEXT,
  work_units_created_at TEXT,
  relationships_completed_at TEXT,
  settled_at TEXT,
  CHECK ((settled_at IS NULL) OR (relationships_completed_at IS NOT NULL)),
  CHECK ((relationships_completed_at IS NULL) OR (work_units_created_at IS NOT NULL)),
  CHECK ((work_units_created_at IS NULL) OR (attempt_recorded_at IS NOT NULL))
);
CREATE TABLE IF NOT EXISTS work_units (
  work_unit_id TEXT PRIMARY KEY,
  materialization_id TEXT NOT NULL REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  work_slice_id TEXT NOT NULL,
  accepted_revision_id TEXT NOT NULL,
  lane_ordinal INTEGER NOT NULL CHECK (lane_ordinal >= 0),
  lane_title TEXT NOT NULL,
  specification TEXT NOT NULL,
  UNIQUE(materialization_id, lane_ordinal),
  UNIQUE(materialization_id, lane_title)
);
CREATE TABLE IF NOT EXISTS work_unit_relationships (
  relationship_id TEXT PRIMARY KEY,
  materialization_id TEXT NOT NULL REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  relationship_kind TEXT NOT NULL CHECK (relationship_kind IN ('planning_point','sprint','lane','order','depends_on')),
  from_id TEXT NOT NULL,
  to_id TEXT NOT NULL,
  ordinal INTEGER,
  UNIQUE(materialization_id, relationship_kind, from_id, to_id)
);
-- Handler activation is an application-owned, initial-only boundary.  It has no Implementer,
-- review, settlement, retry, or continuation semantics.
CREATE TABLE IF NOT EXISTS work_unit_handler_activations (
  work_unit_id TEXT PRIMARY KEY REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  materialization_id TEXT NOT NULL REFERENCES work_unit_materializations(materialization_id) ON DELETE RESTRICT,
  sprint_id TEXT NOT NULL REFERENCES initiated_sprints(id) ON DELETE RESTRICT,
  attempt_id TEXT NOT NULL UNIQUE,
  handler_session_id TEXT NOT NULL UNIQUE,
  handler_invocation_id TEXT NOT NULL UNIQUE,
  handler_harness_key TEXT NOT NULL,
  handler_harness_version INTEGER NOT NULL,
  handler_harness_revision_id TEXT,
  handler_harness_configuration_digest TEXT,
  handler_harness_repository_commit_ref TEXT,
  eligibility_state TEXT NOT NULL CHECK (eligibility_state IN ('blocked','eligible')),
  blocked_reason TEXT,
  requested_at TEXT NOT NULL,
  authorized_at TEXT,
  attempt_created_at TEXT,
  execution_support_granted_at TEXT,
  isolated_worktree_ready_at TEXT,
  handler_session_created_at TEXT,
  handler_invocation_prepared_at TEXT,
  handler_harness_bound_at TEXT,
  launch_requested_at TEXT,
  launch_accepted_at TEXT,
  provider_activation_observed_at TEXT,
  handler_ready_at TEXT,
  CHECK ((eligibility_state = 'eligible' AND blocked_reason IS NULL) OR (eligibility_state = 'blocked' AND blocked_reason IS NOT NULL)),
  CHECK ((handler_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL))
);
CREATE TABLE IF NOT EXISTS work_unit_implementer_activations (
  work_unit_id TEXT PRIMARY KEY REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  handler_attempt_id TEXT NOT NULL UNIQUE,
  handler_invocation_id TEXT NOT NULL UNIQUE,
  attempt_id TEXT NOT NULL UNIQUE,
  implementer_session_id TEXT NOT NULL UNIQUE,
  implementer_invocation_id TEXT NOT NULL UNIQUE,
  implementer_harness_revision_id TEXT NOT NULL,
  implementer_harness_configuration_digest TEXT NOT NULL,
  implementer_harness_repository_commit_ref TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  authorized_at TEXT,
  execution_support_granted_at TEXT,
  isolated_worktree_ready_at TEXT,
  implementer_session_created_at TEXT,
  implementer_invocation_prepared_at TEXT,
  implementer_harness_bound_at TEXT,
  launch_requested_at TEXT,
  launch_accepted_at TEXT,
  provider_activation_observed_at TEXT,
  implementer_ready_at TEXT,
  failure_reason TEXT,
  CHECK ((implementer_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL))
);
-- The v2 action is a separate immutable Handler invocation in the original Handler Session.
-- Its facts are intentionally not folded into the original v1 Handler activation.
CREATE TABLE IF NOT EXISTS work_unit_handler_action_continuations (
  work_unit_id TEXT PRIMARY KEY REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  attempt_id TEXT NOT NULL UNIQUE,
  handler_session_id TEXT NOT NULL,
  original_handler_invocation_id TEXT NOT NULL UNIQUE,
  action_invocation_id TEXT NOT NULL UNIQUE,
  action_harness_revision_id TEXT NOT NULL,
  action_harness_configuration_digest TEXT NOT NULL,
  action_harness_repository_commit_ref TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  authorized_at TEXT,
  invocation_prepared_at TEXT,
  harness_bound_at TEXT,
  launch_requested_at TEXT,
  launch_accepted_at TEXT,
  provider_activation_observed_at TEXT,
  action_ready_at TEXT,
  blocked_reason TEXT,
  CHECK ((action_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL))
);
"#;

const SHARED_IMPLEMENTER_ACTIVATION_TABLE: &str = r#"
CREATE TABLE work_unit_implementer_activations_v4 (
  work_unit_id TEXT PRIMARY KEY REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  handler_attempt_id TEXT NOT NULL UNIQUE,
  handler_invocation_id TEXT NOT NULL UNIQUE,
  attempt_id TEXT NOT NULL UNIQUE,
  implementer_session_id TEXT NOT NULL UNIQUE,
  implementer_invocation_id TEXT NOT NULL UNIQUE,
  implementer_harness_revision_id TEXT NOT NULL,
  implementer_harness_configuration_digest TEXT NOT NULL,
  implementer_harness_repository_commit_ref TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  authorized_at TEXT,
  execution_support_granted_at TEXT,
  isolated_worktree_ready_at TEXT,
  implementer_session_created_at TEXT,
  implementer_invocation_prepared_at TEXT,
  implementer_harness_bound_at TEXT,
  launch_requested_at TEXT,
  launch_accepted_at TEXT,
  provider_activation_observed_at TEXT,
  implementer_ready_at TEXT,
  failure_reason TEXT,
  CHECK ((implementer_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL))
);
"#;

/// Rebuild the short-lived second-attempt table only when every row proves it belongs to the
/// exact Handler attempt and invocation.  The old table stays intact on any mismatch or SQL
/// error, so reopen can safely retry after the missing durable evidence is restored.
fn migrate_legacy_implementer_activations(connection: &Connection) -> Result<(), String> {
    let columns = connection.prepare("PRAGMA table_info(work_unit_implementer_activations)")
        .and_then(|mut statement| statement.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>())
        .map_err(|error| format!("inspect Implementer activation migration: {error}"))?;
    if !columns.iter().any(|column| column == "implementer_attempt_id") {
        if columns.iter().any(|column| column == "attempt_id") {
            if !columns.iter().any(|column| column == "failure_reason") {
                connection.execute_batch("ALTER TABLE work_unit_implementer_activations ADD COLUMN failure_reason TEXT")
                    .map_err(|error| format!("add Implementer activation failure reason: {error}"))?;
            }
            return Ok(());
        }
        return Err("Implementer activation table has neither legacy nor shared attempt identity".into());
    }
    connection.execute_batch("BEGIN IMMEDIATE;")
        .map_err(|error| format!("begin Implementer activation migration: {error}"))?;
    let migrated = (|| -> Result<(), String> {
        let legacy_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM work_unit_implementer_activations", [], |row| row.get(0),
        ).map_err(|error| format!("count legacy Implementer activations: {error}"))?;
        let coherent_count: i64 = connection.query_row(
            "SELECT COUNT(*)
             FROM work_unit_implementer_activations i
             JOIN work_unit_handler_activations h
               ON h.work_unit_id=i.work_unit_id
              AND h.attempt_id=i.handler_attempt_id
              AND h.handler_invocation_id=i.handler_invocation_id",
            [], |row| row.get(0),
        ).map_err(|error| format!("validate Handler-correlated Implementer activations: {error}"))?;
        if legacy_count != coherent_count {
            return Err("legacy Implementer activation lacks coherent Handler correlation".into());
        }
        connection.execute_batch(SHARED_IMPLEMENTER_ACTIVATION_TABLE)
            .map_err(|error| format!("create shared Implementer activation table: {error}"))?;
        let copied = connection.execute(
            "INSERT INTO work_unit_implementer_activations_v4
             SELECT i.work_unit_id,i.handler_attempt_id,i.handler_invocation_id,h.attempt_id,
                    i.implementer_session_id,i.implementer_invocation_id,
                    i.implementer_harness_revision_id,i.implementer_harness_configuration_digest,
                    i.implementer_harness_repository_commit_ref,i.requested_at,i.authorized_at,
                    i.execution_support_granted_at,i.isolated_worktree_ready_at,
                    i.implementer_session_created_at,i.implementer_invocation_prepared_at,
                    i.implementer_harness_bound_at,i.launch_requested_at,i.launch_accepted_at,
                    i.provider_activation_observed_at,i.implementer_ready_at,NULL
             FROM work_unit_implementer_activations i
             JOIN work_unit_handler_activations h
               ON h.work_unit_id=i.work_unit_id
              AND h.attempt_id=i.handler_attempt_id
              AND h.handler_invocation_id=i.handler_invocation_id",
            [],
        ).map_err(|error| format!("copy shared Implementer activations: {error}"))?;
        if i64::try_from(copied).map_err(|_| "legacy Implementer activation count overflow".to_string())? != legacy_count {
            return Err("shared Implementer activation copy was incomplete".into());
        }
        connection.execute_batch(
            "DROP TABLE work_unit_implementer_activations;
             ALTER TABLE work_unit_implementer_activations_v4 RENAME TO work_unit_implementer_activations;",
        ).map_err(|error| format!("replace legacy Implementer activation table: {error}"))?;
        Ok(())
    })();
    match migrated {
        Ok(()) => connection.execute_batch("COMMIT;")
            .map_err(|error| format!("commit Implementer activation migration: {error}")),
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SprintRunnerSelection {
    pub(crate) sprint_id: String,
}

/// The pre-start worker may describe the situation, but never supplies a route or identity.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreStartOutcome {
    pub(crate) forecast_and_concerns: String,
    pub(crate) material_uncertainty: String,
    pub(crate) application_owned_prerequisite: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StartedReevaluation {
    pub(crate) repository_branch_evaluation: String,
    pub(crate) started_forecast_and_concerns: String,
}

/// The planning-control action intentionally has no caller-supplied identity or route.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkSlicePlannerRequest {}

/// All identity, route, authority, and acceptance facts are application-owned. The planner may
/// describe only a bounded proposal.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkSliceProposal {
    pub(crate) objective: String,
    pub(crate) lanes: Vec<WorkSliceLane>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkSliceLane {
    pub(crate) title: String,
    pub(crate) specification: String,
    pub(crate) depends_on: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkSliceRefinement { pub(crate) reason: String }
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkSliceCompletion {}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SprintRunnerTransitionQueryV1 {
    pub(crate) contract: &'static str,
    pub(crate) transitions: Vec<SprintRunnerTransitionStatus>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SprintRunnerTransitionStatus {
    pub(crate) sprint_id: String,
    pub(crate) epic_id: String,
    pub(crate) request_id: String,
    pub(crate) epic_runner_invocation_id: String,
    pub(crate) sprint_runner_session_id: String,
    pub(crate) sprint_runner_invocation_id: String,
    pub(crate) requested_at: String,
    pub(crate) authorized_at: String,
    pub(crate) session_created_at: Option<String>,
    pub(crate) harness_applied_at: Option<String>,
    pub(crate) launch_accepted_at: Option<String>,
    pub(crate) pre_start_ready: bool,
    pub(crate) lifecycle_observed: bool,
    pub(crate) accepted: bool,
    pub(crate) pre_start_semantic_outcome_recorded_at: Option<String>,
    pub(crate) pre_start_lifecycle_observed_at: Option<String>,
    pub(crate) pre_start_outcome_accepted_at: Option<String>,
    pub(crate) parent_continuation_delivery_requested_at: Option<String>,
    pub(crate) parent_continuation_delivery_persisted_at: Option<String>,
    pub(crate) epic_continuation_invocation_id: Option<String>,
    pub(crate) epic_continuation_launch_accepted_at: Option<String>,
    pub(crate) provider_receiver_activation_observed_at: Option<String>,
    pub(crate) sprint_start_authorized_at: Option<String>,
    pub(crate) sprint_start_persisted_at: Option<String>,
    pub(crate) sprint_continuation_invocation_id: Option<String>,
    pub(crate) sprint_continuation_launch_accepted_at: Option<String>,
    pub(crate) repository_branch_reevaluation_recorded_at: Option<String>,
    pub(crate) started_reevaluation_lifecycle_observed_at: Option<String>,
    pub(crate) planning_control_delivery_requested_at: Option<String>,
    pub(crate) planning_control_delivery_persisted_at: Option<String>,
    pub(crate) planning_control_invocation_id: Option<String>,
    pub(crate) planning_control_launch_accepted_at: Option<String>,
    pub(crate) planning_ready_at: Option<String>,
    pub(crate) work_slice_planner_request_id: Option<String>,
    pub(crate) work_slice_planner_requested_at: Option<String>,
    pub(crate) work_slice_planner_authorized_at: Option<String>,
    pub(crate) work_slice_planning_point_id: Option<String>,
    pub(crate) work_slice_planner_repository_worktree_route: Option<String>,
    pub(crate) work_slice_planner_harness_key: Option<String>,
    pub(crate) work_slice_planner_harness_version: Option<u16>,
    pub(crate) work_slice_planner_session_id: Option<String>,
    pub(crate) work_slice_planner_invocation_id: Option<String>,
    pub(crate) work_slice_planner_session_created_at: Option<String>,
    pub(crate) work_slice_planner_invocation_created_at: Option<String>,
    pub(crate) work_slice_planner_harness_applied_at: Option<String>,
    pub(crate) work_slice_planner_launch_requested_at: Option<String>,
    pub(crate) work_slice_planner_launch_accepted_at: Option<String>,
    pub(crate) work_slice_planner_ready_at: Option<String>,
    pub(crate) work_slice_planner_provider_activation_observed_at: Option<String>,
    pub(crate) work_slice_planner_lifecycle_observed_at: Option<String>,
    pub(crate) work_slice_proposal_submitted_at: Option<String>,
    pub(crate) work_slice_proposal_validation_result: Option<String>,
    pub(crate) work_slice_refinement_requested_at: Option<String>,
    pub(crate) work_slice_semantic_completed_at: Option<String>,
    pub(crate) work_slice_terminal_lifecycle_observed_at: Option<String>,
    pub(crate) work_slice_application_accepted_at: Option<String>,
    pub(crate) work_slice_materialization_ready_at: Option<String>,
    pub(crate) downstream_not_started: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SprintRunnerTransitionError {
    Forbidden,
    Invalid,
    Conflict,
    Unavailable(String),
}
impl std::fmt::Display for SprintRunnerTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
        Self::Forbidden => f.write_str("this invocation is not a launch-accepted Epic Runner with the applied Epic Runner Harness"),
        Self::Invalid => f.write_str("the selected Sprint is not an approved Sprint of this Epic"),
        Self::Conflict => f.write_str("a Sprint Runner request already exists with different durable routing"),
        Self::Unavailable(message) => f.write_str(message),
    }
    }
}

#[derive(Eq, PartialEq)]
struct CurrentPlanningRequest {
    planning_point_id: String,
    request_fact_id: String,
    parent_sprint_runner_session_id: String,
    parent_planning_control_invocation_id: String,
    authority_id: String,
    authority_epic_id: String,
    authority_provenance_id: String,
    authority_repository_id: String,
    authority_worktree_id: String,
    authority_baseline_object_id: String,
    authority_current_object_id: String,
    authority_source_fingerprint: String,
    repository_worktree_route: String,
    planner_harness_key: String,
    planner_harness_version: i64,
    planner_session_id: String,
    planner_invocation_id: String,
}

pub(crate) struct SprintRunnerTransitionService {
    connection: Mutex<Connection>,
    authority_repository: SqliteOrchestrationRepository,
    sessions: Arc<AgentSessionApplication>,
    work_unit_handler: Mutex<Option<Arc<WorkUnitExecutionHarnessService>>>,
    mcp: Mutex<HashMap<AgentInvocationId, ManagedEpicRunnerAction>>,
    transition_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    effect_drains: Mutex<HashMap<String, ReconcileDrain>>,
    #[cfg(test)]
    test_reconcile_snapshot_hook: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    #[cfg(test)]
    test_origin_snapshot_hook: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl SprintRunnerTransitionService {
    pub(crate) fn open(
        path: impl AsRef<Path>,
        sessions: Arc<AgentSessionApplication>,
    ) -> Result<Arc<Self>, SprintRunnerTransitionError> {
        let path = path.as_ref();
        let connection = Connection::open(path).map_err(|e| {
            SprintRunnerTransitionError::Unavailable(format!(
                "open Sprint Runner transition database: {e}"
            ))
        })?;
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        connection.execute_batch(SCHEMA).map_err(|e| {
            SprintRunnerTransitionError::Unavailable(format!(
                "initialize Sprint Runner transition schema: {e}"
            ))
        })?;
        // The first pre-start route was shipped before these evidence boundaries.  Keep an
        // existing local database readable without treating an absent fact as a positive fact.
        for column in [
            "pre_start_semantic_outcome_recorded_at TEXT", "pre_start_outcome_fact_id TEXT", "pre_start_outcome_invocation_id TEXT",
            "pre_start_forecast TEXT", "pre_start_material_uncertainty TEXT", "pre_start_prerequisite TEXT",
            "pre_start_upgrade_invocation_id TEXT", "pre_start_upgrade_harness_key TEXT", "pre_start_upgrade_harness_version INTEGER", "pre_start_upgrade_harness_applied_at TEXT", "pre_start_upgrade_launch_accepted_at TEXT",
            "pre_start_lifecycle_status TEXT", "pre_start_lifecycle_invocation_id TEXT", "pre_start_lifecycle_observed_at TEXT",
            "pre_start_outcome_accepted_at TEXT", "parent_continuation_delivery_requested_at TEXT",
            "parent_continuation_delivery_persisted_at TEXT", "epic_continuation_invocation_id TEXT",
            "parent_continuation_delivery_fact_id TEXT", "parent_continuation_delivered_outcome_fact_id TEXT",
            "epic_continuation_harness_key TEXT", "epic_continuation_harness_version INTEGER",
            "epic_continuation_harness_applied_at TEXT", "epic_continuation_launch_accepted_at TEXT",
            "provider_receiver_activation_observed_at TEXT", "epic_start_semantic_authorization_requested_at TEXT",
            "epic_start_semantic_authorization_recorded_at TEXT", "sprint_start_authorized_at TEXT",
            "sprint_start_persisted_at TEXT", "sprint_continuation_invocation_id TEXT",
            "sprint_continuation_harness_key TEXT", "sprint_continuation_harness_version INTEGER",
            "sprint_continuation_harness_applied_at TEXT", "sprint_continuation_launch_accepted_at TEXT",
            "repository_branch_reevaluation_fact_id TEXT", "repository_branch_reevaluation_recorded_at TEXT",
            "repository_branch_evaluation TEXT", "started_forecast_and_concerns TEXT",
            "started_reevaluation_lifecycle_status TEXT", "started_reevaluation_lifecycle_invocation_id TEXT",
            "started_reevaluation_lifecycle_observed_at TEXT", "planning_control_delivery_requested_at TEXT",
            "planning_control_delivery_persisted_at TEXT", "planning_control_invocation_id TEXT",
            "planning_control_harness_key TEXT", "planning_control_harness_version INTEGER",
            "planning_control_harness_applied_at TEXT", "planning_control_launch_accepted_at TEXT",
            "planning_ready_at TEXT",
        ] {
            let name = column.split_whitespace().next().expect("migration column name");
            let exists = connection.prepare("PRAGMA table_info(sprint_runner_transitions)")
                .and_then(|mut statement| statement.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>())
                .map_err(|e| SprintRunnerTransitionError::Unavailable(format!("inspect Sprint Runner transition schema: {e}")))?
                .iter().any(|existing| existing == name);
            if !exists { connection.execute_batch(&format!("ALTER TABLE sprint_runner_transitions ADD COLUMN {column}"))
                .map_err(|e| SprintRunnerTransitionError::Unavailable(format!("migrate Sprint Runner transition schema: {e}")))?; }
        }
        for column in [
            "authority_id TEXT", "authority_epic_id TEXT", "authority_provenance_id TEXT",
            "authority_repository_id TEXT", "authority_worktree_id TEXT",
            "authority_baseline_object_id TEXT", "authority_current_object_id TEXT",
            "authority_source_fingerprint TEXT",
            "planner_session_created_at TEXT", "planner_invocation_created_at TEXT",
            "planner_harness_applied_at TEXT", "planner_harness_json TEXT",
            "planner_launch_requested_at TEXT", "planner_launch_accepted_at TEXT",
            "planner_ready_at TEXT", "planner_provider_activation_observed_at TEXT",
            "planner_lifecycle_observed_at TEXT",
        ] {
            let name = column.split_whitespace().next().expect("planning request migration column name");
            let exists = connection.prepare("PRAGMA table_info(work_slice_planning_requests)")
                .and_then(|mut statement| statement.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>())
                .map_err(|e| SprintRunnerTransitionError::Unavailable(format!("inspect Work Slice planning request schema: {e}")))?
                .iter().any(|existing| existing == name);
            if !exists { connection.execute_batch(&format!("ALTER TABLE work_slice_planning_requests ADD COLUMN {column}"))
                .map_err(|e| SprintRunnerTransitionError::Unavailable(format!("migrate Work Slice planning request schema: {e}")))?; }
        }
        let handler_columns = [
            "handler_harness_revision_id TEXT",
            "handler_harness_configuration_digest TEXT",
            "handler_harness_repository_commit_ref TEXT",
        ];
        for column in handler_columns {
            let name = column.split_whitespace().next().expect("Handler activation migration column name");
            let exists = connection.prepare("PRAGMA table_info(work_unit_handler_activations)")
                .and_then(|mut statement| statement.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>())
                .map_err(|e| SprintRunnerTransitionError::Unavailable(format!("inspect Handler activation schema: {e}")))?
                .iter().any(|existing| existing == name);
            if !exists { connection.execute_batch(&format!("ALTER TABLE work_unit_handler_activations ADD COLUMN {column}"))
                .map_err(|e| SprintRunnerTransitionError::Unavailable(format!("migrate Handler activation schema: {e}")))?; }
        }
        migrate_legacy_implementer_activations(&connection)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        let authority_repository = SqliteOrchestrationRepository::open(path)
            .map_err(|error| SprintRunnerTransitionError::Unavailable(format!("open Sprint Git authority repository: {error}")))?;
        let service = Arc::new(Self {
            connection: Mutex::new(connection),
            authority_repository,
            sessions,
            work_unit_handler: Mutex::new(None),
            mcp: Mutex::new(HashMap::new()),
            transition_locks: Mutex::new(HashMap::new()),
            effect_drains: Mutex::new(HashMap::new()),
            #[cfg(test)]
            test_reconcile_snapshot_hook: Mutex::new(None),
            #[cfg(test)]
            test_origin_snapshot_hook: Mutex::new(None),
        });
        service.reconcile_work_slice_planners()?;
        service.reconcile_materializations()?;
        Ok(service)
    }

    /// Product composition attaches the narrow Handler package only after both the Session
    /// runtime and execution-support authority are available.  Attachment is replay-safe.
    pub(crate) fn attach_work_unit_handler_activation(
        self: &Arc<Self>,
        handler: Arc<WorkUnitExecutionHarnessService>,
    ) -> Result<(), SprintRunnerTransitionError> {
        *self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))? = Some(handler);
        self.reconcile_work_unit_handlers()
    }

    #[cfg(test)]
    pub(crate) fn request_work_unit_implementer_from_authenticated_continuation(
        self: &Arc<Self>,
        invocation_id: &str,
    ) -> Result<(), SprintRunnerTransitionError> {
        let invocation = AgentInvocationId::new(invocation_id.to_owned())
            .map_err(|_| SprintRunnerTransitionError::Forbidden)?;
        self.request_work_unit_implementer(&invocation)
    }

    #[cfg(test)]
    pub(crate) fn prepared_handler_action_injection(
        &self,
        invocation_id: &str,
    ) -> Option<CodexMcpInjection> {
        let invocation = AgentInvocationId::new(invocation_id.to_owned()).ok()?;
        self.mcp.lock().ok()?.get(&invocation).map(|action| action.injection.clone())
    }

    /// Starts the one-invocation semantic boundary before the Epic Runner launch request is made.
    pub(crate) fn prepare_epic_runner_action(
        self: &Arc<Self>,
        invocation_id: AgentInvocationId,
        enabled_tools: &[String],
        required: bool,
    ) -> Result<CodexMcpInjection, SprintRunnerTransitionError> {
        let bearer = uuid::Uuid::new_v4().simple().to_string();
        let server = start_epic_runner_server(
            self.clone(),
            invocation_id.clone(),
            bearer.clone(),
            vec!["tauri://localhost".into()],
        )
        .map_err(|e| {
            SprintRunnerTransitionError::Unavailable(format!(
                "start Epic Runner action server: {e}"
            ))
        })?;
        let injection = CodexMcpInjection::new_named(
            "epic_runner",
            &server.url(),
            bearer,
            enabled_tools,
            required,
        );
        let mut active = self.mcp.lock().map_err(|_| {
            SprintRunnerTransitionError::Unavailable(
                "Epic Runner action registry is poisoned".into(),
            )
        })?;
        if active.contains_key(&invocation_id) {
            return Err(SprintRunnerTransitionError::Conflict);
        }
        active.insert(
            invocation_id,
            ManagedEpicRunnerAction {
                server,
                injection: injection.clone(),
            },
        );
        Ok(injection)
    }

    pub(crate) fn on_epic_runner_terminal(&self, invocation_id: &AgentInvocationId) {
        if let Ok(mut active) = self.mcp.lock() {
            if let Some(managed) = active.remove(invocation_id) {
                managed.server.stop();
            }
        }
    }

    fn prepare_pre_start_action(self: &Arc<Self>, invocation_id: AgentInvocationId) -> Result<CodexMcpInjection, SprintRunnerTransitionError> {
        let bearer=uuid::Uuid::new_v4().simple().to_string();
        let server=start_pre_start_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start pre-start action server: {e}")))?;
        let injection=CodexMcpInjection::new_named("sprint_runner_pre_start",&server.url(),bearer,&["report_pre_start_outcome".into()],true);
        let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner action registry is poisoned".into()))?;
        if let Some(existing)=active.get(&invocation_id) { let existing=existing.injection.clone(); server.stop(); return Ok(existing); }
        active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()}); Ok(injection)
    }
    fn prepare_epic_start_action(self: &Arc<Self>, invocation_id: AgentInvocationId) -> Result<CodexMcpInjection, SprintRunnerTransitionError> {
        let bearer=uuid::Uuid::new_v4().simple().to_string(); let server=start_epic_start_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start selected-Sprint action server: {e}")))?; let injection=CodexMcpInjection::new_named("epic_runner_start",&server.url(),bearer,&["start_selected_sprint".into()],true); let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner action registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)
    }
    fn prepare_started_action(self: &Arc<Self>, invocation_id: AgentInvocationId) -> Result<CodexMcpInjection, SprintRunnerTransitionError> {
        let bearer=uuid::Uuid::new_v4().simple().to_string();let server=start_started_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start reevaluation action server: {e}")))?;let injection=CodexMcpInjection::new_named("sprint_runner_started",&server.url(),bearer,&["record_started_reevaluation".into()],true);let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner action registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)
    }
    fn prepare_planning_control_action(self: &Arc<Self>, invocation_id: AgentInvocationId) -> Result<CodexMcpInjection, SprintRunnerTransitionError> {
        let bearer=uuid::Uuid::new_v4().simple().to_string();let server=start_planning_control_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start planning-control action server: {e}")))?;let injection=CodexMcpInjection::new_named("sprint_runner_planning_control",&server.url(),bearer,&["request_work_slice_planner".into()],true);let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner action registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)
    }
    fn prepare_work_slice_planner_action(self: &Arc<Self>, invocation_id: AgentInvocationId) -> Result<CodexMcpInjection, SprintRunnerTransitionError> {
        let bearer=uuid::Uuid::new_v4().simple().to_string();
        let server=start_work_slice_planner_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start Work Slice Planner action server: {e}")))?;
        let injection=CodexMcpInjection::new_named("work_slice_planner",&server.url(),bearer,&["read_current_planning_context".into(),"submit_work_slice_proposal".into(),"request_work_slice_refinement".into(),"complete_work_slice_planning".into()],true);
        let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Slice Planner action registry is poisoned".into()))?;
        if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}
        active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)
    }
    fn prepare_work_unit_handler_action(self:&Arc<Self>,invocation_id:AgentInvocationId)->Result<CodexMcpInjection,SprintRunnerTransitionError>{let bearer=uuid::Uuid::new_v4().simple().to_string();let server=start_work_unit_handler_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start Work Unit Handler action server: {e}")))?;let injection=CodexMcpInjection::new_named("work_unit_handler",&server.url(),bearer,&["request_work_unit_implementer".into()],true);let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Handler action registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)}


    pub(crate) fn shutdown(&self) {
        if let Ok(mut active) = self.mcp.lock() {
            for (_, managed) in active.drain() {
                managed.server.stop();
            }
        }
    }

    /// The MCP adapter supplies only the chosen durable Sprint identity. Everything else is rederived.
    pub(crate) fn request_next_sprint_runner(
        self: &Arc<Self>,
        invocation_id: &AgentInvocationId,
        selection: SprintRunnerSelection,
    ) -> Result<SprintRunnerTransitionStatus, SprintRunnerTransitionError> {
        if !safe_id(&selection.sprint_id) {
            return Err(SprintRunnerTransitionError::Invalid);
        }
        let transition_lock = self.transition_lock(&selection.sprint_id)?;
        let _transition_guard = transition_lock.lock().map_err(|_| {
            SprintRunnerTransitionError::Unavailable(
                "Sprint Runner transition lock is poisoned".into(),
            )
        })?;
        let runner = self.authorized_runner(invocation_id.as_str(), &selection.sprint_id)?;
        let sprint_harness = conversation_harness::profile(ConversationHarnessRole::SprintRunner)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        let request_id = stable_id("sprint-runner-request", &selection.sprint_id);
        let session_id = stable_id("sprint-runner-session", &selection.sprint_id);
        let invocation = stable_id("sprint-runner-invocation", &selection.sprint_id);
        let now = chrono::Utc::now().to_rfc3339();
        {
            let mut conn = self.connection.lock().map_err(|_| {
                SprintRunnerTransitionError::Unavailable(
                    "Sprint Runner transition database lock is poisoned".into(),
                )
            })?;
            let tx = conn.transaction().map_err(|e| {
                SprintRunnerTransitionError::Unavailable(format!(
                    "authorize Sprint Runner request: {e}"
                ))
            })?;
            let existing: Option<(String, String, String, String, String)> = tx.query_row(
                "SELECT epic_id,epic_runner_invocation_id,sprint_runner_session_id,sprint_runner_invocation_id,request_id FROM sprint_runner_transitions WHERE sprint_id=?1",
                [&selection.sprint_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            if let Some(existing) = existing {
                if existing
                    == (
                        runner.epic_id.clone(),
                        invocation_id.as_str().into(),
                        session_id.clone(),
                        invocation.clone(),
                        request_id.clone(),
                    )
                {
                    tx.commit()
                        .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
                } else {
                    return Err(SprintRunnerTransitionError::Conflict);
                }
            } else {
                tx.execute("INSERT INTO sprint_runner_transitions (sprint_id,epic_id,request_id,epic_runner_session_id,epic_runner_invocation_id,epic_runner_harness_key,epic_runner_harness_version,sprint_runner_harness_key,sprint_runner_harness_version,sprint_runner_session_id,sprint_runner_invocation_id,requested_at,authorized_at) VALUES (?1,?2,?3,?4,?5,'epic_runner',?6,?7,?8,?9,?10,?11,?11)", params![selection.sprint_id, runner.epic_id, request_id, runner.session_id, invocation_id.as_str(), runner.harness_version, sprint_harness.key, sprint_harness.version, session_id, invocation, now]).map_err(|e| SprintRunnerTransitionError::Unavailable(format!("persist Sprint Runner authorization: {e}")))?;
                tx.commit().map_err(|e| {
                    SprintRunnerTransitionError::Unavailable(format!(
                        "commit Sprint Runner authorization: {e}"
                    ))
                })?;
            }
        }
        drop(_transition_guard);
        self.reconcile_sprint(&selection.sprint_id)?;
        // A competing replay may have arrived while the one stable application send was in
        // progress. Give that bounded local effect a chance to publish its durable launch fact;
        // a persisted-not-accepted invocation still returns its truthful attention state.
        for _ in 0..64 {
            let status = self.query()?.transitions.into_iter().find(|status| status.sprint_id == selection.sprint_id).ok_or_else(|| SprintRunnerTransitionError::Unavailable("authorized Sprint Runner disappeared".into()))?;
            if status.harness_applied_at.is_some() {
                return Ok(status);
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
            self.reconcile_sprint(&selection.sprint_id)?;
        }
        self.query()?
            .transitions
            .into_iter()
            .find(|status| status.sprint_id == selection.sprint_id)
            .ok_or_else(|| {
                SprintRunnerTransitionError::Unavailable(
                    "authorized Sprint Runner disappeared".into(),
                )
            })
    }

    pub(crate) fn reconcile_startup(
        self: &Arc<Self>,
    ) -> Result<usize, SprintRunnerTransitionError> {
        let ids = {
            let conn = self.connection.lock().map_err(|_| {
                SprintRunnerTransitionError::Unavailable(
                    "Sprint Runner transition database lock is poisoned".into(),
                )
            })?;
            let mut statement = conn.prepare("SELECT sprint_id FROM sprint_runner_transitions ORDER BY requested_at,sprint_id").map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            let ids = statement
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            ids
        };
        for id in &ids {
            let transition_lock = self.transition_lock(id)?;
            let _transition_guard = transition_lock.lock().map_err(|_| {
                SprintRunnerTransitionError::Unavailable(
                    "Sprint Runner transition lock is poisoned".into(),
                )
            })?;
            self.observe_existing_terminals(id)?;
            drop(_transition_guard);
            self.reconcile_sprint(id)?;
        }
        self.reconcile_work_unit_handlers()?;
        Ok(ids.len())
    }

    fn observe_existing_terminals(&self, sprint_id: &str) -> Result<(), SprintRunnerTransitionError> {
        let record: Option<(String,String)> = self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row("SELECT sprint_runner_session_id,COALESCE(pre_start_outcome_invocation_id,pre_start_upgrade_invocation_id,sprint_runner_invocation_id) FROM sprint_runner_transitions WHERE sprint_id=?1",[sprint_id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((session_id,outcome_invocation))=record else{return Ok(())}; let session=AgentSessionId::new(session_id).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Ok(history)=self.sessions.load_session(&session) else{return Ok(())};
        if let Some(found)=history.invocations.iter().find(|candidate|candidate.invocation.id.as_str()==outcome_invocation) { if found.invocation.status.is_terminal(){self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.execute("UPDATE sprint_runner_transitions SET pre_start_lifecycle_status=?2,pre_start_lifecycle_invocation_id=?3,pre_start_lifecycle_observed_at=CASE WHEN pre_start_lifecycle_invocation_id=?3 THEN pre_start_lifecycle_observed_at ELSE ?4 END WHERE sprint_id=?1",params![sprint_id,lifecycle_status(found.invocation.status),outcome_invocation,chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;}}
        let started: Option<String> = self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row("SELECT sprint_continuation_invocation_id FROM sprint_runner_transitions WHERE sprint_id=?1 AND repository_branch_reevaluation_fact_id IS NOT NULL",[sprint_id],|r|r.get(0)).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?.flatten();
        if let Some(started)=started { if let Some(found)=history.invocations.iter().find(|candidate|candidate.invocation.id.as_str()==started) { if found.invocation.status.is_terminal(){self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.execute("UPDATE sprint_runner_transitions SET started_reevaluation_lifecycle_status=?2,started_reevaluation_lifecycle_invocation_id=?3,started_reevaluation_lifecycle_observed_at=CASE WHEN started_reevaluation_lifecycle_invocation_id=?3 THEN started_reevaluation_lifecycle_observed_at ELSE ?4 END WHERE sprint_id=?1",params![sprint_id,lifecycle_status(found.invocation.status),started,chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;}}}
        Ok(())
    }

    pub(crate) fn query(
        &self,
    ) -> Result<SprintRunnerTransitionQueryV1, SprintRunnerTransitionError> {
        let conn = self.connection.lock().map_err(|_| {
            SprintRunnerTransitionError::Unavailable(
                "Sprint Runner transition database lock is poisoned".into(),
            )
        })?;
        let mut statement = conn.prepare("SELECT t.sprint_id,t.epic_id,t.request_id,t.epic_runner_invocation_id,t.sprint_runner_session_id,t.sprint_runner_invocation_id,t.requested_at,t.authorized_at,t.session_created_at,t.harness_applied_at,t.launch_accepted_at,t.pre_start_semantic_outcome_recorded_at,t.pre_start_lifecycle_observed_at,t.pre_start_outcome_accepted_at,t.parent_continuation_delivery_requested_at,t.parent_continuation_delivery_persisted_at,t.epic_continuation_invocation_id,t.epic_continuation_launch_accepted_at,t.provider_receiver_activation_observed_at,t.sprint_start_authorized_at,t.sprint_start_persisted_at,t.sprint_continuation_invocation_id,t.sprint_continuation_launch_accepted_at,t.repository_branch_reevaluation_recorded_at,t.started_reevaluation_lifecycle_observed_at,t.planning_control_delivery_requested_at,t.planning_control_delivery_persisted_at,t.planning_control_invocation_id,t.planning_control_launch_accepted_at,t.planning_ready_at,p.request_fact_id,p.requested_at,p.authorized_at,p.planning_point_id,p.repository_worktree_route,p.planner_harness_key,p.planner_harness_version,p.planner_session_id,p.planner_invocation_id,p.planner_session_created_at,p.planner_invocation_created_at,p.planner_harness_applied_at,p.planner_launch_requested_at,p.planner_launch_accepted_at,p.planner_ready_at,p.planner_provider_activation_observed_at,p.planner_lifecycle_observed_at,r.submitted_at,r.validation_result,r.refinement_requested_at,r.semantic_completed_at,r.lifecycle_observed_at,r.accepted_at,r.materialization_ready_at FROM sprint_runner_transitions t LEFT JOIN work_slice_planning_requests p ON p.sprint_id=t.sprint_id AND p.is_current=1 LEFT JOIN work_slice_proposal_revisions r ON r.planning_point_id=p.planning_point_id AND r.is_current=1 ORDER BY t.requested_at,t.sprint_id").map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let transitions = statement
            .query_map([], |r| {
                Ok(SprintRunnerTransitionStatus {
                    sprint_id: r.get(0)?,
                    epic_id: r.get(1)?,
                    request_id: r.get(2)?,
                    epic_runner_invocation_id: r.get(3)?,
                    sprint_runner_session_id: r.get(4)?,
                    sprint_runner_invocation_id: r.get(5)?,
                    requested_at: r.get(6)?,
                    authorized_at: r.get(7)?,
                    session_created_at: r.get(8)?,
                    harness_applied_at: r.get(9)?,
                    launch_accepted_at: r.get(10)?,
                    pre_start_ready: r.get::<_, Option<String>>(10)?.is_some(),
                    lifecycle_observed: r.get::<_, Option<String>>(12)?.is_some(),
                    accepted: r.get::<_, Option<String>>(13)?.is_some(),
                    pre_start_semantic_outcome_recorded_at: r.get(11)?,
                    pre_start_lifecycle_observed_at: r.get(12)?,
                    pre_start_outcome_accepted_at: r.get(13)?,
                    parent_continuation_delivery_requested_at: r.get(14)?,
                    parent_continuation_delivery_persisted_at: r.get(15)?,
                    epic_continuation_invocation_id: r.get(16)?,
                    epic_continuation_launch_accepted_at: r.get(17)?,
                    provider_receiver_activation_observed_at: r.get(18)?,
                    sprint_start_authorized_at: r.get(19)?,
                    sprint_start_persisted_at: r.get(20)?,
                    sprint_continuation_invocation_id: r.get(21)?,
                    sprint_continuation_launch_accepted_at: r.get(22)?,
                    repository_branch_reevaluation_recorded_at: r.get(23)?,
                    started_reevaluation_lifecycle_observed_at: r.get(24)?,
                    planning_control_delivery_requested_at: r.get(25)?,
                    planning_control_delivery_persisted_at: r.get(26)?,
                    planning_control_invocation_id: r.get(27)?,
                    planning_control_launch_accepted_at: r.get(28)?,
                    planning_ready_at: r.get(29)?,
                    work_slice_planner_request_id: r.get(30)?,
                    work_slice_planner_requested_at: r.get(31)?,
                    work_slice_planner_authorized_at: r.get(32)?,
                    work_slice_planning_point_id: r.get(33)?,
                    work_slice_planner_repository_worktree_route: r.get(34)?,
                    work_slice_planner_harness_key: r.get(35)?,
                    work_slice_planner_harness_version: r.get(36)?,
                    work_slice_planner_session_id: r.get(37)?,
                    work_slice_planner_invocation_id: r.get(38)?,
                    work_slice_planner_session_created_at: r.get(39)?,
                    work_slice_planner_invocation_created_at: r.get(40)?,
                    work_slice_planner_harness_applied_at: r.get(41)?,
                    work_slice_planner_launch_requested_at: r.get(42)?,
                    work_slice_planner_launch_accepted_at: r.get(43)?,
                    work_slice_planner_ready_at: r.get(44)?,
                    work_slice_planner_provider_activation_observed_at: r.get(45)?,
                    work_slice_planner_lifecycle_observed_at: r.get(46)?,
                    work_slice_proposal_submitted_at: r.get(47)?,
                    work_slice_proposal_validation_result: r.get(48)?,
                    work_slice_refinement_requested_at: r.get(49)?,
                    work_slice_semantic_completed_at: r.get(50)?,
                    work_slice_terminal_lifecycle_observed_at: r.get(51)?,
                    work_slice_application_accepted_at: r.get(52)?,
                    work_slice_materialization_ready_at: r.get(53)?,
                    downstream_not_started: r.get::<_, Option<String>>(33)?.is_none(),
                })
            })
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        Ok(SprintRunnerTransitionQueryV1 {
            contract: SPRINT_RUNNER_QUERY_CONTRACT,
            transitions,
        })
    }

    /// Persist the one allowed pre-start semantic result.  The input deliberately has no
    /// session, invocation, Epic, Sprint, or routing identity.
    pub(crate) fn record_pre_start_outcome(
        self: &Arc<Self>, invocation_id: &AgentInvocationId, input: PreStartOutcome,
    ) -> Result<(), SprintRunnerTransitionError> {
        validate_outcome(&input.forecast_and_concerns)?;
        validate_outcome(&input.material_uncertainty)?;
        validate_outcome(&input.application_owned_prerequisite)?;
        let sprint_id: Option<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row(
            "SELECT sprint_id FROM sprint_runner_transitions WHERE sprint_runner_invocation_id=?1 OR pre_start_upgrade_invocation_id=?1", [invocation_id.as_str()], |row| row.get(0),
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some(sprint_id) = sprint_id else { return Err(SprintRunnerTransitionError::Forbidden) };
        let lock=self.transition_lock(&sprint_id)?; let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition lock is poisoned".into()))?;
        let conn = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?;
        let found: (Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>) = conn.query_row(
            "SELECT pre_start_outcome_fact_id,pre_start_outcome_invocation_id,pre_start_forecast,pre_start_material_uncertainty,pre_start_prerequisite,CASE WHEN sprint_runner_invocation_id=?2 THEN launch_accepted_at ELSE pre_start_upgrade_launch_accepted_at END FROM sprint_runner_transitions WHERE sprint_id=?1", params![sprint_id,invocation_id.as_str()], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if found.5.is_none() { return Err(SprintRunnerTransitionError::Forbidden); }
        let fact = stable_id("pre-start-semantic-outcome", &sprint_id);
        if let Some(existing) = found.0 { return if existing == fact && found.1.as_deref()==Some(invocation_id.as_str()) && found.2.as_deref()==Some(&input.forecast_and_concerns) && found.3.as_deref()==Some(&input.material_uncertainty) && found.4.as_deref()==Some(&input.application_owned_prerequisite) { drop(conn); drop(_guard); self.reconcile_sprint(&sprint_id)?; Ok(()) } else { Err(SprintRunnerTransitionError::Conflict) }; }
        conn.execute("UPDATE sprint_runner_transitions SET pre_start_outcome_fact_id=?2,pre_start_outcome_invocation_id=?3,pre_start_semantic_outcome_recorded_at=?4,pre_start_forecast=?5,pre_start_material_uncertainty=?6,pre_start_prerequisite=?7 WHERE sprint_id=?1", params![sprint_id,fact,invocation_id.as_str(),chrono::Utc::now().to_rfc3339(),input.forecast_and_concerns,input.material_uncertainty,input.application_owned_prerequisite]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        drop(conn); drop(_guard); self.reconcile_sprint(&sprint_id)
    }

    /// Called after Agent Session persistence by the productive notifier.  A terminal lifecycle
    /// has no semantic meaning on its own; acceptance remains conditional on the bound fact.
    pub(crate) fn on_agent_notification(
        self: &Arc<Self>, notification: &AgentSessionNotification,
    ) -> Result<(), SprintRunnerTransitionError> {
        let (notification_invocation, handler_invocation) = match notification {
            AgentSessionNotification::EventPersisted { event, .. } => {
                let handler = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
                    "SELECT EXISTS(SELECT 1 FROM work_unit_handler_activations WHERE handler_invocation_id=?1)",
                    [event.invocation_id.as_str()], |row| row.get::<_, bool>(0)
                ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
                (event.invocation_id.clone(), handler)
            }
            AgentSessionNotification::InvocationTerminal { invocation, .. } => {
                let handler = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
                    "SELECT EXISTS(SELECT 1 FROM work_unit_handler_activations WHERE handler_invocation_id=?1)",
                    [invocation.id.as_str()], |row| row.get::<_, bool>(0)
                ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
                (invocation.id.clone(), handler)
            }
            AgentSessionNotification::DiagnosticRecorded { invocation, .. } => (invocation.id.clone(), false),
        };
        if handler_invocation {
            // Provider activity is projected only from the durable per-invocation observation
            // seam during this reconciliation; terminal state creates no downstream fact.
            let _ = notification_invocation;
            return self.reconcile_work_unit_handlers();
        }
        let AgentSessionNotification::InvocationTerminal { invocation, .. } = notification else { return Ok(()) };
        // Every scoped action server belongs to exactly this invocation, including fresh
        // continuation servers that Bootstrap does not know about.
        self.on_epic_runner_terminal(&invocation.id);
        let status = lifecycle_status(invocation.status);
        let sprint: Option<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row(
            "SELECT sprint_id FROM sprint_runner_transitions WHERE epic_runner_invocation_id=?1 OR sprint_runner_invocation_id=?1 OR pre_start_upgrade_invocation_id=?1 OR epic_continuation_invocation_id=?1 OR sprint_continuation_invocation_id=?1 OR planning_control_invocation_id=?1 UNION SELECT sprint_id FROM work_slice_planning_requests WHERE planner_invocation_id=?1",
            [invocation.id.as_str()], |row| row.get(0),
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some(sprint_id) = sprint else { return Ok(()) };
        let lock=self.transition_lock(&sprint_id)?; let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition lock is poisoned".into()))?;
        let conn = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?;
        conn.execute("UPDATE sprint_runner_transitions SET pre_start_lifecycle_status=CASE WHEN sprint_runner_invocation_id=?2 OR pre_start_upgrade_invocation_id=?2 THEN ?3 ELSE pre_start_lifecycle_status END,pre_start_lifecycle_invocation_id=CASE WHEN sprint_runner_invocation_id=?2 OR pre_start_upgrade_invocation_id=?2 THEN ?2 ELSE pre_start_lifecycle_invocation_id END,pre_start_lifecycle_observed_at=CASE WHEN sprint_runner_invocation_id=?2 OR pre_start_upgrade_invocation_id=?2 THEN CASE WHEN pre_start_lifecycle_invocation_id=?2 THEN pre_start_lifecycle_observed_at ELSE ?4 END ELSE pre_start_lifecycle_observed_at END WHERE sprint_id=?1",params![sprint_id,invocation.id.as_str(),status,chrono::Utc::now().to_rfc3339()]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        conn.execute("UPDATE sprint_runner_transitions SET started_reevaluation_lifecycle_status=CASE WHEN sprint_continuation_invocation_id=?2 AND repository_branch_reevaluation_fact_id IS NOT NULL THEN ?3 ELSE started_reevaluation_lifecycle_status END,started_reevaluation_lifecycle_invocation_id=CASE WHEN sprint_continuation_invocation_id=?2 AND repository_branch_reevaluation_fact_id IS NOT NULL THEN ?2 ELSE started_reevaluation_lifecycle_invocation_id END,started_reevaluation_lifecycle_observed_at=CASE WHEN sprint_continuation_invocation_id=?2 AND repository_branch_reevaluation_fact_id IS NOT NULL THEN CASE WHEN started_reevaluation_lifecycle_invocation_id=?2 THEN started_reevaluation_lifecycle_observed_at ELSE ?4 END ELSE started_reevaluation_lifecycle_observed_at END WHERE sprint_id=?1",params![sprint_id,invocation.id.as_str(),status,chrono::Utc::now().to_rfc3339()]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        conn.execute("UPDATE work_slice_proposal_revisions SET lifecycle_observed_at=COALESCE(lifecycle_observed_at,?2),lifecycle_status=COALESCE(lifecycle_status,?3) WHERE planning_point_id IN (SELECT planning_point_id FROM work_slice_planning_requests WHERE planner_invocation_id=?1 AND planner_ready_at IS NOT NULL) AND is_current=1 AND semantic_completion_invocation_id=?1 AND semantic_completed_at IS NOT NULL",params![invocation.id.as_str(),chrono::Utc::now().to_rfc3339(),status]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        conn.execute("UPDATE work_slice_proposal_revisions SET accepted_at=COALESCE(accepted_at,?2),materialization_ready_at=COALESCE(materialization_ready_at,?2) WHERE planning_point_id IN (SELECT planning_point_id FROM work_slice_planning_requests WHERE planner_invocation_id=?1 AND planner_ready_at IS NOT NULL) AND is_current=1 AND validation_result='valid' AND refinement_requested_at IS NULL AND semantic_completion_invocation_id=?1 AND semantic_completed_at IS NOT NULL AND lifecycle_status='completed'",params![invocation.id.as_str(),chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        drop(conn); drop(_guard);
        match self.reconcile_sprint(&sprint_id) {
            Ok(()) => Ok(()),
            // AgentSessionApplication persists its application-provenance invocation before it
            // starts the runtime. A concurrent start may therefore be rejected by the Session's
            // single-active-invocation guard while the first stable continuation is in flight.
            // Treat that as an idempotent replay only after durable launch evidence proves the
            // exact continuation ID was persisted; all other transition errors remain visible.
            Err(SprintRunnerTransitionError::Unavailable(message))
                if message.contains("session already has active invocation") => {
                    let (continuation, session): (Option<String>, String) = self
                        .connection
                        .lock()
                        .map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?
                        .query_row(
                            "SELECT sprint_continuation_invocation_id,sprint_runner_session_id FROM sprint_runner_transitions WHERE sprint_id=?1",
                            [&sprint_id],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    let Some(continuation) = continuation else {
                        return Err(SprintRunnerTransitionError::Unavailable(message));
                    };
                    let invocation = AgentInvocationId::new(continuation)
                        .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    let session = AgentSessionId::new(session)
                        .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    match self
                        .sessions
                        .application_invocation_launch_evidence(&invocation, &session)
                        .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?
                    {
                        ApplicationInvocationLaunchEvidence::NeverPersisted => {
                            Err(SprintRunnerTransitionError::Unavailable(message))
                        }
                        ApplicationInvocationLaunchEvidence::PersistedNotAccepted
                        | ApplicationInvocationLaunchEvidence::LaunchAccepted => Ok(()),
                    }
                }
            Err(error) => Err(error),
        }
    }

    /// This is the only authority that starts the selected Sprint.  Identity comes solely from
    /// the fresh application-owned Epic continuation invocation.
    pub(crate) fn start_selected_sprint(self: &Arc<Self>, invocation_id: &AgentInvocationId) -> Result<(), SprintRunnerTransitionError> {
        let sprint: Option<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row(
            "SELECT sprint_id FROM sprint_runner_transitions WHERE epic_continuation_invocation_id=?1", [invocation_id.as_str()], |row| row.get(0),
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some(sprint_id) = sprint else { return Err(SprintRunnerTransitionError::Forbidden) };
        let lock=self.transition_lock(&sprint_id)?;let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition lock is poisoned".into()))?;
        let desired=conversation_harness::profile(ConversationHarnessRole::EpicRunner).map_err(SprintRunnerTransitionError::Unavailable)?;
        let authorized:Option<String>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row("SELECT sprint_id FROM sprint_runner_transitions WHERE sprint_id=?1 AND epic_continuation_invocation_id=?2 AND pre_start_outcome_accepted_at IS NOT NULL AND parent_continuation_delivery_fact_id IS NOT NULL AND parent_continuation_delivered_outcome_fact_id=pre_start_outcome_fact_id AND epic_continuation_launch_accepted_at IS NOT NULL AND epic_continuation_harness_key=?3 AND epic_continuation_harness_version=?4 AND epic_continuation_harness_applied_at IS NOT NULL",params![sprint_id,invocation_id.as_str(),desired.key,desired.version],|r|r.get(0)).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if authorized.is_none(){return Err(SprintRunnerTransitionError::Forbidden)}
        let now=chrono::Utc::now().to_rfc3339();
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.execute(
            "UPDATE sprint_runner_transitions SET epic_start_semantic_authorization_requested_at=COALESCE(epic_start_semantic_authorization_requested_at,?2),epic_start_semantic_authorization_recorded_at=COALESCE(epic_start_semantic_authorization_recorded_at,?2),sprint_start_authorized_at=COALESCE(sprint_start_authorized_at,?2),sprint_start_persisted_at=COALESCE(sprint_start_persisted_at,?2) WHERE sprint_id=?1", params![sprint_id,now]
        ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        drop(_guard);
        self.reconcile_sprint(&sprint_id)
    }

    pub(crate) fn record_started_reevaluation(self: &Arc<Self>, invocation_id: &AgentInvocationId, input: StartedReevaluation) -> Result<(), SprintRunnerTransitionError> {
        validate_outcome(&input.repository_branch_evaluation)?;
        validate_outcome(&input.started_forecast_and_concerns)?;
        let desired=conversation_harness::profile(ConversationHarnessRole::SprintRunner).map_err(SprintRunnerTransitionError::Unavailable)?;
        let conn=self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?;
        let sprint: Option<(String,Option<String>,Option<String>)>=conn.query_row("SELECT sprint_id,sprint_start_persisted_at,sprint_continuation_launch_accepted_at FROM sprint_runner_transitions WHERE sprint_continuation_invocation_id=?1 AND sprint_continuation_harness_key=?2 AND sprint_continuation_harness_version=?3 AND sprint_continuation_harness_applied_at IS NOT NULL",params![invocation_id.as_str(),desired.key,desired.version],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((sprint_id,started,launched))=sprint else{return Err(SprintRunnerTransitionError::Forbidden)}; drop(conn);
        let lock=self.transition_lock(&sprint_id)?;let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition lock is poisoned".into()))?;let conn=self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?;
        if started.is_none() || launched.is_none(){return Err(SprintRunnerTransitionError::Forbidden)}
        let fact=stable_id("repository-branch-reevaluation",&sprint_id);
        let old:Option<(String,String)>=conn.query_row("SELECT repository_branch_evaluation,started_forecast_and_concerns FROM sprint_runner_transitions WHERE sprint_id=?1 AND repository_branch_reevaluation_fact_id IS NOT NULL",[&sprint_id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if let Some(old)=old {return if old==(input.repository_branch_evaluation,input.started_forecast_and_concerns){Ok(())}else{Err(SprintRunnerTransitionError::Conflict)}}
        conn.execute("UPDATE sprint_runner_transitions SET repository_branch_reevaluation_fact_id=?2,repository_branch_reevaluation_recorded_at=?3,repository_branch_evaluation=?4,started_forecast_and_concerns=?5 WHERE sprint_id=?1",params![sprint_id,fact,chrono::Utc::now().to_rfc3339(),input.repository_branch_evaluation,input.started_forecast_and_concerns]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        Ok(())
    }

    fn authorized_runner(
        &self,
        invocation_id: &str,
        sprint_id: &str,
    ) -> Result<AuthorizedRunner, SprintRunnerTransitionError> {
        let desired = conversation_harness::profile(ConversationHarnessRole::EpicRunner)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        let conn = self.connection.lock().map_err(|_| {
            SprintRunnerTransitionError::Unavailable(
                "Sprint Runner transition database lock is poisoned".into(),
            )
        })?;
        let result = conn.query_row("SELECT transition.epic_id,transition.runner_session_id,transition.runner_harness_version FROM epic_bootstrap_transitions transition JOIN initiated_sprints sprint ON sprint.epic_id=transition.epic_id JOIN agent_session_invocation_launch_acceptances acceptance ON acceptance.invocation_id=transition.runner_invocation_id JOIN agent_session_invocations invocation ON invocation.id=transition.runner_invocation_id AND invocation.session_id=transition.runner_session_id AND invocation.input_provenance='application' WHERE transition.runner_invocation_id=?1 AND sprint.id=?2 AND transition.runner_launched_at IS NOT NULL AND transition.runner_harness_key='epic_runner' AND transition.runner_harness_applied_at IS NOT NULL", params![invocation_id,sprint_id], |r| Ok(AuthorizedRunner { epic_id:r.get(0)?, session_id:r.get(1)?, harness_version:r.get(2)? })).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        match result {
            // A persisted v2 Epic Runner remains a historical applied binding; it may complete
            // its existing request route, but new continuation invocations bind the current v3.
            Some(runner) if runner.harness_version == 2 || runner.harness_version == i64::from(desired.version) => Ok(runner),
            Some(_) => Err(SprintRunnerTransitionError::Conflict),
            None => Err(SprintRunnerTransitionError::Forbidden),
        }
    }

    /// Loads one private, self-validating Sprint Git authority. The authority repository
    /// rechecks its Sprint/Epic/provenance chain and fingerprint before exposing a route.
    fn planning_route_authority(
        &self,
        sprint_id: &str,
    ) -> Result<InitiatedSprintGitAuthority, SprintRunnerTransitionError> {
        match self.authority_repository.load_initiated_sprint_git_authority_for_sprint(sprint_id) {
            Ok(Some(authority)) => Ok(authority),
            Ok(None) | Err(InitiatedSprintGitAuthorityError::Invalid) | Err(InitiatedSprintGitAuthorityError::Forbidden) => Err(SprintRunnerTransitionError::Forbidden),
            Err(InitiatedSprintGitAuthorityError::Conflict) => Err(SprintRunnerTransitionError::Conflict),
            Err(InitiatedSprintGitAuthorityError::Unavailable) => Err(SprintRunnerTransitionError::Unavailable("load Sprint Git authority".into())),
        }
    }

    /// Lossless nonblocking drain. Every caller advances the generation before deciding whether
    /// it owns the pass. A notifier re-entry therefore returns immediately, while the current
    /// owner observes the newer generation and drains a further durable snapshot before release.
    fn reconcile_sprint(
        self: &Arc<Self>,
        sprint_id: &str,
    ) -> Result<(), SprintRunnerTransitionError> {
        let owns = {
            let mut drains = self.effect_drains.lock().map_err(|_| {
                SprintRunnerTransitionError::Unavailable(
                    "Sprint Runner reconciliation drain is poisoned".into(),
                )
            })?;
            let drain = drains.entry(sprint_id.to_owned()).or_default();
            drain.generation = drain.generation.wrapping_add(1);
            if drain.running { false } else { drain.running = true; true }
        };
        if !owns { return Ok(()); }
        loop {
            let observed = self.effect_drains.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner reconciliation drain is poisoned".into()))?
                .get(sprint_id).map(|drain| drain.generation).ok_or_else(|| SprintRunnerTransitionError::Unavailable("Sprint Runner reconciliation drain disappeared".into()))?;
            if let Err(error) = self.reconcile_sprint_pass(sprint_id) {
                let mut drains = self.effect_drains.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner reconciliation drain is poisoned".into()))?;
                if let Some(drain) = drains.get_mut(sprint_id) { drain.running = false; }
                return Err(error);
            }
            let repeat = {
                let mut drains = self.effect_drains.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner reconciliation drain is poisoned".into()))?;
                let drain = drains.get_mut(sprint_id).ok_or_else(|| SprintRunnerTransitionError::Unavailable("Sprint Runner reconciliation drain disappeared".into()))?;
                if drain.generation != observed { true } else { drain.running = false; false }
            };
            if !repeat {
                self.reconcile_work_slice_acceptance(sprint_id)?;
                self.reconcile_work_unit_handlers()?;
                return Ok(());
            }
        }
    }

    fn reconcile_sprint_pass(
        self: &Arc<Self>,
        sprint_id: &str,
    ) -> Result<(), SprintRunnerTransitionError> {
        let record = { let conn=self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?; conn.query_row("SELECT sprint_id,epic_id,sprint_runner_session_id,sprint_runner_invocation_id,sprint_runner_harness_key,sprint_runner_harness_version,session_created_at,harness_applied_at,launch_accepted_at FROM sprint_runner_transitions WHERE sprint_id=?1", [sprint_id], |r| Ok(SprintRecord { sprint_id:r.get(0)?, epic_id:r.get(1)?, session_id:r.get(2)?, invocation_id:r.get(3)?, harness_key:r.get(4)?, harness_version:r.get(5)?, session_created_at:r.get(6)?, harness_applied_at:r.get(7)?, launch_accepted_at:r.get(8)? })).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))? }.ok_or(SprintRunnerTransitionError::Invalid)?;
        let harness = conversation_harness::profile(ConversationHarnessRole::SprintRunner)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        if record.harness_key != harness.key || record.harness_version != harness.version {
            if record.harness_key == "sprint_runner" && record.harness_version == 1 && record.launch_accepted_at.is_some() {
                return self.reconcile_v1_pre_start_upgrade(&record, &harness);
            }
            return Err(SprintRunnerTransitionError::Conflict);
        }
        let session = AgentSessionId::new(record.session_id.clone())
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if record.session_created_at.is_none() {
            self.sessions
                .create_application_session(CreateApplicationAgentSessionCommand {
                    session_id: session.clone(),
                    session: CreateAgentSessionCommand {
                        title: Some(format!("Sprint Runner: {}", record.sprint_id)),
                        working_directory: Some(
                            conversation_harness::role_discovery_root(
                                ConversationHarnessRole::SprintRunner,
                            )
                            .map_err(SprintRunnerTransitionError::Unavailable)?,
                        ),
                        requested_options: harness.runtime_options(),
                    },
                })
                .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            self.mark(&record.sprint_id, "session_created_at")?;
        }
        let invocation = AgentInvocationId::new(record.invocation_id.clone())
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        match self
            .sessions
            .application_invocation_launch_evidence(&invocation, &session)
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?
        {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => {
                self.mark(&record.sprint_id, "launch_accepted_at")?
            }
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {}
            ApplicationInvocationLaunchEvidence::NeverPersisted => {
                let injection=self.prepare_pre_start_action(invocation.clone())?; let mut additional_args=harness.runtime_configuration_args();additional_args.extend(injection.configuration_args); let launch=self.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id:invocation, message:SendAgentSessionMessageCommand { session_id:Some(session), submitted_text:format!("Maintain the selected Sprint in a truthful pre-start ready state. Submit exactly one structured pre-start outcome through report_pre_start_outcome, then stop. Do not create Work Slice planning or Work Units.\n\nEpic ID: {}\nSprint ID: {}",record.epic_id,record.sprint_id), title:None, working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunner).map_err(SprintRunnerTransitionError::Unavailable)?), requested_options:Some(harness.runtime_options()) }}, Some(RuntimeLaunchExtension { additional_args, environment:vec![injection.environment], initial_prompt_prefix:Some(harness.initial_prompt_prefix()) })).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
                self.mark(&record.sprint_id, "harness_applied_at")?;
                if launch.launch_accepted {
                    self.mark(&record.sprint_id, "launch_accepted_at")?;
                }
            }
        }
        self.reconcile_accepted_pre_start(sprint_id)?;
        Ok(())
    }

    /// A v1 launch remains exactly v1.  This adds a new v2, application-owned invocation in the
    /// same Session rather than rewriting the historical applied Harness revision.
    fn reconcile_v1_pre_start_upgrade(self:&Arc<Self>,record:&SprintRecord,harness:&conversation_harness::ConversationHarnessProfile)->Result<(),SprintRunnerTransitionError>{
        let id=stable_id("sprint-runner-v2-pre-start-upgrade",&record.sprint_id);let session=AgentSessionId::new(record.session_id.clone()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let history=self.sessions.load_session(&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let terminal=history.invocations.iter().find(|candidate|candidate.invocation.id.as_str()==record.invocation_id).is_some_and(|candidate|candidate.invocation.status.is_terminal());if !terminal{return Ok(())};self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.execute("UPDATE sprint_runner_transitions SET pre_start_upgrade_invocation_id=COALESCE(pre_start_upgrade_invocation_id,?2),pre_start_upgrade_harness_key=COALESCE(pre_start_upgrade_harness_key,?3),pre_start_upgrade_harness_version=COALESCE(pre_start_upgrade_harness_version,?4) WHERE sprint_id=?1",params![record.sprint_id,id,harness.key,harness.version]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let invocation=AgentInvocationId::new(id).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;match self.sessions.application_invocation_launch_evidence(&invocation,&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?{ApplicationInvocationLaunchEvidence::LaunchAccepted=>self.mark(&record.sprint_id,"pre_start_upgrade_launch_accepted_at")?,ApplicationInvocationLaunchEvidence::PersistedNotAccepted=>{},ApplicationInvocationLaunchEvidence::NeverPersisted=>{let injection=self.prepare_pre_start_action(invocation.clone())?;let mut additional_args=harness.runtime_configuration_args();additional_args.extend(injection.configuration_args);let launch=self.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:invocation,message:SendAgentSessionMessageCommand{session_id:Some(session),submitted_text:format!("The prior pre-start invocation applied Sprint Runner Harness v1. It remains historical. This fresh application-owned v2 pre-start invocation must submit one outcome through report_pre_start_outcome, then stop.\n\nEpic ID: {}\nSprint ID: {}",record.epic_id,record.sprint_id),title:None,working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunner).map_err(SprintRunnerTransitionError::Unavailable)?),requested_options:Some(harness.runtime_options())}},Some(RuntimeLaunchExtension{additional_args,environment:vec![injection.environment],initial_prompt_prefix:Some(harness.initial_prompt_prefix())})).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;self.mark(&record.sprint_id,"pre_start_upgrade_harness_applied_at")?;if launch.launch_accepted{self.mark(&record.sprint_id,"pre_start_upgrade_launch_accepted_at")?;}}};self.reconcile_accepted_pre_start(&record.sprint_id)
    }

    fn reconcile_accepted_pre_start(self: &Arc<Self>, sprint_id: &str) -> Result<(), SprintRunnerTransitionError> {
        let record: Option<(String,String,String,String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row(
            "SELECT epic_id,epic_runner_session_id,epic_runner_invocation_id,sprint_runner_session_id,pre_start_outcome_fact_id,pre_start_outcome_invocation_id,pre_start_lifecycle_status,pre_start_lifecycle_invocation_id,pre_start_outcome_accepted_at,epic_continuation_invocation_id,epic_continuation_launch_accepted_at,sprint_start_persisted_at,sprint_continuation_invocation_id FROM sprint_runner_transitions WHERE sprint_id=?1", [sprint_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?,r.get(12)?)),
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        #[cfg(test)]
        self.run_test_hook(false);
        let Some((epic_id,epic_session,origin_epic_invocation,sprint_session,fact,outcome_invocation,lifecycle,lifecycle_invocation,accepted,_epic_invocation,_epic_launch,_started,_sprint_invocation))=record else{return Ok(())};
        if fact.is_some() && lifecycle.as_deref()==Some("completed") && outcome_invocation==lifecycle_invocation && accepted.is_none() {
            let now=chrono::Utc::now().to_rfc3339(); let invocation=stable_id("epic-runner-start-continuation",sprint_id);
            let harness=conversation_harness::profile(ConversationHarnessRole::EpicRunner).map_err(SprintRunnerTransitionError::Unavailable)?;
            let delivery=stable_id("accepted-pre-start-parent-delivery",sprint_id);self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.execute("UPDATE sprint_runner_transitions SET pre_start_outcome_accepted_at=?2,parent_continuation_delivery_requested_at=?2,epic_continuation_invocation_id=?3,epic_continuation_harness_key=?4,epic_continuation_harness_version=?5,parent_continuation_delivery_fact_id=?6,parent_continuation_delivered_outcome_fact_id=pre_start_outcome_fact_id WHERE sprint_id=?1",params![sprint_id,now,invocation,harness.key,harness.version,delivery]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        }
        let record=self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row("SELECT epic_continuation_invocation_id,epic_continuation_launch_accepted_at,sprint_start_persisted_at,sprint_continuation_invocation_id,sprint_continuation_launch_accepted_at FROM sprint_runner_transitions WHERE sprint_id=?1",[sprint_id],|r|Ok((r.get::<_,Option<String>>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,Option<String>>(4)?))).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if let (Some(invocation_id), None)=(record.0.clone(),record.1) {
            let session=AgentSessionId::new(epic_session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            let history=self.sessions.load_session(&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            let origin_terminal=history.invocations.iter().find(|candidate|candidate.invocation.id.as_str()==origin_epic_invocation).is_some_and(|candidate|candidate.invocation.status.is_terminal());
            #[cfg(test)]
            self.run_test_hook(true);
            // Accepted outcome and durable delivery intent are truthful while the originating
            // Epic Runner is active. A fresh invocation in that Session is illegal until it ends.
            if !origin_terminal { return Ok(()); }
            let invocation=AgentInvocationId::new(invocation_id.clone()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            match self.sessions.application_invocation_launch_evidence(&invocation,&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))? {
                ApplicationInvocationLaunchEvidence::LaunchAccepted => self.mark(sprint_id,"epic_continuation_launch_accepted_at")?,
                ApplicationInvocationLaunchEvidence::PersistedNotAccepted => { self.mark(sprint_id,"parent_continuation_delivery_persisted_at")?; },
                ApplicationInvocationLaunchEvidence::NeverPersisted => {
                    let harness=conversation_harness::profile(ConversationHarnessRole::EpicRunner).map_err(SprintRunnerTransitionError::Unavailable)?;let outcome:(String,String,String)=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row("SELECT pre_start_forecast,pre_start_material_uncertainty,pre_start_prerequisite FROM sprint_runner_transitions WHERE sprint_id=?1",[sprint_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let injection=self.prepare_epic_start_action(invocation.clone())?;let mut additional_args=harness.runtime_configuration_args();additional_args.extend(injection.configuration_args); let launch=self.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:invocation,message:SendAgentSessionMessageCommand{session_id:Some(session),submitted_text:format!("The application accepted and correlates this exact completed pre-start outcome. Review it and use start_selected_sprint only if you semantically authorize start. Delivery is not authorization.\n\nEpic ID: {epic_id}\nSelected Sprint ID: {sprint_id}\n\nForecast and concerns:\n{}\n\nMaterial uncertainty:\n{}\n\nApplication-owned prerequisite:\n{}",outcome.0,outcome.1,outcome.2),title:None,working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunner).map_err(SprintRunnerTransitionError::Unavailable)?),requested_options:Some(harness.runtime_options())}},Some(RuntimeLaunchExtension{additional_args,environment:vec![injection.environment],initial_prompt_prefix:Some(harness.initial_prompt_prefix())})).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
                    self.mark(sprint_id,"parent_continuation_delivery_persisted_at")?; self.mark(sprint_id,"epic_continuation_harness_applied_at")?; if launch.launch_accepted{self.mark(sprint_id,"epic_continuation_launch_accepted_at")?;}
                }
            }
        }
        let started: Option<String>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row("SELECT sprint_start_persisted_at FROM sprint_runner_transitions WHERE sprint_id=?1",[sprint_id],|r|r.get(0)).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?.flatten();
        if started.is_some() {
            let continuation=stable_id("started-sprint-runner-continuation",sprint_id);
            let harness=conversation_harness::profile(ConversationHarnessRole::SprintRunner).map_err(SprintRunnerTransitionError::Unavailable)?;self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.execute("UPDATE sprint_runner_transitions SET sprint_continuation_invocation_id=COALESCE(sprint_continuation_invocation_id,?2),sprint_continuation_harness_key=COALESCE(sprint_continuation_harness_key,?3),sprint_continuation_harness_version=COALESCE(sprint_continuation_harness_version,?4) WHERE sprint_id=?1",params![sprint_id,continuation,harness.key,harness.version]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            let (id,launch): (String,Option<String>)=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row("SELECT sprint_continuation_invocation_id,sprint_continuation_launch_accepted_at FROM sprint_runner_transitions WHERE sprint_id=?1",[sprint_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            if launch.is_none(){ let session=AgentSessionId::new(sprint_session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?; let invocation=AgentInvocationId::new(id).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?; match self.sessions.application_invocation_launch_evidence(&invocation,&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))? { ApplicationInvocationLaunchEvidence::LaunchAccepted=>self.mark(sprint_id,"sprint_continuation_launch_accepted_at")?, ApplicationInvocationLaunchEvidence::PersistedNotAccepted=>{}, ApplicationInvocationLaunchEvidence::NeverPersisted=>{let harness=conversation_harness::profile(ConversationHarnessRole::SprintRunner).map_err(SprintRunnerTransitionError::Unavailable)?;let injection=self.prepare_started_action(invocation.clone())?;let mut additional_args=harness.runtime_configuration_args();additional_args.extend(injection.configuration_args);let launch=self.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:invocation,message:SendAgentSessionMessageCommand{session_id:Some(session),submitted_text:format!("Sprint start is durably authorized. Reevaluate the repository and branch, then submit that semantic evidence through record_started_reevaluation. Do not create a planning point, Work Slice Planner, or Work Units.\n\nEpic ID: {epic_id}\nSprint ID: {sprint_id}"),title:None,working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunner).map_err(SprintRunnerTransitionError::Unavailable)?),requested_options:Some(harness.runtime_options())}},Some(RuntimeLaunchExtension{additional_args,environment:vec![injection.environment],initial_prompt_prefix:Some(harness.initial_prompt_prefix())})).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;self.mark(sprint_id,"sprint_continuation_harness_applied_at")?;if launch.launch_accepted{self.mark(sprint_id,"sprint_continuation_launch_accepted_at")?;}}}}
        }
        self.reconcile_planning_control(sprint_id)
    }

    fn reconcile_planning_control(self: &Arc<Self>, sprint_id: &str) -> Result<(), SprintRunnerTransitionError> {
        let record: Option<(String, String, Option<String>, Option<String>, Option<String>, Option<String>)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row(
            "SELECT sprint_runner_session_id,epic_id,repository_branch_reevaluation_fact_id,started_reevaluation_lifecycle_status,planning_control_invocation_id,planning_control_launch_accepted_at FROM sprint_runner_transitions WHERE sprint_id=?1",
            [sprint_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((session_id, epic_id, fact, lifecycle, control, accepted)) = record else { return Ok(()) };
        if fact.is_none() || lifecycle.as_deref() != Some("completed") { return Ok(()); }
        let harness=conversation_harness::profile(ConversationHarnessRole::SprintRunnerPlanningControl).map_err(SprintRunnerTransitionError::Unavailable)?;
        let invocation_id=control.unwrap_or_else(|| stable_id("sprint-runner-planning-control-continuation",sprint_id));
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.execute(
            "UPDATE sprint_runner_transitions SET planning_control_delivery_requested_at=COALESCE(planning_control_delivery_requested_at,?2),planning_control_invocation_id=COALESCE(planning_control_invocation_id,?3),planning_control_harness_key=COALESCE(planning_control_harness_key,?4),planning_control_harness_version=COALESCE(planning_control_harness_version,?5) WHERE sprint_id=?1",
            params![sprint_id,chrono::Utc::now().to_rfc3339(),invocation_id,harness.key,harness.version],
        ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if accepted.is_some() { return Ok(()); }
        let session=AgentSessionId::new(session_id).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let invocation=AgentInvocationId::new(invocation_id).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        match self.sessions.application_invocation_launch_evidence(&invocation,&session).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))? {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => { self.mark(sprint_id,"planning_control_delivery_persisted_at")?; self.mark(sprint_id,"planning_control_launch_accepted_at")?; self.mark(sprint_id,"planning_ready_at")?; }
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted => self.mark(sprint_id,"planning_control_delivery_persisted_at")?,
            ApplicationInvocationLaunchEvidence::NeverPersisted => {
                let injection=self.prepare_planning_control_action(invocation.clone())?;
                let mut additional_args=harness.runtime_configuration_args(); additional_args.extend(injection.configuration_args);
                let launch=self.sessions.send_idempotent_application_message_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id:invocation, message:SendAgentSessionMessageCommand { session_id:Some(session), submitted_text:format!("The completed started reevaluation is durably observed. This is the only current planning-control invocation. Request exactly one Work Slice Planner through request_work_slice_planner only if this Sprint needs one current temporal planning decision. Do not create Work Units, Handlers, or Implementers.\n\nEpic ID: {epic_id}\nSprint ID: {sprint_id}"), title:None, working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunnerPlanningControl).map_err(SprintRunnerTransitionError::Unavailable)?), requested_options:Some(harness.runtime_options()) }}, Some(RuntimeLaunchExtension { additional_args, environment:vec![injection.environment], initial_prompt_prefix:Some(harness.initial_prompt_prefix()) })).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
                self.mark(sprint_id,"planning_control_delivery_persisted_at")?; self.mark(sprint_id,"planning_control_harness_applied_at")?;
                if launch.launch_accepted { self.mark(sprint_id,"planning_control_launch_accepted_at")?; self.mark(sprint_id,"planning_ready_at")?; }
            }
        }
        Ok(())
    }

    /// Accept exactly one identity-free request from the exact applied, launch-accepted planning
    /// control. This persists intent only; child Session creation, Harness application, and launch
    /// reconciliation are deliberately owned by later steps.
    pub(crate) fn request_work_slice_planner(self: &Arc<Self>, invocation_id: &AgentInvocationId, _input: WorkSlicePlannerRequest) -> Result<SprintRunnerTransitionStatus, SprintRunnerTransitionError> {
        let sprint_id: Option<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row("SELECT sprint_id FROM sprint_runner_transitions WHERE planning_control_invocation_id=?1", [invocation_id.as_str()], |r| r.get(0)).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some(sprint_id) = sprint_id else { return Err(SprintRunnerTransitionError::Forbidden) };
        let planning_control = conversation_harness::profile(ConversationHarnessRole::SprintRunnerPlanningControl).map_err(SprintRunnerTransitionError::Unavailable)?;
        let planner_harness = conversation_harness::profile(ConversationHarnessRole::WorkSlicePlanner).map_err(SprintRunnerTransitionError::Unavailable)?;
        let authority = self.planning_route_authority(&sprint_id)?;
        let point = stable_id("work-slice-planning-point", &sprint_id);
        let request = stable_id("work-slice-planner-request", &sprint_id);
        let planner_session = stable_id("work-slice-planner-session", &point);
        let planner_invocation = stable_id("work-slice-planner-invocation", &point);
        let lock = self.transition_lock(&sprint_id)?;
        let _guard = lock.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition lock is poisoned".into()))?;
        let mut conn = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let session_id: Option<String> = tx.query_row(
            "SELECT sprint_runner_session_id FROM sprint_runner_transitions WHERE sprint_id=?1 AND planning_control_invocation_id=?2 AND planning_ready_at IS NOT NULL AND planning_control_harness_key=?3 AND planning_control_harness_version=?4 AND planning_control_harness_applied_at IS NOT NULL AND planning_control_launch_accepted_at IS NOT NULL",
            params![sprint_id, invocation_id.as_str(), planning_control.key, planning_control.version],
            |r| r.get(0),
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some(session_id) = session_id else { return Err(SprintRunnerTransitionError::Forbidden) };
        let existing: Option<CurrentPlanningRequest> = tx.query_row(
            "SELECT planning_point_id,request_fact_id,parent_sprint_runner_session_id,parent_planning_control_invocation_id,authority_id,authority_epic_id,authority_provenance_id,authority_repository_id,authority_worktree_id,authority_baseline_object_id,authority_current_object_id,authority_source_fingerprint,repository_worktree_route,planner_harness_key,planner_harness_version,planner_session_id,planner_invocation_id FROM work_slice_planning_requests WHERE sprint_id=?1 AND is_current=1",
            [&sprint_id],
            |r| Ok(CurrentPlanningRequest { planning_point_id:r.get(0)?, request_fact_id:r.get(1)?, parent_sprint_runner_session_id:r.get(2)?, parent_planning_control_invocation_id:r.get(3)?, authority_id:r.get(4)?, authority_epic_id:r.get(5)?, authority_provenance_id:r.get(6)?, authority_repository_id:r.get(7)?, authority_worktree_id:r.get(8)?, authority_baseline_object_id:r.get(9)?, authority_current_object_id:r.get(10)?, authority_source_fingerprint:r.get(11)?, repository_worktree_route:r.get(12)?, planner_harness_key:r.get(13)?, planner_harness_version:r.get(14)?, planner_session_id:r.get(15)?, planner_invocation_id:r.get(16)? }),
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if let Some(existing) = existing {
            let expected = CurrentPlanningRequest { planning_point_id:point.clone(), request_fact_id:request.clone(), parent_sprint_runner_session_id:session_id.clone(), parent_planning_control_invocation_id:invocation_id.as_str().to_owned(), authority_id:authority.authority_id.clone(), authority_epic_id:authority.epic_id.clone(), authority_provenance_id:authority.provenance_id.clone(), authority_repository_id:authority.repository_id.clone(), authority_worktree_id:authority.worktree_id.clone(), authority_baseline_object_id:authority.baseline_object_id.clone(), authority_current_object_id:authority.current_object_id.clone(), authority_source_fingerprint:authority.source_fingerprint.clone(), repository_worktree_route:authority.worktree_root.clone(), planner_harness_key:planner_harness.key.clone(), planner_harness_version:i64::from(planner_harness.version), planner_session_id:planner_session.clone(), planner_invocation_id:planner_invocation.clone() };
            if existing != expected { return Err(SprintRunnerTransitionError::Conflict); }
        } else {
            let now = chrono::Utc::now().to_rfc3339();
            tx.execute(
                "INSERT INTO work_slice_planning_requests (planning_point_id,sprint_id,planning_episode,is_current,request_fact_id,parent_sprint_runner_session_id,parent_planning_control_invocation_id,authority_id,authority_epic_id,authority_provenance_id,authority_repository_id,authority_worktree_id,authority_baseline_object_id,authority_current_object_id,authority_source_fingerprint,repository_worktree_route,requested_at,authorized_at,planner_harness_key,planner_harness_version,planner_session_id,planner_invocation_id) VALUES (?1,?2,1,1,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?15,?16,?17,?18,?19)",
                params![point, sprint_id, request, session_id, invocation_id.as_str(), authority.authority_id, authority.epic_id, authority.provenance_id, authority.repository_id, authority.worktree_id, authority.baseline_object_id, authority.current_object_id, authority.source_fingerprint, authority.worktree_root, now, planner_harness.key, planner_harness.version, planner_session, planner_invocation],
            ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        }
        tx.commit().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        drop(conn);
        drop(_guard);
        self.reconcile_work_slice_planner(&sprint_id)?;
        self.query()?.transitions.into_iter().find(|transition| transition.sprint_id == sprint_id).ok_or_else(|| SprintRunnerTransitionError::Unavailable("authorized Work Slice Planner disappeared".into()))
    }

    fn reconcile_work_slice_planners(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        let conn = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?;
        let mut statement = conn.prepare("SELECT sprint_id FROM work_slice_planning_requests WHERE is_current=1 ORDER BY requested_at,planning_point_id").map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let ids = statement.query_map([], |row| row.get::<_, String>(0)).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.collect::<Result<Vec<_>, _>>().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        drop(statement);
        drop(conn);
        for sprint_id in ids { self.reconcile_work_slice_planner(&sprint_id)?; self.reconcile_work_slice_acceptance(&sprint_id)?; }
        Ok(())
    }

    /// Startup repair observes only the persisted exact Planner invocation.  It never prepares a
    /// replacement invocation or starts a provider; it merely converges previously durable facts.
    fn reconcile_work_slice_acceptance(&self,sprint_id:&str)->Result<(),SprintRunnerTransitionError>{
        let record:Option<(String,String,String,String,i64)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT p.planning_point_id,p.planner_session_id,p.planner_invocation_id,p.planner_harness_key,p.planner_harness_version FROM work_slice_planning_requests p JOIN work_slice_proposal_revisions r ON r.planning_point_id=p.planning_point_id AND r.is_current=1 WHERE p.sprint_id=?1 AND p.is_current=1 AND p.planner_ready_at IS NOT NULL AND r.validation_result='valid' AND r.refinement_requested_at IS NULL AND r.semantic_completion_invocation_id=p.planner_invocation_id AND r.semantic_completed_at IS NOT NULL",[sprint_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((point,session_id,invocation_id,key,version))=record else{return Ok(())};let desired=conversation_harness::profile(ConversationHarnessRole::WorkSlicePlanner).map_err(SprintRunnerTransitionError::Unavailable)?;if key!=desired.key || version!=i64::from(desired.version){return Ok(())}
        let session=AgentSessionId::new(session_id).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let history=self.sessions.load_session(&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let Some(found)=history.invocations.iter().find(|candidate|candidate.invocation.id.as_str()==invocation_id && candidate.invocation.status.is_terminal()) else{return Ok(())};let status=lifecycle_status(found.invocation.status);let now=chrono::Utc::now().to_rfc3339();
        self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_slice_proposal_revisions SET lifecycle_observed_at=COALESCE(lifecycle_observed_at,?2),lifecycle_status=COALESCE(lifecycle_status,?3),accepted_at=CASE WHEN ?3='completed' THEN COALESCE(accepted_at,?2) ELSE accepted_at END,materialization_ready_at=CASE WHEN ?3='completed' THEN COALESCE(materialization_ready_at,?2) ELSE materialization_ready_at END WHERE planning_point_id=?1 AND is_current=1 AND validation_result='valid' AND refinement_requested_at IS NULL AND semantic_completion_invocation_id=?4 AND semantic_completed_at IS NOT NULL",params![point,now,status,invocation_id]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if status == "completed" { self.materialize_accepted_work_slice_plan(sprint_id)?; }
        Ok(())
    }

    /// Product-only convergence of the exact current accepted revision. This persists planned
    /// responsibilities and their plan relationships; it never launches downstream work.
    fn materialize_accepted_work_slice_plan(&self, sprint_id: &str) -> Result<(), SprintRunnerTransitionError> {
        let lock = self.transition_lock(sprint_id)?;
        let _guard = lock.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition lock is poisoned".into()))?;
        let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let source: Option<(String, String, String, String)> = tx.query_row(
            "SELECT p.planning_point_id,r.revision_id,t.epic_id,p.sprint_id
             FROM work_slice_planning_requests p
             JOIN sprint_runner_transitions t ON t.sprint_id=p.sprint_id
             JOIN work_slice_planning_episodes e ON e.planning_point_id=p.planning_point_id AND e.sprint_id=p.sprint_id AND e.authority_id=p.authority_id
             JOIN work_slice_proposal_revisions r ON r.planning_point_id=p.planning_point_id
             JOIN initiated_sprints s ON s.id=p.sprint_id AND s.epic_id=t.epic_id
             WHERE p.sprint_id=?1 AND p.is_current=1 AND r.is_current=1
               AND r.validation_result='valid' AND r.refinement_requested_at IS NULL
               AND r.semantic_completion_invocation_id=p.planner_invocation_id AND r.semantic_completed_at IS NOT NULL
               AND r.lifecycle_status='completed' AND r.accepted_at IS NOT NULL AND r.materialization_ready_at IS NOT NULL",
            [sprint_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((point, revision, epic, sprint)) = source else { return Err(SprintRunnerTransitionError::Forbidden) };
        let proposal_json: String = tx.query_row("SELECT proposal_json FROM work_slice_proposal_revisions WHERE revision_id=?1", [&revision], |row| row.get(0)).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let proposal: WorkSliceProposal = serde_json::from_str(&proposal_json).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        validate_work_slice_proposal(&proposal)?;
        let materialization = stable_id("work-unit-materialization", &format!("{epic}:{sprint}:{point}:{revision}"));
        let work_slice = stable_id("work-slice", &format!("{point}:{revision}"));
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute("INSERT OR IGNORE INTO work_unit_materializations (materialization_id,planning_point_id,accepted_revision_id,epic_id,sprint_id,work_slice_id,authorization_recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![materialization,point,revision,epic,sprint,work_slice,now]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let stored: (String,String,String,String,String) = tx.query_row("SELECT materialization_id,accepted_revision_id,epic_id,sprint_id,work_slice_id FROM work_unit_materializations WHERE planning_point_id=?1", [&point], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if stored != (materialization.clone(), revision.clone(), epic.clone(), sprint.clone(), work_slice.clone()) { return Err(SprintRunnerTransitionError::Conflict) }
        tx.execute("UPDATE work_unit_materializations SET attempt_recorded_at=COALESCE(attempt_recorded_at,?2) WHERE materialization_id=?1", params![materialization,now]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let units = proposal.lanes.iter().enumerate().map(|(ordinal,lane)| (stable_id("work-unit", &format!("{materialization}:{ordinal}")), ordinal as i64, lane.title.clone(), lane.specification.clone())).collect::<Vec<_>>();
        for (id, ordinal, title, specification) in &units {
            tx.execute("INSERT OR IGNORE INTO work_units (work_unit_id,materialization_id,work_slice_id,accepted_revision_id,lane_ordinal,lane_title,specification) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![id,materialization,work_slice,revision,ordinal,title,specification]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        }
        let stored_units = tx.prepare("SELECT work_unit_id,lane_ordinal,lane_title,specification FROM work_units WHERE materialization_id=?1 ORDER BY lane_ordinal").map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.query_map([&materialization], |row| Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?))).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let expected_units = units.iter().map(|(id,ordinal,title,specification)|(id.clone(),*ordinal,title.clone(),specification.clone())).collect::<Vec<_>>();
        if stored_units != expected_units { return Err(SprintRunnerTransitionError::Conflict) }
        tx.execute("UPDATE work_unit_materializations SET work_units_created_at=COALESCE(work_units_created_at,?2) WHERE materialization_id=?1", params![materialization,now]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let by_title = units.iter().map(|(id,_,title,_)|(title.as_str(),id.as_str())).collect::<std::collections::HashMap<_,_>>();
        let mut relationships = vec![
            (stable_id("work-unit-relationship", &format!("{materialization}:planning_point:{point}:{work_slice}")), "planning_point", point.clone(), work_slice.clone(), None),
            (stable_id("work-unit-relationship", &format!("{materialization}:sprint:{sprint}:{work_slice}")), "sprint", sprint.clone(), work_slice.clone(), None),
        ];
        for (id, ordinal, title, _) in &units {
            relationships.push((stable_id("work-unit-relationship", &format!("{materialization}:lane:{work_slice}:{id}")), "lane", work_slice.clone(), id.clone(), Some(*ordinal)));
            relationships.push((stable_id("work-unit-relationship", &format!("{materialization}:order:{work_slice}:{id}")), "order", work_slice.clone(), id.clone(), Some(*ordinal)));
            for dependency in &proposal.lanes[*ordinal as usize].depends_on { let dependency_id = by_title.get(dependency.as_str()).ok_or(SprintRunnerTransitionError::Conflict)?; relationships.push((stable_id("work-unit-relationship", &format!("{materialization}:depends_on:{id}:{dependency_id}")), "depends_on", id.clone(), (*dependency_id).to_owned(), None)); }
            let _ = title;
        }
        for (id, kind, from, to, ordinal) in &relationships { tx.execute("INSERT OR IGNORE INTO work_unit_relationships (relationship_id,materialization_id,relationship_kind,from_id,to_id,ordinal) VALUES (?1,?2,?3,?4,?5,?6)", params![id,materialization,kind,from,to,ordinal]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?; }
        let stored_relationships = tx.prepare("SELECT relationship_id,relationship_kind,from_id,to_id,ordinal FROM work_unit_relationships WHERE materialization_id=?1 ORDER BY relationship_id").map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.query_map([&materialization], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,Option<i64>>(4)?))).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let mut expected_relationships = relationships.into_iter().map(|(id,kind,from,to,ordinal)|(id,kind.to_owned(),from,to,ordinal)).collect::<Vec<_>>(); expected_relationships.sort();
        if stored_relationships != expected_relationships { return Err(SprintRunnerTransitionError::Conflict) }
        tx.execute("UPDATE work_unit_materializations SET relationships_completed_at=COALESCE(relationships_completed_at,?2),settled_at=COALESCE(settled_at,?2) WHERE materialization_id=?1", params![materialization,now]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        tx.commit().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        Ok(())
    }

    fn reconcile_materializations(&self) -> Result<(), SprintRunnerTransitionError> {
        let sprints = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare("SELECT sprint_id FROM work_unit_materializations ORDER BY authorization_recorded_at,materialization_id").map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.query_map([], |row| row.get::<_,String>(0)).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        for sprint in sprints { self.materialize_accepted_work_slice_plan(&sprint)?; }
        Ok(())
    }

    /// Reconciles only canonical settled materializations. Plan dependencies remain blocked here
    /// until a later boundary supplies an authoritative prerequisite-satisfaction fact.
    fn reconcile_work_unit_handlers(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        let Some(handler) = self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone() else { return Ok(()) };
        let units = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare(
            "SELECT u.work_unit_id,u.materialization_id,m.sprint_id FROM work_units u JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id WHERE m.settled_at IS NOT NULL ORDER BY m.sprint_id,u.lane_ordinal,u.work_unit_id"
        ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        for (work_unit, materialization, sprint) in units {
            self.reconcile_work_unit_handler(&handler, &work_unit, &materialization, &sprint)?;
        }
        self.reconcile_implementer_activations()?;
        Ok(())
    }

    /// A persisted request is replayed through the same authenticated continuation boundary.
    /// The request method reads the pinned Implementer identity already stored in the row, so a
    /// reopen neither chooses a replacement revision nor relaunches an accepted invocation.
    fn reconcile_implementer_activations(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        let invocations = self.connection.lock()
            .map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .prepare("SELECT handler_invocation_id FROM work_unit_implementer_activations ORDER BY requested_at,work_unit_id")
            .and_then(|mut statement| statement.query_map([], |row| row.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for invocation in invocations {
            let invocation = AgentInvocationId::new(invocation)
                .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            self.request_work_unit_implementer(&invocation)?;
        }
        Ok(())
    }

    fn reconcile_work_unit_handler(
        self: &Arc<Self>,
        handler: &Arc<WorkUnitExecutionHarnessService>,
        work_unit_id: &str,
        materialization_id: &str,
        sprint_id: &str,
    ) -> Result<(), SprintRunnerTransitionError> {
        let lock = self.transition_lock(sprint_id)?;
        let _guard = lock.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit activation lock is poisoned".into()))?;
        let dependency_count: i64 = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT COUNT(*) FROM work_unit_relationships r WHERE r.materialization_id=?1 AND r.relationship_kind='depends_on' AND r.from_id=?2",
            params![materialization_id, work_unit_id], |row| row.get(0)
        ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let authority: Option<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT authority_id FROM initiated_sprint_git_authorities WHERE sprint_id=?1 ORDER BY recorded_at,authority_id LIMIT 1", [sprint_id], |row| row.get(0)
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        // A Handler's readiness is a launch fact, never satisfaction of a plan dependency.
        // This boundary has no authoritative prerequisite-satisfaction fact, so dependents stay
        // blocked even after a prerequisite Handler is launch-accepted or provider-observed.
        let blocked = handler_activation_blocked_reason(dependency_count, authority.is_some());
        let attempt_id = stable_id("work-unit-handler-attempt", work_unit_id);
        let session_id = stable_id("work-unit-handler-session", work_unit_id);
        let invocation_id = stable_id("work-unit-handler-invocation", work_unit_id);
        let now = chrono::Utc::now().to_rfc3339();
        let existing: bool = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT EXISTS(SELECT 1 FROM work_unit_handler_activations WHERE work_unit_id=?1)", [work_unit_id], |row| row.get(0)
        ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if existing {
            self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(
                "UPDATE work_unit_handler_activations SET eligibility_state=?2,blocked_reason=?3 WHERE work_unit_id=?1 AND launch_accepted_at IS NULL",
                params![work_unit_id,if blocked.is_some(){"blocked"}else{"eligible"},blocked]
            ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        } else {
            let desired = handler.current_handler_revision()
                .map_err(|_| SprintRunnerTransitionError::Unavailable("load current immutable Handler revision".into()))?;
            self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(
                "INSERT OR IGNORE INTO work_unit_handler_activations (work_unit_id,materialization_id,sprint_id,attempt_id,handler_session_id,handler_invocation_id,handler_harness_key,handler_harness_version,handler_harness_revision_id,handler_harness_configuration_digest,handler_harness_repository_commit_ref,eligibility_state,blocked_reason,requested_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![work_unit_id,materialization_id,sprint_id,attempt_id,session_id,invocation_id,desired.harness_key,desired.profile.version,desired.revision_id,desired.configuration_digest,desired.repository_commit_ref,if blocked.is_some(){"blocked"}else{"eligible"},blocked,now]
            ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        }
        if blocked.is_some() { return Ok(()) }
        let (harness_key, harness_version, revision_id, configuration_digest, repository_commit_ref): (String, i64, Option<String>, Option<String>, Option<String>) = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT handler_harness_key,handler_harness_version,handler_harness_revision_id,handler_harness_configuration_digest,handler_harness_repository_commit_ref FROM work_unit_handler_activations WHERE work_unit_id=?1", [work_unit_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let revision_id = revision_id.ok_or(SprintRunnerTransitionError::Conflict)?;
        let configuration_digest = configuration_digest.ok_or(SprintRunnerTransitionError::Conflict)?;
        let repository_commit_ref = repository_commit_ref.ok_or(SprintRunnerTransitionError::Conflict)?;
        let harness = handler.load_pinned_handler_revision(&revision_id, &configuration_digest, &repository_commit_ref)
            .map_err(|error| SprintRunnerTransitionError::Unavailable(format!("pinned Handler revision evidence is invalid or unavailable: {error:?}")))?;
        if harness.harness_key != harness_key
            || harness.profile.version != u16::try_from(harness_version).map_err(|_| SprintRunnerTransitionError::Conflict)?
            || harness.profile.mcp.required
            || !harness.profile.mcp.enabled_tools.is_empty()
        {
            return Err(SprintRunnerTransitionError::Conflict);
        }
        let authority = authority.expect("eligible activation has authority");
        self.mark_handler(work_unit_id, "authorized_at")?;
        self.mark_handler(work_unit_id, "attempt_created_at")?;
        handler.authorize_handler_attempt(&attempt_id, work_unit_id, &authority)
            .map_err(|_| SprintRunnerTransitionError::Unavailable("execution-support authorization failed".into()))?;
        let package = handler.construct_for_pinned_profile(&attempt_id, WorkUnitHarnessRole::Handler, harness.profile)
            .map_err(|error| SprintRunnerTransitionError::Unavailable(format!("Handler Harness package construction failed: {error:?}")))?;
        self.mark_handler(work_unit_id, "execution_support_granted_at")?;
        self.mark_handler(work_unit_id, "isolated_worktree_ready_at")?;
        let session = AgentSessionId::new(session_id).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let runtime = package.runtime_launch_configuration();
        self.sessions.create_application_session(CreateApplicationAgentSessionCommand { session_id: session.clone(), session: CreateAgentSessionCommand { title: Some("Work Unit Handler".into()), working_directory: Some(package.working_directory().into()), requested_options: runtime.requested_options.clone() }}).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        self.mark_handler(work_unit_id, "handler_session_created_at")?;
        let invocation = AgentInvocationId::new(invocation_id).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let prompt = format!("Work Unit Handler activation. Work Unit: {work_unit_id}. This original Handler invocation is read-only and has no downstream action. Do not create an attempt, accept output, review, settle, retry, activate dependents, or continue any Sprint or Epic.");
        self.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: prompt.clone(), title: None, working_directory: None, requested_options: Some(runtime.requested_options.clone()) }}).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        self.mark_handler(work_unit_id, "handler_invocation_prepared_at")?;
        package.bind_correlated_invocation(session.clone(), invocation.clone()).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        self.mark_handler(work_unit_id, "handler_harness_bound_at")?;
        match self.sessions.application_invocation_launch_evidence(&invocation, &session).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))? {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => { self.mark_handler(work_unit_id, "launch_accepted_at")?; self.mark_handler(work_unit_id, "handler_ready_at")?; }
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {
                self.mark_handler(work_unit_id, "launch_requested_at")?;
                let launch = self.sessions.launch_prepared_application_invocation_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: prompt, title: None, working_directory: Some(package.working_directory().into()), requested_options: Some(runtime.requested_options) }}, Some(runtime.extension)).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
                if launch.launch_accepted { self.mark_handler(work_unit_id, "launch_accepted_at")?; self.mark_handler(work_unit_id, "handler_ready_at")?; }
            }
            ApplicationInvocationLaunchEvidence::NeverPersisted => return Err(SprintRunnerTransitionError::Conflict),
        }
        if let Ok(observation) = package.observe_correlated_invocation() {
            if let Some(activity) = observation.provider_activity {
                self.mark_handler_at(work_unit_id, "provider_activation_observed_at", activity.recorded_at.to_rfc3339())?;
            }
        }
        self.reconcile_handler_action_continuation(handler, work_unit_id)?;
        Ok(())
    }

    fn reconcile_handler_action_continuation(
        self: &Arc<Self>,
        handler: &Arc<WorkUnitExecutionHarnessService>,
        work_unit_id: &str,
    ) -> Result<(), SprintRunnerTransitionError> {
        let original: Option<(String, String, String, String, String)> = self.connection.lock()
            .map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .query_row(
                "SELECT h.attempt_id,h.handler_session_id,h.handler_invocation_id,h.sprint_id,invocation.status
                 FROM work_unit_handler_activations h
                 JOIN agent_session_invocations invocation
                   ON invocation.id=h.handler_invocation_id
                  AND invocation.session_id=h.handler_session_id
                 WHERE work_unit_id=?1 AND eligibility_state='eligible'
                   AND handler_ready_at IS NOT NULL AND launch_accepted_at IS NOT NULL",
                [work_unit_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((attempt_id, session_id, original_invocation_id, _sprint_id, status)) = original else {
            return Ok(());
        };
        let terminal = matches!(status.as_str(), "completed" | "failed" | "canceled" | "interrupted");
        let action_invocation_id = stable_id("work-unit-handler-action-invocation", &attempt_id);
        let existing: bool = self.connection.lock()
            .map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_action_continuations WHERE work_unit_id=?1)", [work_unit_id], |row| row.get(0))
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if !existing {
            let desired = handler.current_handler_action_revision()
                .map_err(|_| SprintRunnerTransitionError::Unavailable("load immutable Handler action revision".into()))?;
            self.connection.lock()
                .map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
                .execute(
                "INSERT OR IGNORE INTO work_unit_handler_action_continuations
                 (work_unit_id,attempt_id,handler_session_id,original_handler_invocation_id,
                  action_invocation_id,action_harness_revision_id,action_harness_configuration_digest,
                  action_harness_repository_commit_ref,requested_at,blocked_reason)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![work_unit_id, attempt_id, session_id, original_invocation_id,
                    action_invocation_id, desired.revision_id, desired.configuration_digest,
                    desired.repository_commit_ref, chrono::Utc::now().to_rfc3339(),
                    if terminal { None } else { Some("original_handler_invocation_active") }],
                ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        }
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(
            "UPDATE work_unit_handler_action_continuations
             SET blocked_reason=CASE WHEN ?2 THEN NULL ELSE COALESCE(blocked_reason,'original_handler_invocation_active') END
             WHERE work_unit_id=?1",
            params![work_unit_id, terminal],
        ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if !terminal { return Ok(()); }
        let row: (String, String, String, String, String, String) = self.connection.lock()
            .map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .query_row(
                "SELECT attempt_id,handler_session_id,action_invocation_id,action_harness_revision_id,
                        action_harness_configuration_digest,action_harness_repository_commit_ref
                 FROM work_unit_handler_action_continuations WHERE work_unit_id=?1",
                [work_unit_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let (attempt_id, session_id, invocation_id, revision_id, digest, commit) = row;
        let pinned = handler.load_pinned_handler_revision(&revision_id, &digest, &commit)
            .map_err(|_| SprintRunnerTransitionError::Conflict)?;
        if !pinned.profile.mcp.required
            || pinned.profile.mcp.enabled_tools != ["request_work_unit_implementer"] {
            return Err(SprintRunnerTransitionError::Conflict);
        }
        self.mark_handler_action(work_unit_id, "authorized_at")?;
        let package = handler.construct_for_pinned_profile(
            &attempt_id, WorkUnitHarnessRole::Handler, pinned.profile,
        ).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let session = AgentSessionId::new(session_id)
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let invocation = AgentInvocationId::new(invocation_id)
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let mut runtime = package.runtime_launch_configuration();
        let injection = self.prepare_work_unit_handler_action(invocation.clone())?;
        runtime.extension.additional_args.extend(injection.configuration_args);
        runtime.extension.environment.push(injection.environment);
        let prompt = "Handler action continuation. The only exposed action is request_work_unit_implementer. The application derives all identities and does not accept outcomes, review, settlement, retries, dependent activation, or continuation.".to_string();
        self.sessions.prepare_idempotent_application_invocation(
            SendIdempotentApplicationAgentSessionMessageCommand {
                invocation_id: invocation.clone(),
                message: SendAgentSessionMessageCommand {
                    session_id: Some(session.clone()), submitted_text: prompt.clone(), title: None,
                    working_directory: None, requested_options: Some(runtime.requested_options.clone()),
                },
            },
        ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        self.mark_handler_action(work_unit_id, "invocation_prepared_at")?;
        package.bind_correlated_invocation(session.clone(), invocation.clone())
            .map_err(|_| SprintRunnerTransitionError::Conflict)?;
        self.mark_handler_action(work_unit_id, "harness_bound_at")?;
        match self.sessions.application_invocation_launch_evidence(&invocation, &session)
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))? {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => {
                self.mark_handler_action(work_unit_id, "launch_accepted_at")?;
                self.mark_handler_action(work_unit_id, "action_ready_at")?;
            }
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {
                self.mark_handler_action(work_unit_id, "launch_requested_at")?;
                let launch = self.sessions.launch_prepared_application_invocation_with_launch_observation(
                    SendIdempotentApplicationAgentSessionMessageCommand {
                        invocation_id: invocation.clone(),
                        message: SendAgentSessionMessageCommand {
                            session_id: Some(session.clone()), submitted_text: prompt, title: None,
                            working_directory: Some(package.working_directory().into()),
                            requested_options: Some(runtime.requested_options),
                        },
                    }, Some(runtime.extension),
                ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
                if launch.launch_accepted {
                    self.mark_handler_action(work_unit_id, "launch_accepted_at")?;
                    self.mark_handler_action(work_unit_id, "action_ready_at")?;
                }
            }
            ApplicationInvocationLaunchEvidence::NeverPersisted => return Err(SprintRunnerTransitionError::Conflict),
        }
        if let Ok(observation) = package.observe_correlated_invocation() {
            if let Some(activity) = observation.provider_activity {
                self.mark_handler_action_at(work_unit_id, "provider_activation_observed_at", activity.recorded_at.to_rfc3339())?;
            }
        }
        Ok(())
    }

    fn request_work_unit_implementer(self:&Arc<Self>,handler_invocation:&AgentInvocationId)->Result<(),SprintRunnerTransitionError>{
        let handler=self.work_unit_handler.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone().ok_or_else(||SprintRunnerTransitionError::Unavailable("Work Unit Handler activation is unavailable".into()))?;
        let row:Option<(String,String,String,String,String,String,String)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT c.work_unit_id,c.attempt_id,h.sprint_id,c.handler_session_id,c.action_harness_revision_id,c.action_harness_configuration_digest,c.action_harness_repository_commit_ref FROM work_unit_handler_action_continuations c JOIN work_unit_handler_activations h ON h.work_unit_id=c.work_unit_id AND h.attempt_id=c.attempt_id WHERE c.action_invocation_id=?1 AND c.authorized_at IS NOT NULL AND c.harness_bound_at IS NOT NULL AND c.launch_accepted_at IS NOT NULL AND c.action_ready_at IS NOT NULL AND h.eligibility_state='eligible' AND h.handler_ready_at IS NOT NULL",[handler_invocation.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((work_unit,handler_attempt,sprint,handler_session,handler_revision,handler_digest,handler_commit))=row else{return Err(SprintRunnerTransitionError::Forbidden)};
        let action=handler.load_pinned_handler_revision(&handler_revision,&handler_digest,&handler_commit).map_err(|_|SprintRunnerTransitionError::Forbidden)?;
        if !action.profile.mcp.required || action.profile.mcp.enabled_tools != ["request_work_unit_implementer"] { return Err(SprintRunnerTransitionError::Forbidden) }
        let persisted:bool=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM agent_session_invocations WHERE id=?1 AND session_id=?2 AND input_provenance='application')",params![handler_invocation.as_str(),handler_session],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !persisted{return Err(SprintRunnerTransitionError::Forbidden)}
        let authority:String=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT authority_id FROM initiated_sprint_git_authorities WHERE sprint_id=?1 ORDER BY recorded_at,authority_id LIMIT 1",[&sprint],|r|r.get(0)).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?.ok_or(SprintRunnerTransitionError::Forbidden)?;
        let attempt=handler_attempt.clone();let session_id=stable_id("work-unit-implementer-session",&handler_attempt);let invocation_id=stable_id("work-unit-implementer-invocation",&handler_attempt);
        let lock=self.transition_lock(&sprint)?;let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit activation lock is poisoned".into()))?;
        let existing:bool=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_implementer_activations WHERE work_unit_id=?1)",[&work_unit],|row|row.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if !existing { let desired=handler.current_implementer_revision().map_err(|_|SprintRunnerTransitionError::Unavailable("immutable Implementer Harness revision unavailable".into()))?;self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("INSERT INTO work_unit_implementer_activations (work_unit_id,handler_attempt_id,handler_invocation_id,attempt_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref,requested_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![work_unit,handler_attempt,handler_invocation.as_str(),attempt,session_id,invocation_id,desired.revision_id,desired.configuration_digest,desired.repository_commit_ref,chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?; }
        let (attempt,session_id,invocation_id,revision_id,digest,commit):(String,String,String,String,String,String)=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT attempt_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref FROM work_unit_implementer_activations WHERE work_unit_id=?1 AND handler_attempt_id=?2 AND handler_invocation_id=?3",params![work_unit,handler_attempt,handler_invocation.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|_|SprintRunnerTransitionError::Conflict)?;
        let pinned=handler.load_pinned_implementer_revision(&revision_id,&digest,&commit).map_err(|_|SprintRunnerTransitionError::Conflict)?;handler.authorize_implementer_attempt(&attempt,&work_unit,&authority).map_err(|_|SprintRunnerTransitionError::Conflict)?;self.mark_implementer(&work_unit,"authorized_at")?;
        let package=handler.construct_for_pinned_profile(&attempt,WorkUnitHarnessRole::Implementer,pinned.profile).map_err(|_|SprintRunnerTransitionError::Unavailable("Implementer Harness package construction failed".into()))?;self.mark_implementer(&work_unit,"execution_support_granted_at")?;self.mark_implementer(&work_unit,"isolated_worktree_ready_at")?;let session=AgentSessionId::new(session_id).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let invocation=AgentInvocationId::new(invocation_id).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let runtime=package.runtime_launch_configuration();
        self.sessions.create_application_session(CreateApplicationAgentSessionCommand{session_id:session.clone(),session:CreateAgentSessionCommand{title:Some("Work Unit Implementer".into()),working_directory:Some(package.working_directory().into()),requested_options:runtime.requested_options.clone()}}).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;self.mark_implementer(&work_unit,"implementer_session_created_at")?;let prompt="Work Unit Implementer activation. Work only in the application-provided isolated execution workspace. Do not submit outcomes, accept, review, settle, retry, activate dependents, or continue any Sprint or Epic.".to_string();
        self.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:invocation.clone(),message:SendAgentSessionMessageCommand{session_id:Some(session.clone()),submitted_text:prompt.clone(),title:None,working_directory:None,requested_options:Some(runtime.requested_options.clone())}}).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;self.mark_implementer(&work_unit,"implementer_invocation_prepared_at")?;package.bind_correlated_invocation(session.clone(),invocation.clone()).map_err(|_|SprintRunnerTransitionError::Conflict)?;self.mark_implementer(&work_unit,"implementer_harness_bound_at")?;
        match self.sessions.application_invocation_launch_evidence(&invocation,&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?{ApplicationInvocationLaunchEvidence::LaunchAccepted=>{self.mark_implementer(&work_unit,"launch_accepted_at")?;self.mark_implementer(&work_unit,"implementer_ready_at")?},ApplicationInvocationLaunchEvidence::PersistedNotAccepted=>{self.mark_implementer(&work_unit,"launch_requested_at")?;let launch=self.sessions.launch_prepared_application_invocation_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:invocation.clone(),message:SendAgentSessionMessageCommand{session_id:Some(session.clone()),submitted_text:prompt,title:None,working_directory:Some(package.working_directory().into()),requested_options:Some(runtime.requested_options)}},Some(runtime.extension)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if launch.launch_accepted{self.mark_implementer(&work_unit,"launch_accepted_at")?;self.mark_implementer(&work_unit,"implementer_ready_at")?}},ApplicationInvocationLaunchEvidence::NeverPersisted=>return Err(SprintRunnerTransitionError::Conflict)};
        if let Ok(observation)=package.observe_correlated_invocation(){if let Some(activity)=observation.provider_activity{self.mark_implementer_at(&work_unit,"provider_activation_observed_at",activity.recorded_at.to_rfc3339())?}};Ok(())
    }
    fn mark_implementer(&self,work_unit:&str,column:&str)->Result<(),SprintRunnerTransitionError>{self.mark_implementer_at(work_unit,column,chrono::Utc::now().to_rfc3339())}
    fn mark_implementer_at(&self,work_unit:&str,column:&str,at:String)->Result<(),SprintRunnerTransitionError>{if !["authorized_at","execution_support_granted_at","isolated_worktree_ready_at","implementer_session_created_at","implementer_invocation_prepared_at","implementer_harness_bound_at","launch_requested_at","launch_accepted_at","provider_activation_observed_at","implementer_ready_at"].contains(&column){return Err(SprintRunnerTransitionError::Unavailable("invalid Implementer activation stage".into()))}self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(&format!("UPDATE work_unit_implementer_activations SET {column}=COALESCE({column},?2) WHERE work_unit_id=?1"),params![work_unit,at]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;Ok(())}

    fn mark_handler_action(&self, work_unit: &str, column: &str) -> Result<(), SprintRunnerTransitionError> {
        self.mark_handler_action_at(work_unit, column, chrono::Utc::now().to_rfc3339())
    }

    fn mark_handler_action_at(&self, work_unit: &str, column: &str, at: String) -> Result<(), SprintRunnerTransitionError> {
        if !["authorized_at", "invocation_prepared_at", "harness_bound_at", "launch_requested_at", "launch_accepted_at", "provider_activation_observed_at", "action_ready_at"].contains(&column) {
            return Err(SprintRunnerTransitionError::Unavailable("invalid Handler action continuation stage".into()));
        }
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .execute(&format!("UPDATE work_unit_handler_action_continuations SET {column}=COALESCE({column},?2) WHERE work_unit_id=?1"), params![work_unit, at])
            .map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        Ok(())
    }

    fn mark_handler(&self, work_unit_id: &str, column: &str) -> Result<(), SprintRunnerTransitionError> {
        self.mark_handler_at(work_unit_id, column, chrono::Utc::now().to_rfc3339())
    }

    fn mark_handler_at(&self, work_unit_id: &str, column: &str, recorded_at: String) -> Result<(), SprintRunnerTransitionError> {
        if !["authorized_at","attempt_created_at","execution_support_granted_at","isolated_worktree_ready_at","handler_session_created_at","handler_invocation_prepared_at","handler_harness_bound_at","launch_requested_at","launch_accepted_at","provider_activation_observed_at","handler_ready_at"].contains(&column) { return Err(SprintRunnerTransitionError::Unavailable("invalid Handler activation stage".into())); }
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(&format!("UPDATE work_unit_handler_activations SET {column}=COALESCE({column},?2) WHERE work_unit_id=?1"), params![work_unit_id,recorded_at]).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        Ok(())
    }

    fn planner_point_for_invocation(&self, invocation_id: &AgentInvocationId) -> Result<(String,String), SprintRunnerTransitionError> {
        let desired = conversation_harness::profile(ConversationHarnessRole::WorkSlicePlanner)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT planning.planning_point_id,planning.sprint_id
             FROM work_slice_planning_requests planning
             JOIN agent_session_invocations invocation
               ON invocation.id=planning.planner_invocation_id
              AND invocation.session_id=planning.planner_session_id
              AND invocation.input_provenance='application'
             WHERE planning.planner_invocation_id=?1
               AND planning.is_current=1
               AND planning.planner_harness_key=?2
               AND planning.planner_harness_version=?3
               AND planning.planner_harness_applied_at IS NOT NULL
               AND planning.planner_launch_accepted_at IS NOT NULL
               AND planning.planner_ready_at IS NOT NULL",
            params![invocation_id.as_str(), desired.key, desired.version],
            |r| Ok((r.get(0)?,r.get(1)?))
        ).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?.ok_or(SprintRunnerTransitionError::Forbidden)
    }
    pub(crate) fn read_work_slice_planning_context(&self, invocation_id:&AgentInvocationId)->Result<serde_json::Value,SprintRunnerTransitionError>{
        let (point,_)=self.planner_point_for_invocation(invocation_id)?;
        self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT episode.sprint_id,episode.repository_worktree_route,revision.revision_id,revision.validation_result,revision.refinement_requested_at FROM work_slice_planning_episodes episode LEFT JOIN work_slice_proposal_revisions revision ON revision.planning_point_id=episode.planning_point_id AND revision.is_current=1 WHERE episode.planning_point_id=?1",[&point],|r|Ok(serde_json::json!({"planningPoint":point,"sprint":r.get::<_,String>(0)?,"repositoryRoute":r.get::<_,String>(1)?,"hasCurrentRevision":r.get::<_,Option<String>>(2)?.is_some(),"validation":r.get::<_,Option<String>>(3)?,"refinementRequested":r.get::<_,Option<String>>(4)?.is_some()})))
        .map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))
    }
    pub(crate) fn submit_work_slice_proposal(self:&Arc<Self>, invocation_id:&AgentInvocationId, input:WorkSliceProposal)->Result<serde_json::Value,SprintRunnerTransitionError>{
        validate_work_slice_proposal(&input)?;
        let (point,sprint)=self.planner_point_for_invocation(invocation_id)?;let lock=self.transition_lock(&sprint)?;let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning lock is poisoned".into()))?;
        let encoded=serde_json::to_string(&input).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let fingerprint=stable_id("work-slice-proposal-content",&encoded);
        let conn=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        let parent:Option<(String,i64)>=conn.query_row("SELECT revision_id,revision_number FROM work_slice_proposal_revisions WHERE planning_point_id=?1 AND is_current=1",[&point],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if let Some((revision,_))=parent.as_ref(){let state: (String,Option<String>)=conn.query_row("SELECT content_fingerprint,refinement_requested_at FROM work_slice_proposal_revisions WHERE revision_id=?1",[revision],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if state.1.is_none(){return if state.0==fingerprint{Ok(serde_json::json!({"status":"proposal_replayed","accepted":false,"materializationReady":false}))}else{Err(SprintRunnerTransitionError::Conflict)}}}
        if let Some((prior,_))=parent.as_ref(){conn.execute("UPDATE work_slice_proposal_revisions SET is_current=0 WHERE revision_id=?1",[prior]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;}
        let number=parent.as_ref().map_or(1,|(_,n)|n+1);let command=stable_id("work-slice-proposal-command",&format!("{point}:{:?}:{fingerprint}",parent.as_ref().map(|p|&p.0)));let revision=stable_id("work-slice-proposal-revision",&format!("{point}:{number}:{command}"));let now=chrono::Utc::now().to_rfc3339();
        conn.execute("INSERT INTO work_slice_proposal_revisions (revision_id,planning_point_id,revision_number,parent_revision_id,is_current,idempotency_key,content_fingerprint,proposal_json,submitted_at,validation_at,validation_result) VALUES (?1,?2,?3,?4,1,?5,?6,?7,?8,?8,'valid')",params![revision,point,number,parent.map(|v|v.0),command,fingerprint,encoded,now]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        Ok(serde_json::json!({"status":"proposal_validated","accepted":false,"materializationReady":false}))
    }
    pub(crate) fn request_work_slice_refinement(&self,invocation_id:&AgentInvocationId,input:WorkSliceRefinement)->Result<(),SprintRunnerTransitionError>{validate_outcome(&input.reason)?;let(point,sprint)=self.planner_point_for_invocation(invocation_id)?;let lock=self.transition_lock(&sprint)?;let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning lock is poisoned".into()))?;let conn=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;let existing:Option<(Option<String>,Option<String>)>=conn.query_row("SELECT refinement_requested_at,refinement_reason FROM work_slice_proposal_revisions WHERE planning_point_id=?1 AND is_current=1 AND semantic_completed_at IS NULL",[&point],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let Some((requested_at,reason))=existing else{return Err(SprintRunnerTransitionError::Forbidden)};if let Some(reason)=reason {return if reason==input.reason{Ok(())}else{Err(SprintRunnerTransitionError::Conflict)}};if requested_at.is_some(){return Err(SprintRunnerTransitionError::Conflict)};conn.execute("UPDATE work_slice_proposal_revisions SET refinement_requested_at=?2,refinement_reason=?3 WHERE planning_point_id=?1 AND is_current=1 AND semantic_completed_at IS NULL AND refinement_requested_at IS NULL",params![point,chrono::Utc::now().to_rfc3339(),input.reason]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;Ok(())}
    pub(crate) fn complete_work_slice_planning(self:&Arc<Self>,invocation_id:&AgentInvocationId,_input:WorkSliceCompletion)->Result<(),SprintRunnerTransitionError>{let(point,sprint)=self.planner_point_for_invocation(invocation_id)?;let lock=self.transition_lock(&sprint)?;let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning lock is poisoned".into()))?;let changed=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_slice_proposal_revisions SET semantic_completed_at=COALESCE(semantic_completed_at,?3),semantic_completion_invocation_id=COALESCE(semantic_completion_invocation_id,?2) WHERE planning_point_id=?1 AND is_current=1 AND validation_result='valid' AND refinement_requested_at IS NULL AND (semantic_completion_invocation_id IS NULL OR semantic_completion_invocation_id=?2)",params![point,invocation_id.as_str(),chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed==1{Ok(())}else{Err(SprintRunnerTransitionError::Forbidden)}}

    fn reconcile_work_slice_planner(self: &Arc<Self>, sprint_id: &str) -> Result<(), SprintRunnerTransitionError> {
        let lock = self.transition_lock(sprint_id)?;
        let _guard = lock.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition lock is poisoned".into()))?;
        let request: Option<(String,String,String,String,String,i64,Option<String>)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.query_row(
            "SELECT planning_point_id,planner_session_id,planner_invocation_id,repository_worktree_route,planner_harness_key,planner_harness_version,planner_harness_json FROM work_slice_planning_requests WHERE sprint_id=?1 AND is_current=1", [sprint_id],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((point,session_id,invocation_id,route,key,version,harness_json)) = request else { return Ok(()) };
        let version = u16::try_from(version).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let (harness, serialized) = match harness_json {
            Some(snapshot) => {
                let harness = conversation_harness::pinned_profile_snapshot(&key, version, &snapshot)
                    .map_err(|_| SprintRunnerTransitionError::Conflict)?;
                (harness, snapshot)
            }
            None => {
                let harness = conversation_harness::pinned_profile(&key, version)
                    .map_err(SprintRunnerTransitionError::Unavailable)?;
                let snapshot = serde_json::to_string(&harness)
                    .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                (harness, snapshot)
            }
        };
        let session = AgentSessionId::new(session_id).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(
            "INSERT OR IGNORE INTO work_slice_planning_episodes (planning_point_id,sprint_id,authority_id,planner_session_id,planner_invocation_id,harness_json,repository_worktree_route,created_at) SELECT planning_point_id,sprint_id,authority_id,planner_session_id,planner_invocation_id,?2,repository_worktree_route,?3 FROM work_slice_planning_requests WHERE planning_point_id=?1",
            params![point,serialized,chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        self.sessions.create_application_session(CreateApplicationAgentSessionCommand { session_id: session.clone(), session: CreateAgentSessionCommand { title: Some("Work Slice Planner".into()), working_directory: Some(route.clone()), requested_options: harness.runtime_options() }}).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        self.mark_planner(sprint_id, "planner_session_created_at", None)?;
        let invocation = AgentInvocationId::new(invocation_id).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        self.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: format!("Work Slice Planner launch boundary. Planning point: {point}. Parent Sprint: {sprint_id}. Submit only proposal-local lanes through the supplied actions and complete only the application-derived current validated revision. Do not accept a proposal, create Work Units, Handler or Implementer Sessions, settle the Sprint, or advance to a later planning point."), title: None, working_directory: None, requested_options: Some(harness.runtime_options()) }}).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        self.mark_planner(sprint_id, "planner_invocation_created_at", Some(serialized))?;
        self.mark_planner(sprint_id, "planner_harness_applied_at", None)?;
        self.reconcile_productive_work_slice_planner(sprint_id, &point, session, invocation, route, harness)
    }

    fn reconcile_productive_work_slice_planner(
        self: &Arc<Self>,
        sprint_id: &str,
        point: &str,
        session: AgentSessionId,
        invocation: AgentInvocationId,
        route: String,
        harness: conversation_harness::ConversationHarnessProfile,
    ) -> Result<(), SprintRunnerTransitionError> {
        if !harness.mcp.required || harness.mcp.enabled_tools != ["read_current_planning_context", "submit_work_slice_proposal", "request_work_slice_refinement", "complete_work_slice_planning"] {
            return Err(SprintRunnerTransitionError::Conflict);
        }
        match self
            .sessions
            .application_invocation_launch_evidence(&invocation, &session)
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?
        {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => {
                self.mark_planner(sprint_id, "planner_launch_accepted_at", None)?;
                self.mark_planner(sprint_id, "planner_ready_at", None)
            }
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {
                self.mark_planner(sprint_id, "planner_launch_requested_at", None)?;
                let injection=self.prepare_work_slice_planner_action(invocation.clone())?;
                let mut args=harness.runtime_configuration_args();args.extend(injection.configuration_args);
                let launch = self
                    .sessions
                    .launch_prepared_application_invocation_with_launch_observation(
                        SendIdempotentApplicationAgentSessionMessageCommand {
                            invocation_id: invocation,
                            message: SendAgentSessionMessageCommand {
                                session_id: Some(session),
                                submitted_text: format!("Work Slice Planner launch boundary. Planning point: {point}. Parent Sprint: {sprint_id}. Submit only proposal-local lanes through the supplied actions and complete only the application-derived current validated revision. Do not accept a proposal, create Work Units, Handler or Implementer Sessions, settle the Sprint, or advance to a later planning point."),
                                title: None,
                                working_directory: Some(route),
                                requested_options: Some(harness.runtime_options()),
                            },
                        },
                        Some(RuntimeLaunchExtension {
                            additional_args: args,
                            environment: vec![injection.environment],
                            initial_prompt_prefix: Some(harness.initial_prompt_prefix()),
                        }),
                    )
                    .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if launch.launch_accepted {
                    self.mark_planner(sprint_id, "planner_launch_accepted_at", None)?;
                    self.mark_planner(sprint_id, "planner_ready_at", None)?;
                }
                Ok(())
            }
            ApplicationInvocationLaunchEvidence::NeverPersisted => {
                Err(SprintRunnerTransitionError::Conflict)
            }
        }
    }

    fn mark_planner(&self, sprint_id: &str, column: &str, harness_json: Option<String>) -> Result<(), SprintRunnerTransitionError> {
        if !["planner_session_created_at", "planner_invocation_created_at", "planner_harness_applied_at", "planner_launch_requested_at", "planner_launch_accepted_at", "planner_ready_at"].contains(&column) { return Err(SprintRunnerTransitionError::Unavailable("invalid Planner materialization stage".into())); }
        let sql = if harness_json.is_some() { format!("UPDATE work_slice_planning_requests SET {column}=COALESCE({column},?2),planner_harness_json=COALESCE(planner_harness_json,?3) WHERE sprint_id=?1 AND is_current=1") } else { format!("UPDATE work_slice_planning_requests SET {column}=COALESCE({column},?2) WHERE sprint_id=?1 AND is_current=1") };
        let conn = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?;
        if let Some(harness_json) = harness_json { conn.execute(&sql, params![sprint_id, chrono::Utc::now().to_rfc3339(), harness_json]) } else { conn.execute(&sql, params![sprint_id, chrono::Utc::now().to_rfc3339()]) }.map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        Ok(())
    }

    fn mark(&self, sprint_id: &str, column: &str) -> Result<(), SprintRunnerTransitionError> {
        if ![
            "session_created_at",
            "harness_applied_at",
            "launch_accepted_at",
            "pre_start_lifecycle_observed_at",
            "parent_continuation_delivery_persisted_at",
            "epic_continuation_harness_applied_at",
            "epic_continuation_launch_accepted_at",
            "sprint_continuation_harness_applied_at",
            "sprint_continuation_launch_accepted_at",
            "planning_control_delivery_persisted_at",
            "planning_control_harness_applied_at",
            "planning_control_launch_accepted_at",
            "planning_ready_at",
            "pre_start_upgrade_harness_applied_at",
            "pre_start_upgrade_launch_accepted_at",
        ]
        .contains(&column)
        {
            return Err(SprintRunnerTransitionError::Unavailable(
                "invalid Sprint Runner transition stage".into(),
            ));
        }
        self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner transition database lock is poisoned".into()))?.execute(&format!("UPDATE sprint_runner_transitions SET {column}=COALESCE({column},?2) WHERE sprint_id=?1"),params![sprint_id,chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        Ok(())
    }

    fn transition_lock(
        &self,
        sprint_id: &str,
    ) -> Result<Arc<Mutex<()>>, SprintRunnerTransitionError> {
        let mut locks = self.transition_locks.lock().map_err(|_| {
            SprintRunnerTransitionError::Unavailable(
                "Sprint Runner transition lock registry is poisoned".into(),
            )
        })?;
        Ok(locks
            .entry(sprint_id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone())
    }

    #[cfg(test)]
    pub(crate) fn set_test_reconcile_snapshot_hook(&self, hook: Arc<dyn Fn() + Send + Sync>) {
        *self.test_reconcile_snapshot_hook.lock().expect("test reconciliation hook") = Some(hook);
    }

    #[cfg(test)]
    pub(crate) fn set_test_origin_snapshot_hook(&self, hook: Arc<dyn Fn() + Send + Sync>) {
        *self.test_origin_snapshot_hook.lock().expect("test origin hook") = Some(hook);
    }

    #[cfg(test)]
    fn run_test_hook(&self, origin: bool) {
        let hook = if origin {
            self.test_origin_snapshot_hook.lock().expect("test origin hook").clone()
        } else {
            self.test_reconcile_snapshot_hook.lock().expect("test reconciliation hook").clone()
        };
        if let Some(hook) = hook { hook(); }
    }
}

#[derive(Default)]
struct ReconcileDrain { running: bool, generation: u64 }

struct AuthorizedRunner {
    epic_id: String,
    session_id: String,
    harness_version: i64,
}
struct SprintRecord {
    sprint_id: String,
    epic_id: String,
    session_id: String,
    invocation_id: String,
    harness_key: String,
    harness_version: u16,
    session_created_at: Option<String>,
    harness_applied_at: Option<String>,
    launch_accepted_at: Option<String>,
}
fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4000
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn validate_outcome(value: &str) -> Result<(), SprintRunnerTransitionError> {
    if value.trim().is_empty() || value.len() > 20_000 {
        Err(SprintRunnerTransitionError::Invalid)
    } else { Ok(()) }
}
fn validate_work_slice_proposal(value:&WorkSliceProposal)->Result<(),SprintRunnerTransitionError>{
    if value.lanes.is_empty() || value.lanes.len()>32 {return Err(SprintRunnerTransitionError::Invalid)}
    validate_outcome(&value.objective)?;let mut names=std::collections::HashSet::new();
    for lane in &value.lanes {validate_outcome(&lane.title)?;validate_outcome(&lane.specification)?;if !names.insert(lane.title.as_str()){return Err(SprintRunnerTransitionError::Invalid)}}
    for lane in &value.lanes {for dependency in &lane.depends_on {if dependency==&lane.title || !names.contains(dependency.as_str()){return Err(SprintRunnerTransitionError::Invalid)}}}
    fn visit<'a>(name:&'a str,lanes:&'a [WorkSliceLane],active:&mut std::collections::HashSet<&'a str>,seen:&mut std::collections::HashSet<&'a str>)->bool{if !active.insert(name){return false}if seen.insert(name){let lane=lanes.iter().find(|lane|lane.title==name).expect("validated lane");for dep in &lane.depends_on{if !visit(dep,lanes,active,seen){return false}}}active.remove(name);true}
    let mut active=std::collections::HashSet::new();let mut seen=std::collections::HashSet::new();for lane in &value.lanes{if !visit(&lane.title,&value.lanes,&mut active,&mut seen){return Err(SprintRunnerTransitionError::Invalid)}}Ok(())
}
fn lifecycle_status(status: AgentInvocationStatus) -> &'static str {
    match status {
        AgentInvocationStatus::Pending => "pending",
        AgentInvocationStatus::Running => "running",
        AgentInvocationStatus::Completed => "completed",
        AgentInvocationStatus::Failed => "failed",
        AgentInvocationStatus::Canceled => "canceled",
        AgentInvocationStatus::Interrupted => "interrupted",
    }
}
fn stable_id(prefix: &str, value: &str) -> String {
    let mut h = sha2::Sha256::new();
    use sha2::Digest;
    h.update(prefix.as_bytes());
    h.update([0]);
    h.update(value.as_bytes());
    format!("{prefix}-{:x}", h.finalize())
}

fn handler_activation_blocked_reason(
    dependency_count: i64,
    has_initiated_sprint_git_authority: bool,
) -> Option<&'static str> {
    if dependency_count > 0 {
        // This Plan Step creates no prerequisite-satisfaction fact. Handler readiness, provider
        // activity, lifecycle, transcript content, and silence are therefore never sufficient.
        Some("prerequisite_satisfaction_not_authoritative")
    } else if !has_initiated_sprint_git_authority {
        Some("initiated_sprint_git_authority_missing")
    } else {
        None
    }
}

#[cfg(test)]
mod handler_activation_boundary_tests {
    use super::handler_activation_blocked_reason;

    #[test]
    fn prerequisite_handler_readiness_cannot_satisfy_a_plan_dependency() {
        // The caller intentionally supplies only the authoritative plan edge count. A separate
        // Handler-ready fact cannot alter this result at the activation boundary.
        assert_eq!(
            handler_activation_blocked_reason(1, true),
            Some("prerequisite_satisfaction_not_authoritative")
        );
    }

    #[test]
    fn root_requires_the_initiated_sprint_git_authority() {
        assert_eq!(
            handler_activation_blocked_reason(0, false),
            Some("initiated_sprint_git_authority_missing")
        );
        assert_eq!(handler_activation_blocked_reason(0, true), None);
    }
}

struct EpicRunnerActionMcp {
    service: Arc<SprintRunnerTransitionService>,
    invocation_id: AgentInvocationId,
    tool_router: ToolRouter<Self>,
}
impl EpicRunnerActionMcp {
    fn new(service: Arc<SprintRunnerTransitionService>, invocation_id: AgentInvocationId) -> Self {
        Self {
            service,
            invocation_id,
            tool_router: Self::tool_router(),
        }
    }
    fn error(code: &str, message: impl Into<String>) -> CallToolResult {
        CallToolResult::success(vec![ContentBlock::text(
            serde_json::json!({"status":"rejected","code":code,"message":message.into()})
                .to_string(),
        )])
    }
}
#[tool_router]
impl EpicRunnerActionMcp {
    #[tool(
        description = "Request exactly one approved Sprint Runner for this Epic Runner invocation. Input is ONLY {sprintId: string}. The application derives Epic, originating session and invocation, applied Harness binding, correlation, and launch authority. A request never proves provider observation or Sprint acceptance."
    )]
    fn request_next_sprint_runner(
        &self,
        Parameters(input): Parameters<SprintRunnerSelection>,
    ) -> CallToolResult {
        match self.service.request_next_sprint_runner(&self.invocation_id, input) {
            Ok(status) => CallToolResult::success(vec![ContentBlock::text(serde_json::json!({"status":if status.launch_accepted_at.is_some(){"launch_accepted_pre_start_ready"}else{"authorized_not_launch_accepted"},"sprintRunnerRequestId":status.request_id,"sprintRunnerSessionId":status.sprint_runner_session_id,"sprintRunnerInvocationId":status.sprint_runner_invocation_id,"preStartReady":status.pre_start_ready,"lifecycleObserved":false,"accepted":false,"guidance":"Do not create Work Slice planning or Work Units. Launch acceptance is not lifecycle observation or Sprint acceptance."}).to_string())]),
            Err(SprintRunnerTransitionError::Forbidden) => Self::error("forbidden", "This invocation is not the registered launch-accepted Epic Runner."),
            Err(SprintRunnerTransitionError::Invalid) => Self::error("invalid_selection", "The selected Sprint is not approved for this Epic."),
            Err(SprintRunnerTransitionError::Conflict) => Self::error("idempotency_conflict", "This Sprint already has a different durable Sprint Runner route."),
            Err(SprintRunnerTransitionError::Unavailable(_)) => Self::error("internal_error", "The application could not record the Sprint Runner request."),
        }
    }
}
#[tool_handler(router = self.tool_router)]
impl ServerHandler for EpicRunnerActionMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Select one approved Sprint only through request_next_sprint_runner. The application owns all routing and launch authority. Stop after the returned pre-start state.")
    }
}

struct SprintPreStartMcp { service: Arc<SprintRunnerTransitionService>, invocation_id: AgentInvocationId, tool_router: ToolRouter<Self> }
impl SprintPreStartMcp { fn new(service: Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} }
#[tool_router] impl SprintPreStartMcp {
    #[tool(description="Record the one pre-start outcome. Input is ONLY {forecastAndConcerns, materialUncertainty, applicationOwnedPrerequisite}; the application derives all identities and accepts it only after the matching terminal lifecycle.")]
    fn report_pre_start_outcome(&self, Parameters(input):Parameters<PreStartOutcome>)->CallToolResult { match self.service.record_pre_start_outcome(&self.invocation_id,input){Ok(())=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"pre_start_semantic_outcome_recorded\",\"accepted\":false}")]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"invalid_or_unavailable\"}")])} }
}
#[tool_handler(router=self.tool_router)] impl ServerHandler for SprintPreStartMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Report one pre-start outcome only. Completion alone is not acceptance.")} }

struct EpicStartMcp { service: Arc<SprintRunnerTransitionService>, invocation_id: AgentInvocationId, tool_router: ToolRouter<Self> }
impl EpicStartMcp { fn new(service: Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} }
#[tool_router] impl EpicStartMcp {
    #[tool(description="Semantically authorize the correlated selected Sprint start. Input is ONLY {}. The application derives Epic, Sprint, authority, and all routing identities.")]
    fn start_selected_sprint(&self)->CallToolResult { match self.service.start_selected_sprint(&self.invocation_id){Ok(())=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"sprint_start_authorized\"}")]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"unavailable\"}")])} }
}
#[tool_handler(router=self.tool_router)] impl ServerHandler for EpicStartMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Only start_selected_sprint can authorize start; delivery and lifecycle do not.")} }

struct SprintStartedMcp { service: Arc<SprintRunnerTransitionService>, invocation_id: AgentInvocationId, tool_router: ToolRouter<Self> }
impl SprintStartedMcp { fn new(service: Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} }
#[tool_router] impl SprintStartedMcp {
    #[tool(description="Record repository and branch reevaluation after the application-authorized start. Input is ONLY {repositoryBranchEvaluation, startedForecastAndConcerns}. It may make the Sprint planning-ready but cannot create planning or Work Units.")]
    fn record_started_reevaluation(&self, Parameters(input):Parameters<StartedReevaluation>)->CallToolResult { match self.service.record_started_reevaluation(&self.invocation_id,input){Ok(())=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"planning_ready\",\"downstreamNotStarted\":true}")]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"invalid_or_unavailable\"}")])} }
}
#[tool_handler(router=self.tool_router)] impl ServerHandler for SprintStartedMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Record only durable repository/branch reevaluation. Do not create downstream work.")} }

struct SprintPlanningControlMcp { service: Arc<SprintRunnerTransitionService>, invocation_id: AgentInvocationId, tool_router: ToolRouter<Self> }
impl SprintPlanningControlMcp { fn new(service: Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} }
#[tool_router] impl SprintPlanningControlMcp {
    #[tool(description="Request exactly one Work Slice Planner for the current temporal planning decision. Input is ONLY {}. The application derives Sprint, planning point, parent Session/invocation, repository route, child identities, and launch authority.")]
    fn request_work_slice_planner(&self, Parameters(input):Parameters<WorkSlicePlannerRequest>)->CallToolResult { match self.service.request_work_slice_planner(&self.invocation_id,input){Ok(status)=>CallToolResult::success(vec![ContentBlock::text(serde_json::json!({"status":if status.work_slice_planner_ready_at.is_some(){"work_slice_planner_ready"}else if status.work_slice_planner_launch_accepted_at.is_some(){"work_slice_planner_launch_accepted"}else{"work_slice_planner_authorized"},"planningPointId":status.work_slice_planning_point_id,"workSlicePlannerSessionId":status.work_slice_planner_session_id,"workSlicePlannerInvocationId":status.work_slice_planner_invocation_id,"plannerReady":status.work_slice_planner_ready_at.is_some(),"providerActivationObserved":status.work_slice_planner_provider_activation_observed_at.is_some(),"lifecycleObserved":status.work_slice_planner_lifecycle_observed_at.is_some(),"guidance":"Planner readiness follows durable prelaunch facts and runtime launch acceptance only; it is not provider activation, lifecycle, a Planner result, Work Unit creation, or downstream acceptance."}).to_string())]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(SprintRunnerTransitionError::Conflict)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"idempotency_conflict\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"unavailable\"}")] )} }
}
#[tool_handler(router=self.tool_router)] impl ServerHandler for SprintPlanningControlMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Use only request_work_slice_planner. Do not create or accept Work Units, Handlers, Implementers, or Planner results.")} }

struct WorkSlicePlannerMcp { service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId,tool_router:ToolRouter<Self> }
impl WorkSlicePlannerMcp {fn new(service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}}fn rejected(error:SprintRunnerTransitionError)->CallToolResult{let code=match error{SprintRunnerTransitionError::Forbidden=>"forbidden",SprintRunnerTransitionError::Invalid=>"invalid",SprintRunnerTransitionError::Conflict=>"conflict",SprintRunnerTransitionError::Unavailable(_)=>"unavailable"};CallToolResult::success(vec![ContentBlock::text(serde_json::json!({"status":"rejected","code":code}).to_string())])}}
#[tool_router] impl WorkSlicePlannerMcp {
 #[tool(description="Read the exact application-bound planning context. Input is ONLY {}.")]
 fn read_current_planning_context(&self)->CallToolResult{match self.service.read_work_slice_planning_context(&self.invocation_id){Ok(context)=>CallToolResult::success(vec![ContentBlock::text(context.to_string())]),Err(error)=>Self::rejected(error)}}
 #[tool(description="Submit a bounded proposal. Input has only objective and proposal-local lanes {title,specification,dependsOn}; no identities, tokens, routes, Work Unit IDs, idempotency key, or acceptance.")]
 fn submit_work_slice_proposal(&self,Parameters(input):Parameters<WorkSliceProposal>)->CallToolResult{match self.service.submit_work_slice_proposal(&self.invocation_id,input){Ok(result)=>CallToolResult::success(vec![ContentBlock::text(result.to_string())]),Err(error)=>Self::rejected(error)}}
 #[tool(description="Request bounded refinement of the current uncompleted revision using only {reason}.")]
 fn request_work_slice_refinement(&self,Parameters(input):Parameters<WorkSliceRefinement>)->CallToolResult{match self.service.request_work_slice_refinement(&self.invocation_id,input){Ok(())=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"refinement_recorded\"}")]),Err(error)=>Self::rejected(error)}}
 #[tool(description="Complete planning only for the application-derived current validated revision. Input is ONLY {}. The application alone later observes lifecycle and accepts it.")]
 fn complete_work_slice_planning(&self,Parameters(input):Parameters<WorkSliceCompletion>)->CallToolResult{match self.service.complete_work_slice_planning(&self.invocation_id,input){Ok(())=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"semantic_completion_recorded\",\"accepted\":false}")]),Err(error)=>Self::rejected(error)}}
}
#[tool_handler(router=self.tool_router)] impl ServerHandler for WorkSlicePlannerMcp{fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Use only the listed application-owned planning actions. No action accepts a revision, creates Work Units, or launches downstream work.")}}

struct WorkUnitHandlerMcp { service: Arc<SprintRunnerTransitionService>, invocation_id: AgentInvocationId, tool_router: ToolRouter<Self> }
impl WorkUnitHandlerMcp { fn new(service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} }
#[tool_router] impl WorkUnitHandlerMcp { #[tool(description="Request the one Implementer for this exact ready Handler invocation. Input is ONLY {}. The application derives and validates attempt, worktree, Harness revision, Session, invocation, and launch authority.")] fn request_work_unit_implementer(&self)->CallToolResult { match self.service.request_work_unit_implementer(&self.invocation_id){Ok(())=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"implementer_request_recorded\",\"accepted\":false}")]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(SprintRunnerTransitionError::Conflict)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"conflict\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"unavailable\"}")])} } }
#[tool_handler(router=self.tool_router)] impl ServerHandler for WorkUnitHandlerMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Use only request_work_unit_implementer. It cannot submit outcomes or perform review, settlement, retry, dependent activation, or continuation.")} }

struct ManagedEpicRunnerAction {
    server: ManagedEpicRunnerActionServer,
    injection: CodexMcpInjection,
}
pub(crate) struct ManagedEpicRunnerActionServer {
    address: SocketAddr,
    cancellation: CancellationToken,
    join: Option<thread::JoinHandle<()>>,
}
impl ManagedEpicRunnerActionServer {
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
macro_rules! start_scoped_server {
    ($name:ident,$adapter:ty) => { fn $name(service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId,bearer:String,origins:Vec<String>)->io::Result<ManagedEpicRunnerActionServer>{
        let listener=std::net::TcpListener::bind("127.0.0.1:0")?;listener.set_nonblocking(true)?;let address=listener.local_addr()?;let cancellation=CancellationToken::new();let server_cancel=cancellation.clone();let join=thread::spawn(move||{let runtime=tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().expect("scoped MCP runtime");runtime.block_on(async move{let config=rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default().with_allowed_hosts([format!("127.0.0.1:{}",address.port())]).with_allowed_origins(origins.clone()).with_cancellation_token(server_cancel.clone());let adapter:rmcp::transport::streamable_http_server::StreamableHttpService<$adapter,rmcp::transport::streamable_http_server::session::local::LocalSessionManager>=rmcp::transport::streamable_http_server::StreamableHttpService::new(move||Ok(<$adapter>::new(service.clone(),invocation_id.clone())),Default::default(),config);let expected=Arc::new(bearer);let allowed_host=format!("127.0.0.1:{}",address.port());let allowed_origins=Arc::new(origins);let listener=tokio::net::TcpListener::from_std(listener).expect("async scoped MCP listener");loop{let accepted=tokio::select!{_=server_cancel.cancelled()=>break,accepted=listener.accept()=>accepted};let Ok((stream,_))=accepted else{continue};let adapter=adapter.clone();let expected=expected.clone();let allowed_host=allowed_host.clone();let allowed_origins=allowed_origins.clone();tokio::spawn(async move{let guard=service_fn(move|request|{let adapter=adapter.clone();let expected=expected.clone();let allowed_host=allowed_host.clone();let allowed_origins=allowed_origins.clone();async move{if let Some(status)=super::mcp::transport_denial(&expected,&allowed_host,&allowed_origins,&request){return Ok::<_,std::convert::Infallible>(Response::builder().status(status).body(Empty::<Bytes>::new()).expect("scoped MCP denial response").map(axum::body::Body::new));}let response=adapter.oneshot(request).await.expect("scoped MCP response");Ok::<_,std::convert::Infallible>(response.map(axum::body::Body::new))}});let _=http1::Builder::new().serve_connection(TokioIo::new(stream),guard).await;});}})});Ok(ManagedEpicRunnerActionServer{address,cancellation,join:Some(join)})} }
}
start_scoped_server!(start_pre_start_server,SprintPreStartMcp);
start_scoped_server!(start_epic_start_server,EpicStartMcp);
start_scoped_server!(start_started_server,SprintStartedMcp);
start_scoped_server!(start_planning_control_server,SprintPlanningControlMcp);
start_scoped_server!(start_work_slice_planner_server,WorkSlicePlannerMcp);
start_scoped_server!(start_work_unit_handler_server,WorkUnitHandlerMcp);
#[cfg(test)]
pub(crate) fn start_work_slice_planner_test_server(
    service: Arc<SprintRunnerTransitionService>,
    invocation_id: AgentInvocationId,
    bearer: String,
    origins: Vec<String>,
) -> io::Result<ManagedEpicRunnerActionServer> {
    start_work_slice_planner_server(service, invocation_id, bearer, origins)
}
fn start_epic_runner_server(
    service: Arc<SprintRunnerTransitionService>,
    invocation_id: AgentInvocationId,
    bearer: String,
    origins: Vec<String>,
) -> io::Result<ManagedEpicRunnerActionServer> {
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
            .expect("Epic Runner MCP runtime");
        runtime.block_on(async move {
        let config=rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default().with_allowed_hosts([format!("127.0.0.1:{}",address.port())]).with_allowed_origins(origins.clone()).with_cancellation_token(server_cancel.clone());
        let adapter: rmcp::transport::streamable_http_server::StreamableHttpService<EpicRunnerActionMcp,rmcp::transport::streamable_http_server::session::local::LocalSessionManager>=rmcp::transport::streamable_http_server::StreamableHttpService::new(move||Ok(EpicRunnerActionMcp::new(service.clone(),invocation_id.clone())),Default::default(),config);
        let expected=Arc::new(bearer); let allowed_host=format!("127.0.0.1:{}",address.port()); let allowed_origins=Arc::new(origins); let listener=tokio::net::TcpListener::from_std(listener).expect("async Epic Runner MCP listener");
        loop { let accepted=tokio::select!{_=server_cancel.cancelled()=>break,accepted=listener.accept()=>accepted}; let Ok((stream,_))=accepted else {continue}; let adapter=adapter.clone(); let expected=expected.clone(); let allowed_host=allowed_host.clone(); let allowed_origins=allowed_origins.clone(); tokio::spawn(async move { let guard=service_fn(move|request|{let adapter=adapter.clone();let expected=expected.clone();let allowed_host=allowed_host.clone();let allowed_origins=allowed_origins.clone();async move { if let Some(status)=super::mcp::transport_denial(&expected,&allowed_host,&allowed_origins,&request){return Ok::<_,std::convert::Infallible>(Response::builder().status(status).body(Empty::<Bytes>::new()).expect("Epic Runner MCP denial response").map(axum::body::Body::new));} let response=adapter.oneshot(request).await.expect("Epic Runner MCP response");Ok::<_,std::convert::Infallible>(response.map(axum::body::Body::new)) }}); let _=http1::Builder::new().serve_connection(TokioIo::new(stream),guard).await; }); }
    });
    });
    Ok(ManagedEpicRunnerActionServer {
        address,
        cancellation,
        join: Some(join),
    })
}

#[cfg(test)]
mod implementer_activation_migration_tests {
    use super::migrate_legacy_implementer_activations;
    use rusqlite::{params, Connection};

    fn legacy_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "CREATE TABLE work_units (work_unit_id TEXT PRIMARY KEY);
             CREATE TABLE work_unit_handler_activations (
               work_unit_id TEXT PRIMARY KEY, attempt_id TEXT NOT NULL,
               handler_invocation_id TEXT NOT NULL
             );
             CREATE TABLE work_unit_implementer_activations (
               work_unit_id TEXT PRIMARY KEY, handler_attempt_id TEXT NOT NULL UNIQUE,
               handler_invocation_id TEXT NOT NULL UNIQUE,
               implementer_attempt_id TEXT NOT NULL UNIQUE,
               implementer_session_id TEXT NOT NULL UNIQUE,
               implementer_invocation_id TEXT NOT NULL UNIQUE,
               implementer_harness_revision_id TEXT NOT NULL,
               implementer_harness_configuration_digest TEXT NOT NULL,
               implementer_harness_repository_commit_ref TEXT NOT NULL,
               requested_at TEXT NOT NULL, authorized_at TEXT,
               execution_support_granted_at TEXT, isolated_worktree_ready_at TEXT,
               implementer_session_created_at TEXT, implementer_invocation_prepared_at TEXT,
               implementer_harness_bound_at TEXT, launch_requested_at TEXT,
               launch_accepted_at TEXT, provider_activation_observed_at TEXT,
               implementer_ready_at TEXT,
               CHECK ((implementer_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL))
             );
             INSERT INTO work_units VALUES ('unit');
             INSERT INTO work_unit_implementer_activations VALUES
               ('unit','handler-attempt','handler-invocation','obsolete-attempt','session','invocation',
                'revision','digest','commit','requested','authorized','support','worktree','session-created',
                'prepared','bound','launch-requested','launch-accepted','observed','ready');",
        ).unwrap();
        connection
    }

    #[test]
    fn legacy_implementer_activation_migrates_to_the_exact_handler_attempt() {
        let connection = legacy_connection();
        connection.execute(
            "INSERT INTO work_unit_handler_activations VALUES (?1,?2,?3)",
            params!["unit", "handler-attempt", "handler-invocation"],
        ).unwrap();
        migrate_legacy_implementer_activations(&connection).unwrap();
        let row: (String, String, String, String, String) = connection.query_row(
            "SELECT attempt_id,implementer_session_id,implementer_invocation_id,
                    provider_activation_observed_at,implementer_ready_at
             FROM work_unit_implementer_activations", [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).unwrap();
        assert_eq!(row, ("handler-attempt".into(), "session".into(), "invocation".into(), "observed".into(), "ready".into()));
        migrate_legacy_implementer_activations(&connection).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_implementer_activations", [], |row| row.get(0)).unwrap(), 1);
    }

    #[test]
    fn incoherent_legacy_activation_rolls_back_then_retries_after_handler_recovery() {
        let connection = legacy_connection();
        assert!(migrate_legacy_implementer_activations(&connection).is_err());
        let legacy_column: String = connection.query_row(
            "SELECT name FROM pragma_table_info('work_unit_implementer_activations')
             WHERE name='implementer_attempt_id'", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(legacy_column, "implementer_attempt_id");
        assert_eq!(connection.query_row::<i64, _, _>("SELECT COUNT(*) FROM sqlite_master WHERE name='work_unit_implementer_activations_v4'", [], |row| row.get(0)).unwrap(), 0);
        connection.execute(
            "INSERT INTO work_unit_handler_activations VALUES (?1,?2,?3)",
            params!["unit", "handler-attempt", "handler-invocation"],
        ).unwrap();
        migrate_legacy_implementer_activations(&connection).unwrap();
        assert_eq!(connection.query_row::<String, _, _>("SELECT attempt_id FROM work_unit_implementer_activations", [], |row| row.get(0)).unwrap(), "handler-attempt");
    }
}
