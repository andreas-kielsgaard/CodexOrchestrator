//! The first downstream boundary: one Epic Runner semantic request creates one ready Sprint Runner.

use super::conversation_harness::{self, ConversationHarnessRole};
use super::accepted_candidate_authority::{reconcile_accepted_candidate_authorities, ACCEPTED_CANDIDATE_AUTHORITY_SCHEMA};
use super::accepted_integration::reconcile_accepted_integrations;
use super::work_unit_execution_harness::{WorkUnitExecutionHarnessService, WorkUnitHarnessRole};
use super::work_unit_dependency_wave::{reconcile_work_slice_execution_settlement, reconcile_work_unit_dependency_wave};
use super::sprint_continuation_settlement;
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
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, OnceLock},
    thread,
};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

#[cfg(test)]
mod accepted_integration_gateway_tests;

pub(crate) const SPRINT_RUNNER_QUERY_CONTRACT: &str = "sprint-runner-transition-query/v1";

pub(crate) const SCHEMA: &str = r#"
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
  failure_reason TEXT,
  CHECK ((action_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL))
);
-- A reporting continuation is separate from the original, actionless Implementer invocation.
-- Acceptance here means only ready for independent Handler review; it cannot move the Work Unit.
CREATE TABLE IF NOT EXISTS work_unit_implementer_outcomes (
  attempt_id TEXT PRIMARY KEY,
  attempt_ordinal INTEGER NOT NULL CHECK (attempt_ordinal >= 0),
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  implementer_session_id TEXT NOT NULL,
  implementer_invocation_id TEXT NOT NULL UNIQUE,
  reporting_invocation_id TEXT NOT NULL UNIQUE,
  reporting_harness_revision_id TEXT NOT NULL,
  reporting_harness_configuration_digest TEXT NOT NULL,
  reporting_harness_repository_commit_ref TEXT NOT NULL,
  reporting_requested_at TEXT NOT NULL,
  reporting_prepared_at TEXT,
  reporting_harness_bound_at TEXT,
  reporting_launch_requested_at TEXT,
  reporting_launch_accepted_at TEXT,
  reporting_ready_at TEXT,
  submitted_summary TEXT,
  outcome_variant TEXT,
  submitted_validation_statement TEXT,
  semantic_payload_json TEXT CHECK (semantic_payload_json IS NULL OR json_valid(semantic_payload_json)),
  submission_fingerprint TEXT,
  submitted_at TEXT,
  validation_at TEXT,
  validation_result TEXT,
  evidence_manifest_json TEXT CHECK (evidence_manifest_json IS NULL OR json_valid(evidence_manifest_json)),
  comparison_fingerprint TEXT,
  evidence_content_fingerprints_json TEXT CHECK (evidence_content_fingerprints_json IS NULL OR json_valid(evidence_content_fingerprints_json)),
  file_review_capture_authorization_id TEXT REFERENCES file_review_git_capture_authorizations(capture_authorization_id) ON DELETE RESTRICT,
  evidence_ready_at TEXT,
  semantic_completed_at TEXT,
  semantic_completion_invocation_id TEXT,
  lifecycle_observed_at TEXT,
  lifecycle_status TEXT,
  application_accepted_at TEXT,
  handler_review_ready_at TEXT,
  failure_reason TEXT,
  UNIQUE(work_unit_id, attempt_id),
  UNIQUE(work_unit_id, attempt_ordinal),
  CHECK ((semantic_completed_at IS NULL) OR (submitted_at IS NOT NULL AND validation_result='valid')),
  CHECK ((handler_review_ready_at IS NULL) OR (application_accepted_at IS NOT NULL))
);
CREATE TABLE IF NOT EXISTS work_unit_handler_reviews (
  attempt_id TEXT PRIMARY KEY REFERENCES work_unit_implementer_outcomes(attempt_id) ON DELETE RESTRICT,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  reporting_invocation_id TEXT NOT NULL UNIQUE,
  handler_session_id TEXT NOT NULL,
  original_handler_invocation_id TEXT NOT NULL,
  action_handler_invocation_id TEXT NOT NULL,
  review_invocation_id TEXT NOT NULL UNIQUE,
  review_harness_revision_id TEXT NOT NULL,
  review_harness_configuration_digest TEXT NOT NULL,
  review_harness_repository_commit_ref TEXT NOT NULL,
  delivery_requested_at TEXT NOT NULL,
  delivery_persisted_at TEXT,
  harness_bound_at TEXT,
  launch_requested_at TEXT,
  launch_accepted_at TEXT,
  review_ready_at TEXT,
  delivered_payload_json TEXT NOT NULL CHECK (json_valid(delivered_payload_json)),
  delivered_payload_fingerprint TEXT NOT NULL,
  semantic_judgment_variant TEXT CHECK (semantic_judgment_variant IN ('accept','return')),
  semantic_return_reason_json TEXT CHECK (semantic_return_reason_json IS NULL OR json_valid(semantic_return_reason_json)),
  semantic_judgment_fingerprint TEXT,
  semantic_judgment_at TEXT,
  lifecycle_observed_at TEXT,
  lifecycle_status TEXT,
  conflict_at TEXT,
  conflict_reason TEXT,
  UNIQUE(work_unit_id, attempt_id),
  CHECK ((review_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL)),
  CHECK ((semantic_judgment_at IS NULL) OR (semantic_judgment_variant IS NOT NULL AND launch_accepted_at IS NOT NULL))
);
CREATE TABLE IF NOT EXISTS work_unit_handler_decisions (
  review_invocation_id TEXT PRIMARY KEY REFERENCES work_unit_handler_reviews(review_invocation_id) ON DELETE RESTRICT,
  attempt_id TEXT NOT NULL UNIQUE REFERENCES work_unit_implementer_outcomes(attempt_id) ON DELETE RESTRICT,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  decision_variant TEXT NOT NULL CHECK (decision_variant IN ('accepted','returned')),
  decision_fingerprint TEXT NOT NULL UNIQUE,
  return_reason_json TEXT CHECK (return_reason_json IS NULL OR json_valid(return_reason_json)),
  decision_recorded_at TEXT NOT NULL,
  implementation_accepted_at TEXT,
  implementation_returned_at TEXT,
  retry_required_at TEXT,
  settlement_ready_at TEXT,
  CHECK ((decision_variant='accepted' AND implementation_accepted_at IS NOT NULL AND implementation_returned_at IS NULL AND retry_required_at IS NULL) OR (decision_variant='returned' AND implementation_returned_at IS NOT NULL AND implementation_accepted_at IS NULL)),
  CHECK (settlement_ready_at IS NULL)
);
-- A returned review is not itself a retry launch.  These facts are deliberately separate so a
-- later bounded attempt or an upward reassessment cannot be inferred from Handler prose.
CREATE TABLE IF NOT EXISTS work_unit_handler_incomplete_dispositions (
  attempt_id TEXT PRIMARY KEY REFERENCES work_unit_implementer_outcomes(attempt_id) ON DELETE RESTRICT,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  review_invocation_id TEXT NOT NULL UNIQUE REFERENCES work_unit_handler_reviews(review_invocation_id) ON DELETE RESTRICT,
  decision_fingerprint TEXT NOT NULL UNIQUE,
  classification TEXT NOT NULL CHECK (classification IN ('refinement_needed','functional_objective_not_satisfied','blocked')),
  meaningful_progress INTEGER NOT NULL CHECK (meaningful_progress IN (0,1)),
  recorded_at TEXT NOT NULL,
  next_attempt_authorized_at TEXT,
  CHECK ((meaningful_progress=1 AND next_attempt_authorized_at IS NOT NULL) OR (meaningful_progress=0 AND next_attempt_authorized_at IS NULL))
);
CREATE TABLE IF NOT EXISTS work_unit_handler_incomplete_judgments (
  review_invocation_id TEXT PRIMARY KEY REFERENCES work_unit_handler_reviews(review_invocation_id) ON DELETE RESTRICT,
  attempt_id TEXT NOT NULL UNIQUE REFERENCES work_unit_implementer_outcomes(attempt_id) ON DELETE RESTRICT,
  classification TEXT NOT NULL CHECK (classification IN ('refinement_needed','functional_objective_not_satisfied','blocked')),
  meaningful_progress INTEGER NOT NULL CHECK (meaningful_progress IN (0,1)),
  judgment_fingerprint TEXT NOT NULL UNIQUE,
  recorded_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS work_unit_no_progress_handbacks (
  handback_id TEXT PRIMARY KEY,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  source_attempt_id TEXT NOT NULL UNIQUE REFERENCES work_unit_implementer_outcomes(attempt_id) ON DELETE RESTRICT,
  source_review_invocation_id TEXT NOT NULL UNIQUE REFERENCES work_unit_handler_reviews(review_invocation_id) ON DELETE RESTRICT,
  decision_fingerprint TEXT NOT NULL UNIQUE,
  classification TEXT NOT NULL CHECK (classification IN ('refinement_needed','functional_objective_not_satisfied','blocked')),
  context_json TEXT NOT NULL CHECK (json_valid(context_json)),
  context_fingerprint TEXT NOT NULL UNIQUE,
  persisted_at TEXT NOT NULL,
  delivery_intended_at TEXT NOT NULL,
  sprint_runner_receiver_activated_at TEXT,
  sprint_runner_receiver_decision_at TEXT,
  CHECK (sprint_runner_receiver_activated_at IS NULL AND sprint_runner_receiver_decision_at IS NULL)
);
-- Handback consumption is deliberately a separate receiver route.  The source Handback stays
-- immutable; neither delivery nor a selected movement can be mistaken for Work Unit settlement.
CREATE TABLE IF NOT EXISTS sprint_runner_handback_deliveries (
  handback_id TEXT PRIMARY KEY REFERENCES work_unit_no_progress_handbacks(handback_id) ON DELETE RESTRICT,
  sprint_id TEXT NOT NULL REFERENCES sprint_runner_transitions(sprint_id) ON DELETE RESTRICT,
  receiver_session_id TEXT NOT NULL,
  reassessment_invocation_id TEXT NOT NULL UNIQUE,
  delivery_fact_id TEXT NOT NULL UNIQUE,
  delivery_requested_at TEXT NOT NULL,
  delivery_persisted_at TEXT,
  harness_key TEXT NOT NULL,
  harness_version INTEGER NOT NULL,
  harness_bound_at TEXT,
  launch_requested_at TEXT,
  launch_accepted_at TEXT,
  provider_activation_observed_at TEXT,
  reassessment_lifecycle_status TEXT,
  reassessment_lifecycle_observed_at TEXT,
  semantic_reassessment_fact_id TEXT UNIQUE,
  semantic_reassessment_recorded_at TEXT,
  context_fingerprint TEXT NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS sprint_runner_handback_dispositions (
  handback_id TEXT PRIMARY KEY REFERENCES sprint_runner_handback_deliveries(handback_id) ON DELETE RESTRICT,
  disposition_id TEXT NOT NULL UNIQUE,
  movement_kind TEXT NOT NULL,
  details_json TEXT NOT NULL CHECK (json_valid(details_json)),
  disposition_fingerprint TEXT NOT NULL UNIQUE,
  selected_at TEXT NOT NULL,
  preserves_handback INTEGER NOT NULL CHECK (preserves_handback=1)
);
CREATE TABLE IF NOT EXISTS sprint_runner_handback_escalations (
  handback_id TEXT PRIMARY KEY REFERENCES sprint_runner_handback_dispositions(handback_id) ON DELETE RESTRICT,
  escalation_intent_id TEXT NOT NULL UNIQUE,
  delivery_request_id TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL,
  delivery_requested_at TEXT NOT NULL,
  delivery_persisted_at TEXT
);
CREATE TABLE IF NOT EXISTS epic_runner_escalation_receivers (
  handback_id TEXT PRIMARY KEY REFERENCES sprint_runner_handback_escalations(handback_id) ON DELETE RESTRICT,
  escalation_intent_id TEXT NOT NULL UNIQUE, delivery_request_id TEXT NOT NULL UNIQUE,
  sprint_id TEXT NOT NULL REFERENCES sprint_runner_transitions(sprint_id) ON DELETE RESTRICT, epic_id TEXT NOT NULL,
  governing_runner_session_id TEXT NOT NULL, governing_runner_invocation_id TEXT NOT NULL,
  reassessment_invocation_id TEXT NOT NULL UNIQUE, delivery_fact_id TEXT NOT NULL UNIQUE, delivery_requested_at TEXT NOT NULL,
  delivery_persisted_at TEXT, harness_key TEXT NOT NULL, harness_version INTEGER NOT NULL, harness_bound_at TEXT,
  launch_requested_at TEXT, launch_accepted_at TEXT, provider_activation_observed_at TEXT,
  reassessment_lifecycle_status TEXT, reassessment_lifecycle_observed_at TEXT,
  semantic_reassessment_fact_id TEXT UNIQUE, semantic_reassessment_recorded_at TEXT,
  correlation_fingerprint TEXT NOT NULL UNIQUE
);
-- Epic reassessment preserves the upstream Handback.  A selected movement is not delivery,
-- activation, continuation, Sprint selection, settlement, or acceptance.
CREATE TABLE IF NOT EXISTS epic_runner_escalation_dispositions (
  handback_id TEXT PRIMARY KEY REFERENCES epic_runner_escalation_receivers(handback_id) ON DELETE RESTRICT,
  disposition_id TEXT NOT NULL UNIQUE,
  movement_kind TEXT NOT NULL,
  details_json TEXT NOT NULL CHECK (json_valid(details_json)),
  disposition_fingerprint TEXT NOT NULL UNIQUE,
  selected_at TEXT NOT NULL,
  preserves_handback INTEGER NOT NULL CHECK (preserves_handback=1)
);
CREATE TABLE IF NOT EXISTS epic_runner_escalation_downstream_requests (
  handback_id TEXT PRIMARY KEY REFERENCES epic_runner_escalation_dispositions(handback_id) ON DELETE RESTRICT,
  request_id TEXT NOT NULL UNIQUE,
  request_kind TEXT NOT NULL,
  request_json TEXT NOT NULL CHECK (json_valid(request_json)),
  request_fingerprint TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS epic_runner_escalation_attentions (
  handback_id TEXT PRIMARY KEY REFERENCES epic_runner_escalation_dispositions(handback_id) ON DELETE RESTRICT,
  attention_id TEXT NOT NULL UNIQUE,
  attention_json TEXT NOT NULL CHECK (json_valid(attention_json)),
  attention_fingerprint TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL
);
-- Each completed meaningful-progress disposition can authorize exactly one later correction
-- attempt. The private ref name and object ids never cross the native-query boundary.
CREATE TABLE IF NOT EXISTS work_unit_retry_attempts (
  retry_attempt_id TEXT PRIMARY KEY,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
  origin_attempt_id TEXT NOT NULL UNIQUE REFERENCES work_unit_implementer_outcomes(attempt_id) ON DELETE RESTRICT,
  review_invocation_id TEXT NOT NULL UNIQUE REFERENCES work_unit_handler_reviews(review_invocation_id) ON DELETE RESTRICT,
  decision_fingerprint TEXT NOT NULL UNIQUE,
  sprint_git_authority_id TEXT NOT NULL REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT,
  sprint_baseline_object_id TEXT NOT NULL,
  sprint_current_object_id TEXT NOT NULL,
  implementer_session_id TEXT NOT NULL UNIQUE,
  implementer_invocation_id TEXT NOT NULL UNIQUE,
  implementer_harness_revision_id TEXT NOT NULL,
  implementer_harness_configuration_digest TEXT NOT NULL,
  implementer_harness_repository_commit_ref TEXT NOT NULL,
  capture_intent_id TEXT NOT NULL UNIQUE,
  capture_fingerprint TEXT NOT NULL UNIQUE,
  handoff_json TEXT NOT NULL CHECK(json_valid(handoff_json)),
  handoff_fingerprint TEXT NOT NULL UNIQUE,
  candidate_commit_id TEXT NOT NULL,
  candidate_tree_id TEXT NOT NULL,
  private_ref_name TEXT NOT NULL UNIQUE,
  capture_requested_at TEXT NOT NULL,
  candidate_pinned_at TEXT,
  authorized_at TEXT,
  execution_support_granted_at TEXT,
  isolated_worktree_ready_at TEXT,
  implementer_session_created_at TEXT,
  implementer_invocation_prepared_at TEXT,
  implementer_harness_bound_at TEXT,
  launch_requested_at TEXT,
  launch_accepted_at TEXT,
  provider_activation_observed_at TEXT,
  retry_ready_at TEXT,
  failure_reason TEXT,
  UNIQUE(work_unit_id, ordinal),
  CHECK ((retry_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL))
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

fn ensure_handler_action_failure_reason(connection: &Connection) -> Result<(), String> {
    let columns = connection.prepare("PRAGMA table_info(work_unit_handler_action_continuations)")
        .and_then(|mut statement| statement.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>())
        .map_err(|error| format!("inspect Handler action continuation schema: {error}"))?;
    if !columns.iter().any(|column| column == "failure_reason") {
        connection.execute_batch("ALTER TABLE work_unit_handler_action_continuations ADD COLUMN failure_reason TEXT")
            .map_err(|error| format!("add Handler action continuation failure reason: {error}"))?;
    }
    Ok(())
}

fn ensure_implementer_outcome_evidence_columns(connection: &Connection) -> Result<(), String> {
    for column in [
        "attempt_ordinal INTEGER NOT NULL DEFAULT 0 CHECK (attempt_ordinal >= 0)", "evidence_manifest_json TEXT", "comparison_fingerprint TEXT", "evidence_content_fingerprints_json TEXT", "file_review_capture_authorization_id TEXT", "evidence_ready_at TEXT", "semantic_completed_at TEXT", "semantic_completion_invocation_id TEXT", "lifecycle_observed_at TEXT", "lifecycle_status TEXT", "application_accepted_at TEXT", "handler_review_ready_at TEXT",
    ] {
        let name = column.split_whitespace().next().expect("outcome migration column");
        let exists = connection.prepare("PRAGMA table_info(work_unit_implementer_outcomes)")
            .and_then(|mut statement| statement.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| error.to_string())?.iter().any(|existing| existing == name);
        if !exists { connection.execute_batch(&format!("ALTER TABLE work_unit_implementer_outcomes ADD COLUMN {column}")).map_err(|error| error.to_string())?; }
    }
    Ok(())
}

/// The original productive boundary stored one row per Work Unit.  A returned result can now
/// create the one already-authorized ordinal-1 attempt, so the durable records must be keyed by
/// their application-owned attempt/review correlations instead.  This rebuild is deliberately
/// lossless: it validates and copies every ordinal-0 row before replacing a table.
fn migrate_work_unit_attempt_history(connection: &Connection) -> Result<(), String> {
    let outcome_primary_key = connection.prepare("PRAGMA table_info(work_unit_implementer_outcomes)")
        .and_then(|mut statement| statement.query_map([], |row| Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?)))?.collect::<Result<Vec<_>, _>>())
        .map_err(|error| format!("inspect Implementer outcome history schema: {error}"))?;
    if outcome_primary_key.iter().any(|(name, position)| name == "attempt_id" && *position == 1) {
        return Ok(());
    }
    if !outcome_primary_key.iter().any(|(name, position)| name == "work_unit_id" && *position == 1) {
        return Err("Implementer outcome history has an unknown primary key".into());
    }
    connection.execute_batch("PRAGMA foreign_keys=OFF; BEGIN IMMEDIATE;")
        .map_err(|error| format!("begin attempt-history migration: {error}"))?;
    let migrated = (|| -> Result<(), String> {
        let outcomes: i64 = connection.query_row("SELECT COUNT(*) FROM work_unit_implementer_outcomes", [], |row| row.get(0))
            .map_err(|error| format!("count ordinal-0 outcomes: {error}"))?;
        let reviews: i64 = connection.query_row("SELECT COUNT(*) FROM work_unit_handler_reviews", [], |row| row.get(0))
            .map_err(|error| format!("count ordinal-0 reviews: {error}"))?;
        let decisions: i64 = connection.query_row("SELECT COUNT(*) FROM work_unit_handler_decisions", [], |row| row.get(0))
            .map_err(|error| format!("count ordinal-0 decisions: {error}"))?;
        connection.execute_batch(r#"
CREATE TABLE work_unit_implementer_outcomes_history (
  attempt_id TEXT PRIMARY KEY, attempt_ordinal INTEGER NOT NULL CHECK (attempt_ordinal >= 0), work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  implementer_session_id TEXT NOT NULL, implementer_invocation_id TEXT NOT NULL UNIQUE, reporting_invocation_id TEXT NOT NULL UNIQUE,
  reporting_harness_revision_id TEXT NOT NULL, reporting_harness_configuration_digest TEXT NOT NULL, reporting_harness_repository_commit_ref TEXT NOT NULL,
  reporting_requested_at TEXT NOT NULL, reporting_prepared_at TEXT, reporting_harness_bound_at TEXT, reporting_launch_requested_at TEXT,
  reporting_launch_accepted_at TEXT, reporting_ready_at TEXT, submitted_summary TEXT, outcome_variant TEXT, submitted_validation_statement TEXT,
  semantic_payload_json TEXT CHECK (semantic_payload_json IS NULL OR json_valid(semantic_payload_json)), submission_fingerprint TEXT, submitted_at TEXT,
  validation_at TEXT, validation_result TEXT, evidence_manifest_json TEXT CHECK (evidence_manifest_json IS NULL OR json_valid(evidence_manifest_json)),
  comparison_fingerprint TEXT, evidence_content_fingerprints_json TEXT CHECK (evidence_content_fingerprints_json IS NULL OR json_valid(evidence_content_fingerprints_json)),
  evidence_ready_at TEXT, semantic_completed_at TEXT, semantic_completion_invocation_id TEXT, lifecycle_observed_at TEXT, lifecycle_status TEXT,
  application_accepted_at TEXT, handler_review_ready_at TEXT, failure_reason TEXT,
  UNIQUE(work_unit_id, attempt_id), CHECK ((semantic_completed_at IS NULL) OR (submitted_at IS NOT NULL AND validation_result='valid')),
  UNIQUE(work_unit_id, attempt_ordinal),
  CHECK ((handler_review_ready_at IS NULL) OR (application_accepted_at IS NOT NULL))
);
CREATE TABLE work_unit_handler_reviews_history (
  attempt_id TEXT PRIMARY KEY REFERENCES work_unit_implementer_outcomes_history(attempt_id) ON DELETE RESTRICT,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT, reporting_invocation_id TEXT NOT NULL UNIQUE,
  handler_session_id TEXT NOT NULL, original_handler_invocation_id TEXT NOT NULL, action_handler_invocation_id TEXT NOT NULL,
  review_invocation_id TEXT NOT NULL UNIQUE, review_harness_revision_id TEXT NOT NULL, review_harness_configuration_digest TEXT NOT NULL,
  review_harness_repository_commit_ref TEXT NOT NULL, delivery_requested_at TEXT NOT NULL, delivery_persisted_at TEXT, harness_bound_at TEXT,
  launch_requested_at TEXT, launch_accepted_at TEXT, review_ready_at TEXT, delivered_payload_json TEXT NOT NULL CHECK (json_valid(delivered_payload_json)),
  delivered_payload_fingerprint TEXT NOT NULL, semantic_judgment_variant TEXT CHECK (semantic_judgment_variant IN ('accept','return')),
  semantic_return_reason_json TEXT CHECK (semantic_return_reason_json IS NULL OR json_valid(semantic_return_reason_json)), semantic_judgment_fingerprint TEXT,
  semantic_judgment_at TEXT, lifecycle_observed_at TEXT, lifecycle_status TEXT, conflict_at TEXT, conflict_reason TEXT,
  UNIQUE(work_unit_id, attempt_id), CHECK ((review_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL)),
  CHECK ((semantic_judgment_at IS NULL) OR (semantic_judgment_variant IS NOT NULL AND launch_accepted_at IS NOT NULL))
);
CREATE TABLE work_unit_handler_decisions_history (
  review_invocation_id TEXT PRIMARY KEY REFERENCES work_unit_handler_reviews_history(review_invocation_id) ON DELETE RESTRICT,
  attempt_id TEXT NOT NULL UNIQUE REFERENCES work_unit_implementer_outcomes_history(attempt_id) ON DELETE RESTRICT,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT, decision_variant TEXT NOT NULL CHECK (decision_variant IN ('accepted','returned')),
  decision_fingerprint TEXT NOT NULL UNIQUE, return_reason_json TEXT CHECK (return_reason_json IS NULL OR json_valid(return_reason_json)),
  decision_recorded_at TEXT NOT NULL, implementation_accepted_at TEXT, implementation_returned_at TEXT, retry_required_at TEXT, settlement_ready_at TEXT,
  CHECK ((decision_variant='accepted' AND implementation_accepted_at IS NOT NULL AND implementation_returned_at IS NULL AND retry_required_at IS NULL) OR (decision_variant='returned' AND implementation_returned_at IS NOT NULL AND retry_required_at IS NOT NULL AND implementation_accepted_at IS NULL)),
  CHECK (settlement_ready_at IS NULL)
);
"#).map_err(|error| format!("create attempt-history tables: {error}"))?;
        let copied_outcomes = connection.execute("INSERT INTO work_unit_implementer_outcomes_history SELECT attempt_id,attempt_ordinal,work_unit_id,implementer_session_id,implementer_invocation_id,reporting_invocation_id,reporting_harness_revision_id,reporting_harness_configuration_digest,reporting_harness_repository_commit_ref,reporting_requested_at,reporting_prepared_at,reporting_harness_bound_at,reporting_launch_requested_at,reporting_launch_accepted_at,reporting_ready_at,submitted_summary,outcome_variant,submitted_validation_statement,semantic_payload_json,submission_fingerprint,submitted_at,validation_at,validation_result,evidence_manifest_json,comparison_fingerprint,evidence_content_fingerprints_json,evidence_ready_at,semantic_completed_at,semantic_completion_invocation_id,lifecycle_observed_at,lifecycle_status,application_accepted_at,handler_review_ready_at,failure_reason FROM work_unit_implementer_outcomes", [],).map_err(|error| format!("copy ordinal-0 outcomes: {error}"))?;
        let copied_reviews = connection.execute("INSERT INTO work_unit_handler_reviews_history SELECT attempt_id,work_unit_id,reporting_invocation_id,handler_session_id,original_handler_invocation_id,action_handler_invocation_id,review_invocation_id,review_harness_revision_id,review_harness_configuration_digest,review_harness_repository_commit_ref,delivery_requested_at,delivery_persisted_at,harness_bound_at,launch_requested_at,launch_accepted_at,review_ready_at,delivered_payload_json,delivered_payload_fingerprint,semantic_judgment_variant,semantic_return_reason_json,semantic_judgment_fingerprint,semantic_judgment_at,lifecycle_observed_at,lifecycle_status,conflict_at,conflict_reason FROM work_unit_handler_reviews", [],).map_err(|error| format!("copy ordinal-0 reviews: {error}"))?;
        let copied_decisions = connection.execute("INSERT INTO work_unit_handler_decisions_history SELECT d.review_invocation_id,r.attempt_id,d.work_unit_id,d.decision_variant,d.decision_fingerprint,d.return_reason_json,d.decision_recorded_at,d.implementation_accepted_at,d.implementation_returned_at,d.retry_required_at,d.settlement_ready_at FROM work_unit_handler_decisions d JOIN work_unit_handler_reviews r ON r.review_invocation_id=d.review_invocation_id", [],).map_err(|error| format!("copy ordinal-0 decisions: {error}"))?;
        if copied_outcomes as i64 != outcomes || copied_reviews as i64 != reviews || copied_decisions as i64 != decisions { return Err("attempt-history migration copy was incomplete".into()); }
        connection.execute_batch("DROP TABLE work_unit_handler_decisions; DROP TABLE work_unit_handler_reviews; DROP TABLE work_unit_implementer_outcomes; ALTER TABLE work_unit_implementer_outcomes_history RENAME TO work_unit_implementer_outcomes; ALTER TABLE work_unit_handler_reviews_history RENAME TO work_unit_handler_reviews; ALTER TABLE work_unit_handler_decisions_history RENAME TO work_unit_handler_decisions;")
            .map_err(|error| format!("replace attempt-history tables: {error}"))?;
        Ok(())
    })();
    match migrated { Ok(()) => connection.execute_batch("COMMIT; PRAGMA foreign_keys=ON;").map_err(|error| format!("commit attempt-history migration: {error}")), Err(error) => { let _ = connection.execute_batch("ROLLBACK; PRAGMA foreign_keys=ON;"); Err(error) } }
}

/// Retry activation used to be a one-row ordinal-1 record.  Keep the current authorizer
/// narrow, but make durable recovery address records by their application-owned ordinal.
fn migrate_work_unit_retry_attempt_history(connection: &Connection) -> Result<(), String> {
    let schema: Option<String> = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='work_unit_retry_attempts'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("inspect retry-attempt schema: {error}"))?;
    let Some(schema) = schema else { return Ok(()); };
    if schema.contains("retry_attempt_id TEXT PRIMARY KEY")
        && schema.contains("CHECK (ordinal >= 0)")
        && schema.contains("UNIQUE(work_unit_id, ordinal)")
    {
        return Ok(());
    }
    if !schema.contains("work_unit_id TEXT PRIMARY KEY") || !schema.contains("CHECK (ordinal=1)") {
        return Err("retry-attempt history has an unknown primary key or ordinal constraint".into());
    }
    connection.execute_batch("PRAGMA foreign_keys=OFF; BEGIN IMMEDIATE;")
        .map_err(|error| format!("begin retry-attempt migration: {error}"))?;
    let migrated = (|| -> Result<(), String> {
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM work_unit_retry_attempts", [], |row| row.get(0))
            .map_err(|error| format!("count legacy retry attempts: {error}"))?;
        connection.execute_batch(r#"
CREATE TABLE work_unit_retry_attempts_history (
  retry_attempt_id TEXT PRIMARY KEY,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
  origin_attempt_id TEXT NOT NULL UNIQUE REFERENCES work_unit_implementer_outcomes(attempt_id) ON DELETE RESTRICT,
  review_invocation_id TEXT NOT NULL UNIQUE REFERENCES work_unit_handler_reviews(review_invocation_id) ON DELETE RESTRICT,
  decision_fingerprint TEXT NOT NULL UNIQUE,
  sprint_git_authority_id TEXT NOT NULL REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT,
  sprint_baseline_object_id TEXT NOT NULL,
  sprint_current_object_id TEXT NOT NULL,
  implementer_session_id TEXT NOT NULL UNIQUE,
  implementer_invocation_id TEXT NOT NULL UNIQUE,
  implementer_harness_revision_id TEXT NOT NULL,
  implementer_harness_configuration_digest TEXT NOT NULL,
  implementer_harness_repository_commit_ref TEXT NOT NULL,
  capture_intent_id TEXT NOT NULL UNIQUE,
  capture_fingerprint TEXT NOT NULL UNIQUE,
  handoff_json TEXT NOT NULL CHECK(json_valid(handoff_json)),
  handoff_fingerprint TEXT NOT NULL UNIQUE,
  candidate_commit_id TEXT NOT NULL,
  candidate_tree_id TEXT NOT NULL,
  private_ref_name TEXT NOT NULL UNIQUE,
  capture_requested_at TEXT NOT NULL,
  candidate_pinned_at TEXT,
  authorized_at TEXT,
  execution_support_granted_at TEXT,
  isolated_worktree_ready_at TEXT,
  implementer_session_created_at TEXT,
  implementer_invocation_prepared_at TEXT,
  implementer_harness_bound_at TEXT,
  launch_requested_at TEXT,
  launch_accepted_at TEXT,
  provider_activation_observed_at TEXT,
  retry_ready_at TEXT,
  failure_reason TEXT,
  UNIQUE(work_unit_id, ordinal),
  CHECK ((retry_ready_at IS NULL) OR (launch_accepted_at IS NOT NULL))
);
"#).map_err(|error| format!("create retry-attempt history table: {error}"))?;
        let copied = connection.execute(
            "INSERT INTO work_unit_retry_attempts_history SELECT retry_attempt_id,work_unit_id,ordinal,origin_attempt_id,review_invocation_id,decision_fingerprint,sprint_git_authority_id,sprint_baseline_object_id,sprint_current_object_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref,capture_intent_id,capture_fingerprint,handoff_json,handoff_fingerprint,candidate_commit_id,candidate_tree_id,private_ref_name,capture_requested_at,candidate_pinned_at,authorized_at,execution_support_granted_at,isolated_worktree_ready_at,implementer_session_created_at,implementer_invocation_prepared_at,implementer_harness_bound_at,launch_requested_at,launch_accepted_at,provider_activation_observed_at,retry_ready_at,failure_reason FROM work_unit_retry_attempts",
            [],
        ).map_err(|error| format!("copy legacy retry attempts: {error}"))?;
        if copied as i64 != count { return Err("retry-attempt migration copy was incomplete".into()); }
        connection.execute_batch("DROP TABLE work_unit_retry_attempts; ALTER TABLE work_unit_retry_attempts_history RENAME TO work_unit_retry_attempts;")
            .map_err(|error| format!("replace retry-attempt table: {error}"))?;
        Ok(())
    })();
    match migrated {
        Ok(()) => connection.execute_batch("COMMIT; PRAGMA foreign_keys=ON;")
            .map_err(|error| format!("commit retry-attempt migration: {error}")),
        Err(error) => { let _ = connection.execute_batch("ROLLBACK; PRAGMA foreign_keys=ON;"); Err(error) }
    }
}

fn migrate_handler_decision_retry_contract(connection: &Connection) -> Result<(), String> {
    let schema: Option<String> = connection.query_row("SELECT sql FROM sqlite_master WHERE type='table' AND name='work_unit_handler_decisions'", [], |row| row.get(0)).optional().map_err(|error| format!("inspect Handler decision schema: {error}"))?;
    let Some(schema) = schema else { return Ok(()); };
    if !schema.contains("retry_required_at IS NOT NULL") { return Ok(()); }
    connection.execute_batch("PRAGMA foreign_keys=OFF; BEGIN IMMEDIATE;").map_err(|error| format!("begin Handler decision migration: {error}"))?;
    let migrated = (|| -> Result<(), String> {
        let count: i64 = connection.query_row("SELECT COUNT(*) FROM work_unit_handler_decisions", [], |row| row.get(0)).map_err(|error| format!("count Handler decisions: {error}"))?;
        connection.execute_batch(r#"
CREATE TABLE work_unit_handler_decisions_contract_history (
  review_invocation_id TEXT PRIMARY KEY REFERENCES work_unit_handler_reviews(review_invocation_id) ON DELETE RESTRICT,
  attempt_id TEXT NOT NULL UNIQUE REFERENCES work_unit_implementer_outcomes(attempt_id) ON DELETE RESTRICT,
  work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  decision_variant TEXT NOT NULL CHECK (decision_variant IN ('accepted','returned')),
  decision_fingerprint TEXT NOT NULL UNIQUE,
  return_reason_json TEXT CHECK (return_reason_json IS NULL OR json_valid(return_reason_json)),
  decision_recorded_at TEXT NOT NULL,
  implementation_accepted_at TEXT,
  implementation_returned_at TEXT,
  retry_required_at TEXT,
  settlement_ready_at TEXT,
  CHECK ((decision_variant='accepted' AND implementation_accepted_at IS NOT NULL AND implementation_returned_at IS NULL AND retry_required_at IS NULL) OR (decision_variant='returned' AND implementation_returned_at IS NOT NULL AND implementation_accepted_at IS NULL)),
  CHECK (settlement_ready_at IS NULL)
);
INSERT INTO work_unit_handler_decisions_contract_history SELECT review_invocation_id,attempt_id,work_unit_id,decision_variant,decision_fingerprint,return_reason_json,decision_recorded_at,implementation_accepted_at,implementation_returned_at,retry_required_at,settlement_ready_at FROM work_unit_handler_decisions;
DROP TABLE work_unit_handler_decisions;
ALTER TABLE work_unit_handler_decisions_contract_history RENAME TO work_unit_handler_decisions;
"#).map_err(|error| format!("rebuild Handler decision schema: {error}"))?;
        let copied: i64 = connection.query_row("SELECT COUNT(*) FROM work_unit_handler_decisions", [], |row| row.get(0)).map_err(|error| format!("count rebuilt Handler decisions: {error}"))?;
        if copied != count { return Err("Handler decision migration copy was incomplete".into()); }
        Ok(())
    })();
    match migrated {
        Ok(()) => connection.execute_batch("COMMIT; PRAGMA foreign_keys=ON;").map_err(|error| format!("commit Handler decision migration: {error}")),
        Err(error) => { let _ = connection.execute_batch("ROLLBACK; PRAGMA foreign_keys=ON;"); Err(error) }
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

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImplementationOutcomeVariant { ReviewPending }

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImplementationOutcomeClaims { pub(crate) outcome: ImplementationOutcomeVariant, pub(crate) summary: String, pub(crate) validation_statement: String }

#[derive(Clone)]
struct ImplementerReportingContext { work_unit_id: String, attempt_id: String, session_id: String, implementer_invocation_id: String, reporting_invocation_id: String, revision_id: String, configuration_digest: String, repository_commit_ref: String }
#[derive(Clone, Eq, PartialEq)]
struct ImplementationEvidenceSnapshot { manifest_json: String, comparison_fingerprint: String, content_fingerprints_json: String, capture_authorization_id: String }

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HandlerReviewReturnReason { pub(crate) code: String, pub(crate) explanation: String }
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IncompleteAttemptClassification { RefinementNeeded, FunctionalObjectiveNotSatisfied, Blocked }
impl IncompleteAttemptClassification {
    fn as_str(&self) -> &'static str { match self { Self::RefinementNeeded => "refinement_needed", Self::FunctionalObjectiveNotSatisfied => "functional_objective_not_satisfied", Self::Blocked => "blocked" } }
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HandlerReviewIncompleteDisposition {
    pub(crate) code: String,
    pub(crate) explanation: String,
    pub(crate) classification: IncompleteAttemptClassification,
    pub(crate) meaningful_progress: bool,
}
/// The reassessment never supplies identities, routes, or authority.  `movement_kind` remains
/// bounded-extensible; the application validates the three currently understood movements.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SprintHandbackDisposition {
    pub(crate) movement_kind: String,
    pub(crate) rationale: String,
    pub(crate) eligible_work_summary: Option<String>,
    pub(crate) dependency_owner: Option<String>,
    pub(crate) dependency_owner_classification: Option<AgentAchievableDependencyOwner>,
    pub(crate) enabling_result: Option<String>,
    pub(crate) resumption_path: Option<String>,
    pub(crate) local_exhaustion_summary: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentAchievableDependencyOwner {
    WorkUnitHandler,
    WorkUnitImplementer,
    WorkSlicePlanner,
    SprintRunner,
}
/// This action is semantic only: its input carries no identity, route, provider, or start
/// authority.  Unknown movement kinds are retained as intent-only for safe extension.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EpicEscalationReassessmentDisposition {
    pub(crate) movement_kind: String,
    pub(crate) rationale: String,
    pub(crate) considered_intent: Option<String>,
    pub(crate) downstream_request: Option<EpicEscalationDownstreamRequest>,
    pub(crate) human_external_attention: Option<EpicEscalationAttention>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EpicEscalationDownstreamRequest {
    pub(crate) target: EpicEscalationDownstreamTarget,
    pub(crate) dependency: Option<EpicKnownAgentDependency>,
    pub(crate) request: String,
    pub(crate) resumption_path: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EpicEscalationDownstreamTarget { SprintRunner, ExistingAgentAchievableDependency }
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EpicKnownAgentDependency { WorkUnitHandler }
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EpicEscalationAttention {
    pub(crate) reason: String,
    pub(crate) authority_needed: String,
    pub(crate) evidence_context: String,
    pub(crate) resumption_path: String,
}
#[derive(Clone)]
struct HandlerReviewContext { work_unit_id:String, attempt_id:String, handler_authority_attempt_id:String, session_id:String, review_invocation_id:String, revision_id:String, configuration_digest:String, repository_commit_ref:String, delivered_payload_json:String, delivered_payload_fingerprint:String }
#[derive(Clone)]
struct RetrySource {
    work_unit_id: String,
    sprint_id: String,
    authority_id: String,
    origin_attempt_id: String,
    origin_ordinal: i64,
    reporting_invocation_id: String,
    reporting_revision_id: String,
    reporting_configuration_digest: String,
    reporting_repository_commit_ref: String,
    review_invocation_id: String,
    decision_fingerprint: String,
    return_reason_json: String,
    summary: String,
    validation: String,
    manifest_json: String,
    comparison_fingerprint: String,
    content_fingerprints_json: String,
}

fn handler_review_payload(
    summary: &str,
    validation_statement: &str,
    manifest_json: &str,
    comparison_fingerprint: &str,
    content_fingerprints_json: &str,
) -> Result<serde_json::Value, SprintRunnerTransitionError> {
    Ok(serde_json::json!({
        "summary": summary,
        "validationStatement": validation_statement,
        "changedFiles": serde_json::from_str::<serde_json::Value>(manifest_json)
            .map_err(|_| SprintRunnerTransitionError::Conflict)?,
        "comparisonFingerprint": comparison_fingerprint,
        "evidenceContentFingerprints": serde_json::from_str::<serde_json::Value>(content_fingerprints_json)
            .map_err(|_| SprintRunnerTransitionError::Conflict)?,
    }))
}

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
    pub(crate) sprint_decision_state: Option<String>,
    pub(crate) sprint_decision_recorded_at: Option<String>,
    pub(crate) sprint_upward_result_recorded_at: Option<String>,
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
    database_lock_key: String,
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
    #[cfg(test)]
    test_work_unit_handler_post_pass_hook: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl SprintRunnerTransitionService {
    pub(crate) fn open(
        path: impl AsRef<Path>,
        sessions: Arc<AgentSessionApplication>,
    ) -> Result<Arc<Self>, SprintRunnerTransitionError> {
        let path = path.as_ref();
        let database_lock_key = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .into_owned();
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
        connection.execute_batch(ACCEPTED_CANDIDATE_AUTHORITY_SCHEMA).map_err(|e| SprintRunnerTransitionError::Unavailable(format!("initialize accepted candidate authority schema: {e}")))?;
        connection.execute_batch(crate::orchestration::work_unit_dependency_wave::WORK_UNIT_DEPENDENCY_WAVE_SCHEMA).map_err(|e| SprintRunnerTransitionError::Unavailable(format!("initialize dependency-wave schema: {e}")))?;
        sprint_continuation_settlement::initialize(&connection).map_err(SprintRunnerTransitionError::Unavailable)?;
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
        ensure_handler_action_failure_reason(&connection)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        ensure_implementer_outcome_evidence_columns(&connection)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        migrate_work_unit_attempt_history(&connection)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        migrate_work_unit_retry_attempt_history(&connection)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        migrate_handler_decision_retry_contract(&connection)
            .map_err(SprintRunnerTransitionError::Unavailable)?;
        let authority_repository = SqliteOrchestrationRepository::open(path)
            .map_err(|error| SprintRunnerTransitionError::Unavailable(format!("open Sprint Git authority repository: {error}")))?;
        let service = Arc::new(Self {
            connection: Mutex::new(connection),
            database_lock_key,
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
            #[cfg(test)]
            test_work_unit_handler_post_pass_hook: Mutex::new(None),
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
    pub(crate) fn attach_reporting_test_harness(
        &self,
        handler: Arc<WorkUnitExecutionHarnessService>,
    ) {
        *self.work_unit_handler.lock().unwrap() = Some(handler);
    }

    #[cfg(test)]
    pub(crate) fn reconcile_reporting_for_test(
        &self,
    ) -> Result<(), SprintRunnerTransitionError> {
        self.reconcile_implementer_outcomes_v3()
    }

    #[cfg(test)]
    pub(crate) fn reconcile_handler_reviews_for_test(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        self.reconcile_handler_reviews()
    }
    #[cfg(test)]
    pub(crate) fn reconcile_handler_review_terminal_movement_for_test(
        self: &Arc<Self>,
        review_invocation_id: &str,
    ) -> Result<(), SprintRunnerTransitionError> {
        self.reconcile_handler_review_terminal_movement(review_invocation_id, "completed")
    }
    #[cfg(test)]
    pub(crate) fn reconcile_no_progress_handbacks_for_test(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        self.reconcile_no_progress_handbacks()
    }
    #[cfg(test)] pub(crate) fn reconcile_epic_escalation_receivers_for_test(self:&Arc<Self>)->Result<(),SprintRunnerTransitionError>{self.reconcile_epic_escalation_receivers()}
    #[cfg(test)] pub(crate) fn epic_escalation_reassessment_context_for_test(&self,invocation_id:&str)->Result<serde_json::Value,SprintRunnerTransitionError>{let invocation=AgentInvocationId::new(invocation_id.to_owned()).map_err(|_|SprintRunnerTransitionError::Forbidden)?;self.epic_escalation_reassessment_context(&invocation)}
    #[cfg(test)] pub(crate) fn record_epic_escalation_disposition_for_test(self:&Arc<Self>,invocation_id:&str,disposition:EpicEscalationReassessmentDisposition)->Result<(),SprintRunnerTransitionError>{let invocation=AgentInvocationId::new(invocation_id.to_owned()).map_err(|_|SprintRunnerTransitionError::Forbidden)?;self.record_epic_escalation_disposition(&invocation,disposition)}

    #[cfg(test)]
    pub(crate) fn prepare_later_attempt_reporting_for_test(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        self.reconcile_implementer_reporting_continuations()
    }

    #[cfg(test)]
    pub(crate) fn reconcile_later_attempt_for_test(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        self.reconcile_implementer_outcomes_v3()?;
        self.reconcile_handler_reviews()
    }

    #[cfg(test)]
    pub(crate) fn handler_review_evidence_for_test(
        &self,
        invocation_id: &str,
    ) -> Result<String, SprintRunnerTransitionError> {
        let invocation = AgentInvocationId::new(invocation_id.to_owned())
            .map_err(|_| SprintRunnerTransitionError::Forbidden)?;
        Ok(self.handler_review_context(&invocation, true)?.delivered_payload_json)
    }

    #[cfg(test)]
    pub(crate) fn record_handler_review_judgment_for_test(
        &self,
        invocation_id: &str,
        variant: &str,
        reason: Option<HandlerReviewReturnReason>,
    ) -> Result<(), SprintRunnerTransitionError> {
        let invocation = AgentInvocationId::new(invocation_id.to_owned())
            .map_err(|_| SprintRunnerTransitionError::Forbidden)?;
        self.record_handler_review_judgment(&invocation, variant, reason)
    }
    #[cfg(test)]
    pub(crate) fn record_handler_incomplete_disposition_for_test(
        &self,
        invocation_id: &str,
        disposition: HandlerReviewIncompleteDisposition,
    ) -> Result<(), SprintRunnerTransitionError> {
        let invocation = AgentInvocationId::new(invocation_id.to_owned())
            .map_err(|_| SprintRunnerTransitionError::Forbidden)?;
        self.record_handler_incomplete_disposition(&invocation, disposition)
    }
    #[cfg(test)]
    pub(crate) fn record_handback_disposition_for_test(
        self: &Arc<Self>,
        invocation_id: &str,
        disposition: SprintHandbackDisposition,
    ) -> Result<(), SprintRunnerTransitionError> {
        let invocation = AgentInvocationId::new(invocation_id.to_owned())
            .map_err(|_| SprintRunnerTransitionError::Forbidden)?;
        self.record_handback_disposition(&invocation, disposition)
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
    fn prepare_handback_reassessment_action(self: &Arc<Self>, invocation_id: AgentInvocationId) -> Result<CodexMcpInjection, SprintRunnerTransitionError> {
        let bearer=uuid::Uuid::new_v4().simple().to_string();let server=start_handback_reassessment_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start Handback reassessment action server: {e}")))?;let injection=CodexMcpInjection::new_named("sprint_runner_handback_reassessment",&server.url(),bearer,&["read_sprint_handback_reassessment_context".into(),"record_sprint_handback_disposition".into()],true);let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Sprint Runner handback action registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)
    }
    fn prepare_epic_escalation_reassessment_action(self:&Arc<Self>,invocation_id:AgentInvocationId)->Result<CodexMcpInjection,SprintRunnerTransitionError>{let bearer=uuid::Uuid::new_v4().simple().to_string();let server=start_epic_escalation_reassessment_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start Epic escalation receiver server: {e}")))?;let injection=CodexMcpInjection::new_named("epic_runner_escalation_reassessment",&server.url(),bearer,&["read_epic_escalation_reassessment_context".into(),"record_epic_escalation_disposition".into()],true);let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Epic escalation receiver registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)}
    fn prepare_work_slice_planner_action(self: &Arc<Self>, invocation_id: AgentInvocationId) -> Result<CodexMcpInjection, SprintRunnerTransitionError> {
        let bearer=uuid::Uuid::new_v4().simple().to_string();
        let server=start_work_slice_planner_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start Work Slice Planner action server: {e}")))?;
        let injection=CodexMcpInjection::new_named("work_slice_planner",&server.url(),bearer,&["read_current_planning_context".into(),"submit_work_slice_proposal".into(),"request_work_slice_refinement".into(),"complete_work_slice_planning".into()],true);
        let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Slice Planner action registry is poisoned".into()))?;
        if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}
        active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)
    }
    fn prepare_work_unit_handler_action(self:&Arc<Self>,invocation_id:AgentInvocationId)->Result<CodexMcpInjection,SprintRunnerTransitionError>{let bearer=uuid::Uuid::new_v4().simple().to_string();let server=start_work_unit_handler_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start Work Unit Handler action server: {e}")))?;let injection=CodexMcpInjection::new_named("work_unit_handler",&server.url(),bearer,&["request_work_unit_implementer".into()],true);let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Handler action registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)}
    fn prepare_work_unit_implementer_reporting_action(self:&Arc<Self>,invocation_id:AgentInvocationId)->Result<CodexMcpInjection,SprintRunnerTransitionError>{let bearer=uuid::Uuid::new_v4().simple().to_string();let server=start_work_unit_implementer_reporting_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start Work Unit Implementer reporting server: {e}")))?;let injection=CodexMcpInjection::new_named("work_unit_implementer_reporting",&server.url(),bearer,&["submit_implementation_outcome".into(),"complete_implementation_outcome".into()],true);let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Implementer reporting registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)}
    fn prepare_work_unit_handler_review_action(self:&Arc<Self>,invocation_id:AgentInvocationId)->Result<CodexMcpInjection,SprintRunnerTransitionError>{let bearer=uuid::Uuid::new_v4().simple().to_string();let server=start_work_unit_handler_review_server(self.clone(),invocation_id.clone(),bearer.clone(),vec!["tauri://localhost".into()]).map_err(|e|SprintRunnerTransitionError::Unavailable(format!("start Work Unit Handler review server: {e}")))?;let injection=CodexMcpInjection::new_named("work_unit_handler_review",&server.url(),bearer,&["read_handler_review_evidence".into(),"accept_implementation_outcome".into(),"return_implementation_outcome".into()],true);let mut active=self.mcp.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Handler review registry is poisoned".into()))?;if let Some(existing)=active.get(&invocation_id){let existing=existing.injection.clone();server.stop();return Ok(existing)}active.insert(invocation_id,ManagedEpicRunnerAction{server,injection:injection.clone()});Ok(injection)}


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
        self.reconcile_implementer_outcomes_v3()?;
        self.reconcile_handler_reviews()?;
        self.reconcile_work_unit_retries()?;
        self.reconcile_no_progress_handbacks()?;
        self.reconcile_epic_escalation_receivers()?;
        {
            let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
            reconcile_accepted_candidate_authorities(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            reconcile_accepted_integrations(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            reconcile_work_unit_dependency_wave(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            reconcile_work_slice_execution_settlement(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            sprint_continuation_settlement::reconcile(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
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
        let mut statement = conn.prepare("SELECT t.sprint_id,t.epic_id,t.request_id,t.epic_runner_invocation_id,t.sprint_runner_session_id,t.sprint_runner_invocation_id,t.requested_at,t.authorized_at,t.session_created_at,t.harness_applied_at,t.launch_accepted_at,t.pre_start_semantic_outcome_recorded_at,t.pre_start_lifecycle_observed_at,t.pre_start_outcome_accepted_at,t.parent_continuation_delivery_requested_at,t.parent_continuation_delivery_persisted_at,t.epic_continuation_invocation_id,t.epic_continuation_launch_accepted_at,t.provider_receiver_activation_observed_at,t.sprint_start_authorized_at,t.sprint_start_persisted_at,t.sprint_continuation_invocation_id,t.sprint_continuation_launch_accepted_at,t.repository_branch_reevaluation_recorded_at,t.started_reevaluation_lifecycle_observed_at,t.planning_control_delivery_requested_at,t.planning_control_delivery_persisted_at,t.planning_control_invocation_id,t.planning_control_launch_accepted_at,t.planning_ready_at,p.request_fact_id,p.requested_at,p.authorized_at,p.planning_point_id,p.repository_worktree_route,p.planner_harness_key,p.planner_harness_version,p.planner_session_id,p.planner_invocation_id,p.planner_session_created_at,p.planner_invocation_created_at,p.planner_harness_applied_at,p.planner_launch_requested_at,p.planner_launch_accepted_at,p.planner_ready_at,p.planner_provider_activation_observed_at,p.planner_lifecycle_observed_at,r.submitted_at,r.validation_result,r.refinement_requested_at,r.semantic_completed_at,r.lifecycle_observed_at,r.accepted_at,r.materialization_ready_at,c.decision_state,c.recorded_at,u.recorded_at FROM sprint_runner_transitions t LEFT JOIN work_slice_planning_requests p ON p.sprint_id=t.sprint_id AND p.is_current=1 LEFT JOIN work_slice_proposal_revisions r ON r.planning_point_id=p.planning_point_id AND r.is_current=1 LEFT JOIN sprint_continuation_current_decisions current ON current.sprint_id=t.sprint_id LEFT JOIN sprint_continuation_decisions c ON c.decision_id=current.decision_id LEFT JOIN sprint_upward_results u ON u.decision_id=c.decision_id ORDER BY t.requested_at,t.sprint_id").map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
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
                    sprint_decision_state: r.get(54)?,
                    sprint_decision_recorded_at: r.get(55)?,
                    sprint_upward_result_recorded_at: r.get(56)?,
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
        let retry_implementer: bool = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT EXISTS(SELECT 1 FROM work_unit_retry_attempts WHERE implementer_invocation_id=?1 AND retry_ready_at IS NOT NULL AND failure_reason IS NULL)",
            [invocation.id.as_str()], |row| row.get(0),
        ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if retry_implementer {
            if status != "completed" { return self.fail_retry_for_invocation(&invocation.id, "retry_implementer_lifecycle_not_completed"); }
            self.reconcile_implementer_reporting_continuations()?;
            return Ok(());
        }
        if self.record_reporting_lifecycle_v3(&invocation.id, status)? { self.reconcile_implementer_outcome_acceptance_v2()?; return self.reconcile_handler_reviews(); }
        let review:bool=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_reviews WHERE review_invocation_id=?1)",[invocation.id.as_str()],|row|row.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if review {
            self.record_handler_review_lifecycle(&invocation.id,status)?;
            self.finalize_handler_review_decisions()?;
            self.reconcile_work_unit_retries()?;
            return self.reconcile_handler_review_terminal_movement(invocation.id.as_str(), status);
        }
        let handback: Option<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT handback_id FROM sprint_runner_handback_deliveries WHERE reassessment_invocation_id=?1", [invocation.id.as_str()], |row| row.get(0)).optional().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if let Some(handback) = handback {
            let now = chrono::Utc::now().to_rfc3339();
            self.mark_handback_delivery(&handback, "provider_activation_observed_at")?;
            self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE sprint_runner_handback_deliveries SET reassessment_lifecycle_status=COALESCE(reassessment_lifecycle_status,?2),reassessment_lifecycle_observed_at=COALESCE(reassessment_lifecycle_observed_at,?3) WHERE handback_id=?1 AND (reassessment_lifecycle_status IS NULL OR reassessment_lifecycle_status=?2)",params![handback,status,now]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            return Ok(());
        }
        let escalation: Option<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT handback_id FROM epic_runner_escalation_receivers WHERE reassessment_invocation_id=?1",[invocation.id.as_str()],|row|row.get(0)).optional().map_err(|error|SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if escalation.is_some() { return self.observe_epic_escalation_receiver_terminals(); }
        let unblocked_escalation: bool = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM epic_runner_escalation_receivers WHERE governing_runner_session_id=?1 AND launch_accepted_at IS NULL)",[invocation.session_id.as_str()],|row|row.get(0)).map_err(|error|SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if unblocked_escalation { return self.reconcile_epic_escalation_receivers(); }
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
        {
            let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
            reconcile_work_unit_dependency_wave(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            reconcile_work_slice_execution_settlement(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
        }
        let Some(handler) = self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone() else { return Ok(()) };
        // A pass can settle accepted integration effects and make a later dependency generation
        // eligible. Drain one follow-up generation in this activation; later asynchronous
        // Handler/Implementer outcomes require their own durable notification or reopen pass.
        for _generation in 0..2 {
            let units = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare(
                "SELECT u.work_unit_id,u.materialization_id,m.sprint_id FROM work_units u JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id WHERE m.settled_at IS NOT NULL ORDER BY m.sprint_id,u.lane_ordinal,u.work_unit_id"
            ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            for (work_unit, materialization, sprint) in units {
                self.reconcile_work_unit_handler(&handler, &work_unit, &materialization, &sprint)?;
            }
            self.reconcile_implementer_activations()?;
            self.reconcile_work_unit_retries()?;
            #[cfg(test)]
            if let Some(hook) = self
                .test_work_unit_handler_post_pass_hook
                .lock()
                .expect("test work unit handler post-pass hook")
                .take()
            {
                hook();
            }
            let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
            reconcile_accepted_candidate_authorities(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            reconcile_accepted_integrations(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            reconcile_work_unit_dependency_wave(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            reconcile_work_slice_execution_settlement(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            let next_generation: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM work_unit_dependency_activation_intents i LEFT JOIN work_unit_handler_activations h ON h.work_unit_id=i.work_unit_id AND h.materialization_id=i.materialization_id WHERE i.eligibility_state='eligible' AND (h.work_unit_id IS NULL OR h.launch_accepted_at IS NULL))",
                [], |row| row.get(0),
            ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
            if !next_generation { break; }
        }
        Ok(())
    }

    /// A completed accepted review can change dependent eligibility after the integration/wave
    /// pass. Reconcile only missing or partial Handler activations for that durable graph here.
    /// Unrelated Implementer, retry, Handback, and higher-runner lifecycles retain their own
    /// reconciliation boundaries.
    fn reconcile_newly_eligible_work_unit_handlers(
        self: &Arc<Self>,
    ) -> Result<(), SprintRunnerTransitionError> {
        let Some(handler) = self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone() else { return Ok(()) };
        for _generation in 0..2 {
            let units = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare(
                "SELECT u.work_unit_id,u.materialization_id,m.sprint_id
                 FROM work_units u
                 JOIN work_unit_materializations m ON m.materialization_id=u.materialization_id
                 JOIN work_unit_dependency_activation_intents i ON i.work_unit_id=u.work_unit_id AND i.materialization_id=u.materialization_id
                 LEFT JOIN work_unit_handler_activations h ON h.work_unit_id=u.work_unit_id AND h.materialization_id=u.materialization_id
                 WHERE m.settled_at IS NOT NULL
                   AND (h.work_unit_id IS NULL OR h.launch_accepted_at IS NULL OR h.handler_ready_at IS NULL)
                 ORDER BY m.sprint_id,u.lane_ordinal,u.work_unit_id"
            ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?.query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            for (work_unit, materialization, sprint) in &units {
                self.reconcile_work_unit_handler(&handler, work_unit, materialization, sprint)?;
            }
            #[cfg(test)]
            if let Some(hook) = self.test_work_unit_handler_post_pass_hook.lock().expect("test Work Unit Handler post-pass hook").take() { hook(); }
            let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
            reconcile_work_unit_dependency_wave(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            reconcile_work_slice_execution_settlement(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
            if units.is_empty() { break; }
        }
        Ok(())
    }

    fn reconcile_handler_review_terminal_movement(
        self: &Arc<Self>,
        review_invocation_id: &str,
        status: &str,
    ) -> Result<(), SprintRunnerTransitionError> {
        // A non-completed terminal cannot authorize either acceptance or return movement.
        // Its durable lifecycle remains available for diagnosis and later explicit action.
        if status != "completed" {
            return Ok(());
        }
        let (judgment_recorded, decision): (bool, Option<String>) = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT r.semantic_judgment_at IS NOT NULL,d.decision_variant
             FROM work_unit_handler_reviews r
             LEFT JOIN work_unit_handler_decisions d ON d.review_invocation_id=r.review_invocation_id
             WHERE r.review_invocation_id=?1",
            [review_invocation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        match (judgment_recorded, decision.as_deref()) {
            // Completion without a semantic judgment has no movement authority.
            (false, None) => Ok(()),
            // Accepted integration can make a dependent eligible in the finalization pass.
            // Drain it in this same notification activation; no transcript or terminal label
            // supplies the authority.
            (true, Some("accepted")) => self.reconcile_newly_eligible_work_unit_handlers(),
            // A returned review remains unsettled. Reconcile only its retry/Handback route;
            // Handback delivery is not graph completion or later settlement authority.
            (true, Some("returned")) => self.reconcile_no_progress_handbacks(),
            _ => Err(SprintRunnerTransitionError::Conflict),
        }
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
            self.request_work_unit_implementer_inner(&invocation, false)?;
        }
        self.reconcile_implementer_reporting_continuations()?;
        Ok(())
    }

    /// The v2 Implementer is historical and actionless.  Only after its exact durable terminal
    /// invocation has completed does the application reserve one same-Session reporting
    /// continuation and one later immutable reporting revision.  Launch wiring deliberately
    /// remains a later step; this function cannot alter or relaunch the original invocation.
    fn reconcile_implementer_reporting_continuations(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        let Some(handler) = self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone() else { return Ok(()) };
        let activations = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .prepare("SELECT work_unit_id,attempt_id,0,implementer_session_id,implementer_invocation_id FROM work_unit_implementer_activations WHERE implementer_ready_at IS NOT NULL UNION ALL SELECT work_unit_id,retry_attempt_id,ordinal,implementer_session_id,implementer_invocation_id FROM work_unit_retry_attempts WHERE retry_ready_at IS NOT NULL AND failure_reason IS NULL ORDER BY 1,3,2")
            .and_then(|mut statement| statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for (work_unit, attempt, ordinal, session_id, invocation_id) in activations {
            let session = AgentSessionId::new(session_id.clone()).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            let history = self.sessions.load_session(&session).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            let completed = history.invocations.iter().any(|entry| entry.invocation.id.as_str() == invocation_id && entry.invocation.status == AgentInvocationStatus::Completed);
            if !completed { continue; }
            let reporting = handler.current_implementer_reporting_revision().map_err(|_| SprintRunnerTransitionError::Unavailable("immutable Implementer reporting Harness revision unavailable".into()))?;
            let reporting_invocation = stable_id("work-unit-implementer-reporting-invocation", &attempt);
            let changed = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(
                "INSERT OR IGNORE INTO work_unit_implementer_outcomes (work_unit_id,attempt_id,attempt_ordinal,implementer_session_id,implementer_invocation_id,reporting_invocation_id,reporting_harness_revision_id,reporting_harness_configuration_digest,reporting_harness_repository_commit_ref,reporting_requested_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![work_unit,attempt,ordinal,session_id,invocation_id,reporting_invocation,reporting.revision_id,reporting.configuration_digest,reporting.repository_commit_ref,chrono::Utc::now().to_rfc3339()],
            ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            if changed == 0 {
                let exact: bool = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
                    "SELECT EXISTS(SELECT 1 FROM work_unit_implementer_outcomes WHERE work_unit_id=?1 AND attempt_id=?2 AND attempt_ordinal=?3 AND implementer_session_id=?4 AND implementer_invocation_id=?5 AND reporting_invocation_id=?6)",
                    params![work_unit,attempt,ordinal,session_id,invocation_id,reporting_invocation], |row| row.get(0),
                ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if !exact { return Err(SprintRunnerTransitionError::Conflict); }
            }
            let (stored_attempt,stored_session,stored_original,stored_reporting,revision,digest,commit):(String,String,String,String,String,String,String)=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT attempt_id,implementer_session_id,implementer_invocation_id,reporting_invocation_id,reporting_harness_revision_id,reporting_harness_configuration_digest,reporting_harness_repository_commit_ref FROM work_unit_implementer_outcomes WHERE attempt_id=?1",[&attempt],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?))).map_err(|_|SprintRunnerTransitionError::Conflict)?;
            if stored_attempt!=attempt || stored_session!=session_id || stored_original!=invocation_id || stored_reporting!=stable_id("work-unit-implementer-reporting-invocation",&attempt) { return Err(SprintRunnerTransitionError::Conflict) }
            let reporting=handler.load_pinned_implementer_revision(&revision,&digest,&commit).map_err(|_|SprintRunnerTransitionError::Conflict)?;
            if !reporting.profile.mcp.required || reporting.profile.mcp.enabled_tools != ["submit_implementation_outcome","complete_implementation_outcome"] { return Err(SprintRunnerTransitionError::Conflict) }
            let package=handler.construct_for_pinned_profile(&attempt,WorkUnitHarnessRole::Implementer,reporting.profile).map_err(|_|SprintRunnerTransitionError::Conflict)?;
            let reporting_invocation=AgentInvocationId::new(stored_reporting).map_err(|_|SprintRunnerTransitionError::Conflict)?;
            let injection=self.prepare_work_unit_implementer_reporting_action(reporting_invocation.clone())?;
            let mut runtime=package.runtime_launch_configuration();runtime.extension.additional_args.extend(injection.configuration_args);runtime.extension.environment.push(injection.environment);
            let prompt="Work Unit Implementer reporting continuation. Submit exactly one ReviewPending outcome with summary and validationStatement claims, then semantically complete it. Claims are not evidence; tool success is not application acceptance or Handler review. Do not move later workflow.".to_string();
            self.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:reporting_invocation.clone(),message:SendAgentSessionMessageCommand{session_id:Some(session.clone()),submitted_text:prompt.clone(),title:None,working_directory:Some(package.working_directory().into()),requested_options:Some(runtime.requested_options.clone())}}).map_err(|error|SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            self.mark_reporting(&attempt,"reporting_prepared_at")?;
            package.bind_correlated_invocation(session.clone(),reporting_invocation.clone()).map_err(|_|SprintRunnerTransitionError::Conflict)?;
            self.mark_reporting(&attempt,"reporting_harness_bound_at")?;
            match self.sessions.application_invocation_launch_evidence(&reporting_invocation,&session).map_err(|error|SprintRunnerTransitionError::Unavailable(error.to_string()))? { ApplicationInvocationLaunchEvidence::LaunchAccepted=>{self.mark_reporting(&attempt,"reporting_launch_accepted_at")?;self.mark_reporting(&attempt,"reporting_ready_at")?},ApplicationInvocationLaunchEvidence::PersistedNotAccepted=>{self.mark_reporting(&attempt,"reporting_launch_requested_at")?;let launch=self.sessions.launch_prepared_application_invocation_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:reporting_invocation,message:SendAgentSessionMessageCommand{session_id:Some(session),submitted_text:prompt,title:None,working_directory:Some(package.working_directory().into()),requested_options:Some(runtime.requested_options)}},Some(runtime.extension)).map_err(|error|SprintRunnerTransitionError::Unavailable(error.to_string()))?;if launch.launch_accepted{self.mark_reporting(&attempt,"reporting_launch_accepted_at")?;self.mark_reporting(&attempt,"reporting_ready_at")?}else{return Err(SprintRunnerTransitionError::Unavailable("Implementer reporting launch was not accepted".into()))}},ApplicationInvocationLaunchEvidence::NeverPersisted=>return Err(SprintRunnerTransitionError::Conflict) }
        }
        Ok(())
    }

    fn mark_reporting(&self,attempt:&str,column:&str)->Result<(),SprintRunnerTransitionError>{if !["reporting_prepared_at","reporting_harness_bound_at","reporting_launch_requested_at","reporting_launch_accepted_at","reporting_ready_at"].contains(&column){return Err(SprintRunnerTransitionError::Unavailable("invalid reporting stage".into()))}self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(&format!("UPDATE work_unit_implementer_outcomes SET {column}=COALESCE({column},?2) WHERE attempt_id=?1"),params![attempt,chrono::Utc::now().to_rfc3339()]).map_err(|error|SprintRunnerTransitionError::Unavailable(error.to_string()))?;Ok(())}

    fn reporting_row_for_invocation(&self,invocation:&AgentInvocationId)->Result<(String,String,String,String,String),SprintRunnerTransitionError>{let row:Option<(String,String,String,String,String,String,String,String,String,String,String)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT o.work_unit_id,o.attempt_id,o.implementer_session_id,o.implementer_invocation_id,o.reporting_invocation_id,o.reporting_harness_revision_id,o.reporting_harness_configuration_digest,o.reporting_harness_repository_commit_ref,a.attempt_id,a.implementer_session_id,a.implementer_invocation_id FROM work_unit_implementer_outcomes o JOIN work_unit_implementer_activations a ON a.work_unit_id=o.work_unit_id WHERE o.reporting_invocation_id=?1 AND a.launch_accepted_at IS NOT NULL AND a.implementer_ready_at IS NOT NULL AND o.reporting_prepared_at IS NOT NULL AND o.reporting_harness_bound_at IS NOT NULL AND o.reporting_launch_accepted_at IS NOT NULL AND o.reporting_ready_at IS NOT NULL AND o.application_accepted_at IS NULL",[invocation.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let Some((unit,attempt,session,original,reporting,revision,digest,commit,activation_attempt,activation_session,activation_original))=row else{return Err(SprintRunnerTransitionError::Forbidden)};if attempt!=activation_attempt||session!=activation_session||original!=activation_original||reporting!=stable_id("work-unit-implementer-reporting-invocation",&attempt)||reporting!=invocation.as_str(){return Err(SprintRunnerTransitionError::Conflict)};let handler=self.work_unit_handler.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone().ok_or(SprintRunnerTransitionError::Forbidden)?;let pinned=handler.load_pinned_implementer_revision(&revision,&digest,&commit).map_err(|_|SprintRunnerTransitionError::Conflict)?;if !pinned.profile.mcp.required||pinned.profile.mcp.enabled_tools!=["submit_implementation_outcome","complete_implementation_outcome"]{return Err(SprintRunnerTransitionError::Conflict)};handler.construct_for_pinned_profile(&attempt,WorkUnitHarnessRole::Implementer,pinned.profile).map_err(|_|SprintRunnerTransitionError::Forbidden)?;let history=self.sessions.load_session(&AgentSessionId::new(session.clone()).map_err(|_|SprintRunnerTransitionError::Forbidden)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let original_completed=history.invocations.iter().any(|entry|entry.invocation.id.as_str()==original&&entry.invocation.status==AgentInvocationStatus::Completed);let reporting_live=history.invocations.iter().any(|entry|entry.invocation.id==*invocation&&!entry.invocation.status.is_terminal());if !original_completed||!reporting_live{return Err(SprintRunnerTransitionError::Forbidden)};Ok((unit,attempt,session,original,reporting))}
    fn legacy_submit_implementation_outcome(&self,invocation:&AgentInvocationId,input:ImplementationOutcomeClaims)->Result<(),SprintRunnerTransitionError>{validate_outcome(&input.summary)?;validate_outcome(&input.validation_statement)?;let (unit,_attempt,session,_original,stored)=self.reporting_row_for_invocation(invocation)?;if stored!=invocation.as_str(){return Err(SprintRunnerTransitionError::Forbidden)};let history=self.sessions.load_session(&AgentSessionId::new(session).map_err(|_|SprintRunnerTransitionError::Forbidden)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !history.invocations.iter().any(|entry|entry.invocation.id==*invocation && !entry.invocation.status.is_terminal()){return Err(SprintRunnerTransitionError::Forbidden)};let payload=serde_json::to_string(&input).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let fingerprint=stable_id("implementer-outcome",&payload);let now=chrono::Utc::now().to_rfc3339();let changed=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_outcomes SET outcome_variant='review_pending',submitted_summary=?2,submitted_validation_statement=?3,semantic_payload_json=?4,submission_fingerprint=?5,submitted_at=?6,validation_at=?6,validation_result='valid' WHERE work_unit_id=?1 AND submitted_at IS NULL",params![unit,input.summary,input.validation_statement,payload,fingerprint,now]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed==0{let exact:bool=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_implementer_outcomes WHERE work_unit_id=?1 AND submission_fingerprint=?2)",params![unit,fingerprint],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !exact{return Err(SprintRunnerTransitionError::Conflict)}};Ok(())}
    fn legacy_complete_implementation_outcome(&self,invocation:&AgentInvocationId)->Result<(),SprintRunnerTransitionError>{let(unit,_attempt,session,_original,stored)=self.reporting_row_for_invocation(invocation)?;if stored!=invocation.as_str(){return Err(SprintRunnerTransitionError::Forbidden)};let history=self.sessions.load_session(&AgentSessionId::new(session).map_err(|_|SprintRunnerTransitionError::Forbidden)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !history.invocations.iter().any(|entry|entry.invocation.id==*invocation && !entry.invocation.status.is_terminal()){return Err(SprintRunnerTransitionError::Forbidden)};let changed=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_outcomes SET semantic_completed_at=COALESCE(semantic_completed_at,?3),semantic_completion_invocation_id=COALESCE(semantic_completion_invocation_id,?2) WHERE work_unit_id=?1 AND validation_result='valid' AND submitted_at IS NOT NULL AND (semantic_completion_invocation_id IS NULL OR semantic_completion_invocation_id=?2)",params![unit,invocation.as_str(),chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed==1{Ok(())}else{Err(SprintRunnerTransitionError::Forbidden)}}

    fn reporting_context(&self,invocation:&AgentInvocationId,require_live:bool)->Result<ImplementerReportingContext,SprintRunnerTransitionError>{let row:Option<ImplementerReportingContext>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT o.work_unit_id,o.attempt_id,o.implementer_session_id,o.implementer_invocation_id,o.reporting_invocation_id,o.reporting_harness_revision_id,o.reporting_harness_configuration_digest,o.reporting_harness_repository_commit_ref FROM work_unit_implementer_outcomes o LEFT JOIN work_unit_implementer_activations a ON a.work_unit_id=o.work_unit_id AND a.attempt_id=o.attempt_id AND a.implementer_session_id=o.implementer_session_id AND a.implementer_invocation_id=o.implementer_invocation_id LEFT JOIN work_unit_retry_attempts r ON r.work_unit_id=o.work_unit_id AND r.retry_attempt_id=o.attempt_id AND r.implementer_session_id=o.implementer_session_id AND r.implementer_invocation_id=o.implementer_invocation_id WHERE o.reporting_invocation_id=?1 AND ((a.launch_accepted_at IS NOT NULL AND a.implementer_ready_at IS NOT NULL) OR (r.launch_accepted_at IS NOT NULL AND r.retry_ready_at IS NOT NULL AND r.failure_reason IS NULL)) AND o.reporting_prepared_at IS NOT NULL AND o.reporting_harness_bound_at IS NOT NULL AND o.reporting_launch_accepted_at IS NOT NULL AND o.reporting_ready_at IS NOT NULL",[invocation.as_str()],|r|Ok(ImplementerReportingContext{work_unit_id:r.get(0)?,attempt_id:r.get(1)?,session_id:r.get(2)?,implementer_invocation_id:r.get(3)?,reporting_invocation_id:r.get(4)?,revision_id:r.get(5)?,configuration_digest:r.get(6)?,repository_commit_ref:r.get(7)?})).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let context=row.ok_or(SprintRunnerTransitionError::Forbidden)?;if context.reporting_invocation_id!=invocation.as_str()||context.reporting_invocation_id!=stable_id("work-unit-implementer-reporting-invocation",&context.attempt_id){return Err(SprintRunnerTransitionError::Conflict)}let handler=self.work_unit_handler.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone().ok_or(SprintRunnerTransitionError::Forbidden)?;let pinned=handler.load_pinned_implementer_revision(&context.revision_id,&context.configuration_digest,&context.repository_commit_ref).map_err(|_|SprintRunnerTransitionError::Conflict)?;if !pinned.profile.mcp.required||pinned.profile.mcp.enabled_tools!=["submit_implementation_outcome","complete_implementation_outcome"]{return Err(SprintRunnerTransitionError::Conflict)}handler.construct_for_pinned_profile(&context.attempt_id,WorkUnitHarnessRole::Implementer,pinned.profile).map_err(|_|SprintRunnerTransitionError::Forbidden)?;let history=self.sessions.load_session(&AgentSessionId::new(context.session_id.clone()).map_err(|_|SprintRunnerTransitionError::Forbidden)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let original=history.invocations.iter().any(|entry|entry.invocation.id.as_str()==context.implementer_invocation_id&&entry.invocation.status==AgentInvocationStatus::Completed);let reporting=history.invocations.iter().find(|entry|entry.invocation.id==*invocation).ok_or(SprintRunnerTransitionError::Forbidden)?;if !original||(require_live&&reporting.invocation.status.is_terminal()){return Err(SprintRunnerTransitionError::Forbidden)}Ok(context)}
    fn evidence_snapshot(&self,invocation:&AgentInvocationId,require_live:bool)->Result<(ImplementerReportingContext,ImplementationEvidenceSnapshot),SprintRunnerTransitionError>{let context=self.reporting_context(invocation,require_live)?;let handler=self.work_unit_handler.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone().ok_or(SprintRunnerTransitionError::Forbidden)?;let pinned=handler.load_pinned_implementer_revision(&context.revision_id,&context.configuration_digest,&context.repository_commit_ref).map_err(|_|SprintRunnerTransitionError::Conflict)?;let package=handler.construct_for_pinned_profile(&context.attempt_id,WorkUnitHarnessRole::Implementer,pinned.profile).map_err(|_|SprintRunnerTransitionError::Forbidden)?;package.bind_correlated_invocation(AgentSessionId::new(context.session_id.clone()).map_err(|_|SprintRunnerTransitionError::Forbidden)?,invocation.clone()).map_err(|_|SprintRunnerTransitionError::Conflict)?;let manifest=package.changed_file_manifest().map_err(|_|SprintRunnerTransitionError::Forbidden)?;if manifest.is_empty()||manifest.len()>500{return Err(SprintRunnerTransitionError::Forbidden)}let comparison=package.comparison().map_err(|_|SprintRunnerTransitionError::Forbidden)?;if comparison.is_empty()||comparison.len()>256_000{return Err(SprintRunnerTransitionError::Forbidden)}let mut manifest_values=Vec::with_capacity(manifest.len());let mut contents=Vec::with_capacity(manifest.len());for entry in manifest{if !safe_id(&entry.evidence_ref)||entry.display_name.trim().is_empty()||entry.change_kind.trim().is_empty(){return Err(SprintRunnerTransitionError::Forbidden)}let content=package.evidence_content(&entry.evidence_ref).map_err(|_|SprintRunnerTransitionError::Forbidden)?;if content.is_empty()||content.len()>256_000{return Err(SprintRunnerTransitionError::Forbidden)}let reference=entry.evidence_ref;manifest_values.push(serde_json::json!({"evidenceRef":reference.clone(),"displayName":entry.display_name,"changeKind":entry.change_kind}));contents.push(serde_json::json!({"evidenceRef":reference,"contentFingerprint":fingerprint_bytes("implementer-evidence-content",&content)}));}manifest_values.sort_by(|a,b|a["evidenceRef"].as_str().cmp(&b["evidenceRef"].as_str()));contents.sort_by(|a,b|a["evidenceRef"].as_str().cmp(&b["evidenceRef"].as_str()));let capture_authorization_id=package.capture_authorization_id().map_err(|_|SprintRunnerTransitionError::Forbidden)?;Ok((context,ImplementationEvidenceSnapshot{manifest_json:serde_json::to_string(&manifest_values).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?,comparison_fingerprint:fingerprint_bytes("implementer-evidence-comparison",&comparison),content_fingerprints_json:serde_json::to_string(&contents).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?,capture_authorization_id}))}
    fn capture_implementation_evidence(&self,invocation:&AgentInvocationId)->Result<(),SprintRunnerTransitionError>{let(context,snapshot)=self.evidence_snapshot(invocation,true)?;let mut connection=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;let transaction=connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let existing:(Option<String>,Option<String>,Option<String>,Option<String>,Option<String>)=transaction.query_row("SELECT evidence_manifest_json,comparison_fingerprint,evidence_content_fingerprints_json,file_review_capture_authorization_id,evidence_ready_at FROM work_unit_implementer_outcomes WHERE work_unit_id=?1 AND reporting_invocation_id=?2",params![context.work_unit_id,invocation.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?.ok_or(SprintRunnerTransitionError::Forbidden)?;let identical=existing.0.as_deref()==Some(&snapshot.manifest_json)&&existing.1.as_deref()==Some(&snapshot.comparison_fingerprint)&&existing.2.as_deref()==Some(&snapshot.content_fingerprints_json)&&existing.3.as_deref()==Some(&snapshot.capture_authorization_id);if existing.4.is_some()&&identical{transaction.commit().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;return Ok(())}if existing.0.is_some()||existing.1.is_some()||existing.2.is_some()||existing.3.is_some()||existing.4.is_some(){if !identical{return Err(SprintRunnerTransitionError::Conflict)}}transaction.execute("UPDATE work_unit_implementer_outcomes SET evidence_manifest_json=?2,comparison_fingerprint=?3,evidence_content_fingerprints_json=?4,file_review_capture_authorization_id=?5,evidence_ready_at=COALESCE(evidence_ready_at,?6) WHERE work_unit_id=?1 AND reporting_invocation_id=?7",params![context.work_unit_id,snapshot.manifest_json,snapshot.comparison_fingerprint,snapshot.content_fingerprints_json,snapshot.capture_authorization_id,chrono::Utc::now().to_rfc3339(),invocation.as_str()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;transaction.commit().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))}
    pub(crate) fn submit_implementation_outcome(&self,invocation:&AgentInvocationId,input:ImplementationOutcomeClaims)->Result<(),SprintRunnerTransitionError>{validate_outcome(&input.summary)?;validate_outcome(&input.validation_statement)?;if input.outcome!=ImplementationOutcomeVariant::ReviewPending{return Err(SprintRunnerTransitionError::Invalid)}let context=self.reporting_context(invocation,true)?;let payload=serde_json::to_string(&input).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let fingerprint=stable_id("implementer-outcome",&payload);let changed=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_outcomes SET outcome_variant='review_pending',submitted_summary=?2,submitted_validation_statement=?3,semantic_payload_json=?4,submission_fingerprint=?5,submitted_at=?6,validation_at=?6,validation_result='valid' WHERE work_unit_id=?1 AND reporting_invocation_id=?7 AND submitted_at IS NULL",params![context.work_unit_id,input.summary,input.validation_statement,payload,fingerprint,chrono::Utc::now().to_rfc3339(),invocation.as_str()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed==0{let exact:bool=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_implementer_outcomes WHERE work_unit_id=?1 AND reporting_invocation_id=?2 AND submission_fingerprint=?3)",params![context.work_unit_id,invocation.as_str(),fingerprint],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !exact{return Err(SprintRunnerTransitionError::Conflict)}}Ok(())}
    pub(crate) fn complete_implementation_outcome(&self,invocation:&AgentInvocationId)->Result<(),SprintRunnerTransitionError>{let context=self.reporting_context(invocation,true)?;self.capture_implementation_evidence(invocation)?;let changed=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_outcomes SET semantic_completed_at=COALESCE(semantic_completed_at,?3),semantic_completion_invocation_id=COALESCE(semantic_completion_invocation_id,?2) WHERE work_unit_id=?1 AND reporting_invocation_id=?2 AND validation_result='valid' AND submitted_at IS NOT NULL AND evidence_ready_at IS NOT NULL AND (semantic_completion_invocation_id IS NULL OR semantic_completion_invocation_id=?2)",params![context.work_unit_id,invocation.as_str(),chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed==1{Ok(())}else{Err(SprintRunnerTransitionError::Forbidden)}}
    fn record_reporting_lifecycle(&self,invocation:&AgentInvocationId,status:&str)->Result<bool,SprintRunnerTransitionError>{let session:Option<String>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT implementer_session_id FROM work_unit_implementer_outcomes WHERE reporting_invocation_id=?1",[invocation.as_str()],|r|r.get(0)).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let Some(session)=session else{return Ok(false)};let history=self.sessions.load_session(&AgentSessionId::new(session).map_err(|_|SprintRunnerTransitionError::Forbidden)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let observed=history.invocations.iter().find(|entry|entry.invocation.id==*invocation).ok_or(SprintRunnerTransitionError::Forbidden)?;if !observed.invocation.status.is_terminal()||lifecycle_status(observed.invocation.status)!=status{return Err(SprintRunnerTransitionError::Conflict)}let changed=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_outcomes SET lifecycle_observed_at=COALESCE(lifecycle_observed_at,?2),lifecycle_status=COALESCE(lifecycle_status,?3) WHERE reporting_invocation_id=?1 AND (lifecycle_status IS NULL OR lifecycle_status=?3)",params![invocation.as_str(),chrono::Utc::now().to_rfc3339(),status]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed!=1{return Err(SprintRunnerTransitionError::Conflict)}Ok(true)}
    fn reconcile_implementer_outcomes(&self)->Result<(),SprintRunnerTransitionError>{let candidates:Vec<String>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare("SELECT reporting_invocation_id FROM work_unit_implementer_outcomes WHERE lifecycle_observed_at IS NULL ORDER BY work_unit_id").and_then(|mut s|s.query_map([],|r|r.get(0))?.collect::<Result<Vec<_>,_>>()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;for candidate in candidates{let invocation=AgentInvocationId::new(candidate).map_err(|_|SprintRunnerTransitionError::Conflict)?;let context=self.reporting_context(&invocation,false)?;let history=self.sessions.load_session(&AgentSessionId::new(context.session_id).map_err(|_|SprintRunnerTransitionError::Forbidden)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if let Some(entry)=history.invocations.iter().find(|entry|entry.invocation.id==invocation&&entry.invocation.status.is_terminal()){self.record_reporting_lifecycle(&invocation,lifecycle_status(entry.invocation.status))?;}}self.reconcile_implementer_outcome_acceptance()}
    fn reconcile_implementer_outcome_acceptance(&self)->Result<(),SprintRunnerTransitionError>{let candidates:Vec<(String,String,String,String,String,String)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare("SELECT work_unit_id,reporting_invocation_id,semantic_payload_json,evidence_manifest_json,comparison_fingerprint,evidence_content_fingerprints_json FROM work_unit_implementer_outcomes WHERE outcome_variant='review_pending' AND validation_result='valid' AND evidence_ready_at IS NOT NULL AND semantic_completed_at IS NOT NULL AND semantic_completion_invocation_id=reporting_invocation_id AND lifecycle_status='completed' ORDER BY work_unit_id").and_then(|mut s|s.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;for(unit,invocation_id,payload,manifest,comparison,contents)in candidates{let claims:ImplementationOutcomeClaims=serde_json::from_str(&payload).map_err(|_|SprintRunnerTransitionError::Conflict)?;if claims.outcome!=ImplementationOutcomeVariant::ReviewPending{return Err(SprintRunnerTransitionError::Conflict)}let invocation=AgentInvocationId::new(invocation_id).map_err(|_|SprintRunnerTransitionError::Conflict)?;let(context,actual)=self.evidence_snapshot(&invocation,false)?;if context.work_unit_id!=unit||actual.manifest_json!=manifest||actual.comparison_fingerprint!=comparison||actual.content_fingerprints_json!=contents{return Err(SprintRunnerTransitionError::Conflict)}let mut connection=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;let transaction=connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;transaction.execute("UPDATE work_unit_implementer_outcomes SET application_accepted_at=COALESCE(application_accepted_at,?2) WHERE work_unit_id=?1 AND reporting_invocation_id=?3 AND outcome_variant='review_pending' AND validation_result='valid' AND evidence_ready_at IS NOT NULL AND semantic_completed_at IS NOT NULL AND semantic_completion_invocation_id=?3 AND lifecycle_status='completed'",params![unit,chrono::Utc::now().to_rfc3339(),invocation.as_str()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;transaction.execute("UPDATE work_unit_implementer_outcomes SET handler_review_ready_at=COALESCE(handler_review_ready_at,?2) WHERE work_unit_id=?1 AND reporting_invocation_id=?3 AND application_accepted_at IS NOT NULL",params![unit,chrono::Utc::now().to_rfc3339(),invocation.as_str()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;transaction.commit().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;}Ok(())}

    fn record_reporting_lifecycle_v2(&self,invocation:&AgentInvocationId,status:&str)->Result<bool,SprintRunnerTransitionError>{let context=self.reporting_context(invocation,false)?;let history=self.sessions.load_session(&AgentSessionId::new(context.session_id).map_err(|_|SprintRunnerTransitionError::Forbidden)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let observed=history.invocations.iter().find(|entry|entry.invocation.id==*invocation).ok_or(SprintRunnerTransitionError::Forbidden)?;if !observed.invocation.status.is_terminal()||lifecycle_status(observed.invocation.status)!=status{return Err(SprintRunnerTransitionError::Conflict)}let changed=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_outcomes SET lifecycle_observed_at=COALESCE(lifecycle_observed_at,?2),lifecycle_status=COALESCE(lifecycle_status,?3) WHERE work_unit_id=?1 AND reporting_invocation_id=?4 AND (lifecycle_status IS NULL OR lifecycle_status=?3)",params![context.work_unit_id,chrono::Utc::now().to_rfc3339(),status,invocation.as_str()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed!=1{return Err(SprintRunnerTransitionError::Conflict)}Ok(true)}
    fn valid_accepted_outcome(&self,work_unit:&str,invocation:&AgentInvocationId)->Result<bool,SprintRunnerTransitionError>{let row:Option<(String,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>,Option<String>)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT outcome_variant,submitted_summary,submitted_validation_statement,semantic_payload_json,submission_fingerprint,submitted_at,validation_at,validation_result,semantic_completed_at,semantic_completion_invocation_id,lifecycle_observed_at,lifecycle_status FROM work_unit_implementer_outcomes WHERE work_unit_id=?1 AND reporting_invocation_id=?2",params![work_unit,invocation.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let Some((variant,summary,validation,payload,fingerprint,submitted,validated,result,completed,completion_invocation,lifecycle_observed,lifecycle_status))=row else{return Ok(false)};let(Some(summary),Some(validation),Some(payload),Some(fingerprint),Some(submitted),Some(validated),Some(result),Some(completed),Some(completion_invocation),Some(lifecycle_observed),Some(lifecycle_status))=(summary,validation,payload,fingerprint,submitted,validated,result,completed,completion_invocation,lifecycle_observed,lifecycle_status)else{return Ok(false)};if variant!="review_pending"||result!="valid"||completion_invocation!=invocation.as_str()||lifecycle_observed.is_empty()||lifecycle_status!="completed"||submitted.is_empty()||validated.is_empty()||completed.is_empty(){return Ok(false)}let claims:ImplementationOutcomeClaims=match serde_json::from_str(&payload){Ok(value)=>value,Err(_)=>return Ok(false)};let canonical=serde_json::to_string(&claims).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;Ok(claims.outcome==ImplementationOutcomeVariant::ReviewPending&&canonical==payload&&claims.summary==summary&&claims.validation_statement==validation&&fingerprint==stable_id("implementer-outcome",&payload))}
    fn reconcile_implementer_outcomes_v2(&self)->Result<(),SprintRunnerTransitionError>{let candidates:Vec<String>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare("SELECT o.reporting_invocation_id FROM work_unit_implementer_outcomes o JOIN work_unit_implementer_activations a ON a.work_unit_id=o.work_unit_id AND a.attempt_id=o.attempt_id AND a.implementer_session_id=o.implementer_session_id AND a.implementer_invocation_id=o.implementer_invocation_id WHERE o.lifecycle_observed_at IS NULL AND a.launch_accepted_at IS NOT NULL AND a.implementer_ready_at IS NOT NULL AND o.reporting_prepared_at IS NOT NULL AND o.reporting_harness_bound_at IS NOT NULL AND o.reporting_launch_accepted_at IS NOT NULL AND o.reporting_ready_at IS NOT NULL ORDER BY o.work_unit_id").and_then(|mut s|s.query_map([],|r|r.get(0))?.collect::<Result<Vec<_>,_>>()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;for candidate in candidates{let invocation=AgentInvocationId::new(candidate).map_err(|_|SprintRunnerTransitionError::Conflict)?;let context=self.reporting_context(&invocation,false)?;let history=self.sessions.load_session(&AgentSessionId::new(context.session_id).map_err(|_|SprintRunnerTransitionError::Forbidden)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if let Some(entry)=history.invocations.iter().find(|entry|entry.invocation.id==invocation&&entry.invocation.status.is_terminal()){self.record_reporting_lifecycle_v2(&invocation,lifecycle_status(entry.invocation.status))?;}}self.reconcile_implementer_outcome_acceptance_v2()}
    fn reconcile_implementer_outcome_acceptance_v2(&self)->Result<(),SprintRunnerTransitionError>{let candidates:Vec<(String,String,String,String,String)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare("SELECT work_unit_id,reporting_invocation_id,evidence_manifest_json,comparison_fingerprint,evidence_content_fingerprints_json FROM work_unit_implementer_outcomes WHERE evidence_ready_at IS NOT NULL AND semantic_completed_at IS NOT NULL AND semantic_completion_invocation_id=reporting_invocation_id AND lifecycle_observed_at IS NOT NULL AND lifecycle_status='completed' ORDER BY work_unit_id").and_then(|mut s|s.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;for(unit,invocation_id,manifest,comparison,contents)in candidates{let invocation=AgentInvocationId::new(invocation_id).map_err(|_|SprintRunnerTransitionError::Conflict)?;if !self.valid_accepted_outcome(&unit,&invocation)?{return Err(SprintRunnerTransitionError::Conflict)}let(context,actual)=self.evidence_snapshot(&invocation,false)?;if context.work_unit_id!=unit||actual.manifest_json!=manifest||actual.comparison_fingerprint!=comparison||actual.content_fingerprints_json!=contents{return Err(SprintRunnerTransitionError::Conflict)}let mut connection=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;let transaction=connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let now=chrono::Utc::now().to_rfc3339();transaction.execute("UPDATE work_unit_implementer_outcomes SET application_accepted_at=COALESCE(application_accepted_at,?2),handler_review_ready_at=COALESCE(handler_review_ready_at,?2) WHERE work_unit_id=?1 AND reporting_invocation_id=?3 AND lifecycle_observed_at IS NOT NULL AND lifecycle_status='completed' AND evidence_ready_at IS NOT NULL AND semantic_completed_at IS NOT NULL AND semantic_completion_invocation_id=?3",params![unit,now,invocation.as_str()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;transaction.commit().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;}Ok(())}

    fn record_reporting_lifecycle_v3(&self, invocation: &AgentInvocationId, status: &str) -> Result<bool, SprintRunnerTransitionError> {
        let exists: bool = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_implementer_outcomes WHERE reporting_invocation_id=?1)", [invocation.as_str()], |row| row.get(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if !exists { return Ok(false); }
        let context = self.reporting_context(invocation, false)?;
        let history = self.sessions.load_session(&AgentSessionId::new(context.session_id).map_err(|_| SprintRunnerTransitionError::Forbidden)?).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let observed = history.invocations.iter().find(|entry| entry.invocation.id == *invocation).ok_or(SprintRunnerTransitionError::Forbidden)?;
        if !observed.invocation.status.is_terminal() || lifecycle_status(observed.invocation.status) != status { return Err(SprintRunnerTransitionError::Conflict); }
        let changed = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_outcomes SET lifecycle_observed_at=COALESCE(lifecycle_observed_at,?2),lifecycle_status=COALESCE(lifecycle_status,?3) WHERE attempt_id=?1 AND reporting_invocation_id=?4 AND (lifecycle_status IS NULL OR lifecycle_status=?3)", params![context.attempt_id,chrono::Utc::now().to_rfc3339(),status,invocation.as_str()]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if changed != 1 { return Err(SprintRunnerTransitionError::Conflict); }
        Ok(true)
    }

    fn reconcile_implementer_outcomes_v3(&self) -> Result<(), SprintRunnerTransitionError> {
        let candidates: Vec<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .prepare("SELECT o.reporting_invocation_id FROM work_unit_implementer_outcomes o LEFT JOIN work_unit_implementer_activations a ON a.work_unit_id=o.work_unit_id AND a.attempt_id=o.attempt_id AND a.implementer_session_id=o.implementer_session_id AND a.implementer_invocation_id=o.implementer_invocation_id LEFT JOIN work_unit_retry_attempts r ON r.work_unit_id=o.work_unit_id AND r.retry_attempt_id=o.attempt_id AND r.implementer_session_id=o.implementer_session_id AND r.implementer_invocation_id=o.implementer_invocation_id WHERE o.lifecycle_observed_at IS NULL AND ((a.launch_accepted_at IS NOT NULL AND a.implementer_ready_at IS NOT NULL) OR (r.launch_accepted_at IS NOT NULL AND r.retry_ready_at IS NOT NULL AND r.failure_reason IS NULL)) AND o.reporting_prepared_at IS NOT NULL AND o.reporting_harness_bound_at IS NOT NULL AND o.reporting_launch_accepted_at IS NOT NULL AND o.reporting_ready_at IS NOT NULL ORDER BY o.work_unit_id,o.attempt_ordinal")
            .and_then(|mut statement| statement.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for candidate in candidates {
            let invocation = AgentInvocationId::new(candidate).map_err(|_| SprintRunnerTransitionError::Conflict)?;
            let context = self.reporting_context(&invocation, false)?;
            let history = self.sessions.load_session(&AgentSessionId::new(context.session_id).map_err(|_| SprintRunnerTransitionError::Forbidden)?).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            if let Some(entry) = history.invocations.iter().find(|entry| entry.invocation.id == invocation && entry.invocation.status.is_terminal()) { self.record_reporting_lifecycle_v3(&invocation, lifecycle_status(entry.invocation.status))?; }
        }
        self.reconcile_implementer_outcome_acceptance_v2()
    }

    fn reconcile_handler_reviews(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        let reconciliation_lock = handler_review_reconciliation_lock(&self.database_lock_key)?;
        let _reconciliation_guard = reconciliation_lock
            .lock()
            .map_err(|_| SprintRunnerTransitionError::Unavailable("Handler review reconciliation lock is poisoned".into()))?;
        let ready: Vec<(String, String)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .prepare("SELECT o.work_unit_id,o.attempt_id FROM work_unit_implementer_outcomes o LEFT JOIN work_unit_handler_reviews r ON r.attempt_id=o.attempt_id WHERE o.handler_review_ready_at IS NOT NULL AND o.application_accepted_at IS NOT NULL AND r.attempt_id IS NULL ORDER BY o.work_unit_id,o.attempt_ordinal")
            .and_then(|mut statement| statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for (work_unit, attempt) in ready { self.prepare_handler_review(&work_unit, &attempt)?; }
        let incomplete: Vec<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .prepare("SELECT review_invocation_id FROM work_unit_handler_reviews WHERE review_ready_at IS NULL AND lifecycle_observed_at IS NULL ORDER BY delivery_requested_at,work_unit_id,attempt_id")
            .and_then(|mut statement| statement.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for review in incomplete { self.launch_handler_review(&review)?; }
        let reviews: Vec<(String, String)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .prepare("SELECT handler_session_id,review_invocation_id FROM work_unit_handler_reviews WHERE lifecycle_observed_at IS NULL AND review_ready_at IS NOT NULL ORDER BY delivery_requested_at,work_unit_id,attempt_id")
            .and_then(|mut statement| statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for (session, review) in reviews {
            let session = AgentSessionId::new(session).map_err(|_| SprintRunnerTransitionError::Conflict)?;
            let invocation = AgentInvocationId::new(review).map_err(|_| SprintRunnerTransitionError::Conflict)?;
            let history = self.sessions.load_session(&session).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            if let Some(entry) = history.invocations.iter().find(|entry| entry.invocation.id == invocation && entry.invocation.status.is_terminal()) {
                self.record_handler_review_lifecycle(&invocation, lifecycle_status(entry.invocation.status))?;
            }
        }
        self.finalize_handler_review_decisions()?;
        self.reconcile_work_unit_retries()?;
        self.reconcile_no_progress_handbacks()
    }

    fn prepare_handler_review(self: &Arc<Self>, work_unit: &str, attempt: &str) -> Result<(), SprintRunnerTransitionError> {
        let row: Option<(String, String, String, String, String, String, String, String, String, String)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .query_row("SELECT o.reporting_invocation_id,o.submitted_summary,o.submitted_validation_statement,o.evidence_manifest_json,o.comparison_fingerprint,o.evidence_content_fingerprints_json,h.handler_session_id,h.handler_invocation_id,a.action_invocation_id,h.attempt_id FROM work_unit_implementer_outcomes o JOIN work_unit_handler_activations h ON h.work_unit_id=o.work_unit_id JOIN work_unit_handler_action_continuations a ON a.work_unit_id=h.work_unit_id AND a.attempt_id=h.attempt_id AND a.handler_session_id=h.handler_session_id AND a.original_handler_invocation_id=h.handler_invocation_id WHERE o.work_unit_id=?1 AND o.attempt_id=?2 AND o.application_accepted_at IS NOT NULL AND o.handler_review_ready_at IS NOT NULL AND h.handler_ready_at IS NOT NULL AND a.action_ready_at IS NOT NULL", params![work_unit,attempt], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?)))
            .optional().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let Some((reporting, summary, validation, manifest, comparison, contents, session, original, action, handler_attempt)) = row else { return Ok(()); };
        let reporting_id = AgentInvocationId::new(reporting.clone()).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        if !self.valid_accepted_outcome(work_unit, &reporting_id)? { return Err(SprintRunnerTransitionError::Conflict); }
        let (_, actual) = self.evidence_snapshot(&reporting_id, false)?;
        if actual.manifest_json != manifest || actual.comparison_fingerprint != comparison || actual.content_fingerprints_json != contents { return Err(SprintRunnerTransitionError::Conflict); }
        let handler = self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone().ok_or(SprintRunnerTransitionError::Forbidden)?;
        let revision = handler.current_handler_review_revision().map_err(|_| SprintRunnerTransitionError::Unavailable("immutable Handler review Harness revision unavailable".into()))?;
        let review = stable_id("work-unit-handler-review-invocation", attempt);
        let payload = handler_review_payload(&summary, &validation, &manifest, &comparison, &contents)?.to_string();
        let fingerprint = stable_id("work-unit-handler-review-delivery", &payload);
        let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let changed = transaction.execute("INSERT OR IGNORE INTO work_unit_handler_reviews (work_unit_id,attempt_id,reporting_invocation_id,handler_session_id,original_handler_invocation_id,action_handler_invocation_id,review_invocation_id,review_harness_revision_id,review_harness_configuration_digest,review_harness_repository_commit_ref,delivery_requested_at,delivered_payload_json,delivered_payload_fingerprint) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)", params![work_unit,attempt,reporting,session,original,action,review,revision.revision_id,revision.configuration_digest,revision.repository_commit_ref,chrono::Utc::now().to_rfc3339(),payload,fingerprint]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if changed == 0 {
            let exact: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_reviews WHERE attempt_id=?1 AND reporting_invocation_id=?2 AND review_invocation_id=?3 AND delivered_payload_fingerprint=?4)", params![attempt,reporting,review,fingerprint], |row| row.get(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            if !exact { return Err(SprintRunnerTransitionError::Conflict); }
        }
        transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        drop(connection);
        let _ = handler_attempt;
        self.launch_handler_review(&review)
    }

    fn launch_handler_review(self: &Arc<Self>, review: &str) -> Result<(), SprintRunnerTransitionError> {
        let invocation = AgentInvocationId::new(review.to_owned()).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let context = self.prelaunch_handler_review_context(&invocation)?;
        let handler = self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone().ok_or(SprintRunnerTransitionError::Forbidden)?;
        let pinned = handler.load_pinned_handler_revision(&context.revision_id, &context.configuration_digest, &context.repository_commit_ref).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let package = handler.construct_for_pinned_profile(&context.handler_authority_attempt_id, WorkUnitHarnessRole::Handler, pinned.profile).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let session = AgentSessionId::new(context.session_id.clone()).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let injection = self.prepare_work_unit_handler_review_action(invocation.clone())?;
        let mut runtime = package.runtime_launch_configuration(); runtime.extension.additional_args.extend(injection.configuration_args); runtime.extension.environment.push(injection.environment);
        let prompt = "Independent Handler review continuation. The application has bound the exact accepted outcome and evidence. Use read_handler_review_evidence, submit exactly one accept or structured return judgment, then end successfully. Do not perform later workflow.".to_string();
        self.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: prompt.clone(), title: None, working_directory: Some(package.working_directory().into()), requested_options: Some(runtime.requested_options.clone()) } }).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        self.mark_handler_review(&context.attempt_id, "delivery_persisted_at")?;
        package.bind_correlated_invocation(session.clone(), invocation.clone()).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        self.mark_handler_review(&context.attempt_id, "harness_bound_at")?;
        match self.sessions.application_invocation_launch_evidence(&invocation, &session).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))? {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => { self.mark_handler_review(&context.attempt_id, "launch_requested_at")?; self.mark_handler_review(&context.attempt_id, "launch_accepted_at")?; self.mark_handler_review(&context.attempt_id, "review_ready_at")?; }
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {
                self.mark_handler_review(&context.attempt_id, "launch_requested_at")?;
                let launched = self.sessions.launch_prepared_application_invocation_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session), submitted_text: prompt, title: None, working_directory: Some(package.working_directory().into()), requested_options: Some(runtime.requested_options) } }, Some(runtime.extension)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if launched.launch_accepted { self.mark_handler_review(&context.attempt_id, "launch_accepted_at")?; self.mark_handler_review(&context.attempt_id, "review_ready_at")?; } else { return Err(SprintRunnerTransitionError::Unavailable("Handler review launch was not accepted".into())); }
            }
            ApplicationInvocationLaunchEvidence::NeverPersisted => return Err(SprintRunnerTransitionError::Conflict),
        }
        Ok(())
    }

    fn mark_handler_review(&self, attempt: &str, column: &str) -> Result<(), SprintRunnerTransitionError> {
        if !["delivery_persisted_at","harness_bound_at","launch_requested_at","launch_accepted_at","review_ready_at"].contains(&column) { return Err(SprintRunnerTransitionError::Unavailable("invalid review stage".into())); }
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(&format!("UPDATE work_unit_handler_reviews SET {column}=COALESCE({column},?2) WHERE attempt_id=?1"), params![attempt,chrono::Utc::now().to_rfc3339()]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        Ok(())
    }

    fn prelaunch_handler_review_context(&self, invocation: &AgentInvocationId) -> Result<HandlerReviewContext, SprintRunnerTransitionError> {
        let context: Option<HandlerReviewContext> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .query_row("SELECT r.work_unit_id,r.attempt_id,h.attempt_id,r.handler_session_id,r.review_invocation_id,r.review_harness_revision_id,r.review_harness_configuration_digest,r.review_harness_repository_commit_ref,r.delivered_payload_json,r.delivered_payload_fingerprint FROM work_unit_handler_reviews r JOIN work_unit_implementer_outcomes o ON o.attempt_id=r.attempt_id AND o.reporting_invocation_id=r.reporting_invocation_id JOIN work_unit_handler_activations h ON h.work_unit_id=r.work_unit_id AND h.handler_session_id=r.handler_session_id AND h.handler_invocation_id=r.original_handler_invocation_id JOIN work_unit_handler_action_continuations a ON a.work_unit_id=h.work_unit_id AND a.attempt_id=h.attempt_id AND a.handler_session_id=h.handler_session_id AND a.original_handler_invocation_id=h.handler_invocation_id AND a.action_invocation_id=r.action_handler_invocation_id WHERE r.review_invocation_id=?1 AND o.application_accepted_at IS NOT NULL AND o.handler_review_ready_at IS NOT NULL", [invocation.as_str()], |row| Ok(HandlerReviewContext { work_unit_id: row.get(0)?, attempt_id: row.get(1)?, handler_authority_attempt_id: row.get(2)?, session_id: row.get(3)?, review_invocation_id: row.get(4)?, revision_id: row.get(5)?, configuration_digest: row.get(6)?, repository_commit_ref: row.get(7)?, delivered_payload_json: row.get(8)?, delivered_payload_fingerprint: row.get(9)? }))
            .optional().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let context = context.ok_or(SprintRunnerTransitionError::Forbidden)?;
        if context.review_invocation_id != invocation.as_str() || context.review_invocation_id != stable_id("work-unit-handler-review-invocation", &context.attempt_id) || context.delivered_payload_fingerprint != stable_id("work-unit-handler-review-delivery", &context.delivered_payload_json) { return Err(SprintRunnerTransitionError::Conflict); }
        let handler = self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone().ok_or(SprintRunnerTransitionError::Forbidden)?;
        let pinned = handler.load_pinned_handler_revision(&context.revision_id, &context.configuration_digest, &context.repository_commit_ref).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        if pinned.profile.mcp.enabled_tools != ["read_handler_review_evidence","accept_implementation_outcome","return_implementation_outcome"] { return Err(SprintRunnerTransitionError::Conflict); }
        handler.construct_for_pinned_profile(&context.handler_authority_attempt_id, WorkUnitHarnessRole::Handler, pinned.profile).map_err(|_| SprintRunnerTransitionError::Forbidden)?;
        Ok(context)
    }
    fn handler_review_context(&self,invocation:&AgentInvocationId,live:bool)->Result<HandlerReviewContext,SprintRunnerTransitionError>{
        let context=self.prelaunch_handler_review_context(invocation)?;
        let ready:bool=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_reviews WHERE review_invocation_id=?1 AND delivery_persisted_at IS NOT NULL AND harness_bound_at IS NOT NULL AND launch_accepted_at IS NOT NULL AND review_ready_at IS NOT NULL)",[invocation.as_str()],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if !ready{return Err(SprintRunnerTransitionError::Forbidden)}
        let reporting=AgentInvocationId::new(stable_id("work-unit-implementer-reporting-invocation",&context.attempt_id)).map_err(|_|SprintRunnerTransitionError::Conflict)?;
        if !self.valid_accepted_outcome(&context.work_unit_id,&reporting)?{return Err(SprintRunnerTransitionError::Conflict)}
        let(_,snapshot)=self.evidence_snapshot(&reporting,false)?;
        let (summary,validation,manifest,comparison,contents):(String,String,String,String,String)=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT submitted_summary,submitted_validation_statement,evidence_manifest_json,comparison_fingerprint,evidence_content_fingerprints_json FROM work_unit_implementer_outcomes WHERE work_unit_id=?1 AND reporting_invocation_id=?2",params![context.work_unit_id,reporting.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if manifest!=snapshot.manifest_json||comparison!=snapshot.comparison_fingerprint||contents!=snapshot.content_fingerprints_json{return Err(SprintRunnerTransitionError::Conflict)}
        let expected=handler_review_payload(&summary,&validation,&snapshot.manifest_json,&snapshot.comparison_fingerprint,&snapshot.content_fingerprints_json)?;
        let delivered:serde_json::Value=serde_json::from_str(&context.delivered_payload_json).map_err(|_|SprintRunnerTransitionError::Conflict)?;
        if delivered!=expected{return Err(SprintRunnerTransitionError::Conflict)}
        let history=self.sessions.load_session(&AgentSessionId::new(context.session_id.clone()).map_err(|_|SprintRunnerTransitionError::Conflict)?).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let entry=history.invocations.iter().find(|entry|entry.invocation.id==*invocation).ok_or(SprintRunnerTransitionError::Forbidden)?;
        if live&&entry.invocation.status.is_terminal(){return Err(SprintRunnerTransitionError::Forbidden)}
        Ok(context)
    }

    fn record_handler_review_judgment(&self, invocation: &AgentInvocationId, variant: &str, reason: Option<HandlerReviewReturnReason>) -> Result<(), SprintRunnerTransitionError> {
        self.record_handler_review_judgment_inner(invocation, variant, reason, None)
    }

    fn record_handler_incomplete_disposition(&self, invocation: &AgentInvocationId, disposition: HandlerReviewIncompleteDisposition) -> Result<(), SprintRunnerTransitionError> {
        validate_handler_review_return(&HandlerReviewReturnReason { code: disposition.code.clone(), explanation: disposition.explanation.clone() })?;
        self.record_handler_review_judgment_inner(invocation, "return", Some(HandlerReviewReturnReason { code: disposition.code, explanation: disposition.explanation }), Some((disposition.classification, disposition.meaningful_progress)))
    }

    fn record_handler_review_judgment_inner(&self, invocation: &AgentInvocationId, variant: &str, reason: Option<HandlerReviewReturnReason>, incomplete: Option<(IncompleteAttemptClassification, bool)>) -> Result<(), SprintRunnerTransitionError> {
        if (variant == "return" && reason.is_none()) || (variant != "return" && (reason.is_some() || incomplete.is_some())) { return Err(SprintRunnerTransitionError::Invalid); }
        let context = self.handler_review_context(invocation, true)?;
        let reason = reason.map(|value| serde_json::to_string(&value).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))).transpose()?;
        let incomplete_fingerprint = incomplete.as_ref().map(|(classification, meaningful_progress)| stable_id("work-unit-handler-incomplete-judgment", &format!("{}:{}:{}", context.review_invocation_id, classification.as_str(), meaningful_progress)));
        let fingerprint = stable_id("work-unit-handler-review-judgment", &format!("{}:{variant}:{}", context.review_invocation_id, reason.as_deref().unwrap_or("")));
        let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let existing: Option<(Option<String>, Option<String>, Option<String>)> = transaction.query_row("SELECT semantic_judgment_variant,semantic_return_reason_json,semantic_judgment_fingerprint FROM work_unit_handler_reviews WHERE attempt_id=?1 AND review_invocation_id=?2", params![context.attempt_id,invocation.as_str()], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let Some((old_variant, old_reason, old_fingerprint)) = existing else { return Err(SprintRunnerTransitionError::Forbidden); };
        if let Some(old_variant) = old_variant {
            let exact_incomplete = match (&incomplete, &incomplete_fingerprint) {
                (Some((classification, meaningful_progress)), Some(fingerprint)) => transaction.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_incomplete_judgments WHERE review_invocation_id=?1 AND attempt_id=?2 AND classification=?3 AND meaningful_progress=?4 AND judgment_fingerprint=?5)", params![context.review_invocation_id,context.attempt_id,classification.as_str(),*meaningful_progress as i64,fingerprint], |row| row.get::<_,bool>(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?,
                (None, None) => true,
                _ => false,
            };
            if old_variant == variant && old_reason == reason && old_fingerprint.as_deref() == Some(&fingerprint) && exact_incomplete { transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?; return Ok(()); }
            transaction.execute("UPDATE work_unit_handler_reviews SET conflict_at=COALESCE(conflict_at,?2),conflict_reason=COALESCE(conflict_reason,'divergent_review_judgment') WHERE attempt_id=?1", params![context.attempt_id,chrono::Utc::now().to_rfc3339()]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            return Err(SprintRunnerTransitionError::Conflict);
        }
        let now = chrono::Utc::now().to_rfc3339();
        transaction.execute("UPDATE work_unit_handler_reviews SET semantic_judgment_variant=?2,semantic_return_reason_json=?3,semantic_judgment_fingerprint=?4,semantic_judgment_at=?5 WHERE attempt_id=?1 AND review_invocation_id=?6 AND semantic_judgment_at IS NULL", params![context.attempt_id,variant,reason,fingerprint,now,invocation.as_str()]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if let (Some((classification, meaningful_progress)), Some(incomplete_fingerprint)) = (incomplete, incomplete_fingerprint) {
            transaction.execute("INSERT INTO work_unit_handler_incomplete_judgments (review_invocation_id,attempt_id,classification,meaningful_progress,judgment_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6)", params![context.review_invocation_id,context.attempt_id,classification.as_str(),meaningful_progress as i64,incomplete_fingerprint,now]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        }
        transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        Ok(())
    }

    fn record_handler_review_lifecycle(&self, invocation: &AgentInvocationId, status: &str) -> Result<bool, SprintRunnerTransitionError> {
        let context = self.handler_review_context(invocation, false)?;
        let history = self.sessions.load_session(&AgentSessionId::new(context.session_id).map_err(|_| SprintRunnerTransitionError::Conflict)?).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let entry = history.invocations.iter().find(|entry| entry.invocation.id == *invocation).ok_or(SprintRunnerTransitionError::Forbidden)?;
        if !entry.invocation.status.is_terminal() || lifecycle_status(entry.invocation.status) != status { return Err(SprintRunnerTransitionError::Conflict); }
        let changed = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_handler_reviews SET lifecycle_observed_at=COALESCE(lifecycle_observed_at,?2),lifecycle_status=COALESCE(lifecycle_status,?3) WHERE attempt_id=?1 AND review_invocation_id=?4 AND (lifecycle_status IS NULL OR lifecycle_status=?3)", params![context.attempt_id,chrono::Utc::now().to_rfc3339(),status,invocation.as_str()]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if changed != 1 { return Err(SprintRunnerTransitionError::Conflict); }
        Ok(true)
    }

    fn finalize_handler_review_decisions(&self) -> Result<(), SprintRunnerTransitionError> {
        let rows: Vec<(String, String, String, String, Option<String>, String, String, Option<String>, Option<i64>)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .prepare("SELECT r.work_unit_id,r.attempt_id,r.review_invocation_id,r.semantic_judgment_variant,r.semantic_return_reason_json,r.semantic_judgment_fingerprint,r.delivered_payload_json,j.classification,j.meaningful_progress FROM work_unit_handler_reviews r LEFT JOIN work_unit_handler_incomplete_judgments j ON j.review_invocation_id=r.review_invocation_id AND j.attempt_id=r.attempt_id WHERE r.semantic_judgment_at IS NOT NULL AND r.lifecycle_observed_at IS NOT NULL AND r.lifecycle_status='completed' ORDER BY r.work_unit_id,r.attempt_id")
            .and_then(|mut statement| statement.query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?)))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for (work_unit, attempt, review, judgment, reason, fingerprint, delivered_payload, classification, meaningful_progress) in rows {
            let decision = match judgment.as_str() { "accept" => "accepted", "return" => "returned", _ => return Err(SprintRunnerTransitionError::Conflict) };
            let incomplete = match (decision, classification, meaningful_progress) {
                ("accepted", None, None) => None,
                ("returned", Some(classification), Some(progress @ (0 | 1))) => Some((classification, progress == 1)),
                // Historical ordinal-0/ordinal-1 returns retain their accepted retry-required
                // meaning; only the new MCP contract creates a generalized disposition.
                ("returned", None, None) => None,
                _ => return Err(SprintRunnerTransitionError::Conflict),
            };
            let decision_fingerprint = stable_id("work-unit-handler-decision", &format!("{review}:{fingerprint}"));
            let now = chrono::Utc::now().to_rfc3339();
            let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
            let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            let legacy_retry_required = decision == "returned" && incomplete.is_none();
            let changed = transaction.execute("INSERT OR IGNORE INTO work_unit_handler_decisions (attempt_id,work_unit_id,review_invocation_id,decision_variant,decision_fingerprint,return_reason_json,decision_recorded_at,implementation_accepted_at,implementation_returned_at,retry_required_at,settlement_ready_at) VALUES (?1,?2,?3,?4,?5,?6,?7,CASE WHEN ?4='accepted' THEN ?7 END,CASE WHEN ?4='returned' THEN ?7 END,CASE WHEN ?8 THEN ?7 END,NULL)", params![attempt,work_unit,review,decision,decision_fingerprint,reason,now,legacy_retry_required]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            if changed == 0 {
                let exact: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_decisions WHERE attempt_id=?1 AND review_invocation_id=?2 AND decision_variant=?3 AND decision_fingerprint=?4)", params![attempt,review,decision,decision_fingerprint], |row| row.get(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if !exact {
                    transaction.execute("UPDATE work_unit_handler_reviews SET conflict_at=COALESCE(conflict_at,?2),conflict_reason=COALESCE(conflict_reason,'divergent_final_decision') WHERE attempt_id=?1", params![attempt,now]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    return Err(SprintRunnerTransitionError::Conflict);
                }
            }
            if let Some((classification, meaningful_progress)) = incomplete {
                let authorized_at = meaningful_progress.then_some(now.as_str());
                let changed = transaction.execute("INSERT OR IGNORE INTO work_unit_handler_incomplete_dispositions (attempt_id,work_unit_id,review_invocation_id,decision_fingerprint,classification,meaningful_progress,recorded_at,next_attempt_authorized_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![attempt,work_unit,review,decision_fingerprint,classification,meaningful_progress as i64,now,authorized_at]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if changed == 0 {
                    let exact: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_incomplete_dispositions WHERE attempt_id=?1 AND work_unit_id=?2 AND review_invocation_id=?3 AND decision_fingerprint=?4 AND classification=?5 AND meaningful_progress=?6 AND ((?6=1 AND next_attempt_authorized_at IS NOT NULL) OR (?6=0 AND next_attempt_authorized_at IS NULL)))", params![attempt,work_unit,review,decision_fingerprint,classification,meaningful_progress as i64], |row| row.get(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    if !exact { return Err(SprintRunnerTransitionError::Conflict); }
                }
                if !meaningful_progress {
                    let reason = reason.clone().ok_or(SprintRunnerTransitionError::Conflict)?;
                    let context = serde_json::json!({"sourceAttemptId": attempt, "sourceReviewInvocationId": review, "classification": classification, "meaningfulProgress": false, "reason": serde_json::from_str::<serde_json::Value>(&reason).map_err(|_| SprintRunnerTransitionError::Conflict)?, "evidence": serde_json::from_str::<serde_json::Value>(&delivered_payload).map_err(|_| SprintRunnerTransitionError::Conflict)?}).to_string();
                    let handback_id = stable_id("work-unit-no-progress-handback", &format!("{attempt}:{review}"));
                    let context_fingerprint = stable_id("work-unit-no-progress-handback-context", &context);
                    let changed = transaction.execute("INSERT OR IGNORE INTO work_unit_no_progress_handbacks (handback_id,work_unit_id,source_attempt_id,source_review_invocation_id,decision_fingerprint,classification,context_json,context_fingerprint,persisted_at,delivery_intended_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)", params![handback_id,work_unit,attempt,review,decision_fingerprint,classification,context,context_fingerprint,now]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    if changed == 0 {
                        let exact: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_no_progress_handbacks WHERE handback_id=?1 AND work_unit_id=?2 AND source_attempt_id=?3 AND source_review_invocation_id=?4 AND decision_fingerprint=?5 AND classification=?6 AND context_fingerprint=?7 AND sprint_runner_receiver_activated_at IS NULL AND sprint_runner_receiver_decision_at IS NULL)", params![handback_id,work_unit,attempt,review,decision_fingerprint,classification,context_fingerprint], |row| row.get(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                        if !exact { return Err(SprintRunnerTransitionError::Conflict); }
                    }
                }
            }
            transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        }
        let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        reconcile_accepted_candidate_authorities(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
        reconcile_accepted_integrations(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
        reconcile_work_unit_dependency_wave(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)?;
        reconcile_work_slice_execution_settlement(&mut connection).map_err(SprintRunnerTransitionError::Unavailable)
    }

    /// A completed meaningful-progress disposition is application authority for exactly one
    /// later correction attempt. This path is separate from Handler review finalization: it is
    /// neither acceptance nor an update to Sprint Git authority.
    fn reconcile_work_unit_retries(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        let Some(handler) = self.work_unit_handler.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone() else { return Ok(()); };
        let sources = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare(
            "SELECT o.work_unit_id,h.sprint_id,a.sprint_git_authority_id,o.attempt_id,o.reporting_invocation_id,
                    o.reporting_harness_revision_id,o.reporting_harness_configuration_digest,o.reporting_harness_repository_commit_ref,
                    d.review_invocation_id,d.decision_fingerprint,d.return_reason_json,
                    o.submitted_summary,o.submitted_validation_statement,o.evidence_manifest_json,
                    o.comparison_fingerprint,o.evidence_content_fingerprints_json,o.attempt_ordinal
             FROM work_unit_implementer_outcomes o
             JOIN work_unit_handler_activations h ON h.work_unit_id=o.work_unit_id
             JOIN work_unit_handler_reviews r ON r.work_unit_id=o.work_unit_id AND r.attempt_id=o.attempt_id AND r.reporting_invocation_id=o.reporting_invocation_id
             JOIN execution_support_attempt_authorizations a ON a.attempt_id=o.attempt_id AND a.work_unit_id=o.work_unit_id AND a.role_kind='work_unit_implementer'
             JOIN work_unit_handler_decisions d ON d.attempt_id=o.attempt_id AND d.review_invocation_id=r.review_invocation_id
             LEFT JOIN work_unit_handler_incomplete_dispositions i ON i.attempt_id=o.attempt_id AND i.work_unit_id=o.work_unit_id AND i.review_invocation_id=r.review_invocation_id AND i.decision_fingerprint=d.decision_fingerprint
             WHERE d.decision_variant='returned' AND (
                 (i.meaningful_progress=1 AND i.next_attempt_authorized_at IS NOT NULL)
                 OR (i.attempt_id IS NULL AND d.retry_required_at IS NOT NULL)
             )
               AND r.semantic_judgment_variant='return' AND r.semantic_judgment_at IS NOT NULL
               AND r.lifecycle_observed_at IS NOT NULL AND r.lifecycle_status='completed'
               AND o.application_accepted_at IS NOT NULL AND o.handler_review_ready_at IS NOT NULL
               AND o.evidence_ready_at IS NOT NULL AND o.semantic_completed_at IS NOT NULL
               AND o.attempt_ordinal=(SELECT MAX(previous.attempt_ordinal) FROM work_unit_implementer_outcomes previous WHERE previous.work_unit_id=o.work_unit_id)
             ORDER BY h.sprint_id,o.work_unit_id"
        ).and_then(|mut statement| statement.query_map([], |row| Ok(RetrySource {
            work_unit_id: row.get(0)?, sprint_id: row.get(1)?, authority_id: row.get(2)?, origin_attempt_id: row.get(3)?, origin_ordinal: row.get(16)?, reporting_invocation_id: row.get(4)?,
            reporting_revision_id: row.get(5)?, reporting_configuration_digest: row.get(6)?, reporting_repository_commit_ref: row.get(7)?,
            review_invocation_id: row.get(8)?, decision_fingerprint: row.get(9)?, return_reason_json: row.get(10)?,
            summary: row.get(11)?, validation: row.get(12)?, manifest_json: row.get(13)?, comparison_fingerprint: row.get(14)?, content_fingerprints_json: row.get(15)?,
        }))?.collect::<Result<Vec<_>, _>>()).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for source in sources { self.reconcile_work_unit_retry(&handler, source)?; }
        Ok(())
    }

    /// Consume each durable no-progress Handback through the Sprint that owns its Handler.
    /// Creating this row is not delivery: it records the one durable receiver route that later
    /// reconciliation must either launch or leave visibly pending behind an active invocation.
    fn reconcile_no_progress_handbacks(self: &Arc<Self>) -> Result<(), SprintRunnerTransitionError> {
        let sources: Vec<(String, String, String, String)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .prepare("SELECT h.handback_id,a.sprint_id,t.sprint_runner_session_id,h.context_fingerprint FROM work_unit_no_progress_handbacks h JOIN work_unit_handler_activations a ON a.work_unit_id=h.work_unit_id JOIN sprint_runner_transitions t ON t.sprint_id=a.sprint_id LEFT JOIN sprint_runner_handback_deliveries d ON d.handback_id=h.handback_id WHERE d.handback_id IS NULL OR d.delivery_persisted_at IS NULL OR d.launch_accepted_at IS NULL ORDER BY h.persisted_at,h.handback_id")
            .and_then(|mut statement| statement.query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))?.collect::<Result<Vec<_>, _>>())
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        for (handback, sprint, session, context_fingerprint) in sources {
            self.reconcile_no_progress_handback(&handback, &sprint, &session, &context_fingerprint)?;
        }
        Ok(())
    }

    fn reconcile_no_progress_handback(self: &Arc<Self>, handback: &str, sprint: &str, session: &str, context_fingerprint: &str) -> Result<(), SprintRunnerTransitionError> {
        let lock = self.transition_lock(sprint)?;
        let _guard = lock.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Sprint Runner transition lock is poisoned".into()))?;
        let harness = conversation_harness::profile(ConversationHarnessRole::SprintRunnerHandbackReassessment).map_err(SprintRunnerTransitionError::Unavailable)?;
        let invocation = stable_id("sprint-runner-handback-reassessment", handback);
        let delivery = stable_id("sprint-runner-handback-delivery", handback);
        let now = chrono::Utc::now().to_rfc3339();
        let mut conn = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let changed = transaction.execute("INSERT OR IGNORE INTO sprint_runner_handback_deliveries (handback_id,sprint_id,receiver_session_id,reassessment_invocation_id,delivery_fact_id,delivery_requested_at,harness_key,harness_version,context_fingerprint) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![handback,sprint,session,invocation,delivery,now,harness.key,harness.version,context_fingerprint]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if changed == 0 {
            let exact: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM sprint_runner_handback_deliveries WHERE handback_id=?1 AND sprint_id=?2 AND receiver_session_id=?3 AND reassessment_invocation_id=?4 AND delivery_fact_id=?5 AND harness_key=?6 AND harness_version=?7 AND context_fingerprint=?8)", params![handback,sprint,session,invocation,delivery,harness.key,harness.version,context_fingerprint], |row| row.get(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            if !exact { return Err(SprintRunnerTransitionError::Conflict); }
        }
        transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        drop(conn);
        let session = AgentSessionId::new(session.to_owned()).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let history = self.sessions.load_session(&session).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if history.invocations.iter().any(|entry| !entry.invocation.status.is_terminal() && entry.invocation.id.as_str() != invocation) { return Ok(()); }
        let invocation = AgentInvocationId::new(invocation).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        match self.sessions.application_invocation_launch_evidence(&invocation, &session).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))? {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => {
                let _ = self.prepare_handback_reassessment_action(invocation.clone())?;
                self.mark_handback_delivery(handback, "delivery_persisted_at")?;
                self.mark_handback_delivery(handback, "harness_bound_at")?;
                self.mark_handback_delivery(handback, "launch_requested_at")?;
                self.mark_handback_delivery(handback, "launch_accepted_at")?;
            }
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {
                // A reopen has durable application provenance but not the ephemeral action
                // server. Recreate the scoped server, re-bind the catalog revision, and launch
                // the exact prepared invocation; merely restamping delivery would strand it.
                let injection = self.prepare_handback_reassessment_action(invocation.clone())?;
                let mut args = harness.runtime_configuration_args();
                args.extend(injection.configuration_args);
                self.mark_handback_delivery(handback, "delivery_persisted_at")?;
                self.mark_handback_delivery(handback, "harness_bound_at")?;
                self.mark_handback_delivery(handback, "launch_requested_at")?;
                let launch = self.sessions.launch_prepared_application_invocation_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation, message: SendAgentSessionMessageCommand { session_id: Some(session), submitted_text: "The application delivered one exact no-progress Work Unit concern. Read only the supplied reassessment context, record one truthful next movement, then stop. Continuing eligible work does not settle the concern; do not contact an Epic Runner or declare Sprint/Epic blockage.".into(), title: None, working_directory: Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunnerHandbackReassessment).map_err(SprintRunnerTransitionError::Unavailable)?), requested_options: Some(harness.runtime_options()) } }, Some(RuntimeLaunchExtension { additional_args: args, environment: vec![injection.environment], initial_prompt_prefix: Some(harness.initial_prompt_prefix()) })).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if launch.launch_accepted { self.mark_handback_delivery(handback, "launch_accepted_at")?; }
            }
            ApplicationInvocationLaunchEvidence::NeverPersisted => {
                let injection = self.prepare_handback_reassessment_action(invocation.clone())?;
                let mut args = harness.runtime_configuration_args();
                args.extend(injection.configuration_args);
                self.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: "The application delivered one exact no-progress Work Unit concern. Read only the supplied reassessment context, record one truthful next movement, then stop. Continuing eligible work does not settle the concern; do not contact an Epic Runner or declare Sprint/Epic blockage.".into(), title: None, working_directory: Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunnerHandbackReassessment).map_err(SprintRunnerTransitionError::Unavailable)?), requested_options: Some(harness.runtime_options()) } }).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                self.mark_handback_delivery(handback, "delivery_persisted_at")?;
                self.mark_handback_delivery(handback, "harness_bound_at")?;
                self.mark_handback_delivery(handback, "launch_requested_at")?;
                let launch = self.sessions.launch_prepared_application_invocation_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation, message: SendAgentSessionMessageCommand { session_id: Some(session), submitted_text: "The application delivered one exact no-progress Work Unit concern. Read only the supplied reassessment context, record one truthful next movement, then stop. Continuing eligible work does not settle the concern; do not contact an Epic Runner or declare Sprint/Epic blockage.".into(), title: None, working_directory: Some(conversation_harness::role_discovery_root(ConversationHarnessRole::SprintRunnerHandbackReassessment).map_err(SprintRunnerTransitionError::Unavailable)?), requested_options: Some(harness.runtime_options()) } }, Some(RuntimeLaunchExtension { additional_args: args, environment: vec![injection.environment], initial_prompt_prefix: Some(harness.initial_prompt_prefix()) })).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if launch.launch_accepted { self.mark_handback_delivery(handback, "launch_accepted_at")?; }
            }
        }
        Ok(())
    }

    fn mark_handback_delivery(&self, handback: &str, column: &str) -> Result<(), SprintRunnerTransitionError> {
        if !["delivery_persisted_at", "harness_bound_at", "launch_requested_at", "launch_accepted_at", "provider_activation_observed_at"].contains(&column) { return Err(SprintRunnerTransitionError::Unavailable("invalid Handback delivery stage".into())); }
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(&format!("UPDATE sprint_runner_handback_deliveries SET {column}=COALESCE({column},?2) WHERE handback_id=?1"), params![handback,chrono::Utc::now().to_rfc3339()]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        Ok(())
    }

    fn handback_reassessment_context(&self, invocation: &AgentInvocationId) -> Result<(String, serde_json::Value), SprintRunnerTransitionError> {
        let row: Option<(String, String, String)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT d.handback_id,h.context_json,d.context_fingerprint FROM sprint_runner_handback_deliveries d JOIN work_unit_no_progress_handbacks h ON h.handback_id=d.handback_id WHERE d.reassessment_invocation_id=?1 AND d.delivery_persisted_at IS NOT NULL AND d.harness_bound_at IS NOT NULL AND d.launch_accepted_at IS NOT NULL", [invocation.as_str()], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let Some((handback, context, fingerprint)) = row else { return Err(SprintRunnerTransitionError::Forbidden) };
        if fingerprint != stable_id("work-unit-no-progress-handback-context", &context) { return Err(SprintRunnerTransitionError::Conflict); }
        let concern: serde_json::Value = serde_json::from_str(&context).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let concern = serde_json::json!({"classification": concern.get("classification"), "reason": concern.get("reason"), "evidence": concern.get("evidence")});
        let (eligible, blocked): (i64, i64) = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT SUM(CASE WHEN a.eligibility_state='eligible' THEN 1 ELSE 0 END),SUM(CASE WHEN a.eligibility_state='blocked' THEN 1 ELSE 0 END) FROM work_unit_handler_activations a JOIN sprint_runner_handback_deliveries d ON d.sprint_id=a.sprint_id WHERE d.handback_id=?1", [handback.as_str()], |row| Ok((row.get::<_,Option<i64>>(0)?.unwrap_or(0),row.get::<_,Option<i64>>(1)?.unwrap_or(0)))).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        Ok((handback, serde_json::json!({"handedBackConcern": concern, "currentSprintWorkState": {"eligibleWorkUnitCount": eligible, "blockedWorkUnitCount": blocked}})))
    }

    fn record_handback_disposition(self: &Arc<Self>, invocation: &AgentInvocationId, input: SprintHandbackDisposition) -> Result<(), SprintRunnerTransitionError> {
        validate_handback_disposition(&input)?;
        let (handback, _) = self.handback_reassessment_context(invocation)?;
        let serialized = serde_json::to_string(&input).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let fingerprint = stable_id("sprint-runner-handback-disposition", &format!("{handback}:{serialized}"));
        let disposition = stable_id("sprint-runner-handback-disposition-id", &handback);
        let semantic = stable_id("sprint-runner-handback-semantic-reassessment", &handback);
        let mut conn = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let ready: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM sprint_runner_handback_deliveries WHERE handback_id=?1 AND reassessment_invocation_id=?2 AND delivery_persisted_at IS NOT NULL AND harness_bound_at IS NOT NULL AND launch_accepted_at IS NOT NULL)", params![handback,invocation.as_str()], |row| row.get(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if !ready { return Err(SprintRunnerTransitionError::Forbidden); }
        let now = chrono::Utc::now().to_rfc3339();
        let changed = transaction.execute("INSERT OR IGNORE INTO sprint_runner_handback_dispositions (handback_id,disposition_id,movement_kind,details_json,disposition_fingerprint,selected_at,preserves_handback) VALUES (?1,?2,?3,?4,?5,?6,1)", params![handback,disposition,input.movement_kind,serialized,fingerprint,now]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if changed == 0 { let exact: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM sprint_runner_handback_dispositions WHERE handback_id=?1 AND disposition_id=?2 AND disposition_fingerprint=?3 AND preserves_handback=1)",params![handback,disposition,fingerprint],|row|row.get(0)).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?; if !exact { return Err(SprintRunnerTransitionError::Conflict); } }
        transaction.execute("UPDATE sprint_runner_handback_deliveries SET semantic_reassessment_fact_id=COALESCE(semantic_reassessment_fact_id,?2),semantic_reassessment_recorded_at=COALESCE(semantic_reassessment_recorded_at,?3) WHERE handback_id=?1",params![handback,semantic,now]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let local_exhaustion=input.movement_kind=="local_exhaustion_escalate";if local_exhaustion { let intent=stable_id("sprint-runner-handback-escalation-intent",&handback);let request=stable_id("sprint-runner-handback-escalation-delivery-request",&handback); transaction.execute("INSERT OR IGNORE INTO sprint_runner_handback_escalations (handback_id,escalation_intent_id,delivery_request_id,requested_at,delivery_requested_at) VALUES (?1,?2,?3,?4,?4)",params![handback,intent,request,now]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?; }
        transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        drop(conn);
        if local_exhaustion {self.reconcile_epic_escalation_receivers()?;}
        Ok(())
    }

    fn reconcile_epic_escalation_receivers(self:&Arc<Self>)->Result<(),SprintRunnerTransitionError>{let sources:Vec<(String,String,String,String,String,String,String,String)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare("SELECT e.handback_id,e.escalation_intent_id,e.delivery_request_id,d.sprint_id,t.epic_id,t.epic_runner_session_id,t.epic_runner_invocation_id,d.context_fingerprint FROM sprint_runner_handback_escalations e JOIN sprint_runner_handback_deliveries d ON d.handback_id=e.handback_id JOIN sprint_runner_handback_dispositions m ON m.handback_id=e.handback_id JOIN sprint_runner_transitions t ON t.sprint_id=d.sprint_id LEFT JOIN epic_runner_escalation_receivers r ON r.handback_id=e.handback_id WHERE m.movement_kind='local_exhaustion_escalate' AND d.semantic_reassessment_recorded_at IS NOT NULL AND (r.handback_id IS NULL OR r.delivery_persisted_at IS NULL OR r.launch_accepted_at IS NULL) ORDER BY e.requested_at,e.handback_id").and_then(|mut s|s.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;for source in sources{self.reconcile_epic_escalation_receiver(source)?;}self.observe_epic_escalation_receiver_terminals()}
    fn reconcile_epic_escalation_receiver(self:&Arc<Self>,(handback,intent,request,sprint,epic,session,governing,context):(String,String,String,String,String,String,String,String))->Result<(),SprintRunnerTransitionError>{let lock=self.transition_lock(&sprint)?;let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Epic escalation transition lock is poisoned".into()))?;let harness=conversation_harness::profile(ConversationHarnessRole::EpicRunnerEscalationReassessment).map_err(SprintRunnerTransitionError::Unavailable)?;let invocation=stable_id("epic-runner-escalation-reassessment",&handback);let delivery=stable_id("epic-runner-escalation-delivery",&handback);let correlation=stable_id("epic-runner-escalation-correlation",&format!("{handback}:{intent}:{request}:{sprint}:{epic}:{session}:{governing}:{context}"));let mut conn=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let changed=tx.execute("INSERT OR IGNORE INTO epic_runner_escalation_receivers (handback_id,escalation_intent_id,delivery_request_id,sprint_id,epic_id,governing_runner_session_id,governing_runner_invocation_id,reassessment_invocation_id,delivery_fact_id,delivery_requested_at,harness_key,harness_version,correlation_fingerprint) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![handback,intent,request,sprint,epic,session,governing,invocation,delivery,chrono::Utc::now().to_rfc3339(),harness.key,harness.version,correlation]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed==0{let exact:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM epic_runner_escalation_receivers WHERE handback_id=?1 AND escalation_intent_id=?2 AND delivery_request_id=?3 AND sprint_id=?4 AND epic_id=?5 AND governing_runner_session_id=?6 AND governing_runner_invocation_id=?7 AND reassessment_invocation_id=?8 AND delivery_fact_id=?9 AND harness_key=?10 AND harness_version=?11 AND correlation_fingerprint=?12)",params![handback,intent,request,sprint,epic,session,governing,invocation,delivery,harness.key,harness.version,correlation],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !exact{return Err(SprintRunnerTransitionError::Conflict)}}tx.commit().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;drop(conn);let session=AgentSessionId::new(session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let history=self.sessions.load_session(&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if history.invocations.iter().any(|entry|!entry.invocation.status.is_terminal()&&entry.invocation.id.as_str()!=invocation){return Ok(())}let invocation=AgentInvocationId::new(invocation).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let message=SendIdempotentApplicationAgentSessionMessageCommand{invocation_id:invocation.clone(),message:SendAgentSessionMessageCommand{session_id:Some(session.clone()),submitted_text:"The application delivered one exact locally exhausted Sprint concern. Read only the supplied Epic reassessment context, then stop. Do not record an Epic disposition, request downstream work, select or start a Sprint, or claim settlement, completion, or acceptance.".into(),title:None,working_directory:Some(conversation_harness::role_discovery_root(ConversationHarnessRole::EpicRunnerEscalationReassessment).map_err(SprintRunnerTransitionError::Unavailable)?),requested_options:Some(harness.runtime_options())}};match self.sessions.application_invocation_launch_evidence(&invocation,&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?{ApplicationInvocationLaunchEvidence::LaunchAccepted=>{let _=self.prepare_epic_escalation_reassessment_action(invocation)?;for stage in ["delivery_persisted_at","harness_bound_at","launch_requested_at","launch_accepted_at"]{self.mark_epic_escalation_receiver(&handback,stage)?;}}ApplicationInvocationLaunchEvidence::PersistedNotAccepted=>{let injection=self.prepare_epic_escalation_reassessment_action(invocation.clone())?;let mut args=harness.runtime_configuration_args();args.extend(injection.configuration_args);for stage in ["delivery_persisted_at","harness_bound_at","launch_requested_at"]{self.mark_epic_escalation_receiver(&handback,stage)?;}let launch=self.sessions.launch_prepared_application_invocation_with_launch_observation(message,Some(RuntimeLaunchExtension{additional_args:args,environment:vec![injection.environment],initial_prompt_prefix:Some(harness.initial_prompt_prefix())})).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if launch.launch_accepted{self.mark_epic_escalation_receiver(&handback,"launch_accepted_at")?;}}ApplicationInvocationLaunchEvidence::NeverPersisted=>{let injection=self.prepare_epic_escalation_reassessment_action(invocation.clone())?;let mut args=harness.runtime_configuration_args();args.extend(injection.configuration_args);self.sessions.prepare_idempotent_application_invocation(message.clone()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;for stage in ["delivery_persisted_at","harness_bound_at","launch_requested_at"]{self.mark_epic_escalation_receiver(&handback,stage)?;}let launch=self.sessions.launch_prepared_application_invocation_with_launch_observation(message,Some(RuntimeLaunchExtension{additional_args:args,environment:vec![injection.environment],initial_prompt_prefix:Some(harness.initial_prompt_prefix())})).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if launch.launch_accepted{self.mark_epic_escalation_receiver(&handback,"launch_accepted_at")?;}}}Ok(())}
    fn mark_epic_escalation_receiver(&self,handback:&str,column:&str)->Result<(),SprintRunnerTransitionError>{if !["delivery_persisted_at","harness_bound_at","launch_requested_at","launch_accepted_at","provider_activation_observed_at"].contains(&column){return Err(SprintRunnerTransitionError::Unavailable("invalid Epic escalation receiver stage".into()))}let now=chrono::Utc::now().to_rfc3339();let connection=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;connection.execute(&format!("UPDATE epic_runner_escalation_receivers SET {column}=COALESCE({column},?2) WHERE handback_id=?1"),params![handback,now]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if column=="delivery_persisted_at"{let changed=connection.execute("UPDATE sprint_runner_handback_escalations SET delivery_persisted_at=COALESCE(delivery_persisted_at,?2) WHERE handback_id=?1",params![handback,chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed!=1{return Err(SprintRunnerTransitionError::Conflict)}}Ok(())}
    fn observe_epic_escalation_receiver_terminals(&self)->Result<(),SprintRunnerTransitionError>{let receivers:Vec<(String,String,String)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.prepare("SELECT handback_id,governing_runner_session_id,reassessment_invocation_id FROM epic_runner_escalation_receivers WHERE launch_accepted_at IS NOT NULL AND reassessment_lifecycle_observed_at IS NULL").and_then(|mut s|s.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;for(handback,session,invocation)in receivers{let session=AgentSessionId::new(session).map_err(|_|SprintRunnerTransitionError::Conflict)?;let invocation=AgentInvocationId::new(invocation).map_err(|_|SprintRunnerTransitionError::Conflict)?;let history=self.sessions.load_session(&session).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if let Some(entry)=history.invocations.iter().find(|entry|entry.invocation.id==invocation&&entry.invocation.status.is_terminal()){let status=lifecycle_status(entry.invocation.status);self.mark_epic_escalation_receiver(&handback,"provider_activation_observed_at")?;self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE epic_runner_escalation_receivers SET reassessment_lifecycle_status=COALESCE(reassessment_lifecycle_status,?2),reassessment_lifecycle_observed_at=COALESCE(reassessment_lifecycle_observed_at,?3) WHERE handback_id=?1 AND (reassessment_lifecycle_status IS NULL OR reassessment_lifecycle_status=?2)",params![handback,status,chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;}}Ok(())}
    fn epic_escalation_reassessment_context(&self,invocation:&AgentInvocationId)->Result<serde_json::Value,SprintRunnerTransitionError>{
        let row:Option<(String,String,String,String,String,String,String)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT r.handback_id,h.context_json,m.details_json,r.epic_id,r.sprint_id,s.title,s.intended_movement FROM epic_runner_escalation_receivers r JOIN work_unit_no_progress_handbacks h ON h.handback_id=r.handback_id JOIN sprint_runner_handback_dispositions m ON m.handback_id=r.handback_id JOIN initiated_sprints s ON s.id=r.sprint_id WHERE r.reassessment_invocation_id=?1 AND r.delivery_persisted_at IS NOT NULL AND r.harness_bound_at IS NOT NULL AND r.launch_accepted_at IS NOT NULL",[invocation.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((_handback,context,details,epic,sprint,title,movement))=row else{return Err(SprintRunnerTransitionError::Forbidden)};
        let concern:serde_json::Value=serde_json::from_str(&context).map_err(|_|SprintRunnerTransitionError::Conflict)?;
        let disposition:serde_json::Value=serde_json::from_str(&details).map_err(|_|SprintRunnerTransitionError::Conflict)?;
        if disposition.get("movementKind").and_then(|value|value.as_str())!=Some("local_exhaustion_escalate"){return Err(SprintRunnerTransitionError::Conflict)}
        let connection=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        let plan:Option<String>=connection.query_row("SELECT proposal_json FROM epic_bootstrap_transitions WHERE epic_id=?1",[&epic],|r|r.get(0)).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let plan=plan.map(|json|serde_json::from_str::<serde_json::Value>(&json).map_err(|_|SprintRunnerTransitionError::Conflict)).transpose()?;
        let (eligible,blocked):(i64,i64)=connection.query_row("SELECT COALESCE(SUM(CASE WHEN a.eligibility_state='eligible' THEN 1 ELSE 0 END),0),COALESCE(SUM(CASE WHEN a.eligibility_state='blocked' THEN 1 ELSE 0 END),0) FROM work_unit_handler_activations a WHERE a.sprint_id=?1",[&sprint],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let mut statement=connection.prepare("SELECT title,intended_movement,concern_summaries_json FROM initiated_sprints WHERE epic_id=?1 AND id<>?2 ORDER BY ordinal").map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let other=statement.query_map(params![epic,sprint],|r|Ok(serde_json::json!({"title":r.get::<_,String>(0)?,"intendedMovement":r.get::<_,String>(1)?,"concernSummaries":serde_json::from_str::<serde_json::Value>(&r.get::<_,String>(2)?).unwrap_or(serde_json::Value::Null)}))).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let handler_known:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_activations WHERE sprint_id=?1 AND eligibility_state='eligible' AND handler_ready_at IS NOT NULL)",[&sprint],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let dependencies=if handler_known{serde_json::json!([{ "owner":"work_unit_handler", "enablingResult":"application-observed Handler result", "resumptionPath":"reassess this exact escalation after that result" }])}else{serde_json::json!([])};
        Ok(serde_json::json!({"acceptedEpicPlan":{"available":plan.is_some(),"suggestedEpicName":plan.as_ref().and_then(|value|value.get("suggestedEpicName")),"otherAvailableEpicWork":other},"currentSprintState":{"title":title,"intendedMovement":movement,"eligibleWorkUnitCount":eligible,"blockedWorkUnitCount":blocked},"knownAgentAchievableDependencies":dependencies,"handedBackConcern":{"classification":concern.get("classification"),"reason":concern.get("reason"),"evidence":concern.get("evidence")},"localExhaustion":{"rationale":disposition.get("rationale"),"summary":disposition.get("localExhaustionSummary")}}))
    }

    fn record_epic_escalation_disposition(self:&Arc<Self>,invocation:&AgentInvocationId,input:EpicEscalationReassessmentDisposition)->Result<(),SprintRunnerTransitionError>{
        validate_epic_escalation_disposition(&input)?;
        let _=self.epic_escalation_reassessment_context(invocation)?;
        let serialized=serde_json::to_string(&input).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let mut conn=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let handback:String=tx.query_row("SELECT handback_id FROM epic_runner_escalation_receivers WHERE reassessment_invocation_id=?1 AND delivery_persisted_at IS NOT NULL AND harness_bound_at IS NOT NULL AND launch_accepted_at IS NOT NULL",[invocation.as_str()],|r|r.get(0)).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?.ok_or(SprintRunnerTransitionError::Forbidden)?;
        if matches!(input.downstream_request.as_ref().map(|r|&r.target),Some(EpicEscalationDownstreamTarget::ExistingAgentAchievableDependency)){let known:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_handler_activations a JOIN epic_runner_escalation_receivers r ON r.sprint_id=a.sprint_id WHERE r.handback_id=?1 AND a.eligibility_state='eligible' AND a.handler_ready_at IS NOT NULL)",[&handback],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !known{return Err(SprintRunnerTransitionError::Forbidden)}}
        let fingerprint=stable_id("epic-runner-escalation-disposition",&format!("{handback}:{serialized}"));let disposition_id=stable_id("epic-runner-escalation-disposition-id",&handback);let semantic=stable_id("epic-runner-escalation-semantic-reassessment",&handback);let now=chrono::Utc::now().to_rfc3339();
        let changed=tx.execute("INSERT OR IGNORE INTO epic_runner_escalation_dispositions (handback_id,disposition_id,movement_kind,details_json,disposition_fingerprint,selected_at,preserves_handback) VALUES (?1,?2,?3,?4,?5,?6,1)",params![handback,disposition_id,input.movement_kind,serialized,fingerprint,now]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if changed==0{let exact:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM epic_runner_escalation_dispositions WHERE handback_id=?1 AND disposition_id=?2 AND disposition_fingerprint=?3 AND preserves_handback=1)",params![handback,disposition_id,fingerprint],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !exact{return Err(SprintRunnerTransitionError::Conflict)}}
        if let Some(request)=input.downstream_request {let request_json=serde_json::to_string(&request).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let request_id=stable_id("epic-runner-escalation-downstream-request",&handback);let request_fingerprint=stable_id("epic-runner-escalation-downstream-request",&format!("{handback}:{request_json}"));let kind=match request.target{EpicEscalationDownstreamTarget::SprintRunner=>"sprint_runner",EpicEscalationDownstreamTarget::ExistingAgentAchievableDependency=>"existing_agent_achievable_dependency"};let changed=tx.execute("INSERT OR IGNORE INTO epic_runner_escalation_downstream_requests (handback_id,request_id,request_kind,request_json,request_fingerprint,requested_at) VALUES (?1,?2,?3,?4,?5,?6)",params![handback,request_id,kind,request_json,request_fingerprint,now]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed==0{let exact:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM epic_runner_escalation_downstream_requests WHERE handback_id=?1 AND request_id=?2 AND request_fingerprint=?3)",params![handback,request_id,request_fingerprint],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !exact{return Err(SprintRunnerTransitionError::Conflict)}}}
        if let Some(attention)=input.human_external_attention {let attention_json=serde_json::to_string(&attention).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;let attention_id=stable_id("epic-runner-escalation-attention",&handback);let attention_fingerprint=stable_id("epic-runner-escalation-attention",&format!("{handback}:{attention_json}"));let changed=tx.execute("INSERT OR IGNORE INTO epic_runner_escalation_attentions (handback_id,attention_id,attention_json,attention_fingerprint,requested_at) VALUES (?1,?2,?3,?4,?5)",params![handback,attention_id,attention_json,attention_fingerprint,now]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if changed==0{let exact:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM epic_runner_escalation_attentions WHERE handback_id=?1 AND attention_id=?2 AND attention_fingerprint=?3)",params![handback,attention_id,attention_fingerprint],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !exact{return Err(SprintRunnerTransitionError::Conflict)}}}
        tx.execute("UPDATE epic_runner_escalation_receivers SET semantic_reassessment_fact_id=COALESCE(semantic_reassessment_fact_id,?2),semantic_reassessment_recorded_at=COALESCE(semantic_reassessment_recorded_at,?3) WHERE handback_id=?1",params![handback,semantic,now]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        tx.commit().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;Ok(())
    }

    fn reconcile_work_unit_retry(self: &Arc<Self>, handler: &Arc<WorkUnitExecutionHarnessService>, source: RetrySource) -> Result<(), SprintRunnerTransitionError> {
        let lock = self.transition_lock(&source.sprint_id)?;
        let _guard = lock.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit retry lock is poisoned".into()))?;
        let retry_exists: bool = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT EXISTS(SELECT 1 FROM work_unit_retry_attempts WHERE origin_attempt_id=?1)",
            [&source.origin_attempt_id],
            |row| row.get(0),
        ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        let reporting = AgentInvocationId::new(source.reporting_invocation_id.clone()).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let (_, snapshot) = match self.evidence_snapshot(&reporting, false) {
            Ok(value) => value,
            Err(error) if retry_exists => return self.fail_retry_for_origin(&source.origin_attempt_id, "retry_evidence_revalidation_failed", error),
            Err(error) => return Err(error),
        };
        if snapshot.manifest_json != source.manifest_json || snapshot.comparison_fingerprint != source.comparison_fingerprint || snapshot.content_fingerprints_json != source.content_fingerprints_json {
            return if retry_exists { self.fail_retry_for_origin(&source.origin_attempt_id, "retry_evidence_revalidation_failed", SprintRunnerTransitionError::Conflict) } else { Err(SprintRunnerTransitionError::Conflict) };
        }
        let authority = match self.authority_repository.load_initiated_sprint_git_authority(&source.authority_id) {
            Ok(Some(authority)) => authority,
            Ok(None) if retry_exists => return self.fail_retry_for_origin(&source.origin_attempt_id, "retry_authority_revalidation_failed", SprintRunnerTransitionError::Conflict),
            Ok(None) => return Err(SprintRunnerTransitionError::Forbidden),
            Err(_) if retry_exists => return self.fail_retry_for_origin(&source.origin_attempt_id, "retry_authority_revalidation_failed", SprintRunnerTransitionError::Unavailable("load retry Sprint Git authority".into())),
            Err(_) => return Err(SprintRunnerTransitionError::Unavailable("load retry Sprint Git authority".into())),
        };
        let pinned_reporting = match handler.load_pinned_implementer_revision(&source.reporting_revision_id, &source.reporting_configuration_digest, &source.reporting_repository_commit_ref) {
            Ok(value) => value,
            Err(_) if retry_exists => return self.fail_retry_for_origin(&source.origin_attempt_id, "retry_harness_revalidation_failed", SprintRunnerTransitionError::Conflict),
            Err(_) => return Err(SprintRunnerTransitionError::Conflict),
        };
        let origin_package = match handler.construct_for_pinned_profile(&source.origin_attempt_id, WorkUnitHarnessRole::Implementer, pinned_reporting.profile) {
            Ok(value) => value,
            Err(_) if retry_exists => return self.fail_retry_for_origin(&source.origin_attempt_id, "retry_execution_support_revalidation_failed", SprintRunnerTransitionError::Conflict),
            Err(_) => return Err(SprintRunnerTransitionError::Conflict),
        };
        let source_root = PathBuf::from(origin_package.working_directory());
        let source_baseline = match self.connection.lock()
            .map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .query_row(
                "SELECT baseline_object_id FROM execution_support_attempt_authorizations
                 WHERE attempt_id=?1 AND role_kind='work_unit_implementer'",
                [&source.origin_attempt_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?
        {
            Some(value) => value,
            None if retry_exists => return self.fail_retry_for_origin(&source.origin_attempt_id, "retry_authorization_revalidation_failed", SprintRunnerTransitionError::Forbidden),
            None => return Err(SprintRunnerTransitionError::Forbidden),
        };
        let (candidate_commit, candidate_tree) = match retry_git_candidate_facts(&authority, &source_root, &source_baseline) {
            Ok(value) => value,
            Err(error) if retry_exists => return self.fail_retry_for_origin(&source.origin_attempt_id, "retry_candidate_revalidation_failed", error),
            Err(error) => return Err(error),
        };
        let next_ordinal: i64 = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT COALESCE(MAX(attempt_ordinal),-1)+1 FROM work_unit_implementer_outcomes WHERE work_unit_id=?1",
            [&source.work_unit_id], |row| row.get(0),
        ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if next_ordinal != source.origin_ordinal + 1 { return Err(SprintRunnerTransitionError::Conflict); }
        let retry_attempt = stable_id("work-unit-next-attempt", &source.origin_attempt_id);
        let session_id = stable_id("work-unit-next-attempt-implementer-session", &retry_attempt);
        let invocation_id = stable_id("work-unit-next-attempt-implementer-invocation", &retry_attempt);
        let capture_intent = stable_id("work-unit-next-attempt-capture", &source.origin_attempt_id);
        let private_ref = format!("refs/codex-orchestrator/retry/{}", stable_id("work-unit-next-attempt-ref", &retry_attempt));
        let handoff = serde_json::json!({
            "handlerReturnReason": serde_json::from_str::<serde_json::Value>(&source.return_reason_json).map_err(|_| SprintRunnerTransitionError::Conflict)?,
            "priorClaims": {"summary": source.summary, "validationStatement": source.validation},
            "evidence": {"changedFiles": serde_json::from_str::<serde_json::Value>(&source.manifest_json).map_err(|_| SprintRunnerTransitionError::Conflict)?, "comparisonFingerprint": source.comparison_fingerprint, "contentFingerprints": serde_json::from_str::<serde_json::Value>(&source.content_fingerprints_json).map_err(|_| SprintRunnerTransitionError::Conflict)?}
        }).to_string();
        let handoff_fingerprint = stable_id("work-unit-next-attempt-handoff", &handoff);
        let capture_fingerprint = stable_id("work-unit-next-attempt-capture-lineage", &format!("{}:{}:{}:{}:{}", source.origin_attempt_id, source.review_invocation_id, source.decision_fingerprint, candidate_commit, candidate_tree));
        let desired = handler.current_implementer_revision().map_err(|_| SprintRunnerTransitionError::Unavailable("immutable later Implementer Harness revision unavailable".into()))?;
        let now = chrono::Utc::now().to_rfc3339();
        {
            let mut connection = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?;
            let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            let changed = transaction.execute(
                "INSERT OR IGNORE INTO work_unit_retry_attempts (work_unit_id,ordinal,origin_attempt_id,review_invocation_id,decision_fingerprint,sprint_git_authority_id,sprint_baseline_object_id,sprint_current_object_id,retry_attempt_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref,capture_intent_id,capture_fingerprint,handoff_json,handoff_fingerprint,candidate_commit_id,candidate_tree_id,private_ref_name,capture_requested_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
                params![source.work_unit_id,next_ordinal,source.origin_attempt_id,source.review_invocation_id,source.decision_fingerprint,source.authority_id,authority.baseline_object_id,authority.current_object_id,retry_attempt,session_id,invocation_id,desired.revision_id,desired.configuration_digest,desired.repository_commit_ref,capture_intent,capture_fingerprint,handoff,handoff_fingerprint,candidate_commit,candidate_tree,private_ref,now]
            ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
            if changed == 0 {
                let exact: bool = transaction.query_row(
                    "SELECT EXISTS(SELECT 1 FROM work_unit_retry_attempts WHERE work_unit_id=?1 AND ordinal=?2 AND origin_attempt_id=?3 AND review_invocation_id=?4 AND decision_fingerprint=?5 AND sprint_git_authority_id=?6 AND sprint_baseline_object_id=?7 AND sprint_current_object_id=?8 AND retry_attempt_id=?9 AND implementer_session_id=?10 AND implementer_invocation_id=?11 AND implementer_harness_revision_id=?12 AND implementer_harness_configuration_digest=?13 AND implementer_harness_repository_commit_ref=?14 AND capture_intent_id=?15 AND capture_fingerprint=?16 AND handoff_json=?17 AND handoff_fingerprint=?18 AND candidate_commit_id=?19 AND candidate_tree_id=?20 AND private_ref_name=?21)",
                    params![source.work_unit_id,next_ordinal,source.origin_attempt_id,source.review_invocation_id,source.decision_fingerprint,source.authority_id,authority.baseline_object_id,authority.current_object_id,retry_attempt,session_id,invocation_id,desired.revision_id,desired.configuration_digest,desired.repository_commit_ref,capture_intent,capture_fingerprint,handoff,handoff_fingerprint,candidate_commit,candidate_tree,private_ref], |row| row.get(0)
                ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if !exact {
                    transaction.execute("UPDATE work_unit_retry_attempts SET failure_reason='retry_immutable_lineage_mismatch' WHERE origin_attempt_id=?1", [&source.origin_attempt_id]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    return Err(SprintRunnerTransitionError::Conflict);
                }
            }
            transaction.commit().map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        }
        let (verified_commit, verified_tree) = match retry_git_candidate_facts(&authority, &source_root, &source_baseline) {
            Ok(facts) => facts,
            Err(error) => return self.fail_retry(&retry_attempt, "retry_candidate_revalidation_failed", error),
        };
        if verified_commit != candidate_commit || verified_tree != candidate_tree { return self.fail_retry(&retry_attempt, "retry_candidate_drift", SprintRunnerTransitionError::Conflict); }
        if let Err(error) = ensure_private_retry_ref(Path::new(&authority.repository_root), &private_ref, &candidate_commit) {
            return self.fail_retry(&retry_attempt, "retry_private_ref_pin_failed", error);
        }
        if let Err(error) = self.mark_retry(&retry_attempt, "candidate_pinned_at") {
            return self.fail_retry(&retry_attempt, "retry_candidate_pin_record_failed", error);
        }
        self.launch_retry_implementer(handler, &retry_attempt)
    }

    fn launch_retry_implementer(self: &Arc<Self>, handler: &Arc<WorkUnitExecutionHarnessService>, retry_attempt_id: &str) -> Result<(), SprintRunnerTransitionError> {
        let row: (String,String,String,String,String,String,String,String,String,String,String,String) = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT work_unit_id,retry_attempt_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref,sprint_git_authority_id,candidate_commit_id,candidate_tree_id,handoff_json,private_ref_name FROM work_unit_retry_attempts WHERE retry_attempt_id=?1 AND candidate_pinned_at IS NOT NULL", [retry_attempt_id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?,row.get(10)?,row.get(11)?))
        ).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let (work_unit_id,attempt,session_id,invocation_id,revision,digest,commit,authority,seed,tree,handoff,private_ref) = row;
        let git_authority = match self.authority_repository.load_initiated_sprint_git_authority(&authority) {
            Ok(Some(authority)) => authority,
            Ok(None) => return self.fail_retry(&attempt, "retry_authority_missing", SprintRunnerTransitionError::Conflict),
            Err(_) => return self.fail_retry(&attempt, "retry_authority_load_failed", SprintRunnerTransitionError::Unavailable("load retry Sprint Git authority".into())),
        };
        let pinned_target = match private_retry_ref_target(Path::new(&git_authority.repository_root), &private_ref) {
            Ok(target) => target,
            Err(error) => return self.fail_retry(&attempt, "retry_private_ref_verification_failed", error),
        };
        if pinned_target != seed { return self.fail_retry(&attempt, "retry_private_ref_mismatch", SprintRunnerTransitionError::Conflict); }
        let pinned = match handler.load_pinned_implementer_revision(&revision, &digest, &commit) {
            Ok(pinned) => pinned,
            Err(_) => return self.fail_retry(&attempt, "retry_harness_revision_invalid", SprintRunnerTransitionError::Conflict),
        };
        if let Err(error) = handler.authorize_implementer_attempt_at_seed(&attempt, &work_unit_id, &authority, Some(seed.clone())) {
            let _ = error;
            return self.fail_retry(&attempt, "retry_execution_authorization_failed", SprintRunnerTransitionError::Conflict);
        }
        self.mark_retry(&attempt, "authorized_at")?;
        let package = match handler.construct_for_pinned_profile(&attempt, WorkUnitHarnessRole::Implementer, pinned.profile) {
            Ok(package) => package,
            Err(_) => return self.fail_retry(&attempt, "retry_execution_support_failed", SprintRunnerTransitionError::Unavailable("retry execution-support grant failed".into())),
        };
        if let Err(error) = retry_git_workspace_facts(&git_authority, Path::new(package.working_directory()), &seed, &tree) {
            return self.fail_retry(&attempt, "retry_workspace_validation_failed", error);
        }
        self.mark_retry(&attempt, "execution_support_granted_at")?;
        self.mark_retry(&attempt, "isolated_worktree_ready_at")?;
        let session = AgentSessionId::new(session_id).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let invocation = AgentInvocationId::new(invocation_id).map_err(|_| SprintRunnerTransitionError::Conflict)?;
        let runtime = package.runtime_launch_configuration();
        if let Err(error) = self.sessions.create_application_session(CreateApplicationAgentSessionCommand { session_id: session.clone(), session: CreateAgentSessionCommand { title: Some("Work Unit Later Attempt Implementer".into()), working_directory: Some(package.working_directory().into()), requested_options: runtime.requested_options.clone() }}) { return self.fail_retry(&attempt, "retry_session_creation_failed", SprintRunnerTransitionError::Unavailable(error.to_string())); }
        self.mark_retry(&attempt, "implementer_session_created_at")?;
        // Policy B: a runtime launch error terminally fails this exact prepared invocation.  A
        // reopen observes that durable fact; it neither launches it again nor allocates another.
        let existing_history = self.sessions.load_session(&session).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if let Some(existing) = existing_history.invocations.iter().find(|entry| entry.invocation.id == invocation) {
            if existing.invocation.status == AgentInvocationStatus::Failed {
                self.record_retry_failure(&attempt, "retry_terminal_launch_failed")?;
                return Ok(());
            }
            if existing.invocation.status.is_terminal() {
                return self.fail_retry(&attempt, "retry_terminal_invocation_incompatible", SprintRunnerTransitionError::Conflict);
            }
        }
        let prompt = format!("Work Unit Implementer later correction attempt. Work only in the application-provided isolated workspace. The bounded correction handoff is below; it contains no private route, path, ref, or object id. Do not accept, review, create another attempt, settle, activate dependents, or continue Sprint or Epic work.\n\n{handoff}");
        if let Err(error) = self.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: prompt.clone(), title: None, working_directory: Some(package.working_directory().into()), requested_options: Some(runtime.requested_options.clone()) }}) { return self.fail_retry(&attempt, "retry_invocation_preparation_failed", SprintRunnerTransitionError::Unavailable(error.to_string())); }
        self.mark_retry(&attempt, "implementer_invocation_prepared_at")?;
        if let Err(error) = package.bind_correlated_invocation(session.clone(), invocation.clone()) {
            let _ = error;
            return self.fail_retry(&attempt, "retry_harness_binding_failed", SprintRunnerTransitionError::Conflict);
        }
        self.mark_retry(&attempt, "implementer_harness_bound_at")?;
        // This intent precedes inspecting or requesting launch so a reopen that observes an
        // accepted prepared invocation still has a truthful launch-request stage.
        self.mark_retry(&attempt, "launch_requested_at")?;
        match self.sessions.application_invocation_launch_evidence(&invocation, &session).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string())) {
            Err(error) => return self.fail_retry(&attempt, "retry_launch_inspection_failed", error),
            Ok(ApplicationInvocationLaunchEvidence::LaunchAccepted) => { self.mark_retry(&attempt, "launch_accepted_at")?; }
            Ok(ApplicationInvocationLaunchEvidence::PersistedNotAccepted) => {
                let launch = match self.sessions.launch_prepared_application_invocation_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: prompt, title: None, working_directory: Some(package.working_directory().into()), requested_options: Some(runtime.requested_options) }}, Some(runtime.extension)) { Ok(launch) => launch, Err(error) => return self.fail_retry(&attempt, "retry_launch_failed", SprintRunnerTransitionError::Unavailable(error.to_string())), };
                if launch.launch_accepted { self.mark_retry(&attempt, "launch_accepted_at")?; } else {
                    let history = self.sessions.load_session(&session).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                    let terminal = history.invocations.iter().find(|entry| entry.invocation.id == invocation).is_some_and(|entry| entry.invocation.status == AgentInvocationStatus::Failed);
                    return self.fail_retry(&attempt, if terminal { "retry_terminal_launch_failed" } else { "retry_launch_not_accepted" }, SprintRunnerTransitionError::Unavailable("retry Implementer launch was not accepted".into()));
                }
            }
            Ok(ApplicationInvocationLaunchEvidence::NeverPersisted) => return self.fail_retry(&attempt, "retry_launch_missing_prepared_invocation", SprintRunnerTransitionError::Conflict),
        }
        self.clear_retry_failure(&attempt)?;
        if let Ok(observation) = package.observe_correlated_invocation() { if let Some(activity) = observation.provider_activity { self.mark_retry_at(&attempt, "provider_activation_observed_at", activity.recorded_at.to_rfc3339())?; } }
        self.mark_retry(&attempt, "retry_ready_at")?;
        Ok(())
    }

    fn mark_retry(&self, retry_attempt_id: &str, column: &str) -> Result<(), SprintRunnerTransitionError> { self.mark_retry_at(retry_attempt_id, column, chrono::Utc::now().to_rfc3339()) }
    fn mark_retry_at(&self, retry_attempt_id: &str, column: &str, at: String) -> Result<(), SprintRunnerTransitionError> {
        if !["candidate_pinned_at","authorized_at","execution_support_granted_at","isolated_worktree_ready_at","implementer_session_created_at","implementer_invocation_prepared_at","implementer_harness_bound_at","launch_requested_at","launch_accepted_at","provider_activation_observed_at","retry_ready_at"].contains(&column) { return Err(SprintRunnerTransitionError::Unavailable("invalid retry stage".into())); }
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(&format!("UPDATE work_unit_retry_attempts SET {column}=COALESCE({column},?2) WHERE retry_attempt_id=?1"), params![retry_attempt_id,at]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        Ok(())
    }
    fn record_retry_failure(&self, retry_attempt_id: &str, reason: &str) -> Result<(), SprintRunnerTransitionError> {
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_retry_attempts SET failure_reason=?2 WHERE retry_attempt_id=?1", params![retry_attempt_id,reason]).map_err(|database_error| SprintRunnerTransitionError::Unavailable(database_error.to_string()))?;
        Ok(())
    }
    fn fail_retry_for_invocation<T>(&self, invocation: &AgentInvocationId, reason: &str) -> Result<T, SprintRunnerTransitionError> {
        let changed = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
            .execute("UPDATE work_unit_retry_attempts SET failure_reason=COALESCE(failure_reason,?2) WHERE implementer_invocation_id=?1", params![invocation.as_str(), reason])
            .map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        if changed != 1 { return Err(SprintRunnerTransitionError::Conflict); }
        Err(SprintRunnerTransitionError::Forbidden)
    }
    fn fail_retry<T>(&self, retry_attempt_id: &str, reason: &str, error: SprintRunnerTransitionError) -> Result<T, SprintRunnerTransitionError> {
        self.record_retry_failure(retry_attempt_id, reason)?;
        Err(error)
    }
    fn fail_retry_for_origin<T>(&self, origin_attempt_id: &str, reason: &str, error: SprintRunnerTransitionError) -> Result<T, SprintRunnerTransitionError> {
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_retry_attempts SET failure_reason=?2 WHERE origin_attempt_id=?1", params![origin_attempt_id,reason]).map_err(|database_error| SprintRunnerTransitionError::Unavailable(database_error.to_string()))?;
        Err(error)
    }
    fn clear_retry_failure(&self, retry_attempt_id: &str) -> Result<(), SprintRunnerTransitionError> { self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_retry_attempts SET failure_reason=NULL WHERE retry_attempt_id=?1", [retry_attempt_id]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?; Ok(()) }

    fn reconcile_work_unit_handler(
        self: &Arc<Self>,
        handler: &Arc<WorkUnitExecutionHarnessService>,
        work_unit_id: &str,
        materialization_id: &str,
        sprint_id: &str,
    ) -> Result<(), SprintRunnerTransitionError> {
        let lock = self.transition_lock(sprint_id)?;
        let _guard = lock.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("Work Unit activation lock is poisoned".into()))?;
        let authority: Option<String> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT authority_id FROM initiated_sprint_git_authorities WHERE sprint_id=?1 ORDER BY recorded_at,authority_id LIMIT 1", [sprint_id], |row| row.get(0)
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let eligibility: Option<(String, Option<String>)> = self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT eligibility_state,blocked_reason FROM work_unit_dependency_activation_intents WHERE work_unit_id=?1 AND materialization_id=?2",
            params![work_unit_id, materialization_id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let blocked = match eligibility {
            Some((state, None)) if state == "eligible" && authority.is_some() => None,
            Some((state, Some(reason))) if state == "blocked" => Some(reason),
            Some(_) => return Err(SprintRunnerTransitionError::Conflict),
            None => Some("dependency_eligibility_not_recorded".into()),
        };
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
                params![work_unit_id,if blocked.is_some(){"blocked"}else{"eligible"},blocked.as_deref()]
            ).map_err(|e| SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        } else {
            let desired = handler.current_handler_revision()
                .map_err(|_| SprintRunnerTransitionError::Unavailable("load current immutable Handler revision".into()))?;
            self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(
                "INSERT OR IGNORE INTO work_unit_handler_activations (work_unit_id,materialization_id,sprint_id,attempt_id,handler_session_id,handler_invocation_id,handler_harness_key,handler_harness_version,handler_harness_revision_id,handler_harness_configuration_digest,handler_harness_repository_commit_ref,eligibility_state,blocked_reason,requested_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![work_unit_id,materialization_id,sprint_id,attempt_id,session_id,invocation_id,desired.harness_key,desired.profile.version,desired.revision_id,desired.configuration_digest,desired.repository_commit_ref,if blocked.is_some(){"blocked"}else{"eligible"},blocked.as_deref(),now]
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
        match self.sessions.application_invocation_launch_evidence(&invocation, &session) {
            Err(error) => return self.fail_handler_action(
                work_unit_id,
                "handler_action_launch_evidence_failed",
                SprintRunnerTransitionError::Unavailable(error.to_string()),
            ),
            Ok(ApplicationInvocationLaunchEvidence::LaunchAccepted) => {
                self.mark_handler_action(work_unit_id, "launch_accepted_at")?;
                self.mark_handler_action(work_unit_id, "action_ready_at")?;
            }
            Ok(ApplicationInvocationLaunchEvidence::PersistedNotAccepted) => {
                self.mark_handler_action(work_unit_id, "launch_requested_at")?;
                let action_status: String = self.connection.lock()
                    .map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?
                    .query_row(
                        "SELECT status FROM agent_session_invocations WHERE id=?1 AND session_id=?2",
                        params![invocation.as_str(), session.as_str()],
                        |row| row.get(0),
                    ).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
                if action_status != "pending" {
                    return self.fail_handler_action(
                        work_unit_id,
                        "handler_action_launch_not_accepted",
                        SprintRunnerTransitionError::Unavailable(
                            "Handler action continuation is terminal without launch acceptance".into(),
                        ),
                    );
                }
                let injection = self.prepare_work_unit_handler_action(invocation.clone())?;
                runtime.extension.additional_args.extend(injection.configuration_args);
                runtime.extension.environment.push(injection.environment);
                let launch = match self.sessions.launch_prepared_application_invocation_with_launch_observation(
                    SendIdempotentApplicationAgentSessionMessageCommand {
                        invocation_id: invocation.clone(),
                        message: SendAgentSessionMessageCommand {
                            session_id: Some(session.clone()), submitted_text: prompt, title: None,
                            working_directory: Some(package.working_directory().into()),
                            requested_options: Some(runtime.requested_options),
                        },
                    }, Some(runtime.extension),
                ) {
                    Ok(launch) => launch,
                    Err(error) => return self.fail_handler_action(
                        work_unit_id,
                        "handler_action_launch_failed",
                        SprintRunnerTransitionError::Unavailable(error.to_string()),
                    ),
                };
                if launch.launch_accepted {
                    self.mark_handler_action(work_unit_id, "launch_accepted_at")?;
                    self.mark_handler_action(work_unit_id, "action_ready_at")?;
                } else {
                    return self.fail_handler_action(
                        work_unit_id,
                        "handler_action_launch_not_accepted",
                        SprintRunnerTransitionError::Unavailable(
                            "Handler action continuation launch was not accepted".into(),
                        ),
                    );
                }
            }
            Ok(ApplicationInvocationLaunchEvidence::NeverPersisted) => return self.fail_handler_action(
                work_unit_id,
                "handler_action_launch_missing_prepared_invocation",
                SprintRunnerTransitionError::Conflict,
            ),
        }
        self.clear_handler_action_failure(work_unit_id)?;
        if let Ok(observation) = package.observe_correlated_invocation() {
            if let Some(activity) = observation.provider_activity {
                self.mark_handler_action_at(work_unit_id, "provider_activation_observed_at", activity.recorded_at.to_rfc3339())?;
            }
        }
        Ok(())
    }

    fn request_work_unit_implementer(self:&Arc<Self>,handler_invocation:&AgentInvocationId)->Result<(),SprintRunnerTransitionError>{
        self.request_work_unit_implementer_inner(handler_invocation, true)
    }

    fn request_work_unit_implementer_inner(self:&Arc<Self>,handler_invocation:&AgentInvocationId,require_active_action:bool)->Result<(),SprintRunnerTransitionError>{
        let handler=self.work_unit_handler.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit Handler registry is poisoned".into()))?.clone().ok_or_else(||SprintRunnerTransitionError::Unavailable("Work Unit Handler activation is unavailable".into()))?;
        let row:Option<(String,String,String,String,String,String,String,String)>=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row(
            "SELECT c.work_unit_id,c.attempt_id,h.sprint_id,c.handler_session_id,h.handler_invocation_id,
                    c.action_harness_revision_id,c.action_harness_configuration_digest,c.action_harness_repository_commit_ref
             FROM work_unit_handler_action_continuations c
             JOIN work_unit_handler_activations h
               ON h.work_unit_id=c.work_unit_id
              AND h.attempt_id=c.attempt_id
              AND h.handler_session_id=c.handler_session_id
              AND h.handler_invocation_id=c.original_handler_invocation_id
             WHERE c.action_invocation_id=?1
               AND c.blocked_reason IS NULL
               AND c.failure_reason IS NULL
               AND c.authorized_at IS NOT NULL
               AND c.invocation_prepared_at IS NOT NULL
               AND c.harness_bound_at IS NOT NULL
               AND c.launch_requested_at IS NOT NULL
               AND c.launch_accepted_at IS NOT NULL
               AND c.action_ready_at IS NOT NULL
               AND h.eligibility_state='eligible'
               AND h.blocked_reason IS NULL
               AND h.handler_invocation_prepared_at IS NOT NULL
               AND h.handler_harness_bound_at IS NOT NULL
               AND h.launch_requested_at IS NOT NULL
               AND h.launch_accepted_at IS NOT NULL
               AND h.handler_ready_at IS NOT NULL
               AND EXISTS (
                   SELECT 1 FROM agent_session_invocations original
                   WHERE original.id=c.original_handler_invocation_id
                     AND original.session_id=c.handler_session_id
                     AND original.input_provenance='application'
                     AND original.status IN ('completed','failed','canceled','interrupted')
               )
               AND EXISTS (
                   SELECT 1 FROM agent_session_invocations action
                   WHERE action.id=c.action_invocation_id
                     AND action.session_id=c.handler_session_id
                     AND action.input_provenance='application'
                     AND (?2=0 OR action.status IN ('pending','running'))
               )
               AND (?2=1 OR EXISTS (
                   SELECT 1 FROM work_unit_implementer_activations persisted
                   WHERE persisted.work_unit_id=c.work_unit_id
                     AND persisted.handler_attempt_id=c.attempt_id
                     AND persisted.handler_invocation_id=c.action_invocation_id
                     AND persisted.attempt_id=c.attempt_id
               ))",
            params![handler_invocation.as_str(), if require_active_action { 1 } else { 0 }],
            |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)),
        ).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        let Some((work_unit,handler_attempt,sprint,handler_session,original_handler_invocation,handler_revision,handler_digest,handler_commit))=row else{return Err(SprintRunnerTransitionError::Forbidden)};
        if handler_attempt != stable_id("work-unit-handler-attempt", &work_unit)
            || handler_session != stable_id("work-unit-handler-session", &work_unit)
            || original_handler_invocation != stable_id("work-unit-handler-invocation", &work_unit)
            || handler_invocation.as_str() != stable_id("work-unit-handler-action-invocation", &handler_attempt)
        { return Err(SprintRunnerTransitionError::Forbidden) }
        let action=handler.load_pinned_handler_revision(&handler_revision,&handler_digest,&handler_commit).map_err(|_|SprintRunnerTransitionError::Forbidden)?;
        if !action.profile.mcp.required || action.profile.mcp.enabled_tools != ["request_work_unit_implementer"] { return Err(SprintRunnerTransitionError::Forbidden) }
        let persisted:bool=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM agent_session_invocations WHERE id=?1 AND session_id=?2 AND input_provenance='application')",params![handler_invocation.as_str(),handler_session],|r|r.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;if !persisted{return Err(SprintRunnerTransitionError::Forbidden)}
        let authority:String=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT authority_id FROM initiated_sprint_git_authorities WHERE sprint_id=?1 ORDER BY recorded_at,authority_id LIMIT 1",[&sprint],|r|r.get(0)).optional().map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?.ok_or(SprintRunnerTransitionError::Forbidden)?;
        let attempt=handler_attempt.clone();let session_id=stable_id("work-unit-implementer-session",&handler_attempt);let invocation_id=stable_id("work-unit-implementer-invocation",&handler_attempt);
        let lock=self.transition_lock(&sprint)?;let _guard=lock.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("Work Unit activation lock is poisoned".into()))?;
        let existing:bool=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_implementer_activations WHERE work_unit_id=?1)",[&work_unit],|row|row.get(0)).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;
        if !existing { let desired=handler.current_implementer_revision().map_err(|_|SprintRunnerTransitionError::Unavailable("immutable Implementer Harness revision unavailable".into()))?;self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("INSERT OR IGNORE INTO work_unit_implementer_activations (work_unit_id,handler_attempt_id,handler_invocation_id,attempt_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref,requested_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![work_unit,handler_attempt,handler_invocation.as_str(),attempt,session_id,invocation_id,desired.revision_id,desired.configuration_digest,desired.repository_commit_ref,chrono::Utc::now().to_rfc3339()]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?; }
        let (attempt,session_id,invocation_id,revision_id,digest,commit):(String,String,String,String,String,String)=self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.query_row("SELECT attempt_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref FROM work_unit_implementer_activations WHERE work_unit_id=?1 AND handler_attempt_id=?2 AND handler_invocation_id=?3",params![work_unit,handler_attempt,handler_invocation.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|_|SprintRunnerTransitionError::Conflict)?;
        if attempt != handler_attempt
            || session_id != stable_id("work-unit-implementer-session", &handler_attempt)
            || invocation_id != stable_id("work-unit-implementer-invocation", &handler_attempt)
        { return self.fail_implementer(&work_unit, "pinned_implementer_identity_invalid", SprintRunnerTransitionError::Conflict) }
        let pinned = match handler.load_pinned_implementer_revision(&revision_id, &digest, &commit) {
            Ok(pinned) => pinned,
            Err(_) => return self.fail_implementer(&work_unit, "pinned_implementer_harness_invalid", SprintRunnerTransitionError::Conflict),
        };
        if handler.authorize_implementer_attempt(&attempt, &work_unit, &authority).is_err() {
            return self.fail_implementer(&work_unit, "implementer_authorization_failed", SprintRunnerTransitionError::Conflict);
        }
        self.mark_implementer(&work_unit, "authorized_at")?;
        let package = match handler.construct_for_pinned_profile(&attempt, WorkUnitHarnessRole::Implementer, pinned.profile) {
            Ok(package) => package,
            Err(_) => return self.fail_implementer(&work_unit, "implementer_execution_support_failed", SprintRunnerTransitionError::Unavailable("Implementer Harness package construction failed".into())),
        };
        self.mark_implementer(&work_unit, "execution_support_granted_at")?;
        self.mark_implementer(&work_unit, "isolated_worktree_ready_at")?;
        let session = match AgentSessionId::new(session_id) {
            Ok(session) => session,
            Err(error) => return self.fail_implementer(&work_unit, "implementer_session_identity_invalid", SprintRunnerTransitionError::Unavailable(error.to_string())),
        };
        let invocation = match AgentInvocationId::new(invocation_id) {
            Ok(invocation) => invocation,
            Err(error) => return self.fail_implementer(&work_unit, "implementer_invocation_identity_invalid", SprintRunnerTransitionError::Unavailable(error.to_string())),
        };
        let runtime = package.runtime_launch_configuration();
        if let Err(error) = self.sessions.create_application_session(CreateApplicationAgentSessionCommand { session_id: session.clone(), session: CreateAgentSessionCommand { title: Some("Work Unit Implementer".into()), working_directory: Some(package.working_directory().into()), requested_options: runtime.requested_options.clone() }}) {
            return self.fail_implementer(&work_unit, "implementer_session_creation_failed", SprintRunnerTransitionError::Unavailable(error.to_string()));
        }
        self.mark_implementer(&work_unit, "implementer_session_created_at")?;
        let prompt = "Work Unit Implementer activation. Work only in the application-provided isolated execution workspace. Do not submit outcomes, accept, review, settle, retry, activate dependents, or continue any Sprint or Epic.".to_string();
        if let Err(error) = self.sessions.prepare_idempotent_application_invocation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: prompt.clone(), title: None, working_directory: None, requested_options: Some(runtime.requested_options.clone()) }}) {
            return self.fail_implementer(&work_unit, "implementer_invocation_preparation_failed", SprintRunnerTransitionError::Unavailable(error.to_string()));
        }
        self.mark_implementer(&work_unit, "implementer_invocation_prepared_at")?;
        if package.bind_correlated_invocation(session.clone(), invocation.clone()).is_err() {
            return self.fail_implementer(&work_unit, "implementer_harness_binding_failed", SprintRunnerTransitionError::Conflict);
        }
        self.mark_implementer(&work_unit, "implementer_harness_bound_at")?;
        match self.sessions.application_invocation_launch_evidence(&invocation, &session) {
            Err(error) => return self.fail_implementer(&work_unit, "implementer_launch_evidence_failed", SprintRunnerTransitionError::Unavailable(error.to_string())),
            Ok(ApplicationInvocationLaunchEvidence::LaunchAccepted) => {
                self.mark_implementer(&work_unit, "launch_accepted_at")?;
                self.mark_implementer(&work_unit, "implementer_ready_at")?;
            }
            Ok(ApplicationInvocationLaunchEvidence::PersistedNotAccepted) => {
                self.mark_implementer(&work_unit, "launch_requested_at")?;
                let launch = match self.sessions.launch_prepared_application_invocation_with_launch_observation(SendIdempotentApplicationAgentSessionMessageCommand { invocation_id: invocation.clone(), message: SendAgentSessionMessageCommand { session_id: Some(session.clone()), submitted_text: prompt, title: None, working_directory: Some(package.working_directory().into()), requested_options: Some(runtime.requested_options) }}, Some(runtime.extension)) {
                    Ok(launch) => launch,
                    Err(error) => return self.fail_implementer(&work_unit, "implementer_launch_failed", SprintRunnerTransitionError::Unavailable(error.to_string())),
                };
                if launch.launch_accepted {
                    self.mark_implementer(&work_unit, "launch_accepted_at")?;
                    self.mark_implementer(&work_unit, "implementer_ready_at")?;
                } else {
                    return self.fail_implementer(
                        &work_unit,
                        "implementer_launch_not_accepted",
                        SprintRunnerTransitionError::Unavailable(
                            "Implementer launch was not accepted".into(),
                        ),
                    );
                }
            }
            Ok(ApplicationInvocationLaunchEvidence::NeverPersisted) => return self.fail_implementer(&work_unit, "implementer_launch_missing_prepared_invocation", SprintRunnerTransitionError::Conflict),
        };
        self.clear_implementer_failure(&work_unit)?;
        if let Ok(observation)=package.observe_correlated_invocation(){if let Some(activity)=observation.provider_activity{self.mark_implementer_at(&work_unit,"provider_activation_observed_at",activity.recorded_at.to_rfc3339())?}};Ok(())
    }
    fn mark_implementer(&self,work_unit:&str,column:&str)->Result<(),SprintRunnerTransitionError>{self.mark_implementer_at(work_unit,column,chrono::Utc::now().to_rfc3339())}
    fn mark_implementer_at(&self,work_unit:&str,column:&str,at:String)->Result<(),SprintRunnerTransitionError>{if !["authorized_at","execution_support_granted_at","isolated_worktree_ready_at","implementer_session_created_at","implementer_invocation_prepared_at","implementer_harness_bound_at","launch_requested_at","launch_accepted_at","provider_activation_observed_at","implementer_ready_at"].contains(&column){return Err(SprintRunnerTransitionError::Unavailable("invalid Implementer activation stage".into()))}self.connection.lock().map_err(|_|SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute(&format!("UPDATE work_unit_implementer_activations SET {column}=COALESCE({column},?2) WHERE work_unit_id=?1"),params![work_unit,at]).map_err(|e|SprintRunnerTransitionError::Unavailable(e.to_string()))?;Ok(())}
    fn fail_implementer<T>(&self, work_unit: &str, reason: &str, error: SprintRunnerTransitionError) -> Result<T, SprintRunnerTransitionError> {
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_activations SET failure_reason=?2 WHERE work_unit_id=?1", params![work_unit, reason]).map_err(|database_error| SprintRunnerTransitionError::Unavailable(database_error.to_string()))?;
        Err(error)
    }
    fn clear_implementer_failure(&self, work_unit: &str) -> Result<(), SprintRunnerTransitionError> {
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_implementer_activations SET failure_reason=NULL WHERE work_unit_id=?1", [work_unit]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        Ok(())
    }

    fn fail_handler_action<T>(&self, work_unit: &str, reason: &str, error: SprintRunnerTransitionError) -> Result<T, SprintRunnerTransitionError> {
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_handler_action_continuations SET failure_reason=?2 WHERE work_unit_id=?1", params![work_unit, reason]).map_err(|database_error| SprintRunnerTransitionError::Unavailable(database_error.to_string()))?;
        Err(error)
    }
    fn clear_handler_action_failure(&self, work_unit: &str) -> Result<(), SprintRunnerTransitionError> {
        self.connection.lock().map_err(|_| SprintRunnerTransitionError::Unavailable("planning database lock is poisoned".into()))?.execute("UPDATE work_unit_handler_action_continuations SET failure_reason=NULL WHERE work_unit_id=?1", [work_unit]).map_err(|error| SprintRunnerTransitionError::Unavailable(error.to_string()))?;
        Ok(())
    }

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
    pub(crate) fn set_test_work_unit_handler_post_pass_hook(
        &self,
        hook: Arc<dyn Fn() + Send + Sync>,
    ) {
        *self
            .test_work_unit_handler_post_pass_hook
            .lock()
            .expect("test work unit handler post-pass hook") = Some(hook);
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

fn handler_review_reconciliation_lock(
    database_lock_key: &str,
) -> Result<Arc<Mutex<()>>, SprintRunnerTransitionError> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut locks = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| SprintRunnerTransitionError::Unavailable("Handler review lock registry is poisoned".into()))?;
    Ok(locks
        .entry(database_lock_key.to_owned())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone())
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
fn validate_handler_review_return(value:&HandlerReviewReturnReason)->Result<(),SprintRunnerTransitionError>{if !safe_id(&value.code)||value.code.len()>96||value.explanation.trim().is_empty()||value.explanation.len()>2_000{return Err(SprintRunnerTransitionError::Invalid)}Ok(())}
fn validate_handback_disposition(value:&SprintHandbackDisposition)->Result<(),SprintRunnerTransitionError>{
    if !safe_id(&value.movement_kind) || value.movement_kind.len()>96 { return Err(SprintRunnerTransitionError::Invalid); }
    validate_outcome(&value.rationale)?;
    match value.movement_kind.as_str() {
        "continue_eligible_work" => if value.eligible_work_summary.as_deref().map(validate_outcome).transpose()?.is_none() || value.dependency_owner.is_some() || value.dependency_owner_classification.is_some() || value.enabling_result.is_some() || value.resumption_path.is_some() || value.local_exhaustion_summary.is_some() { return Err(SprintRunnerTransitionError::Invalid); },
        "wait_for_agent_dependency" => {
            let Some(owner) = value.dependency_owner.as_deref() else { return Err(SprintRunnerTransitionError::Invalid); };
            validate_outcome(owner)?;
            if value.dependency_owner_classification.is_none() || value.enabling_result.as_deref().map(validate_outcome).transpose()?.is_none() || value.resumption_path.as_deref().map(validate_outcome).transpose()?.is_none() || value.eligible_work_summary.is_some() || value.local_exhaustion_summary.is_some() { return Err(SprintRunnerTransitionError::Invalid); }
            let lowered = owner.to_ascii_lowercase();
            if ["human", "external", "approval", "manual", "user"].iter().any(|term| lowered.contains(term)) { return Err(SprintRunnerTransitionError::Invalid); }
        },
        "local_exhaustion_escalate" => if value.local_exhaustion_summary.as_deref().map(validate_outcome).transpose()?.is_none() || value.eligible_work_summary.is_some() || value.dependency_owner.is_some() || value.dependency_owner_classification.is_some() || value.enabling_result.is_some() || value.resumption_path.is_some() { return Err(SprintRunnerTransitionError::Invalid); },
        _ => { for text in [&value.eligible_work_summary,&value.dependency_owner,&value.enabling_result,&value.resumption_path,&value.local_exhaustion_summary] { if let Some(text)=text { validate_outcome(text)?; } } }
    }
    Ok(())
}
fn validate_epic_escalation_disposition(value:&EpicEscalationReassessmentDisposition)->Result<(),SprintRunnerTransitionError>{
    if !safe_id(&value.movement_kind)||value.movement_kind.len()>96{return Err(SprintRunnerTransitionError::Invalid)}
    validate_outcome(&value.rationale)?;
    if let Some(intent)=value.considered_intent.as_deref(){validate_outcome(intent)?}
    if let Some(request)=&value.downstream_request{validate_outcome(&request.request)?;validate_outcome(&request.resumption_path)?}
    if let Some(attention)=&value.human_external_attention{for text in [&attention.reason,&attention.authority_needed,&attention.evidence_context,&attention.resumption_path]{validate_outcome(text)?}}
    match value.movement_kind.as_str(){
        "return_context_to_sprint_runner"=>if !matches!(value.downstream_request.as_ref().map(|r|&r.target),Some(EpicEscalationDownstreamTarget::SprintRunner))||value.downstream_request.as_ref().and_then(|r|r.dependency.as_ref()).is_some()||value.human_external_attention.is_some()||value.considered_intent.is_some(){return Err(SprintRunnerTransitionError::Invalid)},
        "await_existing_agent_dependency"=>if !matches!(value.downstream_request.as_ref().map(|r|&r.target),Some(EpicEscalationDownstreamTarget::ExistingAgentAchievableDependency))||value.downstream_request.as_ref().and_then(|r|r.dependency.as_ref()).is_none()||value.human_external_attention.is_some()||value.considered_intent.is_some(){return Err(SprintRunnerTransitionError::Invalid)},
        "human_or_external_attention"=>if value.downstream_request.is_some()||value.human_external_attention.is_none()||value.considered_intent.is_some(){return Err(SprintRunnerTransitionError::Invalid)},
        "consider_other_epic_work"=>if value.downstream_request.is_some()||value.human_external_attention.is_some()||value.considered_intent.is_none(){return Err(SprintRunnerTransitionError::Invalid)},
        _=>if value.downstream_request.is_some()||value.human_external_attention.is_some()||value.considered_intent.is_none(){return Err(SprintRunnerTransitionError::Invalid)},
    }
    Ok(())
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
fn fingerprint_bytes(prefix: &str,value:&[u8])->String{stable_id(prefix,&value.iter().map(|byte|format!("{byte:02x}")).collect::<String>())}

fn retry_git_text(root: &Path, arguments: &[&str]) -> Result<String, SprintRunnerTransitionError> {
    let output = Command::new("git").args(arguments).current_dir(root).env("GIT_TERMINAL_PROMPT", "0").output()
        .map_err(|error| SprintRunnerTransitionError::Unavailable(format!("retry Git invocation failed: {error}")))?;
    if !output.status.success() || output.stdout.len() > 256_000 || output.stderr.len() > 256_000 { return Err(SprintRunnerTransitionError::Conflict); }
    String::from_utf8(output.stdout).map(|value| value.trim().to_owned()).map_err(|_| SprintRunnerTransitionError::Conflict)
}
fn retry_git_candidate_facts(authority: &InitiatedSprintGitAuthority, root: &Path, source_baseline: &str) -> Result<(String, String), SprintRunnerTransitionError> {
    let root = root.canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let authority_root = PathBuf::from(&authority.worktree_root).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let repository_root = PathBuf::from(&authority.repository_root).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let expected_common = PathBuf::from(&authority.repository_common_dir).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    if root == authority_root || root == repository_root || retry_git_text(&root, &["status", "--porcelain"])? != "" { return Err(SprintRunnerTransitionError::Conflict); }
    let top = PathBuf::from(retry_git_text(&root, &["rev-parse", "--show-toplevel"])?).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let common = PathBuf::from(retry_git_text(&root, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    if top != root || common != expected_common { return Err(SprintRunnerTransitionError::Conflict); }
    let commit = retry_git_text(&root, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let tree = retry_git_text(&root, &["rev-parse", "--verify", "HEAD^{tree}"])?;
    if commit.len() != 40 || tree.len() != 40
        || retry_git_text(&root, &["merge-base", "--is-ancestor", &authority.current_object_id, &commit]).is_err()
        || retry_git_text(&root, &["merge-base", "--is-ancestor", source_baseline, &commit]).is_err()
    { return Err(SprintRunnerTransitionError::Conflict); }
    Ok((commit, tree))
}
fn retry_git_workspace_facts(authority: &InitiatedSprintGitAuthority, root: &Path, expected_commit: &str, expected_tree: &str) -> Result<(), SprintRunnerTransitionError> {
    let root = root.canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let authority_root = PathBuf::from(&authority.worktree_root).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let repository_root = PathBuf::from(&authority.repository_root).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let expected_common = PathBuf::from(&authority.repository_common_dir).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    if root == authority_root || root == repository_root || retry_git_text(&root, &["status", "--porcelain"])? != "" || retry_git_text(&root, &["rev-parse", "--abbrev-ref", "HEAD"])? != "HEAD" { return Err(SprintRunnerTransitionError::Conflict); }
    let top = PathBuf::from(retry_git_text(&root, &["rev-parse", "--show-toplevel"])?).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let common = PathBuf::from(retry_git_text(&root, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?).canonicalize().map_err(|_| SprintRunnerTransitionError::Conflict)?;
    let commit = retry_git_text(&root, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let tree = retry_git_text(&root, &["rev-parse", "--verify", "HEAD^{tree}"])?;
    if top != root || common != expected_common || commit != expected_commit || tree != expected_tree { return Err(SprintRunnerTransitionError::Conflict); }
    Ok(())
}
fn private_retry_ref_target(repository_root: &Path, private_ref: &str) -> Result<String, SprintRunnerTransitionError> {
    if !private_ref.starts_with("refs/codex-orchestrator/retry/") || !safe_id(private_ref.rsplit('/').next().unwrap_or("")) { return Err(SprintRunnerTransitionError::Conflict); }
    retry_git_text(repository_root, &["rev-parse", "--verify", &format!("{private_ref}^{{commit}}")])
}
fn ensure_private_retry_ref(repository_root: &Path, private_ref: &str, commit: &str) -> Result<(), SprintRunnerTransitionError> {
    if !private_ref.starts_with("refs/codex-orchestrator/retry/") || !safe_id(private_ref.rsplit('/').next().unwrap_or("")) || commit.len() != 40 { return Err(SprintRunnerTransitionError::Conflict); }
    let existing = Command::new("git").args(["show-ref", "--verify", "--quiet", private_ref]).current_dir(repository_root).env("GIT_TERMINAL_PROMPT", "0").output()
        .map_err(|error| SprintRunnerTransitionError::Unavailable(format!("inspect retry candidate pin: {error}")))?;
    if existing.status.success() { return if private_retry_ref_target(repository_root, private_ref)? == commit { Ok(()) } else { Err(SprintRunnerTransitionError::Conflict) }; }
    if existing.status.code() != Some(1) { return Err(SprintRunnerTransitionError::Conflict); }
    // The all-zero old value atomically proves absence.  A concurrent or foreign pin therefore
    // fails closed instead of being overwritten or adopted by name alone.
    let output = Command::new("git").args(["update-ref", private_ref, commit, "0000000000000000000000000000000000000000"]).current_dir(repository_root).env("GIT_TERMINAL_PROMPT", "0").output()
        .map_err(|error| SprintRunnerTransitionError::Unavailable(format!("pin retry candidate: {error}")))?;
    if !output.status.success() || private_retry_ref_target(repository_root, private_ref)? != commit { return Err(SprintRunnerTransitionError::Conflict); }
    Ok(())
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
struct EpicRunnerEscalationReassessmentMcp { service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId,tool_router:ToolRouter<Self> }
impl EpicRunnerEscalationReassessmentMcp { fn new(service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} }
#[tool_router] impl EpicRunnerEscalationReassessmentMcp { #[tool(description="Read only the application-correlated accepted Epic plan horizon, current Sprint state, concern, known dependencies, and other available Epic work. Input is ONLY {}.")] fn read_epic_escalation_reassessment_context(&self)->CallToolResult{match self.service.epic_escalation_reassessment_context(&self.invocation_id){Ok(context)=>CallToolResult::success(vec![ContentBlock::text(context.to_string())]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(SprintRunnerTransitionError::Conflict)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"conflict\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"unavailable\"}")] )}} #[tool(description="Record one identity-free, concern-preserving Epic disposition. It may persist one bounded request to the Sprint Runner or an existing agent-achievable dependency, one human/external attention record, or intent only. It never delivers, activates, starts a Sprint, settles, completes, or accepts.")] fn record_epic_escalation_disposition(&self,Parameters(input):Parameters<EpicEscalationReassessmentDisposition>)->CallToolResult{match self.service.record_epic_escalation_disposition(&self.invocation_id,input){Ok(())=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"epic_escalation_disposition_recorded\",\"accepted\":false}")]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(SprintRunnerTransitionError::Conflict)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"conflict\"}")]),Err(SprintRunnerTransitionError::Invalid)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"invalid\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"unavailable\"}")] )}} }
#[tool_handler(router=self.tool_router)] impl ServerHandler for EpicRunnerEscalationReassessmentMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Read the bounded Epic reassessment context, then record one concern-preserving disposition. A request is not delivery, activation, or continuation. No Sprint selection/start, settlement, completion, or acceptance action is available.")} }

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

struct SprintHandbackReassessmentMcp { service: Arc<SprintRunnerTransitionService>, invocation_id: AgentInvocationId, tool_router: ToolRouter<Self> }
impl SprintHandbackReassessmentMcp { fn new(service: Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} fn result(result:Result<(),SprintRunnerTransitionError>,status:&str)->CallToolResult{match result{Ok(())=>CallToolResult::success(vec![ContentBlock::text(serde_json::json!({"status":status,"accepted":false}).to_string())]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(SprintRunnerTransitionError::Conflict)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"conflict\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"invalid_or_unavailable\"}")] )}} }
#[tool_router] impl SprintHandbackReassessmentMcp { #[tool(description="Read only the application-bound no-progress concern and aggregate current Sprint work state. Input is ONLY {}.")] fn read_sprint_handback_reassessment_context(&self)->CallToolResult{match self.service.handback_reassessment_context(&self.invocation_id){Ok((_,context))=>CallToolResult::success(vec![ContentBlock::text(context.to_string())]),Err(error)=>Self::result(Err(error),"handback_reassessment_context")}} #[tool(description="Record one identity-free, concern-preserving next movement. Known movementKind values are continue_eligible_work, wait_for_agent_dependency, and local_exhaustion_escalate; other safe bounded kinds remain extensible. This does not settle the concern or activate an Epic receiver.")] fn record_sprint_handback_disposition(&self,Parameters(input):Parameters<SprintHandbackDisposition>)->CallToolResult{Self::result(self.service.record_handback_disposition(&self.invocation_id,input),"handback_disposition_recorded")} }
#[tool_handler(router=self.tool_router)] impl ServerHandler for SprintHandbackReassessmentMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Read only the bounded concern and aggregate Sprint state; record one concern-preserving movement. No Epic activation, final blockage, settlement, or new work creation is available.")} }

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

struct WorkUnitImplementerReportingMcp { service:Arc<SprintRunnerTransitionService>, invocation_id: AgentInvocationId, tool_router: ToolRouter<Self> }
impl WorkUnitImplementerReportingMcp { fn new(service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} fn result(result:Result<(),SprintRunnerTransitionError>,status:&str)->CallToolResult{match result{Ok(())=>CallToolResult::success(vec![ContentBlock::text(serde_json::json!({"status":status,"accepted":false}).to_string())]),Err(SprintRunnerTransitionError::Conflict)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"conflict\"}")]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"invalid_or_unavailable\"}")])}} }
#[tool_router] impl WorkUnitImplementerReportingMcp { #[tool(description="Submit only {summary,validationStatement}; these are claims, not evidence. The application derives all identities.")] fn submit_implementation_outcome(&self,Parameters(input):Parameters<ImplementationOutcomeClaims>)->CallToolResult{Self::result(self.service.submit_implementation_outcome(&self.invocation_id,input),"implementation_outcome_recorded")} #[tool(description="Semantically complete only the already-valid application-bound outcome. Input is ONLY {}.")] fn complete_implementation_outcome(&self)->CallToolResult{Self::result(self.service.complete_implementation_outcome(&self.invocation_id),"implementation_semantic_completion_recorded")} }
#[tool_handler(router=self.tool_router)] impl ServerHandler for WorkUnitImplementerReportingMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Both identity-free, context-bound reporting tools are available and non-accepting. Claims are not evidence; Handler review remains absent.")} }

struct WorkUnitHandlerReviewMcp { service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId,tool_router:ToolRouter<Self> }
impl WorkUnitHandlerReviewMcp { fn new(service:Arc<SprintRunnerTransitionService>,invocation_id:AgentInvocationId)->Self{Self{service,invocation_id,tool_router:Self::tool_router()}} fn result(result:Result<(),SprintRunnerTransitionError>,status:&str)->CallToolResult{match result{Ok(())=>CallToolResult::success(vec![ContentBlock::text(serde_json::json!({"status":status,"accepted":false}).to_string())]),Err(SprintRunnerTransitionError::Conflict)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"conflict\"}")]),Err(SprintRunnerTransitionError::Forbidden)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"forbidden\"}")]),Err(_)=>CallToolResult::success(vec![ContentBlock::text("{\"status\":\"rejected\",\"code\":\"invalid_or_unavailable\"}")] )}} }
#[tool_router] impl WorkUnitHandlerReviewMcp { #[tool(description="Read the exact application-delivered claims and bounded evidence for this review invocation. Input is ONLY {}.")] fn read_handler_review_evidence(&self)->CallToolResult{match self.service.handler_review_context(&self.invocation_id,true){Ok(context)=>CallToolResult::success(vec![ContentBlock::text(context.delivered_payload_json)]),Err(error)=>Self::result(Err(error),"review_evidence")}} #[tool(description="Accept the exact application-bound implementation outcome. Input is ONLY {} and acceptance remains pending until this exact review invocation is observed Completed.")] fn accept_implementation_outcome(&self)->CallToolResult{Self::result(self.service.record_handler_review_judgment(&self.invocation_id,"accept",None),"review_acceptance_recorded")} #[tool(description="Return the exact application-bound incomplete outcome with {code,explanation,classification,meaningfulProgress}. This records a disposition only; it never launches a later attempt or contacts an upward receiver.")] fn return_implementation_outcome(&self,Parameters(disposition):Parameters<HandlerReviewIncompleteDisposition>)->CallToolResult{Self::result(self.service.record_handler_incomplete_disposition(&self.invocation_id,disposition),"incomplete_disposition_recorded")} }
#[tool_handler(router=self.tool_router)] impl ServerHandler for WorkUnitHandlerReviewMcp { fn get_info(&self)->ServerInfo{ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("Only bounded application-delivered evidence and one identity-free accept or structured return judgment are available. Neither action settles work, creates a retry, or activates dependents.")} }

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
start_scoped_server!(start_handback_reassessment_server,SprintHandbackReassessmentMcp);
start_scoped_server!(start_epic_escalation_reassessment_server,EpicRunnerEscalationReassessmentMcp);
start_scoped_server!(start_work_slice_planner_server,WorkSlicePlannerMcp);
start_scoped_server!(start_work_unit_handler_server,WorkUnitHandlerMcp);
start_scoped_server!(start_work_unit_implementer_reporting_server,WorkUnitImplementerReportingMcp);
start_scoped_server!(start_work_unit_handler_review_server,WorkUnitHandlerReviewMcp);
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
    use super::{ensure_handler_action_failure_reason, migrate_legacy_implementer_activations};
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

    #[test]
    fn handler_action_failure_reason_migrates_idempotently() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "CREATE TABLE work_unit_handler_action_continuations (
               work_unit_id TEXT PRIMARY KEY, blocked_reason TEXT
             );",
        ).unwrap();
        ensure_handler_action_failure_reason(&connection).unwrap();
        ensure_handler_action_failure_reason(&connection).unwrap();
        assert_eq!(connection.query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM pragma_table_info('work_unit_handler_action_continuations')
             WHERE name='failure_reason'", [], |row| row.get(0),
        ).unwrap(), 1);
    }
}

#[cfg(test)]
mod handler_review_payload_tests {
    use super::handler_review_payload;
    use rusqlite::Connection;

    #[test]
    fn review_payload_revalidates_structured_evidence_across_json_key_ordering() {
        let expected = handler_review_payload(
            "completed implementation",
            "focused test passed",
            r#"[{"displayName":"README.md","evidenceRef":"file:README.md"}]"#,
            "comparison-fingerprint",
            r#"{"file:README.md":"content-fingerprint"}"#,
        ).unwrap();
        let canonical = expected.to_string();
        let connection = Connection::open_in_memory().unwrap();
        let sqlite_payload: String = connection.query_row(
            "SELECT json_object('summary','completed implementation','validationStatement','focused test passed','changedFiles',json('[{\"displayName\":\"README.md\",\"evidenceRef\":\"file:README.md\"}]'),'comparisonFingerprint','comparison-fingerprint','evidenceContentFingerprints',json('{\"file:README.md\":\"content-fingerprint\"}'))",
            [],
            |row| row.get(0),
        ).unwrap();

        assert_ne!(sqlite_payload, canonical);
        assert_eq!(serde_json::from_str::<serde_json::Value>(&sqlite_payload).unwrap(), expected);
    }
}

#[cfg(test)]
mod handback_disposition_tests {
    use super::{validate_epic_escalation_disposition, validate_handback_disposition, AgentAchievableDependencyOwner, EpicEscalationAttention, EpicEscalationDownstreamRequest, EpicEscalationDownstreamTarget, EpicEscalationReassessmentDisposition, SprintHandbackDisposition, SprintRunnerTransitionError};

    fn disposition(kind: &str) -> SprintHandbackDisposition {
        SprintHandbackDisposition { movement_kind: kind.into(), rationale: "bounded current concern".into(), eligible_work_summary: None, dependency_owner: None, dependency_owner_classification: None, enabling_result: None, resumption_path: None, local_exhaustion_summary: None }
    }

    #[test]
    fn known_handback_movements_preserve_a_bounded_non_human_route() {
        let mut alternate = disposition("continue_eligible_work");
        alternate.eligible_work_summary = Some("another eligible Work Unit remains authorized".into());
        assert!(validate_handback_disposition(&alternate).is_ok());

        let mut dependency = disposition("wait_for_agent_dependency");
        dependency.dependency_owner = Some("bounded Work Unit Handler".into());
        dependency.dependency_owner_classification = Some(AgentAchievableDependencyOwner::WorkUnitHandler);
        dependency.enabling_result = Some("a persisted handler result".into());
        dependency.resumption_path = Some("reconcile this exact Handback after that result".into());
        assert!(validate_handback_disposition(&dependency).is_ok());

        let mut exhaustion = disposition("local_exhaustion_escalate");
        exhaustion.local_exhaustion_summary = Some("all local Sprint-runner movements are exhausted".into());
        assert!(validate_handback_disposition(&exhaustion).is_ok());
    }

    #[test]
    fn dependency_wait_rejects_human_external_and_unclassified_gates() {
        for owner in ["human approval", "external vendor", "manual confirmation"] {
            let mut value = disposition("wait_for_agent_dependency");
            value.dependency_owner = Some(owner.into());
            value.dependency_owner_classification = Some(AgentAchievableDependencyOwner::WorkUnitHandler);
            value.enabling_result = Some("a result".into());
            value.resumption_path = Some("resume here".into());
            assert!(matches!(validate_handback_disposition(&value), Err(SprintRunnerTransitionError::Invalid)));
        }
    }

    #[test]
    fn epic_reassessment_keeps_requests_and_attention_on_separate_safe_routes() {
        let request = EpicEscalationReassessmentDisposition { movement_kind: "return_context_to_sprint_runner".into(), rationale: "the unresolved concern needs a bounded Sprint decision".into(), considered_intent: None, downstream_request: Some(EpicEscalationDownstreamRequest { target: EpicEscalationDownstreamTarget::SprintRunner, dependency: None, request: "return missing context only".into(), resumption_path: "reassess this concern".into() }), human_external_attention: None };
        assert!(validate_epic_escalation_disposition(&request).is_ok());
        let attention = EpicEscalationReassessmentDisposition { movement_kind: "human_or_external_attention".into(), rationale: "the unresolved concern needs outside authority".into(), considered_intent: None, downstream_request: None, human_external_attention: Some(EpicEscalationAttention { reason: "a policy decision is absent".into(), authority_needed: "designated product authority".into(), evidence_context: "the exact bounded concern and local exhaustion rationale".into(), resumption_path: "resume this exact reassessment after that decision".into() }) };
        assert!(validate_epic_escalation_disposition(&attention).is_ok());
        let mut unsafe_request = request;
        unsafe_request.movement_kind = "human_or_external_attention".into();
        assert!(matches!(validate_epic_escalation_disposition(&unsafe_request), Err(SprintRunnerTransitionError::Invalid)));
    }
}
