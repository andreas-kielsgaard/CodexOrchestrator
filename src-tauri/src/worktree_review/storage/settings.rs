use super::{decode_time, encode_time, sql_error, ConnectionProvider, StorageError, StorageResult};
use crate::worktree_review::domain::{RetentionPolicy, ReviewSettings};
use rusqlite::{params, OptionalExtension};

pub(crate) trait ReviewSettingsRepository {
    fn load(&self) -> StorageResult<Option<ReviewSettings>>;
    fn save(&self, settings: &ReviewSettings) -> StorageResult<()>;
}

pub(crate) struct SqliteReviewSettingsRepository<'owner, Owner> {
    owner: &'owner Owner,
}

impl<'owner, Owner> SqliteReviewSettingsRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<Owner: ConnectionProvider> ReviewSettingsRepository
    for SqliteReviewSettingsRepository<'_, Owner>
{
    fn load(&self) -> StorageResult<Option<ReviewSettings>> {
        self.owner.with_connection(|connection| {
            let raw = connection
                .query_row(
                    "SELECT retention_policy_json, updated_at
                     FROM worktree_review_product_settings WHERE singleton = 1",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(sql_error("load worktree review settings"))?;
            let Some(raw) = raw else {
                return Ok(None);
            };
            let settings = ReviewSettings {
                retention_policy: serde_json::from_str::<RetentionPolicy>(&raw.0).map_err(
                    |error| StorageError::corrupt(format!("invalid retention policy: {error}")),
                )?,
                updated_at: decode_time(raw.1, "worktree review settings update")?,
            };
            settings
                .validate()
                .map_err(|error| StorageError::corrupt(error.to_string()))?;
            Ok(Some(settings))
        })
    }

    fn save(&self, settings: &ReviewSettings) -> StorageResult<()> {
        settings
            .validate()
            .map_err(|error| StorageError::corrupt(error.to_string()))?;
        let retention = serde_json::to_string(&settings.retention_policy)
            .map_err(|error| StorageError::corrupt(format!("encode retention policy: {error}")))?;
        self.owner.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO worktree_review_product_settings(
                       singleton, retention_policy_json, updated_at
                     ) VALUES (1, ?1, ?2)
                     ON CONFLICT(singleton) DO UPDATE SET
                       retention_policy_json = excluded.retention_policy_json,
                       updated_at = excluded.updated_at",
                    params![retention, encode_time(settings.updated_at)],
                )
                .map_err(sql_error("save worktree review settings"))?;
            Ok(())
        })
    }
}
