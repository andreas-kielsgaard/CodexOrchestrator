pub(crate) mod domain;
pub(crate) mod endpoints;
mod inventory;
mod preparation;
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
        Ok(inventory::targets(
            &self.profiles.list().map_err(|e| e.to_string())?,
            &repository,
            &self.locations.list()?,
            branch,
            |execution, root, branch| self.endpoints.worktrees(execution, root, branch),
        ))
    }

    pub(crate) fn devices(&self) -> Result<Vec<ConfiguredExecutionDevice>, String> {
        Ok(inventory::configured_devices(
            &self.profiles.list().map_err(|e| e.to_string())?,
        ))
    }

    pub(crate) fn worktree_choices(
        &self,
        scope: &WorktreeChoiceScope,
    ) -> Result<Vec<RepositoryWorktreeChoices>, String> {
        Ok(inventory::worktree_choices(
            &self.profiles.list().map_err(|e| e.to_string())?,
            &self.repositories.list()?,
            &self.locations.list()?,
            scope,
            |execution, root, branch| self.endpoints.worktrees(execution, root, branch),
        ))
    }
}
