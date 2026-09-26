//! One-time conversion of Codex-era stored configuration into the provider-neutral shape
//! (schema v59). Codex was the only provider before v59, so every stored reference and native
//! option belongs to it. Delete this module once no older database needs to open.
//!
//! Event rows are never touched. A stored profile that cannot be converted is left as it was
//! and reports its error when the Session is next used.
use crate::execution_configuration::{
    SessionCreationResolution, SessionProfile, NATIVE_MCP_GROUP, NATIVE_SKILL_GROUP,
};
use orchid_engine::providers::codex::options::PROVIDER as CODEX;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

const LEGACY_REFERENCE_PREFIX: &str = "native-codex:";
const GROUP_RENAMES: [(&str, &str); 2] = [
    ("codex-profile-mcps", NATIVE_MCP_GROUP),
    ("codex-profile-skills", NATIVE_SKILL_GROUP),
];

pub(crate) fn migrate(connection: &Connection) -> Result<(), String> {
    migrate_capability_profiles(connection)?;
    let mut digests = BTreeMap::new();
    rewrite_json_column(
        connection,
        "agent_sessions",
        "id",
        "session_profile_json",
        |value| upgrade_resolution(value, &mut digests),
    )?;
    rewrite_json_column(
        connection,
        "agent_session_current_execution",
        "session_id",
        "resolution_json",
        |value| upgrade_resolution(value, &mut digests),
    )?;
    rewrite_json_column(
        connection,
        "agent_session_preparations",
        "invocation_id",
        "payload_json",
        |value| upgrade_preparation(value, &mut digests),
    )?;
    Ok(())
}

fn migrate_capability_profiles(connection: &Connection) -> Result<(), String> {
    rewrite_json_column(
        connection,
        "execution_capability_profiles",
        "capability_profile_id",
        "profile_json",
        |profile| {
            let mut changed = false;
            for route in profile
                .get_mut("routePolicies")
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
                .filter_map(Value::as_object_mut)
            {
                changed |= move_personality(route);
                for key in ["mcpGroups", "skillGroups"] {
                    for group in route
                        .get_mut(key)
                        .and_then(Value::as_array_mut)
                        .into_iter()
                        .flatten()
                    {
                        if let Some((_, neutral)) =
                            GROUP_RENAMES.iter().find(|(legacy, _)| group == legacy)
                        {
                            *group = Value::from(*neutral);
                            changed = true;
                        }
                    }
                }
            }
            Ok(changed)
        },
    )
}

/// Converts a stored `SessionCreationResolution` and reseals its digest.
fn upgrade_resolution(
    resolution: &mut Value,
    digests: &mut BTreeMap<String, String>,
) -> Result<bool, String> {
    let Some(profile) = resolution
        .get_mut("sessionProfile")
        .and_then(Value::as_object_mut)
    else {
        return Ok(false);
    };
    let Some(reference) = profile.remove("runtimeProfileRef") else {
        return Ok(false);
    };
    let reference = reference
        .as_str()
        .ok_or("Stored runtime profile reference is not text")?;
    let configuration_id = reference
        .strip_prefix(LEGACY_REFERENCE_PREFIX)
        .unwrap_or(reference);
    profile.insert(
        "configuration".into(),
        json!({"provider": CODEX, "configurationId": configuration_id}),
    );
    move_personality(profile);
    let session_profile: SessionProfile =
        serde_json::from_value(Value::Object(profile.clone())).map_err(|error| error.to_string())?;
    let resealed =
        SessionCreationResolution::reseal(session_profile).map_err(|error| error.to_string())?;
    if let Some(previous) = resolution.get("digest").and_then(Value::as_str) {
        digests.insert(previous.to_owned(), resealed.digest().to_owned());
    }
    *resolution = serde_json::to_value(resealed).map_err(|error| error.to_string())?;
    Ok(true)
}

/// Preparations carry both a pinned resolution and a per-invocation reference to its digest.
fn upgrade_preparation(
    preparation: &mut Value,
    digests: &mut BTreeMap<String, String>,
) -> Result<bool, String> {
    let mut changed = match preparation.get_mut("currentResolution") {
        Some(resolution) if resolution.is_object() => upgrade_resolution(resolution, digests)?,
        _ => false,
    };
    if let Some(digest) = preparation.pointer_mut("/resolution/sessionProfileDigest") {
        if let Some(resealed) = digest.as_str().and_then(|previous| digests.get(previous)) {
            *digest = Value::from(resealed.clone());
            changed = true;
        }
    }
    Ok(changed)
}

fn move_personality(object: &mut Map<String, Value>) -> bool {
    let Some(personality) = object.remove("codexPersonality") else {
        return false;
    };
    if !personality.is_null() {
        object.insert(
            "providerOptions".into(),
            json!({"provider": CODEX, "settings": {"personality": personality}}),
        );
    }
    true
}

fn rewrite_json_column(
    connection: &Connection,
    table: &str,
    key: &str,
    column: &str,
    mut upgrade: impl FnMut(&mut Value) -> Result<bool, String>,
) -> Result<(), String> {
    let exists = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .is_some();
    if !exists {
        return Ok(());
    }
    let rows = {
        let mut statement = connection
            .prepare(&format!(
                "SELECT {key}, {column} FROM {table} WHERE {column} IS NOT NULL"
            ))
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    for (id, json) in rows {
        let Ok(mut value) = serde_json::from_str::<Value>(&json) else {
            continue;
        };
        if !matches!(upgrade(&mut value), Ok(true)) {
            continue;
        }
        connection
            .execute(
                &format!("UPDATE {table} SET {column}=?1 WHERE {key}=?2"),
                params![value.to_string(), id],
            )
            .map_err(|error| format!("Unable to migrate {table} `{id}`: {error}"))?;
    }
    Ok(())
}
