pub(crate) mod domain;
pub(crate) mod endpoints;
mod inventory;
mod preparation;
mod remote_runtime;
pub(crate) mod sisters;
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
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

pub(crate) struct ExecutionTargetService {
    pub(crate) endpoints: Arc<ExecutionEndpoints>,
    pub(crate) devices: Arc<crate::execution_devices::ExecutionDeviceService>,
    profiles: Arc<CapabilityProfileService>,
    repositories: SqliteRepositoryCatalogRepository,
    pub(crate) locations: RepositoryDeviceLocations,
    pub(crate) sisters: sisters::SisterWorktreeStore,
    readiness_gates: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl ExecutionTargetService {
    pub(crate) fn new(
        database: Arc<crate::persistence::ActiveDatabase>,
        endpoints: Arc<ExecutionEndpoints>,
        profiles: Arc<CapabilityProfileService>,
    ) -> Self {
        let devices = Arc::new(crate::execution_devices::ExecutionDeviceService::new(
            database.clone(),
        ));
        Self {
            endpoints,
            devices,
            profiles,
            repositories: SqliteRepositoryCatalogRepository::new(database.clone()),
            locations: RepositoryDeviceLocations(database.clone()),
            sisters: sisters::SisterWorktreeStore::new(database),
            readiness_gates: Mutex::new(HashMap::new()),
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
            &self.devices.list()?,
            &self.profiles.list().map_err(|e| e.to_string())?,
        ))
    }

    pub(crate) fn synchronize_devices(&self) -> Result<(), String> {
        self.devices.list().map(|_| ())
    }

    pub(crate) fn ensure_ready(&self, binding: &ExecutionBinding) -> Result<(), String> {
        if !binding.is_remote() {
            return Ok(());
        }
        ensure_device_ready(
            &self.readiness_gates,
            &binding.device_id,
            || self.endpoints.describe_runtime(binding, None).map(|_| ()),
            || {
                self.synchronize_devices()?;
                self.devices.run_start(&binding.device_id)
            },
            Duration::from_secs(120),
            Duration::from_secs(1),
        )
    }

    pub(crate) fn runtime(
        &self,
        binding: &ExecutionBinding,
    ) -> Result<Arc<dyn crate::agent_sessions::ports::AgentRuntime>, String> {
        self.ensure_ready(binding)?;
        self.endpoints.runtime(binding)
    }

    pub(crate) fn describe_runtime(
        &self,
        binding: &ExecutionBinding,
        cwd: Option<&str>,
    ) -> Result<ExecutionTargetRuntime, String> {
        self.ensure_ready(binding)?;
        self.endpoints.describe_runtime(binding, cwd)
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
                let profile_lock = self
                    .sisters
                    .lock_for(repository_id, branch_ref.unwrap_or_default())?;
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
                        .map(|lock| {
                            self.sisters.contains_instance(
                                &lock.sister_group_id,
                                &device.device_id,
                                &worktree.worktree_id,
                            )
                        })
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
                    .map(|lock| {
                        self.sisters.contains_instance(
                            &lock.sister_group_id,
                            &profile.profile.execution.device_id,
                            &worktree.worktree_id,
                        )
                    })
                    .transpose()?;
                worktree.sister_lock = lock;
            }
        }
        Ok(())
    }
}

fn ensure_device_ready(
    readiness_gates: &Mutex<HashMap<String, Arc<Mutex<()>>>>,
    device_id: &str,
    mut probe: impl FnMut() -> Result<(), String>,
    start: impl FnOnce() -> Result<(), String>,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<(), String> {
    if probe().is_ok() {
        return Ok(());
    }
    let gate = {
        let mut gates = readiness_gates
            .lock()
            .map_err(|_| "Device readiness is unavailable")?;
        gates
            .entry(device_id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };
    let _operation = gate
        .lock()
        .map_err(|_| "Device readiness is unavailable")?;
    if probe().is_ok() {
        return Ok(());
    }
    start()?;
    let deadline = Instant::now() + timeout;
    let mut last_error = "Device did not become ready".to_string();
    while Instant::now() < deadline {
        match probe() {
            Ok(()) => return Ok(()),
            Err(error) => last_error = error,
        }
        thread::sleep(poll_interval);
    }
    Err(format!(
        "Device startup completed, but the Orchid host did not become ready: {last_error}"
    ))
}

#[cfg(test)]
mod readiness_tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Barrier,
    };

    #[test]
    fn concurrent_readiness_requests_run_one_start_operation() {
        let gates = Arc::new(Mutex::new(HashMap::new()));
        let ready = Arc::new(AtomicBool::new(false));
        let starts = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(2));
        let handles = (0..2)
            .map(|_| {
                let gates = gates.clone();
                let ready = ready.clone();
                let starts = starts.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    ensure_device_ready(
                        &gates,
                        "remote-one",
                        || {
                            ready
                                .load(Ordering::SeqCst)
                                .then_some(())
                                .ok_or_else(|| "offline".to_string())
                        },
                        || {
                            starts.fetch_add(1, Ordering::SeqCst);
                            thread::sleep(Duration::from_millis(20));
                            ready.store(true, Ordering::SeqCst);
                            Ok(())
                        },
                        Duration::from_secs(1),
                        Duration::from_millis(1),
                    )
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            handle.join().unwrap().unwrap();
        }
        assert_eq!(starts.load(Ordering::SeqCst), 1);
    }
}
