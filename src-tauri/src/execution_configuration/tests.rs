use super::{
    capability_profile::{CapabilityProfile, CAPABILITY_PROFILE_CONTRACT_VERSION},
    node_profile::{NodeProfile, NODE_PROFILE_CONTRACT_VERSION},
    ports::{SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
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

struct FixedProfileSource(Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError>);

impl SelectedRuntimeProfileSource for FixedProfileSource {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
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
        profile_ref: "native-codex:selected".into(),
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
    }
}

fn capability_profile() -> CapabilityProfile {
    CapabilityProfile {
        execution: Default::default(),
        contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
        defaults: Default::default(),
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
    }
}

#[test]
fn creation_resolves_an_immutable_session_profile() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let resolution = SessionProfileResolver::resolve_creation(&source, creation_request()).unwrap();
    let profile = resolution.session_profile();

    assert_eq!(profile.runtime_profile_ref(), "native-codex:selected");
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
fn capability_profile_cannot_widen_the_runtime() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let mut request = creation_request();
    request
        .capability_profile
        .allowed_capabilities
        .models
        .insert("unavailable".into());

    assert!(matches!(
        SessionProfileResolver::resolve_creation(&source, request),
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
        SessionProfileResolver::resolve_creation(&source, request),
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
        SessionProfileResolver::resolve_creation(&source, excluding),
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
        SessionProfileResolver::resolve_creation(&source, changing),
        Err(ResolutionError::SelectionConflictsWithLocked(_))
    ));
}

#[test]
fn unavailable_pinned_default_fails_without_fallback() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let mut request = creation_request();
    request.node_profile.pinned_defaults.model = Some("unavailable".into());

    assert!(matches!(
        SessionProfileResolver::resolve_creation(&source, request),
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
    let creation = SessionProfileResolver::resolve_creation(&source, request).unwrap();
    let original_digest = creation.digest().to_owned();
    let original_defaults = creation.session_profile().pinned_defaults().clone();

    let invocation = SessionProfileResolver::validate_direct_user_invocation(
        &source,
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
    let creation = SessionProfileResolver::resolve_creation(&source, creation_request()).unwrap();
    let before = creation.clone();

    let result = SessionProfileResolver::validate_direct_user_invocation(
        &source,
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
    let creation = SessionProfileResolver::resolve_creation(&source, creation_request()).unwrap();
    let mut changed = runtime_profile();
    changed.profile_ref = "native-codex:other".into();
    let changed_source = FixedProfileSource(Ok(changed));

    assert!(matches!(
        SessionProfileResolver::validate_direct_user_invocation(
            &changed_source,
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
    let creation = SessionProfileResolver::resolve_creation(&source, creation_request()).unwrap();
    let mut changed = runtime_profile();
    changed.profile_ref = "native-codex:other".into();
    let changed_source = FixedProfileSource(Ok(changed));

    assert!(matches!(
        SessionProfileResolver::validate_pinned_session(&changed_source, &creation),
        Err(ResolutionError::RuntimeProfileChanged { .. })
    ));
}

#[test]
fn digest_is_stable_for_equivalent_unordered_inputs() {
    let source = FixedProfileSource(Ok(runtime_profile()));
    let first = SessionProfileResolver::resolve_creation(&source, creation_request()).unwrap();
    let mut reordered_runtime = runtime_profile();
    reordered_runtime.exposure = reverse_insertion_order(&reordered_runtime.exposure);
    let reordered_source = FixedProfileSource(Ok(reordered_runtime));
    let mut reordered_request = creation_request();
    reordered_request.capability_profile.allowed_capabilities =
        reverse_insertion_order(&reordered_request.capability_profile.allowed_capabilities);
    reordered_request.node_profile.allowed_capabilities =
        reverse_insertion_order(&reordered_request.node_profile.allowed_capabilities);
    let second =
        SessionProfileResolver::resolve_creation(&reordered_source, reordered_request).unwrap();

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
    let resolution = SessionProfileResolver::resolve_creation(&source, creation_request()).unwrap();
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
    let source = FixedProfileSource(Err(SelectedRuntimeProfileSourceError::unavailable(
        "no ready profile",
    )));
    assert_eq!(
        SessionProfileResolver::resolve_creation(&source, creation_request()),
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
    };
    let second = super::CapabilityProfile {
        capability_profile_id: "second".into(),
        name: "Second".into(),
        ..first.clone()
    };
    repository.insert(&first).unwrap();
    repository.insert(&second).unwrap();
    repository.set_default_profile("first").unwrap();
    assert_eq!(repository.default_profile().unwrap(), Some(first.clone()));
    assert!(repository.remove("first").is_err());
    repository.set_default_profile("second").unwrap();
    repository.remove("first").unwrap();
    let reopened = super::SqliteCapabilityProfileRepository::open(&database).unwrap();
    assert_eq!(reopened.default_profile().unwrap(), Some(second));
}
