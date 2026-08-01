use super::domain::{
    InstanceId, InstanceIdentity, InstanceProjection, InstanceRecord, InstanceSnapshot,
    InstanceState, OwnerRoute, RequestId, RuntimeObservation,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    error::Error,
    fmt,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

const REGISTRY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS worktree_runtime_instances (
  instance_id TEXT PRIMARY KEY,
  identity_json TEXT NOT NULL,
  projection_json TEXT NOT NULL,
  authority_hash TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN (
    'prepared', 'launch_pending', 'running', 'stop_pending',
    'stopped', 'recovery_pending', 'recovered'
  )),
  launch_id TEXT,
  job_name TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  CHECK ((launch_id IS NULL) = (job_name IS NULL))
);
CREATE TABLE IF NOT EXISTS worktree_runtime_port_leases (
  port INTEGER PRIMARY KEY,
  instance_id TEXT NOT NULL REFERENCES worktree_runtime_instances(instance_id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('vite', 'status')),
  UNIQUE(instance_id, role)
);
CREATE TABLE IF NOT EXISTS worktree_runtime_commands (
  request_id TEXT PRIMARY KEY,
  instance_id TEXT NOT NULL,
  operation TEXT NOT NULL CHECK (operation IN ('prepare', 'start', 'stop', 'recover')),
  fingerprint TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'succeeded', 'failed')),
  result_json TEXT,
  failure_json TEXT,
  recorded_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  CHECK (
    (status = 'pending' AND result_json IS NULL AND failure_json IS NULL) OR
    (status = 'succeeded' AND result_json IS NOT NULL AND failure_json IS NULL) OR
    (status = 'failed' AND result_json IS NULL AND failure_json IS NOT NULL)
  )
);
CREATE UNIQUE INDEX IF NOT EXISTS worktree_runtime_one_pending_lifecycle_command
  ON worktree_runtime_commands(instance_id)
  WHERE status = 'pending' AND operation IN ('start', 'stop', 'recover');
CREATE TABLE IF NOT EXISTS worktree_runtime_observations (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id TEXT NOT NULL REFERENCES worktree_runtime_instances(instance_id),
  transition_kind TEXT NOT NULL,
  observation_json TEXT NOT NULL,
  observed_at TEXT NOT NULL
);
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RegistryErrorKind {
    NotFound,
    Unauthorized,
    Conflict,
    PortLeaseConflict,
    InvalidState,
    IdempotencyConflict,
    OperationInProgress,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RegistryError {
    pub(crate) kind: RegistryErrorKind,
    pub(crate) message: String,
}

impl RegistryError {
    fn new(kind: RegistryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RegistryError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CommandStart {
    Execute(InstanceRecord),
    Noop(InstanceSnapshot),
    Replay(InstanceSnapshot),
    ReplayFailure(StoredFailure),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredFailure {
    pub(crate) kind: String,
    pub(crate) message: String,
}

pub(crate) struct PrepareRecord<'a> {
    pub(crate) request_id: &'a RequestId,
    pub(crate) fingerprint: &'a str,
    pub(crate) identity: &'a InstanceIdentity,
    pub(crate) projection: &'a InstanceProjection,
    pub(crate) authority_hash: &'a str,
    pub(crate) recorded_at: DateTime<Utc>,
}

pub(crate) trait InstanceRegistry: Send + Sync {
    fn prepare(&self, record: PrepareRecord<'_>) -> Result<InstanceSnapshot, RegistryError>;

    fn load_authorized(
        &self,
        instance_id: &InstanceId,
        authority_hash: &str,
    ) -> Result<InstanceRecord, RegistryError>;

    fn replay_command(
        &self,
        request_id: &RequestId,
        fingerprint: &str,
        instance_id: &InstanceId,
        operation: &str,
    ) -> Result<Option<CommandStart>, RegistryError>;

    fn begin_start(
        &self,
        request_id: &RequestId,
        fingerprint: &str,
        instance_id: &InstanceId,
        authority_hash: &str,
        route: &OwnerRoute,
        recorded_at: DateTime<Utc>,
    ) -> Result<CommandStart, RegistryError>;

    fn begin_stop(
        &self,
        request_id: &RequestId,
        fingerprint: &str,
        instance_id: &InstanceId,
        authority_hash: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<CommandStart, RegistryError>;

    fn begin_recovery(
        &self,
        request_id: &RequestId,
        fingerprint: &str,
        instance_id: &InstanceId,
        authority_hash: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<CommandStart, RegistryError>;

    fn complete_transition(
        &self,
        request_id: &RequestId,
        expected: InstanceState,
        next: InstanceState,
        transition_kind: &str,
        observation: RuntimeObservation,
    ) -> Result<InstanceSnapshot, RegistryError>;

    fn fail_transition(
        &self,
        request_id: &RequestId,
        expected: InstanceState,
        terminal_state: Option<InstanceState>,
        failure: StoredFailure,
        observation: Option<RuntimeObservation>,
    ) -> Result<(), RegistryError>;

    fn record_observation(
        &self,
        instance_id: &InstanceId,
        transition_kind: &str,
        observation: RuntimeObservation,
    ) -> Result<InstanceSnapshot, RegistryError>;
}

pub(crate) struct SqliteInstanceRegistry {
    _execution_lease: RegistryExecutionLease,
    connection: Mutex<Connection>,
    active_starts: Mutex<HashSet<String>>,
}

impl SqliteInstanceRegistry {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, RegistryError> {
        let path = path.as_ref();
        let execution_lease = RegistryExecutionLease::acquire(path)?;
        let connection = Connection::open(path).map_err(sql_error("open worktree registry"))?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(sql_error("configure worktree registry busy timeout"))?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(sql_error("enable worktree registry foreign keys"))?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(sql_error("enable worktree registry WAL"))?;
        connection
            .execute_batch(REGISTRY_SCHEMA)
            .map_err(sql_error("initialize worktree registry"))?;
        Ok(Self {
            _execution_lease: execution_lease,
            connection: Mutex::new(connection),
            active_starts: Mutex::new(HashSet::new()),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, RegistryError> {
        self.connection.lock().map_err(|_| {
            RegistryError::new(
                RegistryErrorKind::Unavailable,
                "worktree registry lock is unavailable",
            )
        })
    }

    fn lock_active_starts(&self) -> Result<MutexGuard<'_, HashSet<String>>, RegistryError> {
        self.active_starts.lock().map_err(|_| {
            RegistryError::new(
                RegistryErrorKind::Unavailable,
                "active start ownership is unavailable",
            )
        })
    }
}

#[cfg(windows)]
struct RegistryExecutionLease(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
unsafe impl Send for RegistryExecutionLease {}
#[cfg(windows)]
unsafe impl Sync for RegistryExecutionLease {}

#[cfg(windows)]
impl RegistryExecutionLease {
    fn acquire(path: &Path) -> Result<Self, RegistryError> {
        use std::{os::windows::ffi::OsStrExt, ptr::null};
        use windows_sys::Win32::{
            Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS},
            System::Threading::CreateMutexW,
        };

        let identity = registry_path_identity(path)?;
        let digest = format!(
            "{:x}",
            Sha256::digest(identity.to_string_lossy().as_bytes())
        );
        let name = format!("Local\\CodexOrchestrator.WorktreeRuntime.Registry.{digest}");
        let wide = std::ffi::OsStr::new(&name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let handle = unsafe { CreateMutexW(null(), 0, wide.as_ptr()) };
        if handle.is_null() {
            return Err(RegistryError::new(
                RegistryErrorKind::Unavailable,
                format!(
                    "create worktree registry execution lease: Windows error {}",
                    unsafe { GetLastError() }
                ),
            ));
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            return Err(RegistryError::new(
                RegistryErrorKind::Conflict,
                "the worktree registry is already owned by another live application execution",
            ));
        }
        Ok(Self(handle))
    }
}

#[cfg(windows)]
impl Drop for RegistryExecutionLease {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.0);
            }
        }
    }
}

#[cfg(not(windows))]
struct RegistryExecutionLease;

#[cfg(not(windows))]
impl RegistryExecutionLease {
    fn acquire(path: &Path) -> Result<Self, RegistryError> {
        registry_path_identity(path)?;
        Ok(Self)
    }
}

fn registry_path_identity(path: &Path) -> Result<PathBuf, RegistryError> {
    if path.exists() {
        return path.canonicalize().map_err(|error| {
            RegistryError::new(RegistryErrorKind::Unavailable, error.to_string())
        });
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| RegistryError::new(RegistryErrorKind::Unavailable, error.to_string()))?
            .join(path)
    };
    let parent = absolute.parent().ok_or_else(|| {
        RegistryError::new(
            RegistryErrorKind::Unavailable,
            "worktree registry path has no parent directory",
        )
    })?;
    let parent = parent.canonicalize().map_err(|error| {
        RegistryError::new(
            RegistryErrorKind::Unavailable,
            format!("resolve worktree registry directory: {error}"),
        )
    })?;
    let name = absolute.file_name().ok_or_else(|| {
        RegistryError::new(
            RegistryErrorKind::Unavailable,
            "worktree registry path has no file name",
        )
    })?;
    Ok(parent.join(name))
}

impl InstanceRegistry for SqliteInstanceRegistry {
    fn prepare(&self, input: PrepareRecord<'_>) -> Result<InstanceSnapshot, RegistryError> {
        input
            .identity
            .validate()
            .map_err(contract_error("validate instance identity"))?;
        input
            .projection
            .validate()
            .map_err(contract_error("validate instance projection"))?;
        let mut connection = self.lock()?;
        let transaction = immediate(&mut connection, "begin instance preparation")?;
        if let Some(replay) = command_replay(
            &transaction,
            input.request_id,
            input.identity.instance_id.as_str(),
            "prepare",
            input.fingerprint,
        )? {
            return match finish_replay(transaction, replay)? {
                CommandStart::Replay(snapshot) => Ok(snapshot),
                CommandStart::ReplayFailure(failure) => Err(RegistryError::new(
                    RegistryErrorKind::Unavailable,
                    format!("prepared command replay failed: {}", failure.message),
                )),
                CommandStart::Execute(_) | CommandStart::Noop(_) => unreachable!(),
            };
        }

        let current = load_record(&transaction, &input.identity.instance_id)?;
        let record = if let Some((record, authority_hash)) = current {
            if authority_hash != input.authority_hash {
                return Err(RegistryError::new(
                    RegistryErrorKind::Unauthorized,
                    "instance authority does not match",
                ));
            }
            if record.identity != *input.identity || record.projection != *input.projection {
                return Err(RegistryError::new(
                    RegistryErrorKind::Conflict,
                    "prepared instance identity or projection conflicts with the durable record",
                ));
            }
            if !matches!(
                record.state,
                InstanceState::Prepared | InstanceState::Stopped | InstanceState::Recovered
            ) {
                return Err(RegistryError::new(
                    RegistryErrorKind::InvalidState,
                    "a live or unresolved instance cannot be prepared again",
                ));
            }
            record
        } else {
            let record = InstanceRecord {
                identity: input.identity.clone(),
                projection: input.projection.clone(),
                state: InstanceState::Prepared,
                owner_route: None,
                created_at: input.recorded_at,
                updated_at: input.recorded_at,
            };
            transaction
                .execute(
                    "INSERT INTO worktree_runtime_instances
                     (instance_id, identity_json, projection_json, authority_hash, state,
                      launch_id, job_name, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 'prepared', NULL, NULL, ?5, ?5)",
                    params![
                        input.identity.instance_id.as_str(),
                        json(input.identity, "serialize instance identity")?,
                        json(input.projection, "serialize instance projection")?,
                        input.authority_hash,
                        timestamp(input.recorded_at),
                    ],
                )
                .map_err(map_prepare_insert_error)?;
            for (role, port) in [
                ("vite", input.projection.ports.vite),
                ("status", input.projection.ports.status),
            ] {
                transaction
                    .execute(
                        "INSERT INTO worktree_runtime_port_leases (port, instance_id, role)
                         VALUES (?1, ?2, ?3)",
                        params![i64::from(port), input.identity.instance_id.as_str(), role],
                    )
                    .map_err(map_port_lease_error)?;
            }
            record
        };
        let snapshot = InstanceSnapshot::from_record(record);
        insert_succeeded_command(
            &transaction,
            input.request_id,
            input.identity.instance_id.as_str(),
            "prepare",
            input.fingerprint,
            &snapshot,
            input.recorded_at,
        )?;
        transaction
            .commit()
            .map_err(sql_error("commit instance preparation"))?;
        Ok(snapshot)
    }

    fn load_authorized(
        &self,
        instance_id: &InstanceId,
        authority_hash: &str,
    ) -> Result<InstanceRecord, RegistryError> {
        let connection = self.lock()?;
        let Some((record, stored_hash)) = load_record(&connection, instance_id)? else {
            return Err(RegistryError::new(
                RegistryErrorKind::NotFound,
                format!("instance {instance_id} was not found"),
            ));
        };
        if stored_hash != authority_hash {
            return Err(RegistryError::new(
                RegistryErrorKind::Unauthorized,
                "instance authority does not match",
            ));
        }
        Ok(record)
    }

    fn replay_command(
        &self,
        request_id: &RequestId,
        fingerprint: &str,
        instance_id: &InstanceId,
        operation: &str,
    ) -> Result<Option<CommandStart>, RegistryError> {
        let mut connection = self.lock()?;
        let transaction = immediate(&mut connection, "read runtime command replay")?;
        let replay = command_replay(
            &transaction,
            request_id,
            instance_id.as_str(),
            operation,
            fingerprint,
        )?;
        replay
            .map(|replay| finish_replay(transaction, replay))
            .transpose()
    }

    fn begin_start(
        &self,
        request_id: &RequestId,
        fingerprint: &str,
        instance_id: &InstanceId,
        authority_hash: &str,
        route: &OwnerRoute,
        recorded_at: DateTime<Utc>,
    ) -> Result<CommandStart, RegistryError> {
        let mut active_starts = self.lock_active_starts()?;
        if active_starts.contains(instance_id.as_str()) {
            return Err(RegistryError::new(
                RegistryErrorKind::OperationInProgress,
                "the start transition is already executing for this instance",
            ));
        }
        let mut connection = self.lock()?;
        let transaction = immediate(&mut connection, "begin instance start")?;
        if let Some(replay) = command_replay(
            &transaction,
            request_id,
            instance_id.as_str(),
            "start",
            fingerprint,
        )? {
            return finish_replay(transaction, replay);
        }
        let record = authorized_record(&transaction, instance_id, authority_hash)?;
        if !matches!(
            record.state,
            InstanceState::Prepared | InstanceState::Stopped | InstanceState::Recovered
        ) {
            return Err(RegistryError::new(
                RegistryErrorKind::InvalidState,
                format!(
                    "instance {instance_id} cannot start from {:?}",
                    record.state
                ),
            ));
        }
        transaction
            .execute(
                "UPDATE worktree_runtime_instances
                 SET state='launch_pending', launch_id=?2, job_name=?3, updated_at=?4
                 WHERE instance_id=?1",
                params![
                    instance_id.as_str(),
                    route.launch_id.as_str(),
                    route.job_name,
                    timestamp(recorded_at)
                ],
            )
            .map_err(sql_error("persist launch ownership route"))?;
        insert_pending_command(
            &transaction,
            request_id,
            instance_id.as_str(),
            "start",
            fingerprint,
            recorded_at,
        )?;
        let next = load_required_record(&transaction, instance_id)?;
        transaction
            .commit()
            .map_err(sql_error("commit instance start reservation"))?;
        active_starts.insert(instance_id.as_str().to_owned());
        Ok(CommandStart::Execute(next))
    }

    fn begin_stop(
        &self,
        request_id: &RequestId,
        fingerprint: &str,
        instance_id: &InstanceId,
        authority_hash: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<CommandStart, RegistryError> {
        let active_starts = self.lock_active_starts()?;
        let mut connection = self.lock()?;
        begin_terminal_transition(
            &mut connection,
            request_id,
            fingerprint,
            instance_id,
            authority_hash,
            "stop",
            InstanceState::StopPending,
            active_starts.contains(instance_id.as_str()),
            recorded_at,
        )
    }

    fn begin_recovery(
        &self,
        request_id: &RequestId,
        fingerprint: &str,
        instance_id: &InstanceId,
        authority_hash: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<CommandStart, RegistryError> {
        let active_starts = self.lock_active_starts()?;
        let mut connection = self.lock()?;
        begin_terminal_transition(
            &mut connection,
            request_id,
            fingerprint,
            instance_id,
            authority_hash,
            "recover",
            InstanceState::RecoveryPending,
            active_starts.contains(instance_id.as_str()),
            recorded_at,
        )
    }

    fn complete_transition(
        &self,
        request_id: &RequestId,
        expected: InstanceState,
        next: InstanceState,
        transition_kind: &str,
        observation: RuntimeObservation,
    ) -> Result<InstanceSnapshot, RegistryError> {
        let mut active_starts = self.lock_active_starts()?;
        let mut connection = self.lock()?;
        let transaction = immediate(&mut connection, "complete runtime transition")?;
        let command = load_pending_command(&transaction, request_id)?;
        let instance_id = InstanceId::new(command.instance_id.clone())
            .map_err(contract_error("decode command instance ID"))?;
        let record = load_required_record(&transaction, &instance_id)?;
        if record.state != expected {
            return Err(RegistryError::new(
                RegistryErrorKind::InvalidState,
                format!(
                    "instance {instance_id} changed from expected {expected:?} to {:?}",
                    record.state
                ),
            ));
        }
        transaction
            .execute(
                "UPDATE worktree_runtime_instances SET state=?2, updated_at=?3 WHERE instance_id=?1",
                params![
                    instance_id.as_str(),
                    state_name(next),
                    timestamp(observation.observed_at)
                ],
            )
            .map_err(sql_error("persist completed runtime transition"))?;
        insert_observation(&transaction, &instance_id, transition_kind, &observation)?;
        let record = load_required_record(&transaction, &instance_id)?;
        let snapshot = InstanceSnapshot::with_observation(record, observation);
        transaction
            .execute(
                "UPDATE worktree_runtime_commands
                 SET status='succeeded', result_json=?2, updated_at=?3 WHERE request_id=?1",
                params![
                    request_id.as_str(),
                    json(&snapshot, "serialize command result")?,
                    timestamp(snapshot.projected.updated_at)
                ],
            )
            .map_err(sql_error("persist completed command"))?;
        transaction
            .commit()
            .map_err(sql_error("commit completed runtime transition"))?;
        if command.operation == "start" {
            active_starts.remove(&command.instance_id);
        }
        Ok(snapshot)
    }

    fn fail_transition(
        &self,
        request_id: &RequestId,
        expected: InstanceState,
        terminal_state: Option<InstanceState>,
        failure: StoredFailure,
        observation: Option<RuntimeObservation>,
    ) -> Result<(), RegistryError> {
        let mut active_starts = self.lock_active_starts()?;
        let mut connection = self.lock()?;
        let transaction = immediate(&mut connection, "record failed runtime transition")?;
        let command = load_pending_command(&transaction, request_id)?;
        let instance_id = InstanceId::new(command.instance_id.clone())
            .map_err(contract_error("decode command instance ID"))?;
        let record = load_required_record(&transaction, &instance_id)?;
        if record.state != expected {
            return Err(RegistryError::new(
                RegistryErrorKind::InvalidState,
                "runtime state changed before failure could be recorded",
            ));
        }
        let updated_at = observation
            .as_ref()
            .map(|value| value.observed_at)
            .unwrap_or_else(Utc::now);
        if let Some(state) = terminal_state {
            transaction
                .execute(
                    "UPDATE worktree_runtime_instances SET state=?2, updated_at=?3
                     WHERE instance_id=?1",
                    params![
                        instance_id.as_str(),
                        state_name(state),
                        timestamp(updated_at)
                    ],
                )
                .map_err(sql_error("persist failed runtime terminal state"))?;
        }
        if let Some(observation) = observation {
            insert_observation(
                &transaction,
                &instance_id,
                "transition_failed",
                &observation,
            )?;
        }
        transaction
            .execute(
                "UPDATE worktree_runtime_commands
                 SET status='failed', failure_json=?2, updated_at=?3 WHERE request_id=?1",
                params![
                    request_id.as_str(),
                    json(&failure, "serialize command failure")?,
                    timestamp(updated_at)
                ],
            )
            .map_err(sql_error("persist failed command"))?;
        transaction
            .commit()
            .map_err(sql_error("commit failed runtime transition"))?;
        if command.operation == "start" {
            active_starts.remove(&command.instance_id);
        }
        Ok(())
    }

    fn record_observation(
        &self,
        instance_id: &InstanceId,
        transition_kind: &str,
        observation: RuntimeObservation,
    ) -> Result<InstanceSnapshot, RegistryError> {
        let mut connection = self.lock()?;
        let transaction = immediate(&mut connection, "record runtime observation")?;
        let record = load_required_record(&transaction, instance_id)?;
        insert_observation(&transaction, instance_id, transition_kind, &observation)?;
        let snapshot = InstanceSnapshot::with_observation(record, observation);
        transaction
            .commit()
            .map_err(sql_error("commit runtime observation"))?;
        Ok(snapshot)
    }
}

fn begin_terminal_transition(
    connection: &mut Connection,
    request_id: &RequestId,
    fingerprint: &str,
    instance_id: &InstanceId,
    authority_hash: &str,
    operation: &'static str,
    pending_state: InstanceState,
    active_start: bool,
    recorded_at: DateTime<Utc>,
) -> Result<CommandStart, RegistryError> {
    let transaction = immediate(connection, "begin terminal runtime transition")?;
    if let Some(replay) = command_replay(
        &transaction,
        request_id,
        instance_id.as_str(),
        operation,
        fingerprint,
    )? {
        return finish_replay(transaction, replay);
    }
    let record = authorized_record(&transaction, instance_id, authority_hash)?;
    if let Some(active) = load_pending_lifecycle_command(&transaction, instance_id)? {
        if active.operation == "start" {
            if active_start {
                return Err(RegistryError::new(
                    RegistryErrorKind::OperationInProgress,
                    "the start transition is still executing for this instance",
                ));
            }
            if operation != "recover" {
                return Err(RegistryError::new(
                    RegistryErrorKind::Conflict,
                    "an abandoned start must be resolved with recover",
                ));
            }
            if record.state != InstanceState::LaunchPending {
                return Err(RegistryError::new(
                    RegistryErrorKind::Conflict,
                    "the abandoned start command conflicts with durable instance state",
                ));
            }
            fail_abandoned_start(&transaction, &active.request_id, recorded_at)?;
        } else {
            let (kind, message) = if active.operation == operation {
                (
                    RegistryErrorKind::OperationInProgress,
                    format!("the {operation} transition is already in progress for this instance"),
                )
            } else {
                (
                    RegistryErrorKind::Conflict,
                    format!(
                        "cannot begin {operation} while the {} transition is in progress",
                        active.operation
                    ),
                )
            };
            return Err(RegistryError::new(kind, message));
        }
    }
    if matches!(
        record.state,
        InstanceState::Prepared | InstanceState::Stopped | InstanceState::Recovered
    ) {
        let snapshot = InstanceSnapshot::from_record(record);
        insert_succeeded_command(
            &transaction,
            request_id,
            instance_id.as_str(),
            operation,
            fingerprint,
            &snapshot,
            recorded_at,
        )?;
        transaction
            .commit()
            .map_err(sql_error("commit terminal no-op"))?;
        return Ok(CommandStart::Noop(snapshot));
    }
    if record.owner_route.is_none() {
        return Err(RegistryError::new(
            RegistryErrorKind::Conflict,
            "active runtime state has no exact durable owner route",
        ));
    }
    transaction
        .execute(
            "UPDATE worktree_runtime_instances SET state=?2, updated_at=?3 WHERE instance_id=?1",
            params![
                instance_id.as_str(),
                state_name(pending_state),
                timestamp(recorded_at)
            ],
        )
        .map_err(sql_error("persist terminal transition reservation"))?;
    insert_pending_command(
        &transaction,
        request_id,
        instance_id.as_str(),
        operation,
        fingerprint,
        recorded_at,
    )?;
    let next = load_required_record(&transaction, instance_id)?;
    transaction
        .commit()
        .map_err(sql_error("commit terminal transition reservation"))?;
    Ok(CommandStart::Execute(next))
}

enum CommandReplay {
    Success(InstanceSnapshot),
    Failure(StoredFailure),
}

fn command_replay(
    transaction: &Transaction<'_>,
    request_id: &RequestId,
    instance_id: &str,
    operation: &str,
    fingerprint: &str,
) -> Result<Option<CommandReplay>, RegistryError> {
    let existing = transaction
        .query_row(
            "SELECT instance_id, operation, fingerprint, status, result_json, failure_json
             FROM worktree_runtime_commands WHERE request_id=?1",
            params![request_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(sql_error("read command idempotency"))?;
    let Some((stored_instance, stored_operation, stored_fingerprint, status, result, failure)) =
        existing
    else {
        return Ok(None);
    };
    if stored_instance != instance_id
        || stored_operation != operation
        || stored_fingerprint != fingerprint
    {
        return Err(RegistryError::new(
            RegistryErrorKind::IdempotencyConflict,
            "request ID was already used for different runtime semantics",
        ));
    }
    match status.as_str() {
        "pending" => Err(RegistryError::new(
            RegistryErrorKind::OperationInProgress,
            "the identical runtime operation is still pending",
        )),
        "succeeded" => Ok(Some(CommandReplay::Success(parse_json(
            result.as_deref().unwrap_or_default(),
            "decode idempotent runtime result",
        )?))),
        "failed" => Ok(Some(CommandReplay::Failure(parse_json(
            failure.as_deref().unwrap_or_default(),
            "decode idempotent runtime failure",
        )?))),
        _ => Err(RegistryError::new(
            RegistryErrorKind::Unavailable,
            "durable runtime command has an unsupported status",
        )),
    }
}

fn finish_replay(
    transaction: Transaction<'_>,
    replay: CommandReplay,
) -> Result<CommandStart, RegistryError> {
    transaction
        .commit()
        .map_err(sql_error("finish idempotent command read"))?;
    Ok(match replay {
        CommandReplay::Success(mut snapshot) => {
            snapshot.idempotent_replay = true;
            CommandStart::Replay(snapshot)
        }
        CommandReplay::Failure(failure) => CommandStart::ReplayFailure(failure),
    })
}

fn authorized_record(
    connection: &Connection,
    instance_id: &InstanceId,
    authority_hash: &str,
) -> Result<InstanceRecord, RegistryError> {
    let Some((record, stored_hash)) = load_record(connection, instance_id)? else {
        return Err(RegistryError::new(
            RegistryErrorKind::NotFound,
            format!("instance {instance_id} was not found"),
        ));
    };
    if stored_hash != authority_hash {
        return Err(RegistryError::new(
            RegistryErrorKind::Unauthorized,
            "instance authority does not match",
        ));
    }
    Ok(record)
}

fn load_required_record(
    connection: &Connection,
    instance_id: &InstanceId,
) -> Result<InstanceRecord, RegistryError> {
    load_record(connection, instance_id)?
        .map(|(record, _)| record)
        .ok_or_else(|| {
            RegistryError::new(
                RegistryErrorKind::NotFound,
                format!("instance {instance_id} was not found"),
            )
        })
}

fn load_record(
    connection: &Connection,
    instance_id: &InstanceId,
) -> Result<Option<(InstanceRecord, String)>, RegistryError> {
    connection
        .query_row(
            "SELECT identity_json, projection_json, authority_hash, state,
                    launch_id, job_name, created_at, updated_at
             FROM worktree_runtime_instances WHERE instance_id=?1",
            params![instance_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()
        .map_err(sql_error("load worktree instance"))?
        .map(
            |(
                identity,
                projection,
                authority_hash,
                state,
                launch_id,
                job_name,
                created_at,
                updated_at,
            )| {
                let owner_route = match (launch_id, job_name) {
                    (Some(launch_id), Some(job_name)) => Some(OwnerRoute {
                        launch_id: super::domain::LaunchId::new(launch_id)
                            .map_err(contract_error("decode launch ID"))?,
                        job_name,
                    }),
                    (None, None) => None,
                    _ => {
                        return Err(RegistryError::new(
                            RegistryErrorKind::Unavailable,
                            "durable instance has a partial owner route",
                        ))
                    }
                };
                Ok((
                    InstanceRecord {
                        identity: parse_json(&identity, "decode instance identity")?,
                        projection: parse_json(&projection, "decode instance projection")?,
                        state: parse_state(&state)?,
                        owner_route,
                        created_at: parse_timestamp(&created_at)?,
                        updated_at: parse_timestamp(&updated_at)?,
                    },
                    authority_hash,
                ))
            },
        )
        .transpose()
}

struct PendingCommand {
    instance_id: String,
    operation: String,
}

struct PendingLifecycleCommand {
    request_id: String,
    operation: String,
}

fn load_pending_lifecycle_command(
    transaction: &Transaction<'_>,
    instance_id: &InstanceId,
) -> Result<Option<PendingLifecycleCommand>, RegistryError> {
    transaction
        .query_row(
            "SELECT request_id, operation FROM worktree_runtime_commands
             WHERE instance_id=?1 AND status='pending'
               AND operation IN ('start', 'stop', 'recover')",
            params![instance_id.as_str()],
            |row| {
                Ok(PendingLifecycleCommand {
                    request_id: row.get(0)?,
                    operation: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(sql_error("load active lifecycle runtime command"))
}

fn fail_abandoned_start(
    transaction: &Transaction<'_>,
    request_id: &str,
    recorded_at: DateTime<Utc>,
) -> Result<(), RegistryError> {
    let failure = StoredFailure {
        kind: "ownership_ambiguous".into(),
        message: "start was abandoned by the previous runtime execution and superseded by recovery"
            .into(),
    };
    let changed = transaction
        .execute(
            "UPDATE worktree_runtime_commands
             SET status='failed', failure_json=?2, updated_at=?3
             WHERE request_id=?1 AND operation='start' AND status='pending'",
            params![
                request_id,
                json(&failure, "serialize abandoned start failure")?,
                timestamp(recorded_at)
            ],
        )
        .map_err(sql_error("fail abandoned start command"))?;
    if changed != 1 {
        return Err(RegistryError::new(
            RegistryErrorKind::Conflict,
            "the abandoned start command changed before recovery reserved the instance",
        ));
    }
    Ok(())
}

fn load_pending_command(
    transaction: &Transaction<'_>,
    request_id: &RequestId,
) -> Result<PendingCommand, RegistryError> {
    transaction
        .query_row(
            "SELECT instance_id, operation, status
             FROM worktree_runtime_commands WHERE request_id=?1",
            params![request_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(sql_error("load pending runtime command"))?
        .ok_or_else(|| {
            RegistryError::new(
                RegistryErrorKind::NotFound,
                "pending runtime command was not found",
            )
        })
        .and_then(|(instance_id, operation, status)| {
            if status == "pending" {
                Ok(PendingCommand {
                    instance_id,
                    operation,
                })
            } else {
                Err(RegistryError::new(
                    RegistryErrorKind::InvalidState,
                    "runtime command is already terminal",
                ))
            }
        })
}

fn insert_pending_command(
    transaction: &Transaction<'_>,
    request_id: &RequestId,
    instance_id: &str,
    operation: &str,
    fingerprint: &str,
    recorded_at: DateTime<Utc>,
) -> Result<(), RegistryError> {
    transaction
        .execute(
            "INSERT INTO worktree_runtime_commands
             (request_id, instance_id, operation, fingerprint, status, result_json, failure_json,
              recorded_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', NULL, NULL, ?5, ?5)",
            params![
                request_id.as_str(),
                instance_id,
                operation,
                fingerprint,
                timestamp(recorded_at)
            ],
        )
        .map_err(sql_error("insert pending runtime command"))?;
    Ok(())
}

fn insert_succeeded_command(
    transaction: &Transaction<'_>,
    request_id: &RequestId,
    instance_id: &str,
    operation: &str,
    fingerprint: &str,
    snapshot: &InstanceSnapshot,
    recorded_at: DateTime<Utc>,
) -> Result<(), RegistryError> {
    transaction
        .execute(
            "INSERT INTO worktree_runtime_commands
             (request_id, instance_id, operation, fingerprint, status, result_json, failure_json,
              recorded_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'succeeded', ?5, NULL, ?6, ?6)",
            params![
                request_id.as_str(),
                instance_id,
                operation,
                fingerprint,
                json(snapshot, "serialize runtime command result")?,
                timestamp(recorded_at)
            ],
        )
        .map_err(sql_error("insert completed runtime command"))?;
    Ok(())
}

fn insert_observation(
    transaction: &Transaction<'_>,
    instance_id: &InstanceId,
    transition_kind: &str,
    observation: &RuntimeObservation,
) -> Result<(), RegistryError> {
    transaction
        .execute(
            "INSERT INTO worktree_runtime_observations
             (instance_id, transition_kind, observation_json, observed_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                instance_id.as_str(),
                transition_kind,
                json(observation, "serialize runtime observation")?,
                timestamp(observation.observed_at)
            ],
        )
        .map_err(sql_error("record runtime observation"))?;
    Ok(())
}

fn immediate<'a>(
    connection: &'a mut Connection,
    operation: &'static str,
) -> Result<Transaction<'a>, RegistryError> {
    connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error(operation))
}

fn state_name(state: InstanceState) -> &'static str {
    match state {
        InstanceState::Prepared => "prepared",
        InstanceState::LaunchPending => "launch_pending",
        InstanceState::Running => "running",
        InstanceState::StopPending => "stop_pending",
        InstanceState::Stopped => "stopped",
        InstanceState::RecoveryPending => "recovery_pending",
        InstanceState::Recovered => "recovered",
    }
}

fn parse_state(state: &str) -> Result<InstanceState, RegistryError> {
    match state {
        "prepared" => Ok(InstanceState::Prepared),
        "launch_pending" => Ok(InstanceState::LaunchPending),
        "running" => Ok(InstanceState::Running),
        "stop_pending" => Ok(InstanceState::StopPending),
        "stopped" => Ok(InstanceState::Stopped),
        "recovery_pending" => Ok(InstanceState::RecoveryPending),
        "recovered" => Ok(InstanceState::Recovered),
        _ => Err(RegistryError::new(
            RegistryErrorKind::Unavailable,
            "durable instance has an unsupported state",
        )),
    }
}

fn json(value: &impl Serialize, operation: &'static str) -> Result<String, RegistryError> {
    serde_json::to_string(value).map_err(|error| {
        RegistryError::new(
            RegistryErrorKind::Unavailable,
            format!("{operation}: {error}"),
        )
    })
}

fn parse_json<T: for<'de> Deserialize<'de>>(
    value: &str,
    operation: &'static str,
) -> Result<T, RegistryError> {
    serde_json::from_str(value).map_err(|error| {
        RegistryError::new(
            RegistryErrorKind::Unavailable,
            format!("{operation}: {error}"),
        )
    })
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, RegistryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            RegistryError::new(
                RegistryErrorKind::Unavailable,
                format!("decode registry timestamp: {error}"),
            )
        })
}

fn map_prepare_insert_error(error: rusqlite::Error) -> RegistryError {
    RegistryError::new(
        RegistryErrorKind::Conflict,
        format!("persist prepared instance: {error}"),
    )
}

fn map_port_lease_error(error: rusqlite::Error) -> RegistryError {
    RegistryError::new(
        RegistryErrorKind::PortLeaseConflict,
        format!("claim projected port lease: {error}"),
    )
}

fn sql_error(operation: &'static str) -> impl FnOnce(rusqlite::Error) -> RegistryError {
    move |error| {
        RegistryError::new(
            RegistryErrorKind::Unavailable,
            format!("{operation}: {error}"),
        )
    }
}

fn contract_error(
    operation: &'static str,
) -> impl FnOnce(super::domain::RuntimeContractError) -> RegistryError {
    move |error| RegistryError::new(RegistryErrorKind::Conflict, format!("{operation}: {error}"))
}
