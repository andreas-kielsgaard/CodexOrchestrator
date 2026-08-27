use super::{
    decode_optional_time, decode_time, encode_time, sql_error, ConnectionProvider, StorageError,
    StorageErrorKind, StorageResult,
};
use crate::worktree_review::domain::{
    CleanupDisposition, CleanupEffect, CleanupEligibility, CleanupJob, CleanupJobId,
    CleanupJobState, CleanupReceipt, CleanupResource, CleanupResourceId, CleanupTrigger,
    ReviewBuildId,
};
use rusqlite::{params, OptionalExtension};

pub(crate) trait CleanupRepository {
    fn save_job(&self, job: &CleanupJob) -> StorageResult<()>;
    fn find_job(&self, id: &CleanupJobId) -> StorageResult<Option<CleanupJob>>;
    fn list_for_build(&self, build_id: &ReviewBuildId) -> StorageResult<Vec<CleanupJob>>;
    fn list_unsettled(&self) -> StorageResult<Vec<CleanupJob>>;
    fn effects(&self, job_id: &CleanupJobId) -> StorageResult<Vec<CleanupEffect>>;
    fn record_effect(&self, job_id: &CleanupJobId, effect: &CleanupEffect) -> StorageResult<()>;
    fn finish(&self, receipt: &CleanupReceipt) -> StorageResult<()>;
    fn find_receipt(&self, job_id: &CleanupJobId) -> StorageResult<Option<CleanupReceipt>>;
}

pub(crate) struct SqliteCleanupRepository<'owner, Owner> {
    owner: &'owner Owner,
}

impl<'owner, Owner> SqliteCleanupRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<Owner: ConnectionProvider> CleanupRepository for SqliteCleanupRepository<'_, Owner> {
    fn save_job(&self, job: &CleanupJob) -> StorageResult<()> {
        job.validate()
            .map_err(|error| StorageError::corrupt(error.to_string()))?;
        self.owner.with_connection(|connection| {
            connection
                .execute_batch("SAVEPOINT save_cleanup_job")
                .map_err(sql_error("begin cleanup job savepoint"))?;
            let result = save_job(connection, job);
            match result {
                Ok(()) => connection
                    .execute_batch("RELEASE save_cleanup_job")
                    .map_err(sql_error("commit cleanup job savepoint")),
                Err(error) => {
                    let _ = connection
                        .execute_batch("ROLLBACK TO save_cleanup_job; RELEASE save_cleanup_job");
                    Err(error)
                }
            }
        })
    }

    fn find_job(&self, id: &CleanupJobId) -> StorageResult<Option<CleanupJob>> {
        self.owner
            .with_connection(|connection| load_job(connection, id))
    }

    fn list_for_build(&self, build_id: &ReviewBuildId) -> StorageResult<Vec<CleanupJob>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT cleanup_job_id FROM review_cleanup_jobs
                     WHERE build_id = ?1 AND resource_contract_version = 2
                     ORDER BY created_at DESC, cleanup_job_id DESC",
                )
                .map_err(sql_error("prepare build cleanup query"))?;
            let ids = statement
                .query_map([build_id.as_str()], |row| row.get::<_, String>(0))
                .map_err(sql_error("query build cleanup jobs"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read build cleanup job IDs"))?;
            ids.into_iter()
                .map(|id| {
                    let id = CleanupJobId::new(id)
                        .map_err(|error| StorageError::corrupt(error.to_string()))?;
                    load_job(connection, &id)?.ok_or_else(|| {
                        StorageError::corrupt("cleanup job disappeared during durable query")
                    })
                })
                .collect()
        })
    }

    fn list_unsettled(&self) -> StorageResult<Vec<CleanupJob>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT cleanup_job_id FROM review_cleanup_jobs
                     WHERE resource_contract_version = 2
                       AND state IN ('planned', 'running')
                     ORDER BY created_at, cleanup_job_id",
                )
                .map_err(sql_error("prepare unsettled cleanup query"))?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(sql_error("query unsettled cleanup jobs"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read unsettled cleanup job IDs"))?;
            ids.into_iter()
                .map(|id| {
                    let id = CleanupJobId::new(id)
                        .map_err(|error| StorageError::corrupt(error.to_string()))?;
                    load_job(connection, &id)?.ok_or_else(|| {
                        StorageError::corrupt("cleanup job disappeared during durable query")
                    })
                })
                .collect()
        })
    }

    fn record_effect(&self, job_id: &CleanupJobId, effect: &CleanupEffect) -> StorageResult<()> {
        self.owner
            .with_connection(|connection| save_effect(connection, job_id, effect))
    }

    fn effects(&self, job_id: &CleanupJobId) -> StorageResult<Vec<CleanupEffect>> {
        self.owner
            .with_connection(|connection| load_effects(connection, job_id))
    }

    fn finish(&self, receipt: &CleanupReceipt) -> StorageResult<()> {
        self.owner.with_connection(|connection| {
            connection
                .execute_batch("SAVEPOINT finish_cleanup_job")
                .map_err(sql_error("begin cleanup finish savepoint"))?;
            let result = finish_cleanup(connection, receipt);
            match result {
                Ok(()) => connection
                    .execute_batch("RELEASE finish_cleanup_job")
                    .map_err(sql_error("commit cleanup finish savepoint")),
                Err(error) => {
                    let _ = connection.execute_batch(
                        "ROLLBACK TO finish_cleanup_job; RELEASE finish_cleanup_job",
                    );
                    Err(error)
                }
            }
        })
    }

    fn find_receipt(&self, job_id: &CleanupJobId) -> StorageResult<Option<CleanupReceipt>> {
        self.owner.with_connection(|connection| {
            let raw = connection
                .query_row(
                    "SELECT cleanup_job_id, build_id, completed_at
                     FROM review_cleanup_receipts WHERE cleanup_job_id = ?1",
                    [job_id.as_str()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(sql_error("load cleanup receipt"))?;
            let Some(raw) = raw else {
                return Ok(None);
            };
            let receipt = CleanupReceipt {
                job_id: CleanupJobId::new(raw.0)
                    .map_err(|error| StorageError::corrupt(error.to_string()))?,
                build_id: ReviewBuildId::new(raw.1)
                    .map_err(|error| StorageError::corrupt(error.to_string()))?,
                effects: load_effects(connection, job_id)?,
                completed_at: decode_time(raw.2, "cleanup receipt completion")?,
            };
            let job = load_job(connection, job_id)?
                .ok_or_else(|| StorageError::corrupt("cleanup receipt has no cleanup job"))?;
            receipt
                .validate_against(&job)
                .map_err(|error| StorageError::corrupt(error.to_string()))?;
            Ok(Some(receipt))
        })
    }
}

fn save_job(connection: &rusqlite::Connection, job: &CleanupJob) -> StorageResult<()> {
    let changed = connection
        .execute(
            "INSERT INTO review_cleanup_jobs(
               cleanup_job_id, build_id, resource_contract_version, trigger, eligibility, state,
               created_at, started_at, settled_at
             )
             SELECT ?1, ?2, 2, ?3, ?4, ?5, ?6, ?7, ?8
             WHERE EXISTS (
               SELECT 1 FROM review_builds
               WHERE build_id = ?2 AND data_contract_version = 2
             )
             ON CONFLICT(cleanup_job_id) DO UPDATE SET
               state = excluded.state,
               started_at = excluded.started_at,
               settled_at = excluded.settled_at
             WHERE build_id = excluded.build_id
               AND resource_contract_version = 2
               AND trigger = excluded.trigger
               AND eligibility = excluded.eligibility
               AND created_at = excluded.created_at",
            params![
                job.id.as_str(),
                job.build_id.as_str(),
                job.trigger.as_str(),
                job.eligibility.as_str(),
                job.state.as_str(),
                encode_time(job.created_at),
                job.started_at.map(encode_time),
                job.settled_at.map(encode_time),
            ],
        )
        .map_err(sql_error("save cleanup job"))?;
    if changed == 0 {
        return Err(StorageError::new(
            StorageErrorKind::Conflict,
            "cleanup job identity conflicts with its durable evaluation",
        ));
    }
    for (ordinal, resource) in job.resources.iter().enumerate() {
        let encoded = serde_json::to_string(resource)
            .map_err(|error| StorageError::corrupt(format!("encode cleanup resource: {error}")))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO review_cleanup_resources(
                   cleanup_job_id, resource_id, ordinal, resource_json
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    job.id.as_str(),
                    resource.id().as_str(),
                    ordinal as i64,
                    encoded
                ],
            )
            .map_err(sql_error("save cleanup resource"))?;
        let stored = connection
            .query_row(
                "SELECT ordinal, resource_json FROM review_cleanup_resources
                 WHERE cleanup_job_id = ?1 AND resource_id = ?2",
                params![job.id.as_str(), resource.id().as_str()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(sql_error("verify cleanup resource"))?;
        if stored != (ordinal as i64, encoded) {
            return Err(StorageError::new(
                StorageErrorKind::Conflict,
                "cleanup resource ledger is immutable once planned",
            ));
        }
    }
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM review_cleanup_resources WHERE cleanup_job_id = ?1",
            [job.id.as_str()],
            |row| row.get(0),
        )
        .map_err(sql_error("count cleanup resources"))?;
    if count != job.resources.len() as i64 {
        return Err(StorageError::new(
            StorageErrorKind::Conflict,
            "cleanup resource ledger cannot be widened or narrowed after planning",
        ));
    }
    Ok(())
}

fn save_effect(
    connection: &rusqlite::Connection,
    job_id: &CleanupJobId,
    effect: &CleanupEffect,
) -> StorageResult<()> {
    let changed = connection
        .execute(
            "INSERT INTO review_cleanup_effects(
               cleanup_job_id, resource_id, disposition, detail, recorded_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(cleanup_job_id, resource_id) DO UPDATE SET
               disposition = excluded.disposition,
               detail = excluded.detail,
               recorded_at = excluded.recorded_at
             WHERE review_cleanup_effects.disposition = 'pending'
                OR review_cleanup_effects.disposition = excluded.disposition",
            params![
                job_id.as_str(),
                effect.resource_id.as_str(),
                effect.disposition.as_str(),
                effect.detail,
                encode_time(effect.recorded_at),
            ],
        )
        .map_err(sql_error("record cleanup effect"))?;
    if changed == 0 {
        return Err(StorageError::new(
            StorageErrorKind::Conflict,
            "terminal cleanup effect cannot be reclassified",
        ));
    }
    Ok(())
}

fn finish_cleanup(
    connection: &rusqlite::Connection,
    receipt: &CleanupReceipt,
) -> StorageResult<()> {
    let job = load_job(connection, &receipt.job_id)?
        .ok_or_else(|| StorageError::new(StorageErrorKind::NotFound, "cleanup job not found"))?;
    receipt
        .validate_against(&job)
        .map_err(|error| StorageError::corrupt(error.to_string()))?;
    for effect in &receipt.effects {
        save_effect(connection, &receipt.job_id, effect)?;
    }
    let final_state = if receipt
        .effects
        .iter()
        .any(|effect| effect.disposition == CleanupDisposition::Failed)
    {
        CleanupJobState::AttentionRequired
    } else {
        CleanupJobState::Completed
    };
    connection
        .execute(
            "INSERT INTO review_cleanup_receipts(cleanup_job_id, build_id, completed_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(cleanup_job_id) DO NOTHING",
            params![
                receipt.job_id.as_str(),
                receipt.build_id.as_str(),
                encode_time(receipt.completed_at)
            ],
        )
        .map_err(sql_error("save cleanup receipt"))?;
    let stored: (String, String) = connection
        .query_row(
            "SELECT build_id, completed_at FROM review_cleanup_receipts WHERE cleanup_job_id = ?1",
            [receipt.job_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(sql_error("verify cleanup receipt"))?;
    if stored
        != (
            receipt.build_id.as_str().to_owned(),
            encode_time(receipt.completed_at),
        )
    {
        return Err(StorageError::new(
            StorageErrorKind::Conflict,
            "cleanup receipt is immutable once recorded",
        ));
    }
    connection
        .execute(
            "UPDATE review_cleanup_jobs SET state = ?2, settled_at = ?3
             WHERE cleanup_job_id = ?1 AND resource_contract_version = 2",
            params![
                receipt.job_id.as_str(),
                final_state.as_str(),
                encode_time(receipt.completed_at)
            ],
        )
        .map_err(sql_error("settle cleanup job"))?;
    Ok(())
}

type JobRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);

fn load_job(
    connection: &rusqlite::Connection,
    id: &CleanupJobId,
) -> StorageResult<Option<CleanupJob>> {
    let raw: Option<JobRow> = connection
        .query_row(
            "SELECT cleanup_job_id, build_id, trigger, eligibility, state, created_at, started_at,
                    settled_at
             FROM review_cleanup_jobs
             WHERE cleanup_job_id = ?1 AND resource_contract_version = 2",
            [id.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()
        .map_err(sql_error("load cleanup job"))?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let resources = load_resources(connection, id)?;
    let job = CleanupJob {
        id: CleanupJobId::new(raw.0).map_err(|error| StorageError::corrupt(error.to_string()))?,
        build_id: ReviewBuildId::new(raw.1)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        trigger: CleanupTrigger::parse(&raw.2)
            .ok_or_else(|| StorageError::corrupt("unknown cleanup trigger"))?,
        eligibility: CleanupEligibility::parse(&raw.3)
            .ok_or_else(|| StorageError::corrupt("unknown cleanup eligibility"))?,
        state: CleanupJobState::parse(&raw.4)
            .ok_or_else(|| StorageError::corrupt("unknown cleanup job state"))?,
        resources,
        created_at: decode_time(raw.5, "cleanup creation")?,
        started_at: decode_optional_time(raw.6, "cleanup start")?,
        settled_at: decode_optional_time(raw.7, "cleanup settlement")?,
    };
    job.validate()
        .map_err(|error| StorageError::corrupt(error.to_string()))?;
    Ok(Some(job))
}

fn load_resources(
    connection: &rusqlite::Connection,
    id: &CleanupJobId,
) -> StorageResult<Vec<CleanupResource>> {
    let mut statement = connection
        .prepare(
            "SELECT resource_json FROM review_cleanup_resources
             WHERE cleanup_job_id = ?1 ORDER BY ordinal",
        )
        .map_err(sql_error("prepare cleanup resource query"))?;
    let encoded = statement
        .query_map([id.as_str()], |row| row.get::<_, String>(0))
        .map_err(sql_error("query cleanup resources"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read cleanup resources"))?;
    encoded
        .into_iter()
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                StorageError::corrupt(format!("invalid cleanup resource: {error}"))
            })
        })
        .collect()
}

fn load_effects(
    connection: &rusqlite::Connection,
    id: &CleanupJobId,
) -> StorageResult<Vec<CleanupEffect>> {
    let mut statement = connection
        .prepare(
            "SELECT resource_id, disposition, detail, recorded_at FROM review_cleanup_effects
             WHERE cleanup_job_id = ?1 ORDER BY resource_id",
        )
        .map_err(sql_error("prepare cleanup effect query"))?;
    let rows = statement
        .query_map([id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(sql_error("query cleanup effects"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read cleanup effects"))?;
    rows.into_iter()
        .map(|raw| {
            Ok(CleanupEffect {
                resource_id: CleanupResourceId::new(raw.0)
                    .map_err(|error| StorageError::corrupt(error.to_string()))?,
                disposition: CleanupDisposition::parse(&raw.1)
                    .ok_or_else(|| StorageError::corrupt("unknown cleanup disposition"))?,
                detail: raw.2,
                recorded_at: decode_time(raw.3, "cleanup effect")?,
            })
        })
        .collect()
}
