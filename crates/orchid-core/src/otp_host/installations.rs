use crate::persistence::ActiveDatabase;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS otp_job_agent_installation (
  id INTEGER PRIMARY KEY CHECK(id = 1),
  root TEXT NOT NULL,
  python TEXT NOT NULL
);
";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobAgentInstallation {
    pub root: String,
    pub python: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobAgentInstallationStatus {
    pub installation: Option<JobAgentInstallation>,
    pub status: String,
    pub detail: String,
}

pub struct OtpInstallationService {
    database: Arc<ActiveDatabase>,
}

impl OtpInstallationService {
    #[cfg(test)]
    pub fn in_memory() -> std::sync::Arc<Self> {
        let database = ActiveDatabase::from_connection(
            Connection::open_in_memory().expect("OTP installation test database"),
            initialize_otp_installation_storage,
        )
        .expect("OTP installation test schema");
        std::sync::Arc::new(Self {
            database: Arc::new(database),
        })
    }

    pub fn from_database(database: Arc<ActiveDatabase>) -> Arc<Self> {
        Arc::new(Self { database })
    }

    pub fn status(&self) -> Result<JobAgentInstallationStatus, String> {
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

    pub fn save_configuration(
        &self,
        installation: JobAgentInstallation,
    ) -> Result<JobAgentInstallationStatus, String> {
        validate_configuration(&installation)?;
        self.database
            .write("save Job Agent OTP installation", |transaction| {
                transaction
                    .execute(
                        "INSERT INTO otp_job_agent_installation(id,root,python) VALUES(1,?1,?2)
                 ON CONFLICT(id) DO UPDATE SET root=excluded.root,python=excluded.python",
                        params![installation.root.trim(), installation.python.trim()],
                    )
                    .map_err(|error| error.to_string())?;
                Ok::<_, String>(())
            })
            .map_err(crate::persistence::ManagedOperationError::into_string)?;
        self.status()
    }

    pub fn configured_installation(&self) -> Result<Option<JobAgentInstallation>, String> {
        self.load()
    }

    fn load(&self) -> Result<Option<JobAgentInstallation>, String> {
        self.database
            .read("load Job Agent OTP installation", |connection| {
                connection
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
            })
            .map_err(crate::persistence::ManagedOperationError::into_string)
    }
}

pub fn initialize_otp_installation_storage(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(SCHEMA)
        .map_err(|error| error.to_string())
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
