use super::*;
use crate::execution_configuration::CapabilityProfile;
use chrono::Utc;
use serde_json::json;

fn device(id: &str) -> crate::execution_devices::ExecutionDeviceDto {
    crate::execution_devices::ExecutionDeviceDto {
        device_id: id.into(),
        display_name: format!("Registered {id}"),
        connection_summary: "Developer configured".into(),
        lifecycle: crate::execution_devices::DeviceLifecyclePolicy {
            start: None,
            stop: None,
            idle_shutdown_seconds: None,
        },
        active_leases: 0,
        last_orchid_activity_at: None,
        keep_awake_until: None,
        last_lifecycle_message: None,
    }
}

fn profile(id: &str, device: &str, remote: bool) -> CapabilityProfile {
    CapabilityProfile {
        contract_version: 1,
        capability_profile_id: id.into(),
        name: format!("Profile {id}"),
        revision: 7,
        allowed_capabilities: Default::default(),
        defaults: Default::default(),
        execution: ExecutionBinding {
            device_id: device.into(),
            device_name: format!("Device {device}"),
            configuration_ref: id.into(),
            connection: if remote {
                ExecutionConnection::Ssh {
                    target: format!("ssh-{device}"),
                    host_executable: "/opt/orchid-host".into(),
                }
            } else {
                ExecutionConnection::Local
            },
            ..Default::default()
        },
        route_policies: Vec::new(),
        default_route_id: None,
    }
}

fn repository(id: &str) -> RegisteredRepository {
    RegisteredRepository {
        repository_id: id.into(),
        label: format!("Repository {id}"),
        anchor_root: format!("/laptop/{id}").into(),
        git_common_directory: format!("/laptop/{id}/.git").into(),
        first_registered_at: Utc::now(),
        last_verified_at: Utc::now(),
    }
}

fn instance(root: &str, name: &str, branch: Option<&str>) -> WorktreeInstance {
    WorktreeInstance {
        handle: format!("instance-{name}"),
        path: format!("{root}/{name}"),
        branch_ref: branch.map(str::to_owned),
        head: Some("0123456789abcdef".into()),
        dirty: false,
        head_committed_at: Some("2026-01-01T00:00:00Z".into()),
    }
}

#[test]
fn device_listing_is_owned_by_registration_and_profiles_are_optional() {
    let profiles = [
        profile("local-a", "laptop", false),
        profile("local-b", "laptop", false),
        profile("remote", "server", true),
    ];
    let devices = configured_devices(
        &[device("laptop"), device("server"), device("profileless")],
        &profiles,
    );
    assert_eq!(devices.len(), 3);
    assert_eq!(devices[0].device_name, "Registered laptop");
    assert_eq!(devices[0].profiles.len(), 2);
    assert_eq!(devices[1].device_id, "server");
    assert_eq!(devices[1].profiles[0].capability_profile_revision, 7);
    assert_eq!(devices[2].device_id, "profileless");
    assert!(devices[2].profiles.is_empty());
    let value = serde_json::to_value(&devices).unwrap();
    assert_eq!(
        value[1]["profiles"][0]["execution"]["configurationRef"],
        "remote"
    );
    assert!(value[1]["profiles"][0].get("instances").is_none());
}

#[test]
fn local_scope_uses_connection_kind_and_preserves_duplicate_repository_and_profile_choices() {
    let profiles = [
        profile("local-a", "this-machine", false),
        profile("local-b", "this-machine", false),
        // Device IDs alone must not define local execution.
        profile("remote", "local", true),
    ];
    let repositories = [repository("alpha"), repository("beta")];
    let mut calls = Vec::new();
    let choices = worktree_choices(
        &profiles,
        &repositories,
        &[],
        &WorktreeChoiceScope::Local,
        |execution, root, branch| {
            assert!(!execution.is_remote(), "local choices must never call SSH");
            assert_eq!(branch, None);
            calls.push((execution.configuration_ref.clone(), root.to_owned()));
            Ok(vec![
                instance(root, "main", Some("refs/heads/main")),
                instance(root, "work", Some("refs/heads/feature/shared")),
                instance(root, "detached", None),
            ])
        },
    );
    assert_eq!(calls.len(), 4);
    assert_eq!(choices.len(), 2);
    for (index, group) in choices.iter().enumerate() {
        assert_eq!(group.repository_id, repositories[index].repository_id);
        assert_eq!(group.repository_name, repositories[index].label);
        assert_eq!(group.profiles.len(), 2);
        for (profile_index, entry) in group.profiles.iter().enumerate() {
            assert_eq!(
                entry.profile.capability_profile_id,
                profiles[profile_index].capability_profile_id
            );
            assert_eq!(entry.profile.capability_profile_revision, 7);
            assert_eq!(entry.instances.len(), 2);
            assert_eq!(entry.instances[1].branch_ref, "refs/heads/feature/shared");
            assert_eq!(entry.instances[1].worktree_id, "instance-work");
            assert_eq!(entry.instances[1].head.as_deref(), Some("0123456789abcdef"));
            assert!(entry.instances[1]
                .path
                .starts_with(&format!("/laptop/{}/", group.repository_id)));
        }
    }
}

#[test]
fn device_scope_queries_only_its_mapped_repositories_and_keeps_failures_local() {
    let profiles = [
        profile("local", "laptop", false),
        profile("unrelated", "other-server", true),
        profile("broken", "server", true),
        profile("working", "server", true),
    ];
    let locations = [RepositoryDeviceLocation {
        repository_id: "alpha".into(),
        device_id: "server".into(),
        repository_root: "/srv/alpha".into(),
    }];
    let mut calls = Vec::new();
    let choices = worktree_choices(
        &profiles,
        &[repository("alpha"), repository("unmapped")],
        &locations,
        &WorktreeChoiceScope::Device {
            device_id: "server".into(),
        },
        |execution, root, branch| {
            assert_eq!(execution.device_id, "server");
            assert_eq!(root, "/srv/alpha");
            assert_eq!(branch, None);
            calls.push(execution.configuration_ref.clone());
            if execution.configuration_ref == "broken" {
                Err("This profile cannot connect".into())
            } else {
                Ok(vec![instance(root, "main", Some("refs/heads/main"))])
            }
        },
    );
    assert_eq!(calls, ["broken", "working"]);
    assert_eq!(
        choices[0].profiles[0].error.as_deref(),
        Some("This profile cannot connect")
    );
    assert_eq!(choices[0].profiles[1].instances.len(), 1);
    assert!(choices[0].profiles[1].error.is_none());
    assert!(choices[1].profiles.iter().all(|entry| {
        entry.instances.is_empty()
            && entry.error.as_deref() == Some("No repository path is configured on this device")
    }));
}

#[test]
fn branch_filtered_modal_keeps_its_json_shape_and_actual_instance_branch() {
    let profile = profile("codex", "laptop", false);
    let devices = targets(
        &[profile.clone()],
        &repository("alpha"),
        &[],
        "refs/heads/feature/demo",
        |_, root, branch| {
            assert_eq!(branch, Some("refs/heads/feature/demo"));
            Ok(vec![
                instance(root, "demo", Some("refs/heads/feature/demo")),
                instance(root, "main", Some("refs/heads/main")),
                instance(root, "detached", None),
            ])
        },
    );
    assert_eq!(
        serde_json::to_value(devices).unwrap(),
        json!([{
            "deviceId":"laptop",
            "deviceName":"Device laptop",
            "profiles":[{
                "capabilityProfileId":"codex",
                "capabilityProfileRevision":7,
                "capabilityProfileName":"Profile codex",
                "execution":profile.execution,
                "instances":[{
                    "worktreeId":"instance-demo",
                    "path":"/laptop/alpha/demo",
                    "head":"0123456789abcdef",
                    "dirty":false,
                    "headCommittedAt":"2026-01-01T00:00:00Z",
                    "branchRef":"refs/heads/feature/demo"
                }],
                "error":null
            }]
        }])
    );
}
