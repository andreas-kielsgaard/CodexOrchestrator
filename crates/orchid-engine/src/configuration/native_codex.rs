use super::*;
use crate::codex::app_server::environment::CodexEnvironment;
use std::collections::{BTreeMap, BTreeSet};
pub fn runtime_profile(
    native: &CodexEnvironment,
    profile_ref: String,
    product_tools: BTreeMap<String, BTreeSet<String>>,
) -> RuntimeProfileSnapshot {
    let mut exposure = CapabilitySet {
        mcp_tools: product_tools,
        sandbox_modes: [
            SandboxMode::ReadOnly,
            SandboxMode::WorkspaceWrite,
            SandboxMode::DangerFullAccess,
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    if let Some(allowed) = native.requirements["requirements"]["allowedSandboxModes"].as_array() {
        exposure.sandbox_modes.retain(|mode| {
            allowed.iter().any(|value| {
                value.as_str()
                    == Some(match mode {
                        SandboxMode::ReadOnly => "read-only",
                        SandboxMode::WorkspaceWrite => "workspace-write",
                        SandboxMode::DangerFullAccess => "danger-full-access",
                    })
            })
        });
    }
    if let Some(models) = native.models.as_array() {
        for model in models {
            if let Some(name) = model["model"].as_str().or(model["id"].as_str()) {
                exposure.models.insert(name.into());
            }
            if let Some(modes) = model["supportedReasoningEfforts"].as_array() {
                for mode in modes {
                    if let Some(effort) = mode["reasoningEffort"].as_str() {
                        exposure.reasoning_modes.insert(effort.into());
                    }
                }
            }
        }
    }
    if let Some(data) = native.skills["data"].as_array() {
        for entry in data {
            if let Some(skills) = entry["skills"].as_array() {
                for skill in skills {
                    if skill["enabled"] != false {
                        if let Some(name) = skill["name"].as_str() {
                            exposure.skills.insert(name.into());
                        }
                    }
                }
            }
        }
    }
    RuntimeProfileSnapshot {
        contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
        profile_ref,
        exposure,
        locked: RuntimeSelections::default(),
    }
}
