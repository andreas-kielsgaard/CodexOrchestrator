use super::domain::*;
use crate::{
    execution_configuration::CapabilityProfile,
    execution_devices::ExecutionDeviceDto,
    repository_catalog::{device_locations::RepositoryDeviceLocation, RegisteredRepository},
};
use orchid_engine::protocol::WorktreeInstance;

pub(super) fn configured_devices(
    devices: &[ExecutionDeviceDto],
    profiles: &[CapabilityProfile],
) -> Vec<ConfiguredExecutionDevice> {
    devices
        .iter()
        .map(|device| ConfiguredExecutionDevice {
            device_id: device.device_id.clone(),
            device_name: device.display_name.clone(),
            profiles: profiles
                .iter()
                .filter(|profile| profile.execution.device_id == device.device_id)
                .map(profile_identity)
                .collect(),
        })
        .collect()
}

pub(super) fn targets(
    profiles: &[CapabilityProfile],
    repository: &RegisteredRepository,
    locations: &[RepositoryDeviceLocation],
    branch: &str,
    mut discover: impl FnMut(
        &ExecutionBinding,
        &str,
        Option<&str>,
    ) -> Result<Vec<WorktreeInstance>, String>,
) -> Vec<DeviceWorktreeTargets> {
    grouped_profiles(profiles)
        .into_iter()
        .map(|(execution, profiles)| DeviceWorktreeTargets {
            device_id: execution.device_id.clone(),
            device_name: execution.device_name.clone(),
            profiles: profiles
                .into_iter()
                .map(|profile| {
                    profile_targets(profile, repository, locations, Some(branch), &mut discover)
                })
                .collect(),
        })
        .collect()
}

pub(super) fn worktree_choices(
    profiles: &[CapabilityProfile],
    repositories: &[RegisteredRepository],
    locations: &[RepositoryDeviceLocation],
    scope: &WorktreeChoiceScope,
    mut discover: impl FnMut(
        &ExecutionBinding,
        &str,
        Option<&str>,
    ) -> Result<Vec<WorktreeInstance>, String>,
) -> Vec<RepositoryWorktreeChoices> {
    let profiles: Vec<_> = profiles
        .iter()
        .filter(|profile| match scope {
            WorktreeChoiceScope::Local => !profile.execution.is_remote(),
            WorktreeChoiceScope::Device { device_id } => profile.execution.device_id == *device_id,
        })
        .collect();
    if profiles.is_empty() {
        return Vec::new();
    }
    repositories
        .iter()
        .map(|repository| RepositoryWorktreeChoices {
            repository_id: repository.repository_id.clone(),
            repository_name: repository.label.clone(),
            profiles: profiles
                .iter()
                .map(|profile| profile_targets(profile, repository, locations, None, &mut discover))
                .collect(),
        })
        .collect()
}

fn profile_targets(
    profile: &CapabilityProfile,
    repository: &RegisteredRepository,
    locations: &[RepositoryDeviceLocation],
    branch: Option<&str>,
    discover: &mut impl FnMut(
        &ExecutionBinding,
        &str,
        Option<&str>,
    ) -> Result<Vec<WorktreeInstance>, String>,
) -> ProfileWorktreeTargets {
    let root = if profile.execution.is_remote() {
        locations
            .iter()
            .find(|location| {
                location.repository_id == repository.repository_id
                    && location.device_id == profile.execution.device_id
            })
            .map(|location| location.repository_root.clone())
            .ok_or_else(|| "No repository path is configured on this device".to_owned())
    } else {
        Ok(repository.anchor_root.to_string_lossy().into_owned())
    };
    let result = root.and_then(|root| discover(&profile.execution, &root, branch));
    let (instances, error) = match result {
        Ok(instances) => (
            instances
                .into_iter()
                .filter_map(|instance| {
                    let branch_ref = instance.branch_ref?;
                    if branch.is_some_and(|expected| expected != branch_ref) {
                        return None;
                    }
                    Some(TargetWorktree {
                        worktree_id: instance.handle,
                        path: instance.path,
                        head: instance.head,
                        dirty: instance.dirty,
                        head_committed_at: instance.head_committed_at,
                        branch_ref,
                        sister_lock: None,
                        is_sister: None,
                    })
                })
                .collect(),
            None,
        ),
        Err(error) => (Vec::new(), Some(error)),
    };
    ProfileWorktreeTargets {
        profile: profile_identity(profile),
        instances,
        error,
        sister_lock: None,
    }
}

fn profile_identity(profile: &CapabilityProfile) -> ExecutionTargetProfile {
    ExecutionTargetProfile {
        capability_profile_id: profile.capability_profile_id.clone(),
        capability_profile_revision: profile.revision,
        capability_profile_name: profile.name.clone(),
        execution: profile.execution.clone(),
    }
}

fn grouped_profiles(
    profiles: &[CapabilityProfile],
) -> Vec<(&ExecutionBinding, Vec<&CapabilityProfile>)> {
    let mut groups: Vec<(&ExecutionBinding, Vec<&CapabilityProfile>)> = Vec::new();
    for profile in profiles {
        if let Some((_, entries)) = groups
            .iter_mut()
            .find(|(execution, _)| execution.device_id == profile.execution.device_id)
        {
            entries.push(profile);
        } else {
            groups.push((&profile.execution, vec![profile]));
        }
    }
    groups
}

#[cfg(test)]
#[path = "inventory_tests.rs"]
mod tests;
