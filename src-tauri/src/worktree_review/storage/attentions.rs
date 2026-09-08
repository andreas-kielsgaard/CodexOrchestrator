use super::{
    decode_optional_time, decode_time, encode_time, sql_error, ConnectionProvider, StorageError,
    StorageResult,
};
use crate::worktree_review::domain::{
    BuildAttention, BuildAttentionCategory, BuildAttentionId, ReviewBuildId,
};
use rusqlite::params;

pub(crate) trait BuildAttentionRepository {
    fn append(&self, attention: &BuildAttention) -> StorageResult<()>;
    fn find_active_for_build(&self, build_id: &ReviewBuildId)
        -> StorageResult<Vec<BuildAttention>>;
}

pub(crate) struct SqliteBuildAttentionRepository<'owner, Owner> {
    owner: &'owner Owner,
}

impl<'owner, Owner> SqliteBuildAttentionRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<Owner: ConnectionProvider> BuildAttentionRepository
    for SqliteBuildAttentionRepository<'_, Owner>
{
    fn append(&self, attention: &BuildAttention) -> StorageResult<()> {
        attention
            .validate()
            .map_err(|error| StorageError::corrupt(error.to_string()))?;
        self.owner.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO review_build_attentions(
                       attention_id, build_id, category, summary, recorded_at, resolved_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        attention.id.as_str(),
                        attention.build_id.as_str(),
                        attention.category.as_str(),
                        attention.summary,
                        encode_time(attention.recorded_at),
                        attention.resolved_at.map(encode_time),
                    ],
                )
                .map_err(sql_error("append build attention"))?;
            Ok(())
        })
    }

    fn find_active_for_build(
        &self,
        build_id: &ReviewBuildId,
    ) -> StorageResult<Vec<BuildAttention>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT attention_id, build_id, category, summary, recorded_at, resolved_at
                     FROM review_build_attentions
                     WHERE build_id = ?1 AND resolved_at IS NULL
                     ORDER BY recorded_at DESC, attention_id DESC",
                )
                .map_err(sql_error("prepare active build attention query"))?;
            let rows = statement
                .query_map([build_id.as_str()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                })
                .map_err(sql_error("query active build attentions"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read active build attentions"))?;
            rows.into_iter().map(decode_attention).collect()
        })
    }
}

fn decode_attention(
    (id, build_id, category, summary, recorded_at, resolved_at): (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
    ),
) -> StorageResult<BuildAttention> {
    let attention = BuildAttention {
        id: BuildAttentionId::new(id).map_err(|error| StorageError::corrupt(error.to_string()))?,
        build_id: ReviewBuildId::new(build_id)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        category: BuildAttentionCategory::parse(&category).ok_or_else(|| {
            StorageError::corrupt(format!("unknown build attention category: {category}"))
        })?,
        summary,
        recorded_at: decode_time(recorded_at, "build attention recorded")?,
        resolved_at: decode_optional_time(resolved_at, "build attention resolved")?,
    };
    attention
        .validate()
        .map_err(|error| StorageError::corrupt(error.to_string()))?;
    Ok(attention)
}
