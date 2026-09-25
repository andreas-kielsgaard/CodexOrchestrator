//! Portable native rollout data. Product transcripts and credentials never cross this boundary.
use super::history;
use crate::contracts::{ports::RuntimePortError, provider::ProviderContinuationPayload};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexContinuation {
    pub thread_id: String,
    pub codex_version: String,
    pub rollout: String,
    pub fingerprint: String,
    #[serde(default)]
    pub attachments: Vec<ContinuationAttachment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationAttachment {
    pub digest: String,
    pub bytes: Vec<u8>,
}

pub const PROVIDER: &str = "codex";
pub const FORMAT: &str = "codex_rollout_v1";

pub fn encode(value: CodexContinuation) -> Result<ProviderContinuationPayload, RuntimePortError> {
    Ok(ProviderContinuationPayload {
        provider: PROVIDER.into(),
        format: FORMAT.into(),
        payload: serde_json::to_value(value).map_err(unavailable)?,
    })
}

pub fn decode(value: &ProviderContinuationPayload) -> Result<CodexContinuation, RuntimePortError> {
    if value.provider != PROVIDER || value.format != FORMAT {
        return Err(unavailable(
            "Continuation payload is not a supported Codex rollout",
        ));
    }
    serde_json::from_value(value.payload.clone()).map_err(unavailable)
}

fn unavailable(error: impl std::fmt::Display) -> RuntimePortError {
    super::connection::unavailable(error.to_string())
}

fn version(program: &str) -> Result<String, RuntimePortError> {
    let program = crate::providers::codex::resolve_program(program.to_owned()).map_err(unavailable)?;
    let output = Command::new(program)
        .arg("--version")
        .output()
        .map_err(unavailable)?;
    if !output.status.success() {
        return Err(unavailable("Cannot determine Codex continuation version"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
fn fingerprint(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn inspect(rollout: &str, id: &str) -> Result<(), RuntimePortError> {
    let first: Value = serde_json::from_str(
        rollout
            .lines()
            .next()
            .ok_or_else(|| unavailable("Native history is empty"))?,
    )
    .map_err(unavailable)?;
    if first["type"] != "session_meta" || first["payload"]["id"] != id {
        return Err(unavailable(
            "Native history identity does not match the requested conversation",
        ));
    }
    for line in rollout.lines() {
        let _: Value = serde_json::from_str(line).map_err(unavailable)?;
    }
    Ok(())
}
const ATTACHMENT_PREFIX: &str = "orchid-attachment:";
fn attachment_slot(value: &mut Value) -> Option<&mut Value> {
    let kind = value.get("type")?.as_str()?;
    if kind == "local_image" {
        return value.get_mut("path");
    }
    if kind == "input_image"
        && value
            .get("image_url")
            .and_then(Value::as_str)
            .is_some_and(|url| url.starts_with("file://") || url.starts_with(ATTACHMENT_PREFIX))
    {
        return value.get_mut("image_url");
    }
    None
}
fn map_attachments(
    value: &mut Value,
    map: &mut impl FnMut(&str) -> Result<String, RuntimePortError>,
) -> Result<(), RuntimePortError> {
    let image_url = value.get("type").and_then(Value::as_str) == Some("input_image");
    if let Some(slot) = attachment_slot(value) {
        let path = slot
            .as_str()
            .ok_or_else(|| unavailable("Native image attachment path is invalid"))?;
        let mapped = map(path)?;
        *slot = Value::String(
            if image_url && !mapped.starts_with(ATTACHMENT_PREFIX) && !mapped.starts_with("file://")
            {
                format!("file://{}", mapped.replace('\\', "/"))
            } else {
                mapped
            },
        );
        return Ok(());
    }
    match value {
        Value::Object(fields) => {
            for child in fields.values_mut() {
                map_attachments(child, map)?;
            }
        }
        Value::Array(children) => {
            for child in children {
                map_attachments(child, map)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn map_rollout(
    rollout: &str,
    mut map: impl FnMut(&str) -> Result<String, RuntimePortError>,
) -> Result<String, RuntimePortError> {
    let mut result = String::new();
    for line in rollout.lines() {
        let mut value: Value = serde_json::from_str(line).map_err(unavailable)?;
        map_attachments(&mut value, &mut map)?;
        result.push_str(&serde_json::to_string(&value).map_err(unavailable)?);
        result.push('\n');
    }
    Ok(result)
}
fn portable_rollout(
    rollout: &str,
) -> Result<(String, Vec<ContinuationAttachment>), RuntimePortError> {
    let mut attachments = BTreeMap::new();
    let normalized = map_rollout(rollout, |reference| {
        let path = reference.strip_prefix("file://").unwrap_or(reference);
        let bytes = fs::read(path).map_err(|error| {
            unavailable(format!("Cannot read required native image {path}: {error}"))
        })?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        attachments
            .entry(digest.clone())
            .or_insert(ContinuationAttachment {
                digest: digest.clone(),
                bytes,
            });
        Ok(format!("{ATTACHMENT_PREFIX}{digest}"))
    })?;
    Ok((normalized, attachments.into_values().collect()))
}
fn find_rollout(home: &Path, id: &str) -> Result<Option<PathBuf>, RuntimePortError> {
    let mut dirs = vec![home.join("sessions"), home.join("archived_sessions")];
    let mut found = None;
    while let Some(dir) = dirs.pop() {
        if !dir.exists() {
            continue;
        }
        for entry in fs::read_dir(dir).map_err(unavailable)? {
            let entry = entry.map_err(unavailable)?;
            let ty = entry.file_type().map_err(unavailable)?;
            if ty.is_dir() {
                dirs.push(entry.path());
            } else if ty.is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(&format!("{id}.jsonl"))
            {
                if found.is_some() {
                    return Err(unavailable(
                        "Multiple native histories have this conversation identity",
                    ));
                }
                found = Some(entry.path());
            }
        }
    }
    Ok(found)
}
pub fn export(program: &str, home: &Path, id: &str) -> Result<CodexContinuation, RuntimePortError> {
    let native = history::read_thread(program, home.to_owned(), id)?;
    if native["thread"]["turns"]
        .as_array()
        .is_some_and(|turns| turns.iter().any(|turn| turn["status"] == "inProgress"))
    {
        return Err(unavailable(
            "Wait for the native turn to finish before transferring history",
        ));
    }
    let path = find_rollout(home, id)?
        .ok_or_else(|| unavailable("Native persisted conversation history was not found"))?;
    let rollout = fs::read_to_string(path).map_err(unavailable)?;
    inspect(&rollout, id)?;
    let (rollout, attachments) = portable_rollout(&rollout)?;
    Ok(CodexContinuation {
        thread_id: id.into(),
        codex_version: version(program)?,
        fingerprint: fingerprint(&rollout),
        rollout,
        attachments,
    })
}
/// Forks a stored conversation through its last settled turn, so a destination Session instance
/// never writes to the thread that its source Session still owns. Returns the new thread ID.
pub fn fork(program: &str, home: &Path, id: &str, cwd: &str) -> Result<String, RuntimePortError> {
    let native = history::read_thread(program, home.to_owned(), id)?;
    let last = native["thread"]["turns"]
        .as_array()
        .and_then(|turns| {
            turns
                .iter()
                .rev()
                .find(|turn| turn["status"] != "inProgress")
        })
        .and_then(|turn| turn["id"].as_str())
        .ok_or_else(|| unavailable("Native history has no settled turn to continue from"))?
        .to_owned();
    let forked = history::fork_thread(program, home.to_owned(), id, &last, cwd)?;
    forked["thread"]["id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| unavailable("Codex did not return the forked thread ID"))
}
pub fn install(
    program: &str,
    home: &Path,
    continuation: &CodexContinuation,
) -> Result<(), RuntimePortError> {
    if version(program)? != continuation.codex_version {
        return Err(unavailable(format!(
            "Native continuation requires matching Codex versions (source {})",
            continuation.codex_version
        )));
    }
    install_payload(home, continuation)
}
fn install_payload(home: &Path, continuation: &CodexContinuation) -> Result<(), RuntimePortError> {
    inspect(&continuation.rollout, &continuation.thread_id)?;
    if fingerprint(&continuation.rollout) != continuation.fingerprint {
        return Err(unavailable(
            "Native history fingerprint differs from its payload",
        ));
    }
    let path = if let Some(existing) = find_rollout(home, &continuation.thread_id)? {
        let previous = fs::read_to_string(&existing).map_err(unavailable)?;
        let (previous, _) = portable_rollout(&previous)?;
        if !continuation.rollout.starts_with(&previous) {
            return Err(unavailable(
                "Destination native history has diverged; it cannot be replaced",
            ));
        }
        existing
    } else {
        let id = uuid::Uuid::parse_str(&continuation.thread_id).map_err(unavailable)?;
        let metadata: Value = serde_json::from_str(continuation.rollout.lines().next().unwrap())
            .map_err(unavailable)?;
        let timestamp = metadata["payload"]["timestamp"]
            .as_str()
            .or(metadata["timestamp"].as_str())
            .ok_or_else(|| unavailable("Native history has no creation timestamp"))?;
        let time = chrono::DateTime::parse_from_rfc3339(timestamp).map_err(unavailable)?;
        let dir = home
            .join("sessions")
            .join(time.format("%Y/%m/%d").to_string());
        fs::create_dir_all(&dir).map_err(unavailable)?;
        dir.join(format!(
            "rollout-{}-{id}.jsonl",
            time.format("%Y-%m-%dT%H-%M-%S")
        ))
    };
    let mut attachment_paths = BTreeMap::new();
    for attachment in &continuation.attachments {
        if format!("{:x}", Sha256::digest(&attachment.bytes)) != attachment.digest {
            return Err(unavailable("Native attachment payload has changed"));
        }
        let directory = home.join("orchid-continuation-attachments");
        fs::create_dir_all(&directory).map_err(unavailable)?;
        let destination = directory.join(&attachment.digest);
        fs::write(&destination, &attachment.bytes).map_err(unavailable)?;
        attachment_paths.insert(
            attachment.digest.as_str(),
            destination.to_string_lossy().into_owned(),
        );
    }
    let installed = map_rollout(&continuation.rollout, |reference| {
        let digest = reference
            .strip_prefix(ATTACHMENT_PREFIX)
            .ok_or_else(|| unavailable("Native transfer has an unresolved local attachment"))?;
        attachment_paths
            .get(digest)
            .cloned()
            .ok_or_else(|| unavailable("Native transfer is missing a required attachment"))
    })?;
    fs::write(path, installed).map_err(unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn payload(extra: &str) -> CodexContinuation {
        let id = "019a1111-1111-7111-8111-111111111111";
        let rollout = format!("{{\"timestamp\":\"2026-09-14T12:00:00Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"timestamp\":\"2026-09-14T12:00:00Z\"}}}}\n{extra}");
        let (rollout, attachments) = portable_rollout(&rollout).unwrap();
        CodexContinuation {
            thread_id: id.into(),
            codex_version: "test".into(),
            fingerprint: fingerprint(&rollout),
            rollout,
            attachments,
        }
    }
    #[test]
    fn installation_keeps_identity_and_updates_only_a_matching_prefix() {
        let home = tempfile::tempdir().unwrap();
        let first = payload("");
        install_payload(home.path(), &first).unwrap();
        let next = payload("{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n");
        install_payload(home.path(), &next).unwrap();
        assert!(install_payload(home.path(), &first).is_err());
        let path = find_rollout(home.path(), &first.thread_id)
            .unwrap()
            .unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), next.rollout);
    }
    #[test]
    fn mismatched_identity_or_payload_is_rejected() {
        let home = tempfile::tempdir().unwrap();
        let mut bad = payload("");
        bad.thread_id = "different".into();
        assert!(install_payload(home.path(), &bad).is_err());
        let mut bad = payload("");
        bad.rollout.push_str("{}\n");
        assert!(install_payload(home.path(), &bad).is_err());
    }
    #[test]
    fn image_attachments_round_trip_without_rewriting_historical_tool_paths() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let image = source.path().join("image.png");
        fs::write(&image, b"image bytes").unwrap();
        let native = serde_json::json!({"type":"response_item", "payload":{"type":"message","content":[{"type":"local_image","path":image.to_string_lossy()}]}, "historicalToolPath":image.to_string_lossy()});
        let raw = format!("{}{}\n", payload("").rollout, native);
        let (rollout, attachments) = portable_rollout(&raw).unwrap();
        let mut transfer = payload("");
        transfer.fingerprint = fingerprint(&rollout);
        transfer.rollout = rollout;
        transfer.attachments = attachments;
        install_payload(destination.path(), &transfer).unwrap();
        let installed = fs::read_to_string(
            find_rollout(destination.path(), &transfer.thread_id)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let item: Value = serde_json::from_str(installed.lines().last().unwrap()).unwrap();
        assert_eq!(item["historicalToolPath"], image.to_string_lossy().as_ref());
        let image_path = item["payload"]["content"][0]["path"].as_str().unwrap();
        assert!(Path::new(image_path).starts_with(destination.path()));
        assert_eq!(fs::read(image_path).unwrap(), b"image bytes");
        let (returned, attachments) = portable_rollout(&installed).unwrap();
        assert_eq!(returned, transfer.rollout);
        assert_eq!(attachments.len(), 1);
    }
}
