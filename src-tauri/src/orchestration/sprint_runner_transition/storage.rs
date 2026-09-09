//! Existing Sprint storage initialization, shared by production composition and test openers.
use super::*;

pub(crate) fn initialize(connection: &Connection) -> Result<(), String> {
    initialize_schema(connection).map_err(|error| error.to_string())
}

fn initialize_schema(connection: &Connection) -> Result<(), SprintRunnerTransitionError> {
    connection.execute_batch(SCHEMA).map_err(|e| {
        SprintRunnerTransitionError::Unavailable(format!(
            "initialize Sprint Runner transition schema: {e}"
        ))
    })?;
    connection
        .execute_batch(ACCEPTED_CANDIDATE_AUTHORITY_SCHEMA)
        .map_err(|e| {
            SprintRunnerTransitionError::Unavailable(format!(
                "initialize accepted candidate authority schema: {e}"
            ))
        })?;
    connection
        .execute_batch(
            crate::orchestration::work_unit_dependency_wave::WORK_UNIT_DEPENDENCY_WAVE_SCHEMA,
        )
        .map_err(|e| {
            SprintRunnerTransitionError::Unavailable(format!(
                "initialize dependency-wave schema: {e}"
            ))
        })?;
    sprint_continuation_settlement::initialize(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    crate::orchestration::epic_settlement::initialize(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    // The first pre-start route was shipped before these evidence boundaries.  Keep an
    // existing local database readable without treating an absent fact as a positive fact.
    for column in [
        "pre_start_semantic_outcome_recorded_at TEXT",
        "pre_start_outcome_fact_id TEXT",
        "pre_start_outcome_invocation_id TEXT",
        "pre_start_forecast TEXT",
        "pre_start_material_uncertainty TEXT",
        "pre_start_prerequisite TEXT",
        "pre_start_upgrade_invocation_id TEXT",
        "pre_start_upgrade_harness_key TEXT",
        "pre_start_upgrade_harness_version INTEGER",
        "pre_start_upgrade_harness_applied_at TEXT",
        "pre_start_upgrade_launch_accepted_at TEXT",
        "pre_start_lifecycle_status TEXT",
        "pre_start_lifecycle_invocation_id TEXT",
        "pre_start_lifecycle_observed_at TEXT",
        "pre_start_outcome_accepted_at TEXT",
        "parent_continuation_delivery_requested_at TEXT",
        "parent_continuation_delivery_persisted_at TEXT",
        "epic_continuation_invocation_id TEXT",
        "parent_continuation_delivery_fact_id TEXT",
        "parent_continuation_delivered_outcome_fact_id TEXT",
        "epic_continuation_harness_key TEXT",
        "epic_continuation_harness_version INTEGER",
        "epic_continuation_harness_applied_at TEXT",
        "epic_continuation_launch_accepted_at TEXT",
        "provider_receiver_activation_observed_at TEXT",
        "epic_start_semantic_authorization_requested_at TEXT",
        "epic_start_semantic_authorization_recorded_at TEXT",
        "sprint_start_authorized_at TEXT",
        "sprint_start_persisted_at TEXT",
        "sprint_continuation_invocation_id TEXT",
        "sprint_continuation_harness_key TEXT",
        "sprint_continuation_harness_version INTEGER",
        "sprint_continuation_harness_applied_at TEXT",
        "sprint_continuation_launch_accepted_at TEXT",
        "repository_branch_reevaluation_fact_id TEXT",
        "repository_branch_reevaluation_recorded_at TEXT",
        "repository_branch_evaluation TEXT",
        "started_forecast_and_concerns TEXT",
        "started_reevaluation_lifecycle_status TEXT",
        "started_reevaluation_lifecycle_invocation_id TEXT",
        "started_reevaluation_lifecycle_observed_at TEXT",
        "planning_control_delivery_requested_at TEXT",
        "planning_control_delivery_persisted_at TEXT",
        "planning_control_invocation_id TEXT",
        "planning_control_harness_key TEXT",
        "planning_control_harness_version INTEGER",
        "planning_control_harness_applied_at TEXT",
        "planning_control_launch_accepted_at TEXT",
        "planning_ready_at TEXT",
    ] {
        let name = column
            .split_whitespace()
            .next()
            .expect("migration column name");
        let exists = connection
            .prepare("PRAGMA table_info(sprint_runner_transitions)")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| {
                SprintRunnerTransitionError::Unavailable(format!(
                    "inspect Sprint Runner transition schema: {e}"
                ))
            })?
            .iter()
            .any(|existing| existing == name);
        if !exists {
            connection
                .execute_batch(&format!(
                    "ALTER TABLE sprint_runner_transitions ADD COLUMN {column}"
                ))
                .map_err(|e| {
                    SprintRunnerTransitionError::Unavailable(format!(
                        "migrate Sprint Runner transition schema: {e}"
                    ))
                })?;
        }
    }
    for column in [
        "authority_id TEXT",
        "authority_epic_id TEXT",
        "authority_provenance_id TEXT",
        "authority_repository_id TEXT",
        "authority_worktree_id TEXT",
        "authority_baseline_object_id TEXT",
        "authority_current_object_id TEXT",
        "authority_source_fingerprint TEXT",
        "planner_session_created_at TEXT",
        "planner_invocation_created_at TEXT",
        "planner_harness_applied_at TEXT",
        "planner_harness_json TEXT",
        "planner_launch_requested_at TEXT",
        "planner_launch_accepted_at TEXT",
        "planner_ready_at TEXT",
        "planner_provider_activation_observed_at TEXT",
        "planner_lifecycle_observed_at TEXT",
    ] {
        let name = column
            .split_whitespace()
            .next()
            .expect("planning request migration column name");
        let exists = connection
            .prepare("PRAGMA table_info(work_slice_planning_requests)")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| {
                SprintRunnerTransitionError::Unavailable(format!(
                    "inspect Work Slice planning request schema: {e}"
                ))
            })?
            .iter()
            .any(|existing| existing == name);
        if !exists {
            connection
                .execute_batch(&format!(
                    "ALTER TABLE work_slice_planning_requests ADD COLUMN {column}"
                ))
                .map_err(|e| {
                    SprintRunnerTransitionError::Unavailable(format!(
                        "migrate Work Slice planning request schema: {e}"
                    ))
                })?;
        }
    }
    let handler_columns = [
        "handler_harness_revision_id TEXT",
        "handler_harness_configuration_digest TEXT",
        "handler_harness_repository_commit_ref TEXT",
    ];
    for column in handler_columns {
        let name = column
            .split_whitespace()
            .next()
            .expect("Handler activation migration column name");
        let exists = connection
            .prepare("PRAGMA table_info(work_unit_handler_activations)")
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| {
                SprintRunnerTransitionError::Unavailable(format!(
                    "inspect Handler activation schema: {e}"
                ))
            })?
            .iter()
            .any(|existing| existing == name);
        if !exists {
            connection
                .execute_batch(&format!(
                    "ALTER TABLE work_unit_handler_activations ADD COLUMN {column}"
                ))
                .map_err(|e| {
                    SprintRunnerTransitionError::Unavailable(format!(
                        "migrate Handler activation schema: {e}"
                    ))
                })?;
        }
    }
    migrate_legacy_implementer_activations(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    ensure_handler_activation_failure_reason(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    ensure_handler_action_failure_reason(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    ensure_implementer_outcome_evidence_columns(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    migrate_work_unit_attempt_history(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    migrate_work_unit_retry_attempt_history(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    migrate_handler_decision_retry_contract(connection)
        .map_err(SprintRunnerTransitionError::Unavailable)?;
    Ok(())
}
