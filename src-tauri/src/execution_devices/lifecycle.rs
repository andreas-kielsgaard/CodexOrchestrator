use super::*;

impl ExecutionDeviceService {
    pub(crate) fn run_start(&self, device_id: &str) -> Result<(), String> {
        if crate::runtime_coordinator::repository::managed(&self.database, device_id)? {
            return Err("Start this device explicitly in Runtime Coordinator".into());
        }
        let result = self.run(device_id, true);
        self.record_lifecycle_result(device_id, "start", &result, true)?;
        result
    }

    pub(crate) fn run_stop(&self, device_id: &str) -> Result<(), String> {
        if crate::runtime_coordinator::repository::managed(&self.database, device_id)? {
            return Err("Use Runtime Coordinator to stop this device".into());
        }
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
            if crate::runtime_coordinator::repository::managed(&self.database, &device_id)? {
                continue;
            }
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

pub(super) trait DeviceCommandRunner: Send + Sync {
    fn run(&self, spec: &DeviceCommandSpec) -> Result<(), String>;
}

pub(super) struct SystemDeviceCommandRunner;

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
