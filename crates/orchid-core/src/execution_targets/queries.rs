//! Shared target discovery for desktop, control tools and navigation.
use super::{domain::*, ExecutionTargetService};
use crate::agent_sessions::client::HostCommand;
use serde_json::{json, Value};

impl ExecutionTargetService {
    pub fn attached_devices(&self) -> Result<Vec<ConfiguredExecutionDevice>, String> {
        let mut rows = self.devices()?;
        let Some(remote) = self.endpoints.device_transport()? else {
            return Ok(rows);
        };
        let local = crate::synchronization::repository::ReplicaStore::new(self.locations.0.clone())
            .device_id()?;
        for (device, online) in remote
            .devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|(d, _)| d.id != local)
        {
            if !online {
                continue;
            }
            let Ok(Value::Array(values)) = remote.execute(
                &device.id,
                HostCommand {
                    method: "device.profiles".into(),
                    input: json!({}),
                },
            ) else {
                continue;
            };
            let mut observed = Vec::new();
            for mut value in values.into_iter().filter(|v| v["deviceId"] == "local") {
                normalize_device(&mut value, &device.id, &device.name);
                observed.push(serde_json::from_value(value).map_err(|e| e.to_string())?);
            }
            rows.retain(|row| row.device_id != device.id);
            rows.extend(observed);
        }
        Ok(rows)
    }

    pub fn attached_worktree_choices(
        &self,
        scope: &WorktreeChoiceScope,
    ) -> Result<Vec<RepositoryWorktreeChoices>, String> {
        if let WorktreeChoiceScope::Device { device_id } = scope {
            if let Some(remote) = self.endpoints.device_transport()? {
                let local =
                    crate::synchronization::repository::ReplicaStore::new(self.locations.0.clone())
                        .device_id()?;
                if device_id == &local {
                    return self.worktree_choices(&WorktreeChoiceScope::Local);
                }
                if let Some((device, _)) = remote
                    .devices()
                    .unwrap_or_default()
                    .into_iter()
                    .find(|(d, _)| &d.id == device_id)
                {
                    let mut value = remote.execute(
                        device_id,
                        HostCommand {
                            method: "device.worktreeChoices".into(),
                            input: json!({"scope":{"kind":"local"}}),
                        },
                    )?;
                    normalize_device(&mut value, device_id, &device.name);
                    return serde_json::from_value(value).map_err(|e| e.to_string());
                }
            }
        }
        self.worktree_choices(scope)
    }

    pub fn attached_targets(
        &self,
        repository: &str,
        branch: &str,
    ) -> Result<Vec<DeviceWorktreeTargets>, String> {
        let Some(remote) = self.endpoints.device_transport()? else {
            return self.targets(repository, branch);
        };
        let local = crate::synchronization::repository::ReplicaStore::new(self.locations.0.clone())
            .device_id()?;
        let mut rows = self.targets(repository, branch).unwrap_or_default();
        for (device, online) in remote
            .devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|(d, _)| d.id != local)
        {
            if !online {
                continue;
            }
            let Ok(Value::Array(values)) = remote.execute(
                &device.id,
                HostCommand {
                    method: "device.targets".into(),
                    input: json!({"repositoryId":repository,"branchRef":branch}),
                },
            ) else {
                continue;
            };
            let mut observed = Vec::new();
            for mut value in values.into_iter().filter(|v| v["deviceId"] == "local") {
                normalize_device(&mut value, &device.id, &device.name);
                observed.push(serde_json::from_value(value).map_err(|e| e.to_string())?);
            }
            rows.retain(|row| row.device_id != device.id);
            rows.extend(observed);
        }
        Ok(rows)
    }
}

fn normalize_device(value: &mut Value, id: &str, name: &str) {
    match value {
        Value::Object(object) => {
            if object.get("activeDeviceId").and_then(Value::as_str) == Some("local") {
                object.insert("activeDeviceId".into(), json!(id));
            }
            if object.get("deviceId").and_then(Value::as_str) == Some("local") {
                object.insert("deviceId".into(), json!(id));
                if object.contains_key("connection") {
                    object.insert("connection".into(), json!({"kind":"device"}));
                }
                if object.contains_key("deviceName") {
                    object.insert("deviceName".into(), json!(name));
                }
            }
            for child in object.values_mut() {
                normalize_device(child, id, name);
            }
        }
        Value::Array(values) => {
            for child in values {
                normalize_device(child, id, name);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    struct Remote;
    impl crate::agent_sessions::client::DeviceCommandTransport for Remote {
        fn devices(
            &self,
        ) -> Result<Vec<(crate::synchronization::DeviceDescription, bool)>, String> {
            Ok(vec![(
                crate::synchronization::DeviceDescription {
                    id: "remote".into(),
                    name: "Work host".into(),
                    providers: vec![],
                },
                true,
            )])
        }
        fn execute(&self, device: &str, command: HostCommand) -> Result<Value, String> {
            assert_eq!(device, "remote");
            assert_eq!(command.method, "device.worktreeChoices");
            assert_eq!(command.input, json!({"scope":{"kind":"local"}}));
            Ok(
                json!([{"repositoryId":"repo","repositoryName":"Remote repository","profiles":[{
                    "capabilityProfileId":"profile","capabilityProfileRevision":1,"capabilityProfileName":"Agent",
                    "execution":{"deviceId":"local","deviceName":"Local","provider":"codex","configurationRef":"configured","connection":{"kind":"local"}},
                    "error":null,"sisterLock":{"sisterGroupId":"group","activeDeviceId":"local","ownerSessionId":"session"},
                    "instances":[{"worktreeId":"worktree","path":"/host-only/worktree","head":null,"dirty":false,"headCommittedAt":null,"createdAt":null,"branchRef":"refs/heads/main"}]
                }]}]),
            )
        }
    }
    #[test]
    fn remote_worktree_query_needs_no_local_checkout_and_returns_portable_targets() {
        let database = Arc::new(
            crate::persistence::ActiveDatabase::from_connection(
                rusqlite::Connection::open_in_memory().unwrap(),
                crate::schema::initialize,
            )
            .unwrap(),
        );
        let endpoints = Arc::new(super::super::endpoints::ExecutionEndpoints::new(
            Default::default(),
        ));
        endpoints.attach_device_transport(Arc::new(Remote));
        let profiles = Arc::new(
            crate::execution_configuration::CapabilityProfileService::new(Arc::new(
                crate::execution_configuration::SqliteCapabilityProfileRepository::from_database(
                    database.clone(),
                ),
            )),
        );
        let service = ExecutionTargetService::new(database, endpoints, profiles);
        let rows = service
            .attached_worktree_choices(&WorktreeChoiceScope::Device {
                device_id: "remote".into(),
            })
            .unwrap();
        let profile = &rows[0].profiles[0];
        assert_eq!(profile.instances[0].path, "/host-only/worktree");
        assert_eq!(profile.profile.execution.device_id, "remote");
        assert_eq!(
            profile.profile.execution.connection,
            ExecutionConnection::Device
        );
        assert_eq!(
            profile.sister_lock.as_ref().unwrap().active_device_id,
            "remote"
        );
    }
}
