use crate::{
    execution_targets::domain::{ExecutionBinding, ExecutionConnection},
    persistence::ActiveDatabase,
};
use chrono::{DateTime, Utc};
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

    pub(crate) fn run_start(&self, device_id: &str) -> Result<(), String> {
        let result = self.run(device_id, true);
        self.record_lifecycle_result(device_id, "start", &result, true)?;
        result
    }

    pub(crate) fn run_stop(&self, device_id: &str) -> Result<(), String> {
        let active =
            self.database
                .read("check device activity", |connection| {
                    connection.query_row(
                "SELECT COUNT(*) FROM execution_device_activity_leases WHERE device_id=?1",
                [device_id],
                |row| row.get::<_, u64>(0),
            ).map_err(|error| error.to_string())
                })
                .map_err(|error| error.into_string())?;
        if active != 0 {
            return Err("Device cannot be stopped while Orchid-managed work is active".into());
        }
        let result = self.run(device_id, false);
        self.record_lifecycle_result(device_id, "stop", &result, result.is_err())?;
        result
    }

    fn run(&self, device_id: &str, start: bool) -> Result<(), String> {
        let _gate = self
            .operation_gate
            .lock()
            .map_err(|_| "Device lifecycle operation is unavailable")?;
        let command = self
            .database
            .read("load device lifecycle command", |connection| {
                connection
                    .query_row(
                        if start {
                            "SELECT start_command_json FROM execution_devices WHERE device_id=?1"
                        } else {
                            "SELECT stop_command_json FROM execution_devices WHERE device_id=?1"
                        },
                        [device_id],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())
            })
            .map_err(|error| error.into_string())?
            .flatten()
            .and_then(|value| serde_json::from_str::<DeviceCommandSpec>(&value).ok())
            .ok_or_else(|| {
                format!(
                    "This device uses manual {}",
                    if start { "startup" } else { "shutdown" }
                )
            })?;
        self.command_runner.run(&command)
    }

    pub(crate) fn acquire_activity(
        &self,
        device_id: &str,
        owner_kind: &str,
        owner_id: &str,
    ) -> Result<(), String> {
        self.database.write("acquire device activity", |transaction| {
            transaction.execute(
                "INSERT OR IGNORE INTO execution_device_activity_leases(device_id,owner_kind,owner_id,acquired_at) VALUES(?1,?2,?3,?4)",
                params![device_id,owner_kind,owner_id,Utc::now().to_rfc3339()],
            ).map_err(|error| error.to_string())?;
            transaction.execute(
                "UPDATE execution_devices SET last_orchid_activity_at=?2,idle_shutdown_claimed_at=NULL WHERE device_id=?1",
                params![device_id,Utc::now().to_rfc3339()],
            ).map_err(|error| error.to_string())?;
            Ok(())
        }).map_err(|error| error.into_string())
    }

    pub(crate) fn release_activity(
        &self,
        device_id: &str,
        owner_kind: &str,
        owner_id: &str,
    ) -> Result<(), String> {
        self.database.write("release device activity", |transaction| {
            transaction.execute(
                "DELETE FROM execution_device_activity_leases WHERE device_id=?1 AND owner_kind=?2 AND owner_id=?3",
                params![device_id,owner_kind,owner_id],
            ).map_err(|error| error.to_string())?;
            transaction.execute(
                "UPDATE execution_devices SET last_orchid_activity_at=?2 WHERE device_id=?1",
                params![device_id,Utc::now().to_rfc3339()],
            ).map_err(|error| error.to_string())?;
            Ok(())
        }).map_err(|error| error.into_string())
    }

    pub(crate) fn hold_awake(&self, device_id: &str, seconds: u64) -> Result<(), String> {
        if !(60..=86_400).contains(&seconds) {
            return Err("A keep-awake hold must be between one minute and one day".into());
        }
        let until = Utc::now() + chrono::Duration::seconds(seconds as i64);
        self.database
            .write("hold execution device awake", |transaction| {
                let changed = transaction
                    .execute(
                        "UPDATE execution_devices SET keep_awake_until=?2,idle_shutdown_claimed_at=NULL,updated_at=?3 WHERE device_id=?1",
                        params![device_id, until.to_rfc3339(), Utc::now().to_rfc3339()],
                    )
                    .map_err(|error| error.to_string())?;
                if changed == 0 {
                    return Err("Execution device was not found".into());
                }
                Ok::<(), String>(())
            })
            .map_err(|error| error.into_string())
    }

    pub(crate) fn evaluate_idle_shutdowns(&self) -> Result<(), String> {
        let now = Utc::now();
        let candidates = self
            .database
            .read("evaluate idle execution devices", |connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT d.device_id,d.idle_shutdown_seconds,d.last_orchid_activity_at,d.keep_awake_until FROM execution_devices d WHERE d.stop_command_json IS NOT NULL AND d.idle_shutdown_seconds IS NOT NULL AND d.idle_shutdown_claimed_at IS NULL AND NOT EXISTS(SELECT 1 FROM execution_device_activity_leases l WHERE l.device_id=d.device_id)",
                    )
                    .map_err(|error| error.to_string())?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, u64>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                        ))
                    })
                    .map_err(|error| error.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())?;
                Ok(rows)
            })
            .map_err(|error| error.into_string())?;
        for (device_id, idle_seconds, last_activity, keep_awake) in candidates {
            let last_activity = last_activity
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc));
            let keep_awake = keep_awake
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc));
            if keep_awake.is_some_and(|until| until > now)
                || last_activity.is_none_or(|activity| {
                    activity + chrono::Duration::seconds(idle_seconds as i64) > now
                })
            {
                continue;
            }
            let claimed = self
                .database
                .write("claim idle execution device shutdown", |transaction| {
                    transaction
                        .execute(
                            "UPDATE execution_devices SET idle_shutdown_claimed_at=?2 WHERE device_id=?1 AND idle_shutdown_claimed_at IS NULL AND NOT EXISTS(SELECT 1 FROM execution_device_activity_leases l WHERE l.device_id=?1)",
                            params![device_id, now.to_rfc3339()],
                        )
                        .map_err(|error| error.to_string())
                })
                .map_err(|error| error.into_string())?;
            if claimed != 0 {
                let _ = self.run_stop(&device_id);
            }
        }
        Ok(())
    }

    pub(crate) fn start_idle_scheduler(self: &Arc<Self>) {
        let service = Arc::downgrade(self);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(60));
            let Some(service) = service.upgrade() else {
                return;
            };
            let _ = service.evaluate_idle_shutdowns();
        });
    }

    pub(crate) fn reconcile_activity_leases(&self) -> Result<(), String> {
        self.database
            .write("reconcile execution device activity", |transaction| {
                let now = Utc::now().to_rfc3339();
                transaction
                    .execute(
                        "UPDATE execution_devices SET last_orchid_activity_at=?1 WHERE device_id IN (
                           SELECT device_id FROM execution_device_activity_leases l
                           WHERE (l.owner_kind='agent_invocation' AND NOT EXISTS(
                             SELECT 1 FROM agent_session_invocations i WHERE i.id=l.owner_id AND i.status='running'
                           )) OR (l.owner_kind='target_transition' AND NOT EXISTS(
                             SELECT 1 FROM agent_session_target_transitions t WHERE t.session_id=l.owner_id AND json_extract(t.payload_json,'$.phase')='running'
                           ))
                         )",
                        [now],
                    )
                    .map_err(|error| error.to_string())?;
                transaction
                    .execute(
                        "DELETE FROM execution_device_activity_leases AS l
                         WHERE (l.owner_kind='agent_invocation' AND NOT EXISTS(
                           SELECT 1 FROM agent_session_invocations i WHERE i.id=l.owner_id AND i.status='running'
                         )) OR (l.owner_kind='target_transition' AND NOT EXISTS(
                           SELECT 1 FROM agent_session_target_transitions t WHERE t.session_id=l.owner_id AND json_extract(t.payload_json,'$.phase')='running'
                         ))",
                        [],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .map_err(|error| error.into_string())
    }

    fn record_lifecycle_result(
        &self,
        device_id: &str,
        operation: &str,
        result: &Result<(), String>,
        reset_idle_claim: bool,
    ) -> Result<(), String> {
        let message = match result {
            Ok(()) => format!("Device {operation} command completed"),
            Err(error) => format!("Device {operation} command failed: {error}"),
        };
        self.database
            .write("record execution device lifecycle", |transaction| {
                transaction
                    .execute(
                        "UPDATE execution_devices SET last_lifecycle_message=?2,idle_shutdown_claimed_at=CASE WHEN ?3 THEN NULL ELSE idle_shutdown_claimed_at END,updated_at=?4 WHERE device_id=?1",
                        params![device_id, message, reset_idle_claim, Utc::now().to_rfc3339()],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
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

trait DeviceCommandRunner: Send + Sync {
    fn run(&self, spec: &DeviceCommandSpec) -> Result<(), String>;
}

struct SystemDeviceCommandRunner;

impl DeviceCommandRunner for SystemDeviceCommandRunner {
    fn run(&self, spec: &DeviceCommandSpec) -> Result<(), String> {
        run_command(spec)
    }
}

fn run_command(spec: &DeviceCommandSpec) -> Result<(), String> {
    spec.validate()?;
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(directory) = &spec.working_directory {
        command.current_dir(directory);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Unable to launch device lifecycle program: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(spec.timeout_seconds);
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return status
                .success()
                .then_some(())
                .ok_or_else(|| format!("Device lifecycle program failed with {status}"));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Device lifecycle program timed out".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
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
