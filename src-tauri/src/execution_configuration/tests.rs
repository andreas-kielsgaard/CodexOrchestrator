use super::{
    harness::{HarnessDefinition, HARNESS_DEFINITION_CONTRACT_VERSION},
    node_profile::{InstructionDelivery, NodeProfileDefinition, NODE_PROFILE_CONTRACT_VERSION},
    ports::{SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
    resolution::{
        ExecutionConfigurationResolver, ResolutionContext, ResolutionError, ResolutionRequest,
        RESOLUTION_REQUEST_CONTRACT_VERSION,
    },
    runtime_profile::{
        CapabilitySet, InvocationPhase, RuntimeProfileSnapshot, RuntimeSelections, SandboxMode,
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

fn profile() -> RuntimeProfileSnapshot {
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

fn harness() -> HarnessDefinition {
    HarnessDefinition {
        contract_version: HARNESS_DEFINITION_CONTRACT_VERSION,
        harness_id: "implementation".into(),
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

fn node_profile() -> NodeProfileDefinition {
    NodeProfileDefinition {
        contract_version: NODE_PROFILE_CONTRACT_VERSION,
        node_profile_id: "implementation-review".into(),
        revision: 2,
        allowed_capabilities: capabilities(
            &["codex-a"],
            &["high"],
            &[SandboxMode::WorkspaceWrite],
            &[("repository", &["read"])],
            &["review"],
        ),
        selections: RuntimeSelections {
            model: Some("codex-a".into()),
            reasoning_mode: Some("high".into()),
            sandbox_mode: None,
        },
        instructions: InstructionDelivery {
            recurring: Some("Review the evidence.".into()),
            start_only: Some("First inspect the proposed plan.".into()),
        },
    }
}

fn request(phase: InvocationPhase) -> ResolutionRequest {
    ResolutionRequest {
        contract_version: RESOLUTION_REQUEST_CONTRACT_VERSION,
        harness: harness(),
        node_profile: node_profile(),
        context: ResolutionContext { phase },
    }
}

#[test]
fn resolves_the_node_profile_as_a_strict_subset() {
    let source = FixedProfileSource(Ok(profile()));
    let resolved =
        ExecutionConfigurationResolver::resolve(&source, request(InvocationPhase::Start)).unwrap();

    assert_eq!(
        resolved.content.capabilities,
        node_profile().allowed_capabilities
    );
    assert_eq!(
        resolved.content.selections.model.as_deref(),
        Some("codex-a")
    );
    assert_eq!(
        resolved.content.selections.sandbox_mode,
        Some(SandboxMode::WorkspaceWrite)
    );
    assert_eq!(
        resolved.content.instructions.start_only.as_deref(),
        Some("First inspect the proposed plan.")
    );
    resolved.verify_digest().unwrap();
}

#[test]
fn harness_cannot_widen_the_selected_profile() {
    let source = FixedProfileSource(Ok(profile()));
    let mut request = request(InvocationPhase::Start);
    request
        .harness
        .allowed_capabilities
        .models
        .insert("unavailable".into());

    assert!(matches!(
        ExecutionConfigurationResolver::resolve(&source, request),
        Err(ResolutionError::HarnessWidensProfile(capability))
            if capability == "model `unavailable`"
    ));
}

#[test]
fn node_profile_cannot_widen_its_harness() {
    let source = FixedProfileSource(Ok(profile()));
    let mut request = request(InvocationPhase::Start);
    request
        .node_profile
        .allowed_capabilities
        .mcp_tools
        .get_mut("repository")
        .unwrap()
        .insert("admin".into());

    assert!(matches!(
        ExecutionConfigurationResolver::resolve(&source, request),
        Err(ResolutionError::NodeProfileWidensHarness(capability))
            if capability == "MCP tool `repository/admin`"
    ));
}

#[test]
fn locked_profile_control_cannot_be_removed_or_changed() {
    let source = FixedProfileSource(Ok(profile()));
    let mut excluding = request(InvocationPhase::Start);
    excluding
        .node_profile
        .allowed_capabilities
        .sandbox_modes
        .clear();
    assert!(matches!(
        ExecutionConfigurationResolver::resolve(&source, excluding),
        Err(ResolutionError::LockedCapabilityExcluded(capability))
            if capability.contains("sandbox mode")
    ));

    let mut profile_with_selectable_but_locked_sandbox = profile();
    profile_with_selectable_but_locked_sandbox
        .exposure
        .sandbox_modes
        .insert(SandboxMode::DangerFullAccess);
    let source = FixedProfileSource(Ok(profile_with_selectable_but_locked_sandbox));
    let mut changing = request(InvocationPhase::Start);
    changing
        .harness
        .allowed_capabilities
        .sandbox_modes
        .insert(SandboxMode::DangerFullAccess);
    changing
        .node_profile
        .allowed_capabilities
        .sandbox_modes
        .insert(SandboxMode::DangerFullAccess);
    changing.node_profile.selections.sandbox_mode = Some(SandboxMode::DangerFullAccess);
    assert!(matches!(
        ExecutionConfigurationResolver::resolve(&source, changing),
        Err(ResolutionError::SelectionConflictsWithLocked(_))
    ));
}

#[test]
fn unavailable_selection_fails_without_fallback() {
    let source = FixedProfileSource(Ok(profile()));
    let mut request = request(InvocationPhase::Start);
    request.node_profile.selections.model = Some("codex-b".into());

    assert!(matches!(
        ExecutionConfigurationResolver::resolve(&source, request),
        Err(ResolutionError::SelectionUnavailable(capability))
            if capability == "model `codex-b`"
    ));
}

#[test]
fn start_only_instructions_are_absent_on_resume() {
    let source = FixedProfileSource(Ok(profile()));
    let resumed =
        ExecutionConfigurationResolver::resolve(&source, request(InvocationPhase::Resume)).unwrap();

    assert_eq!(
        resumed.content.instructions.recurring.as_deref(),
        Some("Review the evidence.")
    );
    assert_eq!(resumed.content.instructions.start_only, None);
}

#[test]
fn digest_is_stable_for_equivalent_unordered_inputs() {
    let source = FixedProfileSource(Ok(profile()));
    let first =
        ExecutionConfigurationResolver::resolve(&source, request(InvocationPhase::Start)).unwrap();
    let mut reordered_profile = profile();
    reordered_profile.exposure = reverse_insertion_order(&reordered_profile.exposure);
    let reordered_source = FixedProfileSource(Ok(reordered_profile));
    let mut reordered_request = request(InvocationPhase::Start);
    reordered_request.harness.allowed_capabilities =
        reverse_insertion_order(&reordered_request.harness.allowed_capabilities);
    reordered_request.node_profile.allowed_capabilities =
        reverse_insertion_order(&reordered_request.node_profile.allowed_capabilities);
    let second =
        ExecutionConfigurationResolver::resolve(&reordered_source, reordered_request).unwrap();

    assert_eq!(first.digest, second.digest);
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
    let source = FixedProfileSource(Ok(profile()));
    let mut resolved =
        ExecutionConfigurationResolver::resolve(&source, request(InvocationPhase::Start)).unwrap();
    resolved.contract_version += 1;
    assert!(matches!(
        resolved.verify_digest(),
        Err(ResolutionError::InvalidInput(_))
    ));
}

#[test]
fn strict_contracts_reject_unknown_fields() {
    let mut value = serde_json::to_value(harness()).unwrap();
    value["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<HarnessDefinition>(value).is_err());
}

#[test]
fn source_failure_is_a_typed_resolution_error() {
    let source = FixedProfileSource(Err(SelectedRuntimeProfileSourceError::unavailable(
        "no ready profile",
    )));
    assert_eq!(
        ExecutionConfigurationResolver::resolve(&source, request(InvocationPhase::Start)),
        Err(ResolutionError::SourceUnavailable(
            "no ready profile".into()
        ))
    );
}
