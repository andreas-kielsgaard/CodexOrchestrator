//! Filesystem ownership for retained empty Session workspaces.
use crate::agent_sessions::domain::AgentSessionId;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct SessionWorkspaces {
    root: PathBuf,
    owner: String,
}

impl SessionWorkspaces {
    pub(crate) fn skills_root(&self) -> String {
        self.root.join("skills").to_string_lossy().into_owned()
    }
    pub(crate) fn system(owner: String) -> Result<Self, String> {
        Self::new(
            crate::product_home::OrchestrationHomePaths::system()?.root,
            owner,
        )
    }

    pub(crate) fn new(root: PathBuf, owner: String) -> Result<Self, String> {
        if !root.is_absolute() {
            return Err("Orchestration home must be absolute".into());
        }
        for child in ["skills", "workspaces", "workspace-allocations"] {
            std::fs::create_dir_all(root.join(child))
                .map_err(|e| format!("Unable to create orchestration home: {e}"))?;
        }
        Ok(Self { root, owner })
    }

    pub(crate) fn prepare(
        &self,
        id: &AgentSessionId,
        explicit: Option<String>,
    ) -> Result<String, String> {
        if let Some(path) = explicit.filter(|path| !path.trim().is_empty()) {
            let target = Path::new(path.trim());
            if !target.is_absolute() || !target.is_dir() {
                return Err(
                    "Session working directory must be an existing absolute directory".into(),
                );
            }
            return Ok(target.to_string_lossy().into_owned());
        }
        if !id
            .as_str()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err("Session identity cannot be used as a workspace directory name".into());
        }
        let path = self.root.join("workspaces").join(id.as_str());
        let marker = self
            .root
            .join("workspace-allocations")
            .join(format!("{}.json", id.as_str()));
        let expected = serde_json::json!({"sessionId":id.as_str(),"owner":self.owner});
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
        {
            Ok(mut file) => {
                if path.exists() {
                    let _ = std::fs::remove_file(&marker);
                    return Err(
                        "Workspace path already exists without this session's allocation".into(),
                    );
                }
                use std::io::Write;
                file.write_all(expected.to_string().as_bytes())
                    .and_then(|()| file.sync_all())
                    .map_err(|e| format!("Unable to retain workspace ownership: {e}"))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let actual: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&marker).map_err(|e| e.to_string())?)
                        .map_err(|e| format!("Invalid workspace allocation: {e}"))?;
                if actual != expected {
                    return Err(
                        "Workspace allocation belongs to another application database".into(),
                    );
                }
            }
            Err(error) => return Err(format!("Unable to reserve workspace: {error}")),
        }
        if path.exists() {
            let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err("Workspace path is a reparse point".into());
                }
            }
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err("Workspace path is not an owned directory".into());
            }
        } else {
            std::fs::create_dir(&path)
                .map_err(|e| format!("Unable to create session workspace: {e}"))?;
        }
        Ok(path.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_is_empty_retained_and_separate_from_explicit_targets() {
        let temp = tempfile::tempdir().unwrap();
        let workspaces =
            SessionWorkspaces::new(temp.path().join("home"), "database-one".into()).unwrap();
        let id = AgentSessionId::new("session-one").unwrap();
        let path = workspaces.prepare(&id, None).unwrap();
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 0);
        std::fs::write(Path::new(&path).join("retained.txt"), "content").unwrap();
        assert_eq!(workspaces.prepare(&id, Some(" ".into())).unwrap(), path);
        assert!(Path::new(&path).join("retained.txt").is_file());
        assert_eq!(
            workspaces
                .prepare(&id, Some(temp.path().to_string_lossy().into_owned()))
                .unwrap(),
            temp.path().to_string_lossy()
        );
        assert!(workspaces.prepare(&id, Some("relative".into())).is_err());
        assert!(workspaces
            .prepare(&AgentSessionId::new("../outside").unwrap(), None)
            .is_err());
    }

    #[test]
    fn allocation_reopens_but_never_adopts_another_database_or_existing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("home");
        let id = AgentSessionId::new("retained-session").unwrap();
        let original = SessionWorkspaces::new(root.clone(), "database-one".into())
            .unwrap()
            .prepare(&id, None)
            .unwrap();
        let reopened = SessionWorkspaces::new(root.clone(), "database-one".into()).unwrap();
        assert_eq!(reopened.prepare(&id, None).unwrap(), original);
        assert!(SessionWorkspaces::new(root.clone(), "database-two".into())
            .unwrap()
            .prepare(&id, None)
            .is_err());
        let occupied = AgentSessionId::new("occupied").unwrap();
        std::fs::create_dir(root.join("workspaces/occupied")).unwrap();
        assert!(reopened.prepare(&occupied, None).is_err());
        assert!(!root.join("workspace-allocations/occupied.json").exists());
        assert!(Path::new(&original).is_dir());
    }
}
