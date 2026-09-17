use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex};

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
}

impl OtpInstallationService {
    #[cfg(test)]
    pub(crate) fn in_memory() -> std::sync::Arc<Self> {
        let connection = Connection::open_in_memory().expect("OTP installation test database");
        connection
            .execute_batch(SCHEMA)
            .expect("OTP installation test schema");
        std::sync::Arc::new(Self {
            database: Mutex::new(connection),
        })
    }

    pub(crate) fn open(database_path: &Path) -> Result<std::sync::Arc<Self>, String> {
        let connection = Connection::open(database_path).map_err(|error| error.to_string())?;
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|error| error.to_string())?;
        Ok(std::sync::Arc::new(Self {
            database: Mutex::new(connection),
        }))
    }

    pub(crate) fn status(&self) -> Result<JobAgentInstallationStatus, String> {
        let installation = self.load()?;
        Ok(JobAgentInstallationStatus {
            status: if installation.is_some() {
                "configured"
            } else {
                "unconfigured"
            }
            .into(),
            detail: if installation.is_some() {
                "The configured Job Agent bridge is contacted only when an agent calls an exposed endpoint.".into()
            } else {
                "Set the local Job Agent root and Python command for agent endpoint calls.".into()
            },
            installation,
        })
    }

    pub(crate) fn save_configuration(
        &self,
        installation: JobAgentInstallation,
    ) -> Result<JobAgentInstallationStatus, String> {
        validate_configuration(&installation)?;
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

    pub(crate) fn configured_installation(&self) -> Result<Option<JobAgentInstallation>, String> {
        self.load()
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
}

fn validate_configuration(installation: &JobAgentInstallation) -> Result<(), String> {
    if installation.root.trim().is_empty() {
        return Err("Job Agent root is required.".into());
    }
    if installation.python.trim().is_empty() {
        return Err("Job Agent Python command is required.".into());
    }
    Ok(())
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
    state.service.save_configuration(input.installation)
}
