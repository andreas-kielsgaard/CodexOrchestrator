use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    path::{Component, PathBuf},
};

macro_rules! bounded_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn new(value: impl Into<String>) -> Result<Self, RuntimeContractError> {
                let value = value.into();
                let valid = !value.is_empty()
                    && value.len() <= 96
                    && value.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    });
                if !valid {
                    return Err(RuntimeContractError::new(format!(
                        "{} must be a 1-96 character ASCII identifier",
                        $label
                    )));
                }
                Ok(Self(value))
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

bounded_id!(InstanceId, "instance ID");
bounded_id!(BuildId, "build ID");
bounded_id!(SessionLink, "session link");
bounded_id!(LaunchId, "launch ID");
bounded_id!(RequestId, "request ID");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceIdentity {
    pub(crate) instance_id: InstanceId,
    #[serde(default = "default_review_name")]
    pub(crate) review_name: String,
    pub(crate) worktree_path: PathBuf,
    pub(crate) git_commit: String,
    pub(crate) source_fingerprint: String,
    pub(crate) build_id: BuildId,
    pub(crate) session_link: SessionLink,
}

impl InstanceIdentity {
    pub(crate) fn validate(&self) -> Result<(), RuntimeContractError> {
        if self.review_name.is_empty()
            || self.review_name.len() > 96
            || !self.review_name.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'.' | b'_' | b'-')
            })
        {
            return Err(RuntimeContractError::new(
                "review name must be a bounded human-readable label",
            ));
        }
        require_absolute(&self.worktree_path, "worktree path")?;
        if self.git_commit.len() < 7
            || self.git_commit.len() > 64
            || !self.git_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(RuntimeContractError::new(
                "git commit must be a 7-64 character hexadecimal identifier",
            ));
        }
        if self.source_fingerprint.len() != 64
            || !self
                .source_fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(RuntimeContractError::new(
                "source fingerprint must be a 64-character hexadecimal identifier",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheProjection {
    pub(crate) node_key: String,
    pub(crate) node_path: PathBuf,
    pub(crate) node_reuse: CacheReuse,
    pub(crate) rust_key: String,
    pub(crate) rust_path: PathBuf,
    pub(crate) rust_reuse: CacheReuse,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CacheReuse {
    SharedKeyed,
    IsolatedFallback,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimePaths {
    pub(crate) instance_root: PathBuf,
    pub(crate) frontend_dist: PathBuf,
    pub(crate) cargo_target: PathBuf,
    pub(crate) app_data: PathBuf,
    pub(crate) credentials_home: PathBuf,
    pub(crate) temp: PathBuf,
    pub(crate) logs: PathBuf,
    pub(crate) screenshots: PathBuf,
    pub(crate) recordings: PathBuf,
    pub(crate) evidence: PathBuf,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PortProjection {
    pub(crate) vite: u16,
    pub(crate) status: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceProjection {
    pub(crate) caches: CacheProjection,
    pub(crate) paths: RuntimePaths,
    pub(crate) ports: PortProjection,
}

impl InstanceProjection {
    pub(crate) fn validate(&self) -> Result<(), RuntimeContractError> {
        if self.caches.node_key.trim().is_empty() || self.caches.rust_key.trim().is_empty() {
            return Err(RuntimeContractError::new("cache keys must not be blank"));
        }
        for (path, label) in [
            (&self.caches.node_path, "node cache path"),
            (&self.caches.rust_path, "Rust cache path"),
            (&self.paths.instance_root, "instance root"),
            (&self.paths.frontend_dist, "frontend output path"),
            (&self.paths.cargo_target, "Cargo target path"),
            (&self.paths.app_data, "application data path"),
            (&self.paths.credentials_home, "credentials home"),
            (&self.paths.temp, "temporary files path"),
            (&self.paths.logs, "logs path"),
            (&self.paths.screenshots, "screenshots path"),
            (&self.paths.recordings, "recordings path"),
            (&self.paths.evidence, "evidence path"),
        ] {
            require_absolute(path, label)?;
        }
        for path in [
            &self.paths.frontend_dist,
            &self.paths.cargo_target,
            &self.paths.app_data,
            &self.paths.credentials_home,
            &self.paths.temp,
            &self.paths.logs,
            &self.paths.screenshots,
            &self.paths.recordings,
            &self.paths.evidence,
        ] {
            if !path.starts_with(&self.paths.instance_root) {
                return Err(RuntimeContractError::new(
                    "instance paths must remain below the instance root",
                ));
            }
        }
        for (path, reuse, label) in [
            (&self.caches.node_path, self.caches.node_reuse, "node cache"),
            (&self.caches.rust_path, self.caches.rust_reuse, "Rust cache"),
        ] {
            match reuse {
                CacheReuse::IsolatedFallback if !path.starts_with(&self.paths.instance_root) => {
                    return Err(RuntimeContractError::new(format!(
                        "{label} marked as isolated must remain below the instance root"
                    )));
                }
                CacheReuse::SharedKeyed if path.starts_with(&self.paths.instance_root) => {
                    return Err(RuntimeContractError::new(format!(
                        "{label} marked as shared must remain outside the instance root"
                    )));
                }
                CacheReuse::SharedKeyed | CacheReuse::IsolatedFallback => {}
            }
        }
        if self.ports.vite == self.ports.status {
            return Err(RuntimeContractError::new(
                "Vite and status ports must be distinct",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InstanceState {
    Prepared,
    LaunchPending,
    Running,
    StopPending,
    Stopped,
    RecoveryPending,
    Recovered,
}

impl InstanceState {
    pub(crate) fn has_projected_owner(self) -> bool {
        matches!(
            self,
            Self::LaunchPending | Self::Running | Self::StopPending | Self::RecoveryPending
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OwnerRoute {
    pub(crate) launch_id: LaunchId,
    pub(crate) job_name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceRecord {
    pub(crate) identity: InstanceIdentity,
    pub(crate) projection: InstanceProjection,
    pub(crate) state: InstanceState,
    pub(crate) owner_route: Option<OwnerRoute>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProcessRole {
    Vite,
    Status,
    Tauri,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OwnedProcessLaunch {
    pub(crate) role: ProcessRole,
    pub(crate) program: PathBuf,
    pub(crate) arguments: Vec<String>,
    pub(crate) working_directory: PathBuf,
    /// This complete environment is supplied to the child. Ambient credentials are not inherited.
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) log_path: PathBuf,
}

impl OwnedProcessLaunch {
    pub(crate) fn validate(&self) -> Result<(), RuntimeContractError> {
        require_absolute(&self.program, "launch program")?;
        require_absolute(&self.working_directory, "launch working directory")?;
        require_absolute(&self.log_path, "launch log")?;
        if self.program.to_string_lossy().contains('\0')
            || self.working_directory.to_string_lossy().contains('\0')
        {
            return Err(RuntimeContractError::new(
                "launch paths must not contain NUL",
            ));
        }
        if self
            .arguments
            .iter()
            .any(|argument| argument.encode_utf16().any(|unit| unit == 0))
        {
            return Err(RuntimeContractError::new(
                "launch arguments must not contain NUL",
            ));
        }
        let mut environment_names = BTreeSet::new();
        if self.environment.iter().any(|(name, value)| {
            name.is_empty()
                || name.contains('=')
                || name.encode_utf16().any(|unit| unit == 0)
                || value.encode_utf16().any(|unit| unit == 0)
                || !environment_names.insert(name.to_uppercase())
        }) {
            return Err(RuntimeContractError::new(
                "launch environment contains an invalid name or value",
            ));
        }
        Ok(())
    }
}

fn default_review_name() -> String {
    "Isolated review".into()
}

pub(crate) fn validate_launches(
    launches: &[OwnedProcessLaunch],
) -> Result<(), RuntimeContractError> {
    let roles = launches
        .iter()
        .map(|launch| {
            launch.validate()?;
            Ok(launch.role)
        })
        .collect::<Result<BTreeSet<_>, RuntimeContractError>>()?;
    if launches.len() != 3
        || roles != BTreeSet::from([ProcessRole::Vite, ProcessRole::Status, ProcessRole::Tauri])
    {
        return Err(RuntimeContractError::new(
            "one Vite, one status, and one Tauri launch are required",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub(crate) enum OwnerObservation {
    Absent,
    Owned { active_processes: u32 },
}

impl OwnerObservation {
    pub(crate) fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Owned {
                active_processes
            } if *active_processes > 0
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointObservation {
    pub(crate) port: u16,
    pub(crate) reachable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HealthObservation {
    pub(crate) vite: EndpointObservation,
    pub(crate) status: EndpointObservation,
}

impl HealthObservation {
    pub(crate) fn healthy(&self) -> bool {
        self.vite.reachable && self.status.reachable
    }

    pub(crate) fn all_closed(&self) -> bool {
        !self.vite.reachable && !self.status.reachable
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeObservation {
    pub(crate) owner: OwnerObservation,
    pub(crate) health: HealthObservation,
    pub(crate) observed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceSnapshot {
    pub(crate) projected: InstanceRecord,
    pub(crate) observed: Option<RuntimeObservation>,
    pub(crate) stale: bool,
    pub(crate) idempotent_replay: bool,
}

impl InstanceSnapshot {
    pub(crate) fn from_record(record: InstanceRecord) -> Self {
        Self {
            projected: record,
            observed: None,
            stale: false,
            idempotent_replay: false,
        }
    }

    pub(crate) fn with_observation(
        record: InstanceRecord,
        observation: RuntimeObservation,
    ) -> Self {
        let stale = if record.state.has_projected_owner() {
            !observation.owner.is_active() || !observation.health.healthy()
        } else {
            observation.owner.is_active() || !observation.health.all_closed()
        };
        Self {
            projected: record,
            observed: Some(observation),
            stale,
            idempotent_replay: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AuthoritySecret(String);

impl AuthoritySecret {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, RuntimeContractError> {
        let value = value.into();
        if value.len() < 16 || value.len() > 512 {
            return Err(RuntimeContractError::new(
                "authority secret must contain 16-512 characters",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeContractError {
    message: String,
}

impl RuntimeContractError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RuntimeContractError {}

fn require_absolute(path: &PathBuf, label: &str) -> Result<(), RuntimeContractError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(RuntimeContractError::new(format!(
            "{label} must be absolute and normalized"
        )));
    }
    Ok(())
}
