mod activity;
mod lifecycle;
use crate::{
    execution_targets::domain::{ExecutionBinding, ExecutionConnection},
    persistence::ActiveDatabase,
};
use chrono::{DateTime, Utc};
use lifecycle::{DeviceCommandRunner, SystemDeviceCommandRunner};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::State;

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS runtime_coordinator_device_bindings (coordinator_id TEXT NOT NULL, device_id TEXT NOT NULL, execution_device_id TEXT NOT NULL UNIQUE, PRIMARY KEY(coordinator_id,device_id));
CREATE TABLE IF NOT EXISTS execution_devices (
  device_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  connection_json TEXT,
  start_command_json TEXT,
  stop_command_json TEXT,
  idle_shutdown_seconds INTEGER,
  last_orchid_activity_at TEXT,
  keep_awake_until TEXT,
  idle_shutdown_claimed_at TEXT,
  last_lifecycle_message TEXT,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS execution_device_activity_leases (
  device_id TEXT NOT NULL,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  acquired_at TEXT NOT NULL,
  PRIMARY KEY(device_id,owner_kind,owner_id),
  FOREIGN KEY(device_id) REFERENCES execution_devices(device_id) ON DELETE CASCADE
);
"#;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeviceCommandSpec {
    pub(crate) program: String,
    #[serde(default)]
    pub(crate) arguments: Vec<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) timeout_seconds: u64,
}

impl DeviceCommandSpec {
    fn validate(&self) -> Result<(), String> {
        if !Path::new(&self.program).is_absolute() {
            return Err("Device lifecycle program must be an absolute path".into());
        }
        if !(1..=900).contains(&self.timeout_seconds) {
            return Err("Device lifecycle timeout must be between 1 and 900 seconds".into());
        }
        if self
            .working_directory
            .as_deref()
            .is_some_and(|path| !Path::new(path).is_absolute())
        {
            return Err("Device lifecycle working directory must be absolute".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeviceLifecyclePolicy {
    pub(crate) start: Option<DeviceCommandSpec>,
    pub(crate) stop: Option<DeviceCommandSpec>,
    pub(crate) idle_shutdown_seconds: Option<u64>,
}

impl DeviceLifecyclePolicy {
    fn validate(&self) -> Result<(), String> {
        if let Some(command) = &self.start {
            command.validate()?;
        }
        if let Some(command) = &self.stop {
            command.validate()?;
        }
        if self
            .idle_shutdown_seconds
            .is_some_and(|seconds| !(60..=2_592_000).contains(&seconds))
        {
            return Err("Idle shutdown must be between one minute and thirty days".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionDeviceDto {
    pub(crate) device_id: String,
    pub(crate) display_name: String,
    pub(crate) connection_summary: String,
    pub(crate) lifecycle: DeviceLifecyclePolicy,
    pub(crate) active_leases: u64,
    pub(crate) last_orchid_activity_at: Option<String>,
    pub(crate) keep_awake_until: Option<String>,
    pub(crate) last_lifecycle_message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveExecutionDeviceInput {
    device_id: String,
    display_name: String,
    lifecycle: DeviceLifecyclePolicy,
}

pub(crate) struct ExecutionDeviceService {
    database: Arc<ActiveDatabase>,
    operation_gate: Mutex<()>,
    command_runner: Arc<dyn DeviceCommandRunner>,
}

impl ExecutionDeviceService {
    pub(crate) fn new(database: Arc<ActiveDatabase>) -> Self {
        Self::with_command_runner(database, Arc::new(SystemDeviceCommandRunner))
    }

    fn with_command_runner(
        database: Arc<ActiveDatabase>,
        command_runner: Arc<dyn DeviceCommandRunner>,
    ) -> Self {
        Self {
            database,
            operation_gate: Mutex::new(()),
            command_runner,
        }
    }

    pub(crate) fn synchronize_bindings(&self, bindings: &[ExecutionBinding]) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.database
            .write("synchronize execution devices", |transaction| {
                for binding in bindings {
                    let connection = serde_json::to_string(&binding.connection)
                        .map_err(|error| error.to_string())?;
                    let existing_connection = transaction
                        .query_row(
                            "SELECT connection_json FROM execution_devices WHERE device_id=?1",
                            [&binding.device_id],
                            |row| row.get::<_, Option<String>>(0),
                        )
                        .optional()
                        .map_err(|error| error.to_string())?
                        .flatten();
                    if let Some(existing) = existing_connection {
                        let existing = serde_json::from_str::<ExecutionConnection>(&existing)
                            .map_err(|error| {
                                format!(
                                    "Device '{}' has an invalid developer connection: {error}",
                                    binding.device_id
                                )
                            })?;
                        if existing != binding.connection {
                            return Err(format!(
                                "Device '{}' has conflicting developer connection definitions",
                                binding.device_id
                            ));
                        }
                    }
                    transaction.execute(
                        "INSERT INTO execution_devices(device_id,display_name,connection_json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(device_id) DO UPDATE SET display_name=excluded.display_name,connection_json=COALESCE(execution_devices.connection_json,excluded.connection_json),updated_at=excluded.updated_at",
                        params![binding.device_id, binding.device_name, connection, now],
                    ).map_err(|error| error.to_string())?;
                }
                Ok::<(), String>(())
            })
            .map_err(|error| error.into_string())
    }

    pub(crate) fn save(&self, input: SaveExecutionDeviceInput) -> Result<(), String> {
        if input.device_id.trim().is_empty() || input.display_name.trim().is_empty() {
            return Err("Device identity and display name are required".into());
        }
        input.lifecycle.validate()?;
        let start = encode_command(input.lifecycle.start.as_ref())?;
        let stop = encode_command(input.lifecycle.stop.as_ref())?;
        self.database
            .write("save execution device", |transaction| {
                transaction.execute(
                    "INSERT INTO execution_devices(device_id,display_name,start_command_json,stop_command_json,idle_shutdown_seconds,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(device_id) DO UPDATE SET display_name=excluded.display_name,start_command_json=excluded.start_command_json,stop_command_json=excluded.stop_command_json,idle_shutdown_seconds=excluded.idle_shutdown_seconds,idle_shutdown_claimed_at=NULL,updated_at=excluded.updated_at",
                    params![input.device_id, input.display_name, start, stop, input.lifecycle.idle_shutdown_seconds, Utc::now().to_rfc3339()],
                ).map_err(|error| error.to_string())?;
                Ok::<(), String>(())
            })
            .map_err(|error| error.into_string())
    }

    pub(crate) fn list(&self) -> Result<Vec<ExecutionDeviceDto>, String> {
        self.database
            .read("list execution devices", |connection| {
                let mut statement = connection.prepare(
                    "SELECT d.device_id,d.display_name,d.connection_json,d.start_command_json,d.stop_command_json,d.idle_shutdown_seconds,d.last_orchid_activity_at,(SELECT COUNT(*) FROM execution_device_activity_leases l WHERE l.device_id=d.device_id),d.keep_awake_until,d.last_lifecycle_message FROM execution_devices d ORDER BY CASE WHEN d.device_id='local' THEN 0 ELSE 1 END,d.display_name",
                ).map_err(|error| error.to_string())?;
                let rows = statement.query_map([], |row| {
                    let connection_json: Option<String> = row.get(2)?;
                    let connection_summary = connection_json
                        .as_deref()
                        .and_then(|value| serde_json::from_str::<ExecutionConnection>(value).ok())
                        .map(|connection| match connection {
                            ExecutionConnection::Local => "Runs on this device".to_string(),
                            ExecutionConnection::Ssh { .. } => "Developer-configured remote connection".to_string(),
                        })
                        .unwrap_or_else(|| "Connection is configured outside this screen".into());
                    Ok(ExecutionDeviceDto {
                        device_id: row.get(0)?,
                        display_name: row.get(1)?,
                        connection_summary,
                        lifecycle: DeviceLifecyclePolicy {
                            start: decode_command(row.get::<_, Option<String>>(3)?),
                            stop: decode_command(row.get::<_, Option<String>>(4)?),
                            idle_shutdown_seconds: row.get(5)?,
                        },
                        last_orchid_activity_at: row.get(6)?,
                        active_leases: row.get(7)?,
                        keep_awake_until: row.get(8)?,
                        last_lifecycle_message: row.get(9)?,
                    })
                }).map_err(|error| error.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())?;
                Ok(rows)
            })
            .map_err(|error| error.into_string())
    }
}

fn encode_command(value: Option<&DeviceCommandSpec>) -> Result<Option<String>, String> {
    value
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| error.to_string())
}

fn decode_command(value: Option<String>) -> Option<DeviceCommandSpec> {
    value.and_then(|value| serde_json::from_str(&value).ok())
}

pub(crate) struct ExecutionDeviceTauriState(pub(crate) Arc<ExecutionDeviceService>);

#[tauri::command]
pub(crate) fn list_execution_devices(
    state: State<'_, ExecutionDeviceTauriState>,
) -> Result<Vec<ExecutionDeviceDto>, String> {
    state.0.list()
}

#[tauri::command]
pub(crate) fn save_execution_device(
    state: State<'_, ExecutionDeviceTauriState>,
    input: SaveExecutionDeviceInput,
) -> Result<Vec<ExecutionDeviceDto>, String> {
    state.0.save(input)?;
    state.0.list()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExecutionDeviceIdInput {
    device_id: String,
}

#[tauri::command]
pub(crate) async fn start_execution_device(
    state: State<'_, ExecutionDeviceTauriState>,
    input: ExecutionDeviceIdInput,
) -> Result<(), String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || service.run_start(&input.device_id))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn stop_execution_device(
    state: State<'_, ExecutionDeviceTauriState>,
    input: ExecutionDeviceIdInput,
) -> Result<(), String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || service.run_stop(&input.device_id))
        .await
        .map_err(|error| error.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HoldExecutionDeviceInput {
    device_id: String,
    seconds: u64,
}

#[tauri::command]
pub(crate) fn hold_execution_device_awake(
    state: State<'_, ExecutionDeviceTauriState>,
    input: HoldExecutionDeviceInput,
) -> Result<Vec<ExecutionDeviceDto>, String> {
    state.0.hold_awake(&input.device_id, input.seconds)?;
    state.0.list()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn service() -> ExecutionDeviceService {
        let database =
            ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |connection| {
                connection
                    .execute_batch(SCHEMA)
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        ExecutionDeviceService::new(Arc::new(database))
    }

    struct CountingRunner(AtomicUsize);

    impl DeviceCommandRunner for CountingRunner {
        fn run(&self, _: &DeviceCommandSpec) -> Result<(), String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FailingOnceRunner(AtomicUsize);

    impl DeviceCommandRunner for FailingOnceRunner {
        fn run(&self, _: &DeviceCommandSpec) -> Result<(), String> {
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                Err("temporary provider failure".into())
            } else {
                Ok(())
            }
        }
    }

    fn no_lifecycle() -> DeviceLifecyclePolicy {
        DeviceLifecyclePolicy {
            start: None,
            stop: None,
            idle_shutdown_seconds: None,
        }
    }

    #[test]
    fn coordinated_devices_require_explicit_controls_and_skip_idle_shutdown() {
        let database = Arc::new(ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |db| db.execute_batch(SCHEMA).map_err(|e| e.to_string())).unwrap());
        let runner = Arc::new(CountingRunner(AtomicUsize::new(0)));
        let service = ExecutionDeviceService::with_command_runner(database.clone(), runner.clone());
        let command = DeviceCommandSpec { program: if cfg!(windows) { r"C:\Orchid\power.exe" } else { "/usr/bin/power" }.into(), arguments: vec![], working_directory: None, timeout_seconds: 30 };
        service.save(SaveExecutionDeviceInput { device_id: "worker".into(), display_name: "Worker".into(), lifecycle: DeviceLifecyclePolicy { start: Some(command.clone()), stop: Some(command), idle_shutdown_seconds: Some(60) } }).unwrap();
        let bindings = crate::runtime_coordinator::repository::Bindings(database.clone());
        bindings.associate(&crate::runtime_coordinator::repository::Binding { coordinator_id: "coordinator".into(), device_id: "cloud-device".into(), execution_device_id: "worker".into() }).unwrap();
        assert!(service.run_start("worker").unwrap_err().contains("explicitly"));
        assert!(service.run_stop("worker").unwrap_err().contains("Runtime Coordinator"));
        database.write("age managed worker", |db| { db.execute("UPDATE execution_devices SET last_orchid_activity_at=?1", [(Utc::now()-chrono::Duration::minutes(2)).to_rfc3339()]).map(|_|()).map_err(|e|e.to_string()) }).unwrap();
        service.evaluate_idle_shutdowns().unwrap();
        assert_eq!(runner.0.load(Ordering::SeqCst), 0);
        service.acquire_activity("worker", "agent_invocation", "invocation").unwrap();
        assert_eq!(bindings.active("coordinator", "cloud-device").unwrap(), 1);
        service.release_activity("worker", "agent_invocation", "invocation").unwrap();
        assert_eq!(bindings.active("coordinator", "cloud-device").unwrap(), 0);
    }

    #[test]
    fn configured_device_is_independent_of_capability_profiles() {
        let service = service();
        service
            .save(SaveExecutionDeviceInput {
                device_id: "remote-one".into(),
                display_name: "Remote one".into(),
                lifecycle: no_lifecycle(),
            })
            .unwrap();
        let devices = service.list().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "remote-one");
        assert_eq!(
            devices[0].connection_summary,
            "Connection is configured outside this screen"
        );
    }

    #[test]
    fn active_orchid_lease_blocks_stop_and_release_records_activity() {
        let service = service();
        service
            .save(SaveExecutionDeviceInput {
                device_id: "remote-one".into(),
                display_name: "Remote one".into(),
                lifecycle: no_lifecycle(),
            })
            .unwrap();
        service
            .acquire_activity("remote-one", "agent_invocation", "invocation-one")
            .unwrap();
        assert_eq!(service.list().unwrap()[0].active_leases, 1);
        assert!(service
            .run_stop("remote-one")
            .unwrap_err()
            .contains("Orchid-managed work is active"));
        service
            .release_activity("remote-one", "agent_invocation", "invocation-one")
            .unwrap();
        let device = service.list().unwrap().remove(0);
        assert_eq!(device.active_leases, 0);
        assert!(device.last_orchid_activity_at.is_some());
    }

    #[test]
    fn keep_awake_hold_is_persisted() {
        let service = service();
        service
            .save(SaveExecutionDeviceInput {
                device_id: "remote-one".into(),
                display_name: "Remote one".into(),
                lifecycle: no_lifecycle(),
            })
            .unwrap();
        service.hold_awake("remote-one", 3600).unwrap();
        let until = service.list().unwrap()[0]
            .keep_awake_until
            .as_deref()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap();
        assert!(until > Utc::now());
    }

    #[test]
    fn conflicting_connection_definitions_are_rejected() {
        let service = service();
        let local = ExecutionBinding::default();
        service.synchronize_bindings(&[local.clone()]).unwrap();
        let conflicting = ExecutionBinding {
            connection: ExecutionConnection::Ssh {
                target: "example-host".into(),
                host_executable: "C:\\Orchid\\orchid-host.exe".into(),
            },
            ..local
        };

        assert!(service
            .synchronize_bindings(&[conflicting])
            .unwrap_err()
            .contains("conflicting developer connection definitions"));
    }

    #[test]
    fn idle_shutdown_runs_once_after_the_last_orchid_activity() {
        let database = Arc::new(
            ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |connection| {
                connection
                    .execute_batch(SCHEMA)
                    .map_err(|error| error.to_string())
            })
            .unwrap(),
        );
        let runner = Arc::new(CountingRunner(AtomicUsize::new(0)));
        let service = ExecutionDeviceService::with_command_runner(database.clone(), runner.clone());
        service
            .save(SaveExecutionDeviceInput {
                device_id: "remote-one".into(),
                display_name: "Remote one".into(),
                lifecycle: DeviceLifecyclePolicy {
                    start: None,
                    stop: Some(DeviceCommandSpec {
                        program: r"C:\Orchid\stop-device.exe".into(),
                        arguments: vec![],
                        working_directory: None,
                        timeout_seconds: 30,
                    }),
                    idle_shutdown_seconds: Some(60),
                },
            })
            .unwrap();
        database
            .write("age device activity", |transaction| {
                transaction
                    .execute(
                        "UPDATE execution_devices SET last_orchid_activity_at=?2 WHERE device_id=?1",
                        params![
                            "remote-one",
                            (Utc::now() - chrono::Duration::minutes(2)).to_rfc3339()
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok::<(), String>(())
            })
            .unwrap();

        service.evaluate_idle_shutdowns().unwrap();
        service.evaluate_idle_shutdowns().unwrap();
        assert_eq!(runner.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_idle_shutdown_is_released_for_a_later_retry() {
        let database = Arc::new(
            ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |connection| {
                connection
                    .execute_batch(SCHEMA)
                    .map_err(|error| error.to_string())
            })
            .unwrap(),
        );
        let runner = Arc::new(FailingOnceRunner(AtomicUsize::new(0)));
        let service = ExecutionDeviceService::with_command_runner(database.clone(), runner.clone());
        service
            .save(SaveExecutionDeviceInput {
                device_id: "remote-one".into(),
                display_name: "Remote one".into(),
                lifecycle: DeviceLifecyclePolicy {
                    start: None,
                    stop: Some(DeviceCommandSpec {
                        program: r"C:\Orchid\stop-device.exe".into(),
                        arguments: vec![],
                        working_directory: None,
                        timeout_seconds: 30,
                    }),
                    idle_shutdown_seconds: Some(60),
                },
            })
            .unwrap();
        database
            .write("age device activity", |transaction| {
                transaction
                    .execute(
                        "UPDATE execution_devices SET last_orchid_activity_at=?2 WHERE device_id=?1",
                        params![
                            "remote-one",
                            (Utc::now() - chrono::Duration::minutes(2)).to_rfc3339()
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok::<(), String>(())
            })
            .unwrap();

        service.evaluate_idle_shutdowns().unwrap();
        service.evaluate_idle_shutdowns().unwrap();

        assert_eq!(runner.0.load(Ordering::SeqCst), 2);
        assert_eq!(
            service.list().unwrap()[0].last_lifecycle_message.as_deref(),
            Some("Device stop command completed")
        );
    }

    #[test]
    fn keep_awake_and_activity_lease_prevent_idle_shutdown() {
        let database = Arc::new(
            ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |connection| {
                connection
                    .execute_batch(SCHEMA)
                    .map_err(|error| error.to_string())
            })
            .unwrap(),
        );
        let runner = Arc::new(CountingRunner(AtomicUsize::new(0)));
        let service = ExecutionDeviceService::with_command_runner(database.clone(), runner.clone());
        service
            .save(SaveExecutionDeviceInput {
                device_id: "remote-one".into(),
                display_name: "Remote one".into(),
                lifecycle: DeviceLifecyclePolicy {
                    start: None,
                    stop: Some(DeviceCommandSpec {
                        program: r"C:\Orchid\stop-device.exe".into(),
                        arguments: vec![],
                        working_directory: None,
                        timeout_seconds: 30,
                    }),
                    idle_shutdown_seconds: Some(60),
                },
            })
            .unwrap();
        database
            .write("age device activity", |transaction| {
                transaction
                    .execute(
                        "UPDATE execution_devices SET last_orchid_activity_at=?2 WHERE device_id=?1",
                        params![
                            "remote-one",
                            (Utc::now() - chrono::Duration::minutes(2)).to_rfc3339()
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok::<(), String>(())
            })
            .unwrap();
        service.hold_awake("remote-one", 3600).unwrap();
        service.evaluate_idle_shutdowns().unwrap();
        assert_eq!(runner.0.load(Ordering::SeqCst), 0);

        database
            .write("expire keep awake", |transaction| {
                transaction
                    .execute(
                        "UPDATE execution_devices SET keep_awake_until=NULL WHERE device_id=?1",
                        ["remote-one"],
                    )
                    .map_err(|error| error.to_string())?;
                Ok::<(), String>(())
            })
            .unwrap();
        service
            .acquire_activity("remote-one", "agent_invocation", "invocation-one")
            .unwrap();
        service.evaluate_idle_shutdowns().unwrap();
        assert_eq!(runner.0.load(Ordering::SeqCst), 0);
    }
}
