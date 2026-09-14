//! Independent conversation import data. Source timestamps remain optional.
use super::domain::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportedItem {
    pub kind: String,
    pub text: String,
    pub raw: Value,
    pub normalized: Option<NormalizedRuntimeEvent>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportedTurn {
    pub id: String,
    pub status: AgentInvocationStatus,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub items: Vec<ImportedItem>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportedThread {
    pub id: String,
    pub title: String,
    pub cwd: Option<String>,
    pub version: Option<String>,
    pub turns: Vec<ImportedTurn>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportHome {
    pub profile_id: String,
    pub filesystem_identity: String,
    pub path: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportReceipt {
    pub request_id: String,
    pub source_thread_id: String,
    pub last_turn_id: String,
    pub home: ImportHome,
    pub session: AgentSession,
    pub fork: Option<ImportedThread>,
    pub completed: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportPreview {
    pub thread_id: String,
    pub title: String,
    pub source_directory: Option<String>,
    pub allocate_workspace: bool,
    pub turn_count: usize,
    pub last_turn_id: String,
    pub native_home: String,
    pub profile_id: String,
    pub capability_profile: String,
    pub excerpt: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportCommand {
    pub request_id: String,
    pub link: String,
    pub profile_id: String,
    pub last_turn_id: String,
}

pub(crate) fn thread_id_from_link(link: &str) -> Result<String, String> {
    let id = link
        .trim()
        .strip_prefix("codex://threads/")
        .ok_or("Enter a Codex link starting with codex://threads/")?;
    let parsed =
        uuid::Uuid::parse_str(id).map_err(|_| "The Codex link must contain one thread UUID")?;
    if parsed.hyphenated().to_string() != id.to_lowercase() {
        return Err("The Codex link must contain a hyphenated thread UUID".into());
    }
    Ok(parsed.to_string())
}

pub(crate) fn existing_directory(source: Option<&str>) -> Option<String> {
    source
        .filter(|p| {
            let path = std::path::Path::new(p);
            path.is_absolute() && path.is_dir()
        })
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codex_link_is_a_single_thread_identity() {
        assert!(
            thread_id_from_link(" codex://threads/01a0a149-469d-74d3-9924-e7415b61a4e9 ").is_ok()
        );
        for invalid in [
            "https://example.test",
            "codex://threads/../config",
            "codex://threads/01a0a149-469d-74d3-9924-e7415b61a4e9?path=x",
            "codex://threads/",
        ] {
            assert!(thread_id_from_link(invalid).is_err());
        }
    }
    #[test]
    fn missing_source_uses_normal_empty_workspace_allocation() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        let explicit = existing_directory(missing.to_str());
        assert!(explicit.is_none());
        let workspaces = crate::agent_sessions::workspace::SessionWorkspaces::new(
            root.path().join("orchid"),
            "test".into(),
        )
        .unwrap();
        let id = AgentSessionId::new("imported").unwrap();
        let allocated = workspaces.prepare(&id, explicit).unwrap();
        assert_eq!(std::fs::read_dir(&allocated).unwrap().count(), 0);
        assert_eq!(allocated, workspaces.prepare(&id, None).unwrap());
        assert_eq!(
            existing_directory(root.path().to_str()),
            Some(root.path().to_string_lossy().into_owned())
        );
    }
}
