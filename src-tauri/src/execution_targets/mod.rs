pub(crate) mod domain;
pub(crate) mod endpoints;
mod inventory;
mod preparation;
mod remote_runtime;
mod ssh_connection;
pub(crate) mod sisters;
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
    pub(crate) sisters: sisters::SisterWorktreeStore,
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
            locations: RepositoryDeviceLocations(database.clone()),
            sisters: sisters::SisterWorktreeStore::new(database),
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
        let mut targets = inventory::targets(
            &self.profiles.list().map_err(|e| e.to_string())?,
            &repository,
            &self.locations.list()?,
            branch,
            |execution, root, branch| self.endpoints.worktrees(execution, root, branch),
        );
        self.annotate_locks(&mut targets, repository_id, Some(branch))?;
        Ok(targets)
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
        let mut choices = inventory::worktree_choices(
            &self.profiles.list().map_err(|e| e.to_string())?,
            &self.repositories.list()?,
            &self.locations.list()?,
            scope,
            |execution, root, branch| self.endpoints.worktrees(execution, root, branch),
        );
        for repository in &mut choices {
            self.annotate_choice_locks(repository)?;
        }
        Ok(choices)
    }

    fn annotate_locks(
        &self,
        devices: &mut [DeviceWorktreeTargets],
        repository_id: &str,
        branch_ref: Option<&str>,
    ) -> Result<(), String> {
        for device in devices {
            for profile in &mut device.profiles {
                let profile_lock = self.sisters.lock_for(repository_id, branch_ref.unwrap_or_default())?;
                profile.sister_lock = profile_lock.clone();
                for worktree in &mut profile.instances {
                    let branch = branch_ref.unwrap_or(&worktree.branch_ref);
                    let lock = if branch_ref.is_some() {
                        profile_lock.clone()
                    } else {
                        self.sisters.lock_for(repository_id, branch)?
                    };
                    worktree.is_sister = lock
                        .as_ref()
                        .map(|lock| self.sisters.contains_instance(
                            &lock.sister_group_id,
                            &device.device_id,
                            &worktree.worktree_id,
                        ))
                        .transpose()?;
                    worktree.sister_lock = lock;
                }
            }
        }
        Ok(())
    }

    fn annotate_choice_locks(&self, choices: &mut RepositoryWorktreeChoices) -> Result<(), String> {
        for profile in &mut choices.profiles {
            for worktree in &mut profile.instances {
                let lock = self
                    .sisters
                    .lock_for(&choices.repository_id, &worktree.branch_ref)?;
                worktree.is_sister = lock
                    .as_ref()
                    .map(|lock| self.sisters.contains_instance(
                        &lock.sister_group_id,
                        &profile.profile.execution.device_id,
                        &worktree.worktree_id,
                    ))
                    .transpose()?;
                worktree.sister_lock = lock;
            }
        }
        Ok(())
    }
}
