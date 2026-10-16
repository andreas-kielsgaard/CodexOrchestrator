use crate::{
    agent_sessions::client::{DeviceCommandTransport, HostCommand},
    execution_targets::ExecutionTargetService,
    persistence::ActiveDatabase,
};
use serde_json::Value;
use std::sync::Arc;

/// Uses the same addressed transport as session commands; it never picks a destination.
pub struct DeviceOperationsClient(pub Arc<dyn DeviceCommandTransport>);
impl DeviceOperationsClient {
    pub fn execute(&self, device: &str, method: &str, input: Value) -> Result<Value, String> {
        self.0.execute(
            device,
            HostCommand {
                method: format!("device.{method}"),
                input,
            },
        )
    }
}
fn field<T: serde::de::DeserializeOwned>(value: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(value.get(name).cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("{name}: {e}"))
}
fn encode<T: serde::Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

pub fn dispatch(
    device: &str,
    database: Arc<ActiveDatabase>,
    targets: &ExecutionTargetService,
    command: HostCommand,
) -> Result<Value, String> {
    let input = command.input;
    let path = || field::<String>(&input, "path");
    match command.method.as_str() {
        "device.description" => {
            let providers = targets
                .endpoints
                .provider_setups()?
                .into_iter()
                .map(|setup| crate::synchronization::ProviderObservation {
                    provider: setup.provider,
                    configuration_ref: setup.configuration_id,
                    configured: true,
                    observed_at: chrono::Utc::now(),
                    state: serde_json::to_value(setup.state)
                        .ok()
                        .and_then(|s| s.as_str().map(str::to_owned))
                        .unwrap_or_else(|| "unknown".into()),
                    selected: setup.selected,
                })
                .collect();
            encode(crate::synchronization::DeviceDescription {
                id: device.into(),
                name: std::env::var("ORCHID_DEVICE_NAME")
                    .or_else(|_| std::env::var("COMPUTERNAME"))
                    .or_else(|_| std::env::var("HOSTNAME"))
                    .unwrap_or_else(|_| device.into()),
                providers,
            })
        }
        "device.profiles" => encode(targets.devices()?),
        "device.saveProfile" => {
            let profile: crate::execution_configuration::CapabilityProfile =
                field(&input, "profile")?;
            profile.validate()?;
            database
                .write("save host capability profile", |db| {
                    crate::execution_configuration::definitions::import(
                        db,
                        &profile.capability_profile_id,
                        &serde_json::to_value(&profile).map_err(|e| e.to_string())?,
                    )
                })
                .map_err(|e| e.to_string())?;
            if field::<Option<bool>>(&input, "default")?.unwrap_or(false) {
                crate::execution_configuration::CapabilityProfileService::new(Arc::new(crate::execution_configuration::SqliteCapabilityProfileRepository::from_database(database))).set_default_profile(&profile.capability_profile_id).map_err(|e|e.to_string())?;
            }
            encode(profile)
        }
        "device.repositories" => {
            let catalog = crate::repository_catalog::RepositoryCatalog::new(database);
            encode(
                catalog
                    .list_registered()?
                    .iter()
                    .map(crate::repository_catalog::RegisteredRepositoryView::from)
                    .collect::<Vec<_>>(),
            )
        }
        "device.registerRepository" => {
            let catalog = crate::repository_catalog::RepositoryCatalog::new(database);
            encode(
                if let Some(id) = field::<Option<String>>(&input, "repositoryId")? {
                    catalog.register_directory_as(path()?.into(), &id)?
                } else {
                    catalog.register_directory(path()?.into())?
                },
            )
        }
        "device.repositoryRoot" => encode(
            crate::repository_catalog::RepositoryCatalog::new(database)
                .resolve_verified(&field::<String>(&input, "repositoryId")?)?
                .0
                .anchor_root
                .to_string_lossy()
                .into_owned(),
        ),
        "device.targets" => encode(targets.targets(
            &field::<String>(&input, "repositoryId")?,
            &field::<String>(&input, "branchRef")?,
        )?),
        "device.worktreeChoices" => encode(targets.worktree_choices(&field(&input, "scope")?)?),
        "device.worktrees" => encode(
            orchid_engine::host::list_worktrees(
                &path()?,
                field::<Option<String>>(&input, "branch")?.as_deref(),
            )
            .map_err(|e| e.to_string())?,
        ),
        "device.inspectWorktree" => encode(
            orchid_engine::workspaces::inspect_worktree(
                &path()?,
                field::<Option<String>>(&input, "compareToHead")?.as_deref(),
            )
            .map_err(|e| e.to_string())?,
        ),
        "device.captureSnapshot" => encode(
            orchid_engine::workspaces::capture_worktree_snapshot(
                &path()?,
                field::<Option<String>>(&input, "destinationHead")?.as_deref(),
                &field::<String>(&input, "snapshotId")?,
            )
            .map_err(|e| e.to_string())?,
        ),
        "device.applySnapshot" => encode(
            orchid_engine::workspaces::apply_worktree_snapshot(
                &path()?,
                &field(&input, "snapshot")?,
            )
            .map_err(|e| e.to_string())?,
        ),
        "device.materializeWorktree" => encode(
            orchid_engine::workspaces::materialize_worktree(
                &path()?,
                &field::<String>(&input, "branch")?,
                &field::<String>(&input, "commit")?,
                &field::<String>(&input, "instanceId")?,
            )
            .map_err(|e| e.to_string())?,
        ),
        "device.branchCommit" => encode(
            orchid_engine::workspaces::local_branch_commit(
                &path()?,
                &field::<String>(&input, "branch")?,
            )
            .map_err(|e| e.to_string())?,
        ),
        "device.auxiliaryWorkspace" => {
            let mut binding: crate::execution_targets::domain::ExecutionBinding =
                field(&input, "execution")?;
            binding.device_id = "local".into();
            binding.connection = crate::execution_targets::domain::ExecutionConnection::Local;
            encode(
                targets
                    .endpoints
                    .auxiliary_workspace(&binding, &field::<String>(&input, "sessionId")?)?,
            )
        }
        "device.capabilities" => {
            let mut binding: crate::execution_targets::domain::ExecutionBinding =
                field(&input, "execution")?;
            if binding.device_id != "local" && binding.device_id != device {
                return Err("Capability query targets another device".into());
            }
            binding.connection = crate::execution_targets::domain::ExecutionConnection::Local;
            binding.device_id = "local".into();
            encode(targets.describe_runtime(
                &binding,
                field::<Option<String>>(&input, "path")?.as_deref(),
            )?)
        }
        _ => Err(format!("Unknown device operation: {}", command.method)),
    }
}
