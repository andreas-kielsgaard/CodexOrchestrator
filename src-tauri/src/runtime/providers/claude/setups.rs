//! Claude setups on this device: a Claude configuration folder and the CLI that uses it. The
//! default folder is registered automatically once Claude has created it. Signing in stays with
//! Claude itself (`claude auth login`).
use crate::execution_configuration::{ProviderSetup, ProviderSetupState};
use crate::persistence::ActiveDatabase;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tauri::State;

pub(crate) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS claude_setups (
    id TEXT PRIMARY KEY,
    folder TEXT NOT NULL UNIQUE,
    executable TEXT NOT NULL,
    created_at TEXT NOT NULL
);";

const DEFAULT_EXECUTABLE: &str = "claude";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeSetup {
    pub(crate) id: String,
    pub(crate) folder: String,
    pub(crate) executable: String,
}

pub(crate) struct ClaudeSetups {
    database: Arc<ActiveDatabase>,
    /// The folder Claude uses when nothing names one.
    default_folder: Option<PathBuf>,
}

impl ClaudeSetups {
    pub(crate) fn system(database: Arc<ActiveDatabase>) -> Self {
        let default_folder = std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .map(|home| PathBuf::from(home).join(".claude"))
            });
        Self::new(database, default_folder)
    }

    pub(crate) fn new(database: Arc<ActiveDatabase>, default_folder: Option<PathBuf>) -> Self {
        Self {
            database,
            default_folder,
        }
    }

    /// Registered setups, oldest first. The first is the default.
    pub(crate) fn list(&self) -> Result<Vec<ClaudeSetup>, String> {
        let setups = self.stored()?;
        match &self.default_folder {
            Some(folder) if setups.is_empty() && folder.is_dir() => {
                self.add(folder, None)?;
                self.stored()
            }
            _ => Ok(setups),
        }
    }

    pub(crate) fn add(&self, folder: &Path, executable: Option<String>) -> Result<ClaudeSetup, String> {
        if !folder.is_absolute() {
            return Err("Choose an absolute folder for the Claude setup.".into());
        }
        std::fs::create_dir_all(folder)
            .map_err(|error| format!("Unable to create {}: {error}", folder.display()))?;
        let setup = ClaudeSetup {
            id: format!("claude-{}", uuid::Uuid::new_v4().simple()),
            folder: folder.to_string_lossy().into_owned(),
            executable: executable
                .filter(|executable| !executable.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_EXECUTABLE.into()),
        };
        self.database
            .write("add Claude setup", |tx| {
                tx.execute(
                    "INSERT INTO claude_setups(id,folder,executable,created_at) VALUES(?1,?2,?3,?4)",
                    params![setup.id, setup.folder, setup.executable, chrono::Utc::now().to_rfc3339()],
                )
                .map_err(|error| match error {
                    rusqlite::Error::SqliteFailure(failure, _)
                        if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
                    {
                        "This folder is already a Claude setup.".to_string()
                    }
                    error => error.to_string(),
                })
            })
            .map_err(|error| error.into_string())?;
        Ok(setup)
    }

    pub(crate) fn remove(&self, id: &str) -> Result<(), String> {
        self.database
            .write("remove Claude setup", |tx| {
                tx.execute("DELETE FROM claude_setups WHERE id=?1", [id])
                    .map_err(|error| error.to_string())
            })
            .map_err(|error| error.into_string())?;
        Ok(())
    }

    pub(crate) fn resolve(&self, id: &str) -> Result<ClaudeSetup, String> {
        self.list()?
            .into_iter()
            .find(|setup| setup.id == id)
            .ok_or_else(|| format!("Claude setup '{id}' is not registered on this device."))
    }

    /// Points Claude at the setup's folder. The default folder is left unnamed: naming it would
    /// make Claude read its global settings from inside the folder instead of beside it.
    pub(crate) fn environment(&self, setup: &ClaudeSetup) -> Vec<(String, String)> {
        let is_default = self
            .default_folder
            .as_ref()
            .is_some_and(|folder| Path::new(&setup.folder) == folder);
        if is_default {
            Vec::new()
        } else {
            vec![("CLAUDE_CONFIG_DIR".into(), setup.folder.clone())]
        }
    }

    /// Every setup with its sign-in state, from `claude auth status`.
    pub(crate) fn provider_setups(&self) -> Result<Vec<ProviderSetup>, String> {
        Ok(self
            .list()?
            .into_iter()
            .enumerate()
            .map(|(index, setup)| {
                let (state, detail) = self.readiness(&setup);
                ProviderSetup {
                    device_id: "local".into(),
                    provider: orchid_engine::providers::claude::PROVIDER.into(),
                    configuration_id: setup.id,
                    folder: setup.folder,
                    executable: Some(setup.executable),
                    state,
                    detail,
                    selected: index == 0,
                }
            })
            .collect())
    }

    fn readiness(&self, setup: &ClaudeSetup) -> (ProviderSetupState, Option<String>) {
        let output = Command::new(&setup.executable)
            .args(["auth", "status"])
            .envs(self.environment(setup))
            .output();
        let Ok(output) = output else {
            return (
                ProviderSetupState::Unavailable,
                Some(format!("Claude Code CLI '{}' was not found.", setup.executable)),
            );
        };
        let status: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
        if status["loggedIn"] == true {
            return (ProviderSetupState::Ready, None);
        }
        let command = match self.environment(setup).first() {
            Some((variable, folder)) => format!("{variable}=\"{folder}\" claude auth login"),
            None => "claude auth login".into(),
        };
        (
            ProviderSetupState::NeedsLogin,
            Some(format!("Sign in by running `{command}` in a terminal.")),
        )
    }

    fn stored(&self) -> Result<Vec<ClaudeSetup>, String> {
        self.database
            .read("list Claude setups", |connection: &Connection| {
                let mut statement = connection
                    .prepare("SELECT id,folder,executable FROM claude_setups ORDER BY created_at, id")
                    .map_err(|error| error.to_string())?;
                let rows = statement
                    .query_map([], |row| {
                        Ok(ClaudeSetup {
                            id: row.get(0)?,
                            folder: row.get(1)?,
                            executable: row.get(2)?,
                        })
                    })
                    .map_err(|error| error.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
            })
            .map_err(|error| error.into_string())
    }
}

pub(crate) struct ClaudeSetupTauriState(pub(crate) Arc<ClaudeSetups>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AddClaudeSetupInput {
    folder: String,
    #[serde(default)]
    executable: Option<String>,
}

#[tauri::command]
pub(crate) async fn add_claude_setup(
    state: State<'_, ClaudeSetupTauriState>,
    input: AddClaudeSetupInput,
) -> Result<ClaudeSetup, String> {
    let setups = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || setups.add(Path::new(&input.folder), input.executable))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn remove_claude_setup(
    state: State<'_, ClaudeSetupTauriState>,
    setup_id: String,
) -> Result<(), String> {
    let setups = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || setups.remove(&setup_id))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(test)]
pub(crate) fn test_setups(default_folder: Option<PathBuf>) -> ClaudeSetups {
    let database = ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |connection| {
        connection.execute_batch(SCHEMA).map_err(|error| error.to_string())
    })
    .unwrap();
    ClaudeSetups::new(Arc::new(database), default_folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_folder_registers_itself_once_claude_has_created_it() {
        let root = tempfile::tempdir().unwrap();
        let default = root.path().join(".claude");
        let setups = test_setups(Some(default.clone()));
        assert!(setups.list().unwrap().is_empty());
        std::fs::create_dir(&default).unwrap();
        let listed = setups.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].executable, "claude");
        assert!(setups.environment(&listed[0]).is_empty());
        assert_eq!(setups.list().unwrap(), listed);
    }

    #[test]
    fn another_folder_is_named_to_claude_and_registers_once() {
        let root = tempfile::tempdir().unwrap();
        let setups = test_setups(None);
        let work = root.path().join("work-account");
        let setup = setups.add(&work, Some("/opt/claude".into())).unwrap();
        assert!(work.is_dir());
        assert_eq!(
            setups.environment(&setup),
            [("CLAUDE_CONFIG_DIR".to_string(), setup.folder.clone())]
        );
        assert!(setups.add(&work, None).unwrap_err().contains("already"));
        assert!(setups.add(Path::new("relative"), None).is_err());
        assert_eq!(setups.resolve(&setup.id).unwrap().executable, "/opt/claude");
        setups.remove(&setup.id).unwrap();
        assert!(setups.resolve(&setup.id).is_err());
    }

    #[test]
    fn a_missing_cli_marks_the_setup_unavailable() {
        let root = tempfile::tempdir().unwrap();
        let setups = test_setups(None);
        setups
            .add(root.path(), Some(root.path().join("missing-claude").to_string_lossy().into()))
            .unwrap();
        let listed = setups.provider_setups().unwrap();
        assert_eq!(listed[0].state, ProviderSetupState::Unavailable);
        assert!(listed[0].selected);
        assert_eq!(listed[0].provider, "claude");
    }
}
