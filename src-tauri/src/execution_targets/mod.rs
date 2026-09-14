pub(crate) mod domain;
pub(crate) mod endpoints;
mod remote_runtime;
mod ssh_connection;
pub(crate) mod transport;

use crate::{
    execution_configuration::CapabilityProfileService,
    repository_catalog::{
        device_locations::RepositoryDeviceLocations, repository::SqliteRepositoryCatalogRepository,
    },
};
use domain::*;
use endpoints::ExecutionEndpoints;
use std::sync::Arc;

pub(crate) struct ExecutionTargetService {
    pub(crate) endpoints: Arc<ExecutionEndpoints>,
    profiles: Arc<CapabilityProfileService>,
    repositories: SqliteRepositoryCatalogRepository,
    pub(crate) locations: RepositoryDeviceLocations,
}

impl ExecutionTargetService {
    pub(crate) fn new(
        database: Arc<crate::persistence::ActiveDatabase>,
        endpoints: Arc<ExecutionEndpoints>,
        profiles: Arc<CapabilityProfileService>,
    ) -> Self {
        Self {
            endpoints,
            profiles,
            repositories: SqliteRepositoryCatalogRepository::new(database.clone()),
            locations: RepositoryDeviceLocations(database),
        }
    }
    pub(crate) fn targets(
        &self,
        repository_id: &str,
        branch: &str,
    ) -> Result<Vec<DeviceWorktreeTargets>, String> {
        let repository = self
            .repositories
            .find(repository_id)?
            .ok_or("Registered repository was not found")?;
        let locations = self.locations.list()?;
        let mut devices: Vec<DeviceWorktreeTargets> = Vec::new();
        for profile in self.profiles.list().map_err(|e| e.to_string())? {
            let execution = profile.execution.clone();
            let root = if execution.is_remote() {
                locations
                    .iter()
                    .find(|l| {
                        l.repository_id == repository_id && l.device_id == execution.device_id
                    })
                    .map(|l| l.repository_root.clone())
                    .ok_or_else(|| "No repository path is configured on this device".to_owned())
            } else {
                Ok(repository.anchor_root.to_string_lossy().into_owned())
            };
            let discovery =
                root.and_then(|root| self.endpoints.worktrees(&execution, &root, branch));
            let (instances, error) = match discovery {
                Ok(items) => (items, None),
                Err(error) => (Vec::new(), Some(error)),
            };
            let entry = ProfileWorktreeTargets {
                capability_profile_id: profile.capability_profile_id,
                capability_profile_revision: profile.revision,
                capability_profile_name: profile.name,
                execution: execution.clone(),
                instances,
                error,
            };
            if let Some(device) = devices
                .iter_mut()
                .find(|device| device.device_id == execution.device_id)
            {
                device.profiles.push(entry);
            } else {
                devices.push(DeviceWorktreeTargets {
                    device_id: execution.device_id,
                    device_name: execution.device_name,
                    profiles: vec![entry],
                });
            }
        }
        Ok(devices)
    }
}
