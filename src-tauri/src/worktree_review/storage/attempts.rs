use super::{
    decode_optional_time, decode_time, encode_time, sql_error, ConnectionProvider, StorageError,
    StorageErrorKind, StorageResult,
};
use crate::worktree_review::domain::{
    OperationAttemptId, OperationExecutionState, OperationFailure, OperationFailureCategory,
    OperationStage, OperationVerdict, ReviewBuildId, ReviewOperationAttempt, ReviewOperationKind,
};
use rusqlite::{params, OptionalExtension};

pub(crate) trait OperationAttemptRepository {
    fn save(&self, attempt: &ReviewOperationAttempt) -> StorageResult<()>;
    fn find(&self, id: &OperationAttemptId) -> StorageResult<Option<ReviewOperationAttempt>>;
    fn list_for_build(
        &self,
        build_id: &ReviewBuildId,
    ) -> StorageResult<Vec<ReviewOperationAttempt>>;
    fn list_unsettled(&self) -> StorageResult<Vec<ReviewOperationAttempt>>;
}

pub(crate) struct SqliteOperationAttemptRepository<'owner, Owner> {
    owner: &'owner Owner,
}

impl<'owner, Owner> SqliteOperationAttemptRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<Owner: ConnectionProvider> OperationAttemptRepository
    for SqliteOperationAttemptRepository<'_, Owner>
{
    fn save(&self, attempt: &ReviewOperationAttempt) -> StorageResult<()> {
        attempt
            .validate()
            .map_err(|error| StorageError::corrupt(error.to_string()))?;
        self.owner.with_connection(|connection| {
            let changed = connection
                .execute(
                    "INSERT INTO review_operation_attempts(
                       attempt_id, build_id, operation_kind, execution_state, verdict, active_stage,
                       failure_stage, failure_category, failure_message, requested_at, started_at,
                       completed_at
                     )
                     SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12
                     WHERE EXISTS (
                       SELECT 1 FROM review_builds
                       WHERE build_id = ?2 AND data_contract_version = 2
                     )
                     ON CONFLICT(attempt_id) DO UPDATE SET
                       execution_state = excluded.execution_state,
                       verdict = excluded.verdict,
                       active_stage = excluded.active_stage,
                       failure_stage = excluded.failure_stage,
                       failure_category = excluded.failure_category,
                       failure_message = excluded.failure_message,
                       started_at = excluded.started_at,
                       completed_at = excluded.completed_at
                     WHERE build_id = excluded.build_id
                       AND operation_kind = excluded.operation_kind
                       AND requested_at = excluded.requested_at",
                    params![
                        attempt.id.as_str(),
                        attempt.build_id.as_str(),
                        attempt.kind.as_str(),
                        attempt.execution.as_str(),
                        attempt.verdict.as_str(),
                        attempt.active_stage.map(OperationStage::as_str),
                        attempt
                            .failure
                            .as_ref()
                            .map(|failure| failure.stage.as_str()),
                        attempt
                            .failure
                            .as_ref()
                            .map(|failure| failure.category.as_str()),
                        attempt
                            .failure
                            .as_ref()
                            .map(|failure| failure.message.as_str()),
                        encode_time(attempt.requested_at),
                        attempt.started_at.map(encode_time),
                        attempt.completed_at.map(encode_time),
                    ],
                )
                .map_err(sql_error("save review operation attempt"))?;
            if changed == 0 {
                return Err(StorageError::new(
                    StorageErrorKind::Conflict,
                    "operation attempt identity conflicts with its durable request",
                ));
            }
            Ok(())
        })
    }

    fn find(&self, id: &OperationAttemptId) -> StorageResult<Option<ReviewOperationAttempt>> {
        self.owner.with_connection(|connection| {
            let raw = connection
                .query_row(
                    &format!(
                        "{SELECT_ATTEMPT} WHERE attempt_id = ?1
                         AND build_id IN (
                           SELECT build_id FROM review_builds WHERE data_contract_version = 2
                         )"
                    ),
                    [id.as_str()],
                    decode_row,
                )
                .optional()
                .map_err(sql_error("load review operation attempt"))?;
            raw.map(decode_attempt).transpose()
        })
    }

    fn list_for_build(
        &self,
        build_id: &ReviewBuildId,
    ) -> StorageResult<Vec<ReviewOperationAttempt>> {
        self.owner.with_connection(|connection| {
            query_many(
                connection,
                &format!(
                    "{SELECT_ATTEMPT} WHERE build_id = ?1
                     AND build_id IN (
                       SELECT build_id FROM review_builds WHERE data_contract_version = 2
                     )
                     ORDER BY requested_at DESC, attempt_id"
                ),
                [build_id.as_str()],
            )
        })
    }

    fn list_unsettled(&self) -> StorageResult<Vec<ReviewOperationAttempt>> {
        self.owner.with_connection(|connection| {
            query_many(
                connection,
                &format!(
                    "{SELECT_ATTEMPT} WHERE execution_state IN ('pending', 'running')
                     AND build_id IN (
                       SELECT build_id FROM review_builds WHERE data_contract_version = 2
                     )
                     ORDER BY requested_at, attempt_id"
                ),
                [],
            )
        })
    }
}

const SELECT_ATTEMPT: &str =
    "SELECT attempt_id, build_id, operation_kind, execution_state, verdict, active_stage,
            failure_stage, failure_category, failure_message, requested_at, started_at, completed_at
     FROM review_operation_attempts";

type AttemptRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AttemptRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
    ))
}

fn query_many<P: rusqlite::Params>(
    connection: &rusqlite::Connection,
    sql: &str,
    parameters: P,
) -> StorageResult<Vec<ReviewOperationAttempt>> {
    let mut statement = connection
        .prepare(sql)
        .map_err(sql_error("prepare operation attempt query"))?;
    let rows = statement
        .query_map(parameters, decode_row)
        .map_err(sql_error("query operation attempts"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read operation attempts"))?;
    rows.into_iter().map(decode_attempt).collect()
}

fn decode_attempt(raw: AttemptRow) -> StorageResult<ReviewOperationAttempt> {
    let failure = match (raw.6, raw.7, raw.8) {
        (None, None, None) => None,
        (Some(stage), Some(category), Some(message)) => Some(OperationFailure {
            stage: OperationStage::parse(&stage)
                .ok_or_else(|| StorageError::corrupt("unknown operation failure stage"))?,
            category: OperationFailureCategory::parse(&category)
                .ok_or_else(|| StorageError::corrupt("unknown operation failure category"))?,
            message,
        }),
        _ => {
            return Err(StorageError::corrupt(
                "operation failure evidence is only partially recorded",
            ))
        }
    };
    let attempt = ReviewOperationAttempt {
        id: OperationAttemptId::new(raw.0)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        build_id: ReviewBuildId::new(raw.1)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        kind: ReviewOperationKind::parse(&raw.2)
            .ok_or_else(|| StorageError::corrupt("unknown review operation kind"))?,
        execution: OperationExecutionState::parse(&raw.3)
            .ok_or_else(|| StorageError::corrupt("unknown operation execution state"))?,
        verdict: OperationVerdict::parse(&raw.4)
            .ok_or_else(|| StorageError::corrupt("unknown operation verdict"))?,
        active_stage: raw
            .5
            .map(|stage| {
                OperationStage::parse(&stage)
                    .ok_or_else(|| StorageError::corrupt("unknown active operation stage"))
            })
            .transpose()?,
        failure,
        requested_at: decode_time(raw.9, "operation request")?,
        started_at: decode_optional_time(raw.10, "operation start")?,
        completed_at: decode_optional_time(raw.11, "operation completion")?,
    };
    attempt
        .validate()
        .map_err(|error| StorageError::corrupt(error.to_string()))?;
    Ok(attempt)
}
