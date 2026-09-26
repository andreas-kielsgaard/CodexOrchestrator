use super::{
    capability_profile::{CapabilityProfile, CAPABILITY_PROFILE_CONTRACT_VERSION},
    node_profile::{NodeProfile, NODE_PROFILE_CONTRACT_VERSION},
    ports::{ProviderConfigurationSource, ProviderConfigurationSourceError},
    resolution::{
        DirectUserInvocationRequest, ResolutionError, SessionCreationRequest,
        SessionCreationResolution, SessionProfileResolver,
        DIRECT_USER_INVOCATION_REQUEST_CONTRACT_VERSION, SESSION_CREATION_REQUEST_CONTRACT_VERSION,
    },
    runtime_profile::{
        CapabilitySet, RuntimeProfileSnapshot, RuntimeSelections, SandboxMode,
        RUNTIME_PROFILE_CONTRACT_VERSION,
    },
};
use std::collections::{BTreeMap, BTreeSet};

struct FixedProfileSource(Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError>);

impl ProviderConfigurationSource for FixedProfileSource {
    fn profile_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
        self.0.clone()
    }
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn mcp(values: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
    values
        .iter()
        .map(|(connection, tools)| ((*connection).into(), set(tools)))
        .collect()
}

fn capabilities(
    models: &[&str],
    reasoning: &[&str],
    sandboxes: &[SandboxMode],
    mcp_tools: &[(&str, &[&str])],
    skills: &[&str],
) -> CapabilitySet {
    CapabilitySet {
        models: set(models),
        reasoning_modes: set(reasoning),
        sandbox_modes: sandboxes.iter().copied().collect(),
        mcp_tools: mcp(mcp_tools),
        skills: set(skills),
    }
}

fn runtime_profile() -> RuntimeProfileSnapshot {
    RuntimeProfileSnapshot {
        contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
        configuration: orchid_engine::contracts::ProviderConfigurationRef::new("codex", "selected"),
        exposure: capabilities(
            &["codex-a", "codex-b"],
            &["medium", "high"],
            &[SandboxMode::WorkspaceWrite],
            &[("repository", &["read", "write"]), ("issues", &["read"])],
            &["review", "planning"],
        ),
        locked: RuntimeSelections {
            model: None,
            reasoning_mode: None,
            sandbox_mode: Some(SandboxMode::WorkspaceWrite),
        },
        provider_options: None,
    }
}

fn capability_profile() -> CapabilityProfile {
    CapabilityProfile {
        execution: Default::default(),
        contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
        defaults: Default::default(),
        route_policies: Vec::new(),
        default_route_id: None,
        capability_profile_id: "implementation".into(),
        name: "Implementation".into(),
        revision: 3,
        allowed_capabilities: capabilities(
            &["codex-a", "codex-b"],
            &["medium", "high"],
            &[SandboxMode::WorkspaceWrite],
            &[("repository", &["read", "write"])],
            &["review", "planning"],
        ),
    }
}

fn node_profile() -> NodeProfile {
    NodeProfile {
        contract_version: NODE_PROFILE_CONTRACT_VERSION,
        allowed_capabilities: capabilities(
            &["codex-a", "codex-b"],
            &["medium", "high"],
            &[SandboxMode::WorkspaceWrite],
            &[("repository", &["read"])],
            &["review"],
        ),
        pinned_defaults: RuntimeSelections {
            model: Some("codex-a".into()),
            reasoning_mode: Some("high".into()),
            sandbox_mode: None,
        },
    }
}

fn creation_request() -> SessionCreationRequest {
    SessionCreationRequest {
        contract_version: SESSION_CREATION_REQUEST_CONTRACT_VERSION,
        capability_profile: capability_profile(),
        node_profile: node_profile(),
        agent_mcp_configuration: Default::default(),
        session_skill_inputs: Vec::new(),
    }
}

#[test]
fn creation_resolves_an_immutable_session_profile() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let resolution = SessionProfileResolver::resolve_creation(&source, None, creation_request()).unwrap();
    let profile = resolution.session_profile();

    assert_eq!(profile.configuration(), &orchid_engine::contracts::ProviderConfigurationRef::new("codex", "selected"));
    assert_eq!(
        profile.attached_runtime_capabilities(),
        &runtime_profile().exposure
    );
    assert_eq!(profile.capability_profile_id(), "implementation");
    assert_eq!(profile.capability_profile_revision(), 3);
    assert_eq!(
        profile.node_capabilities(),
        &node_profile().allowed_capabilities
    );
    assert_eq!(profile.pinned_defaults().model.as_deref(), Some("codex-a"));
    assert_eq!(
        profile.pinned_defaults().sandbox_mode,
        Some(SandboxMode::WorkspaceWrite)
    );
    resolution.verify_digest().unwrap();
}

#[test]
fn route_groups_shape_session_exposure_without_enforcing_model_allowances() {
    let mut request = creation_request();
    request.capability_profile.route_policies = vec![super::ProfileRoutePolicy {
        route_id: "local".into(),
        execution: Default::default(),
        model_allowances: vec![super::ModelAllowance {
            model_id: "future-only".into(),
            minimum_reasoning: Some("low".into()),
            maximum_reasoning: Some("high".into()),
        }],
        mcp_groups: set(&["otp:repository:mcps"]),
        skill_groups: set(&["orchid-skills"]),
        defaults: RuntimeSelections::default(),
        provider_options: None,
    }];
    request.capability_profile.default_route_id = Some("local".into());
    request.capability_profile.allowed_capabilities = CapabilitySet::default();
    request.session_skill_inputs = vec![crate::agent_sessions::ports::RuntimeSkillInput {
        id: "skill".into(),
        name: "review".into(),
        path: "/approved/SKILL.md".into(),
        content_sha256: "pinned".into(),
        description: "Review".into(),
    }];
    let resolution = SessionProfileResolver::resolve_snapshot(runtime_profile(), request).unwrap();
    let pinned = resolution.session_profile();
    assert_eq!(pinned.native_mcp_enabled(), Some(false));
    assert_eq!(
        pinned.node_capabilities().mcp_tools["repository"],
        set(&["read"])
    );
    assert_eq!(
        pinned.node_capabilities().mcp_tools["orchid_skills"],
        set(&["read_skill"])
    );
    assert!(pinned.node_capabilities().models.contains("codex-b"));
    resolution.verify_digest().unwrap();
}

#[test]
fn capability_profile_cannot_widen_the_runtime() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let mut request = creation_request();
    request
        .capability_profile
        .allowed_capabilities
        .models
        .insert("unavailable".into());

    assert!(matches!(
        SessionProfileResolver::resolve_creation(&source, None, request),
        Err(ResolutionError::CapabilityProfileWidensRuntime(capability))
            if capability == "model `unavailable`"
    ));
}

#[test]
fn node_profile_cannot_widen_its_capability_profile() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let mut request = creation_request();
    request
        .node_profile
        .allowed_capabilities
        .mcp_tools
        .get_mut("repository")
        .unwrap()
        .insert("admin".into());

    assert!(matches!(
        SessionProfileResolver::resolve_creation(&source, None, request),
        Err(ResolutionError::NodeProfileWidensCapabilityProfile(capability))
            if capability == "MCP tool `repository/admin`"
    ));
}

#[test]
fn runtime_locked_control_cannot_be_removed_or_changed() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let mut excluding = creation_request();
    excluding
        .node_profile
        .allowed_capabilities
        .sandbox_modes
        .clear();
    assert!(matches!(
        SessionProfileResolver::resolve_creation(&source, None, excluding),
        Err(ResolutionError::LockedCapabilityExcluded(capability))
            if capability.contains("sandbox mode")
    ));

    let mut selectable_but_locked = runtime_profile();
    selectable_but_locked
        .exposure
        .sandbox_modes
        .insert(SandboxMode::DangerFullAccess);
    let source = FixedProfileSource(Ok(selectable_but_locked));
    let mut changing = creation_request();
    changing
        .capability_profile
        .allowed_capabilities
        .sandbox_modes
        .insert(SandboxMode::DangerFullAccess);
    changing
        .node_profile
        .allowed_capabilities
        .sandbox_modes
        .insert(SandboxMode::DangerFullAccess);
    changing.node_profile.pinned_defaults.sandbox_mode = Some(SandboxMode::DangerFullAccess);
    assert!(matches!(
        SessionProfileResolver::resolve_creation(&source, None, changing),
        Err(ResolutionError::SelectionConflictsWithLocked(_))
    ));
}

#[test]
fn unavailable_pinned_default_fails_without_fallback() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let mut request = creation_request();
    request.node_profile.pinned_defaults.model = Some("unavailable".into());

    assert!(matches!(
        SessionProfileResolver::resolve_creation(&source, None, request),
        Err(ResolutionError::PinnedSelectionUnavailable(capability))
            if capability == "model `unavailable`"
    ));
}

#[test]
fn direct_user_can_select_model_and_reasoning_without_mutating_session_profile() {
    let mut native = runtime_profile();
    native.locked.sandbox_mode = None;
    native.exposure.sandbox_modes.insert(SandboxMode::ReadOnly);
    let source = FixedProfileSource(Ok(native));
    let mut request = creation_request();
    request.node_profile.allowed_capabilities.models = set(&["codex-a"]);
    request.node_profile.allowed_capabilities.reasoning_modes = set(&["high"]);
    let creation = SessionProfileResolver::resolve_creation(&source, None, request).unwrap();
    let original_digest = creation.digest().to_owned();
    let original_defaults = creation.session_profile().pinned_defaults().clone();

    let invocation = SessionProfileResolver::validate_direct_user_invocation(
        &source, None,
        &creation,
        DirectUserInvocationRequest {
            contract_version: DIRECT_USER_INVOCATION_REQUEST_CONTRACT_VERSION,
            model: Some("codex-b".into()),
            reasoning_mode: Some("medium".into()),
            sandbox_mode: Some(SandboxMode::ReadOnly),
        },
    )
    .unwrap();

    assert_eq!(invocation.selections.model.as_deref(), Some("codex-b"));
    assert_eq!(
        invocation.selections.sandbox_mode,
        Some(SandboxMode::ReadOnly)
    );
    assert_eq!(
        invocation.selections.reasoning_mode.as_deref(),
        Some("medium")
    );
    assert_eq!(creation.digest(), original_digest);
    assert_eq!(
        creation.session_profile().pinned_defaults(),
        &original_defaults
    );
    creation.verify_digest().unwrap();
}

#[test]
fn direct_user_selection_must_remain_inside_the_attached_runtime_exposure() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let creation = SessionProfileResolver::resolve_creation(&source, None, creation_request()).unwrap();
    let before = creation.clone();

    let result = SessionProfileResolver::validate_direct_user_invocation(
        &source, None,
        &creation,
        DirectUserInvocationRequest {
            contract_version: DIRECT_USER_INVOCATION_REQUEST_CONTRACT_VERSION,
            model: Some("unavailable".into()),
            reasoning_mode: None,
            sandbox_mode: None,
        },
    );

    assert!(matches!(
        result,
        Err(ResolutionError::DirectUserSelectionUnavailable(capability))
            if capability == "model `unavailable`"
    ));
    assert_eq!(creation, before);
}

#[test]
fn direct_user_validation_rejects_a_different_selected_runtime_profile() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let creation = SessionProfileResolver::resolve_creation(&source, None, creation_request()).unwrap();
    let mut changed = runtime_profile();
    changed.configuration = orchid_engine::contracts::ProviderConfigurationRef::new("codex", "other");
    let changed_source = FixedProfileSource(Ok(changed));

    assert!(matches!(
        SessionProfileResolver::validate_direct_user_invocation(
            &changed_source, None,
            &creation,
            DirectUserInvocationRequest {
                contract_version: DIRECT_USER_INVOCATION_REQUEST_CONTRACT_VERSION,
                model: None,
                reasoning_mode: None,
                sandbox_mode: None,
            },
        ),
        Err(ResolutionError::RuntimeProfileChanged { .. })
    ));
}

#[test]
fn pinned_workflow_validation_rejects_a_different_selected_runtime_profile() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let creation = SessionProfileResolver::resolve_creation(&source, None, creation_request()).unwrap();
    let mut changed = runtime_profile();
    changed.configuration = orchid_engine::contracts::ProviderConfigurationRef::new("codex", "other");
    let changed_source = FixedProfileSource(Ok(changed));

    assert!(matches!(
        SessionProfileResolver::validate_pinned_session(&changed_source, None, &creation),
        Err(ResolutionError::RuntimeProfileChanged { .. })
    ));
}

#[test]
fn digest_is_stable_for_equivalent_unordered_inputs() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let first = SessionProfileResolver::resolve_creation(&source, None, creation_request()).unwrap();
    let mut reordered_runtime = runtime_profile();
    reordered_runtime.exposure = reverse_insertion_order(&reordered_runtime.exposure);
    let reordered_source = FixedProfileSource(Ok(reordered_runtime));
    let mut reordered_request = creation_request();
    reordered_request.capability_profile.allowed_capabilities =
        reverse_insertion_order(&reordered_request.capability_profile.allowed_capabilities);
    reordered_request.node_profile.allowed_capabilities =
        reverse_insertion_order(&reordered_request.node_profile.allowed_capabilities);
    let second =
        SessionProfileResolver::resolve_creation(&reordered_source, None, reordered_request).unwrap();

    assert_eq!(first.digest(), second.digest());
}

fn reverse_insertion_order(capabilities: &CapabilitySet) -> CapabilitySet {
    CapabilitySet {
        models: capabilities.models.iter().rev().cloned().collect(),
        reasoning_modes: capabilities.reasoning_modes.iter().rev().cloned().collect(),
        sandbox_modes: capabilities.sandbox_modes.iter().rev().copied().collect(),
        mcp_tools: capabilities
            .mcp_tools
            .iter()
            .rev()
            .map(|(connection, tools)| (connection.clone(), tools.iter().rev().cloned().collect()))
            .collect(),
        skills: capabilities.skills.iter().rev().cloned().collect(),
    }
}

#[test]
fn digest_verification_rejects_a_contract_version_change() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let resolution = SessionProfileResolver::resolve_creation(&source, None, creation_request()).unwrap();
    let mut value = serde_json::to_value(resolution).unwrap();
    value["contractVersion"] = serde_json::json!(2);
    let changed: SessionCreationResolution = serde_json::from_value(value).unwrap();
    assert!(matches!(
        changed.verify_digest(),
        Err(ResolutionError::InvalidInput(_))
    ));
}

#[test]
fn strict_contracts_reject_unknown_fields_and_node_identity() {
    let mut value = serde_json::to_value(capability_profile()).unwrap();
    value["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<CapabilityProfile>(value).is_err());

    let mut node_value = serde_json::to_value(node_profile()).unwrap();
    node_value["nodeProfileId"] = serde_json::json!("reusable-node");
    assert!(serde_json::from_value::<NodeProfile>(node_value).is_err());
}

#[test]
fn source_failure_is_a_typed_resolution_error() {
    let source = FixedProfileSource(Err(ProviderConfigurationSourceError::unavailable(
        "no ready profile",
    )));
    assert_eq!(
        SessionProfileResolver::resolve_creation(&source, None, creation_request()),
        Err(ResolutionError::SourceUnavailable(
            "no ready profile".into()
        ))
    );
}
#[test]
fn required_default_profile_is_atomic_retained_and_cannot_be_deleted() {
    use super::CapabilityProfileRepository;
    let folder = tempfile::tempdir().unwrap();
    let database = folder.path().join("profiles.sqlite");
    let repository = super::SqliteCapabilityProfileRepository::open(&database).unwrap();
    assert!(repository.default_profile().unwrap().is_none());
    assert!(repository.set_default_profile("missing").is_err());
    let first = super::CapabilityProfile {
        execution: Default::default(),
        contract_version: 1,
        capability_profile_id: "first".into(),
        name: "First".into(),
        revision: 1,
        allowed_capabilities: Default::default(),
        defaults: Default::default(),
        route_policies: Vec::new(),
        default_route_id: None,
    };
    let second = super::CapabilityProfile {
        capability_profile_id: "second".into(),
        name: "Second".into(),
        ..first.clone()
    };
    repository.insert(&first).unwrap();
    repository.insert(&second).unwrap();
    repository.set_default_profile("first").unwrap();
    let mut resolved_first = repository.default_profile().unwrap().unwrap();
    resolved_first.execution.device_name = first.execution.device_name.clone();
    assert_eq!(resolved_first, first.clone());
    assert!(repository.remove("first").is_err());
    repository.set_default_profile("second").unwrap();
    repository.remove("first").unwrap();
    let reopened = super::SqliteCapabilityProfileRepository::open(&database).unwrap();
    let mut resolved_second = reopened.default_profile().unwrap().unwrap();
    resolved_second.execution.device_name = second.execution.device_name.clone();
    assert_eq!(resolved_second, second);
}

#[test]
fn sqlite_profiles_store_route_references_and_resolve_device_owned_connections() {
    use super::CapabilityProfileRepository;
    let folder = tempfile::tempdir().unwrap();
    let database = folder.path().join("profile-route-refs.sqlite");
    let repository = super::SqliteCapabilityProfileRepository::open(&database).unwrap();
    let profile = CapabilityProfile {
        revision: 1,
        ..capability_profile()
    };
    repository.insert(&profile).unwrap();

    let connection = rusqlite::Connection::open(&database).unwrap();
    let json: String = connection
        .query_row(
            "SELECT profile_json FROM execution_capability_profiles WHERE capability_profile_id=?1",
            [&profile.capability_profile_id],
            |row| row.get(0),
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(value["execution"].get("connection").is_none());
    assert!(value["execution"].get("deviceName").is_none());
    assert_eq!(value["execution"]["deviceId"], "local");
    let resolved = repository.find("implementation").unwrap().unwrap();
    assert_eq!(
        resolved.execution.route_ref(),
        profile.execution.route_ref()
    );
    assert_eq!(resolved.execution.device_name, "This device");
    assert_eq!(resolved.execution.connection, profile.execution.connection);
    assert_eq!(
        CapabilityProfile {
            execution: profile.execution.clone(),
            ..resolved.clone()
        },
        profile
    );
}

fn native_options(provider: &str, personality: &str) -> orchid_engine::contracts::ProviderNativeOptions {
    orchid_engine::contracts::ProviderNativeOptions {
        provider: provider.into(),
        settings: serde_json::json!({ "personality": personality }),
    }
}

fn routed_request(route_options: Option<orchid_engine::contracts::ProviderNativeOptions>) -> SessionCreationRequest {
    let mut request = creation_request();
    request.capability_profile.route_policies = vec![super::ProfileRoutePolicy {
        route_id: "local".into(),
        execution: Default::default(),
        model_allowances: Vec::new(),
        mcp_groups: set(&[super::NATIVE_MCP_GROUP]),
        skill_groups: BTreeSet::new(),
        defaults: RuntimeSelections::default(),
        provider_options: route_options,
    }];
    request.capability_profile.default_route_id = Some("local".into());
    request.capability_profile.allowed_capabilities = CapabilitySet::default();
    request.node_profile.allowed_capabilities = CapabilitySet::default();
    request
}

#[test]
fn route_native_options_override_configuration_defaults_and_absence_inherits_them() {
    let mut runtime = runtime_profile();
    runtime.provider_options = Some(native_options("codex", "friendly"));

    let inherited =
        SessionProfileResolver::resolve_snapshot(runtime.clone(), routed_request(None)).unwrap();
    assert_eq!(
        inherited.session_profile().provider_options(),
        Some(&native_options("codex", "friendly"))
    );
    assert_eq!(inherited.session_profile().native_mcp_enabled(), Some(true));

    let overridden = SessionProfileResolver::resolve_snapshot(
        runtime,
        routed_request(Some(native_options("codex", "pragmatic"))),
    )
    .unwrap();
    assert_eq!(
        overridden.session_profile().provider_options(),
        Some(&native_options("codex", "pragmatic"))
    );
    overridden.verify_digest().unwrap();
}

#[test]
fn native_options_cannot_travel_with_another_provider() {
    let request = routed_request(Some(native_options("test-provider", "friendly")));
    assert!(request
        .capability_profile
        .validate()
        .unwrap_err()
        .contains("test-provider"));

    let mut runtime = runtime_profile();
    runtime.provider_options = Some(native_options("test-provider", "friendly"));
    assert!(runtime.validate().unwrap_err().contains("test-provider"));
}
