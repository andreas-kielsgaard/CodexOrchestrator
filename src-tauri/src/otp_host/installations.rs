use crate::otp_host::OtpRegistry;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS otp_job_agent_installation (
  id INTEGER PRIMARY KEY CHECK(id = 1),
  root TEXT NOT NULL,
  python TEXT NOT NULL
);
";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JobAgentInstallation {
    pub(crate) root: String,
    pub(crate) python: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JobAgentInstallationStatus {
    pub(crate) installation: Option<JobAgentInstallation>,
    pub(crate) status: String,
    pub(crate) detail: String,
}

pub(crate) struct OtpInstallationService {
    database: Mutex<Connection>,
    registry: std::sync::Arc<OtpRegistry>,
}

impl OtpInstallationService {
    pub(crate) fn open(
        database_path: &Path,
        registry: std::sync::Arc<OtpRegistry>,
    ) -> Result<std::sync::Arc<Self>, String> {
        let connection = Connection::open(database_path).map_err(|error| error.to_string())?;
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|error| error.to_string())?;
        Ok(std::sync::Arc::new(Self {
            database: Mutex::new(connection),
            registry,
        }))
    }

    pub(crate) fn status(&self) -> Result<JobAgentInstallationStatus, String> {
        let installation = self.load()?;
        match &installation {
            None => Ok(JobAgentInstallationStatus {
                installation,
                status: "unconfigured".into(),
                detail:
                    "Set the local Job Agent root and Python command to enable its OTP MCP service."
                        .into(),
            }),
            Some(value) => match self.verify(value) {
                Ok(()) => Ok(JobAgentInstallationStatus {
                    installation,
                    status: "verified".into(),
                    detail: "The local Job Agent bridge matches the imported OTP declaration."
                        .into(),
                }),
                Err(error) => Ok(JobAgentInstallationStatus {
                    installation,
                    status: "incompatible".into(),
                    detail: error,
                }),
            },
        }
    }

    pub(crate) fn save_and_verify(
        &self,
        installation: JobAgentInstallation,
    ) -> Result<JobAgentInstallationStatus, String> {
        self.verify(&installation)?;
        self.database
            .lock()
            .map_err(|_| "OTP installation storage is unavailable.".to_string())?
            .execute(
                "INSERT INTO otp_job_agent_installation(id,root,python) VALUES(1,?1,?2)
                 ON CONFLICT(id) DO UPDATE SET root=excluded.root,python=excluded.python",
                params![installation.root.trim(), installation.python.trim()],
            )
            .map_err(|error| error.to_string())?;
        self.status()
    }

    pub(crate) fn verified_installation(&self) -> Result<JobAgentInstallation, String> {
        let value = self.load()?.ok_or("Job Agent OTP is not configured.")?;
        self.verify(&value)?;
        Ok(value)
    }

    fn load(&self) -> Result<Option<JobAgentInstallation>, String> {
        self.database
            .lock()
            .map_err(|_| "OTP installation storage is unavailable.".to_string())?
            .query_row(
                "SELECT root,python FROM otp_job_agent_installation WHERE id=1",
                [],
                |row| {
                    Ok(JobAgentInstallation {
                        root: row.get(0)?,
                        python: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    fn verify(&self, installation: &JobAgentInstallation) -> Result<(), String> {
        let root = PathBuf::from(installation.root.trim());
        if !root.is_dir() {
            return Err("Job Agent root must be an existing folder.".into());
        }
        let python = installation.python.trim();
        if python.is_empty() {
            return Err("Job Agent Python command is required.".into());
        }
        let output = Command::new(python)
            .args(["-m", "job_agent.mcp.orchid_host", "describe"])
            .current_dir(&root)
            .env("PYTHONPATH", root.join("app").join("code"))
            .output()
            .map_err(|error| format!("Unable to start Job Agent bridge: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "Job Agent bridge describe failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let manifest: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("Job Agent bridge returned invalid JSON: {error}"))?;
        let (_, declared) = self.registry.agent_mcp_server("job_agent")?;
        if manifest.get("contractVersion").and_then(Value::as_u64) != Some(1)
            || manifest.get("serverName").and_then(Value::as_str) != Some("job_agent")
        {
            return Err("Job Agent bridge has an incompatible orchestration manifest.".into());
        }
        let remote = manifest
            .get("tools")
            .and_then(Value::as_array)
            .ok_or("Job Agent manifest has no tools.")?
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect::<std::collections::BTreeSet<_>>();
        let local = declared
            .tools
            .iter()
            .map(|tool| tool.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if remote != local {
            return Err("Job Agent bridge tools do not match the imported OTP package.".into());
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct OtpInstallationTauriState {
    service: std::sync::Arc<OtpInstallationService>,
}

impl OtpInstallationTauriState {
    pub(crate) fn new(service: std::sync::Arc<OtpInstallationService>) -> Self {
        Self { service }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveJobAgentInstallationInput {
    installation: JobAgentInstallation,
}

#[tauri::command]
pub(crate) fn read_job_agent_otp_installation(
    state: tauri::State<'_, OtpInstallationTauriState>,
) -> Result<JobAgentInstallationStatus, String> {
    state.service.status()
}

#[tauri::command]
pub(crate) fn save_job_agent_otp_installation(
    state: tauri::State<'_, OtpInstallationTauriState>,
    input: SaveJobAgentInstallationInput,
) -> Result<JobAgentInstallationStatus, String> {
    state.service.save_and_verify(input.installation)
}
