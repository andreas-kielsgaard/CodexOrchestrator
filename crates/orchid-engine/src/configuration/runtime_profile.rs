use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const RUNTIME_PROFILE_CONTRACT_VERSION: u32 = 1;

/// A Codex app-server personality override. Absence means the selected Codex home resolves its
/// own configured default.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexPersonality {
    None,
    Friendly,
    Pragmatic,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilitySet {
    pub models: BTreeSet<String>,
    pub reasoning_modes: BTreeSet<String>,
    pub sandbox_modes: BTreeSet<SandboxMode>,
    pub mcp_tools: BTreeMap<String, BTreeSet<String>>,
    pub skills: BTreeSet<String>,
}

impl CapabilitySet {
    pub fn validate(&self, owner: &str) -> Result<(), String> {
        validate_identifiers(owner, "models", self.models.iter())?;
        validate_identifiers(owner, "reasoningModes", self.reasoning_modes.iter())?;
        validate_identifiers(owner, "skills", self.skills.iter())?;
        for (connection, tools) in &self.mcp_tools {
            validate_identifier(owner, "mcpTools connection", connection)?;
            validate_identifiers(owner, "mcpTools tool", tools.iter())?;
        }
        Ok(())
    }

    pub fn first_capability_outside(&self, available: &Self) -> Option<String> {
        self.models
            .difference(&available.models)
            .next()
            .map(|value| format!("model `{value}`"))
            .or_else(|| {
                self.reasoning_modes
                    .difference(&available.reasoning_modes)
                    .next()
                    .map(|value| format!("reasoning mode `{value}`"))
            })
            .or_else(|| {
                self.sandbox_modes
                    .difference(&available.sandbox_modes)
                    .next()
                    .map(|value| format!("sandbox mode `{value:?}`"))
            })
            .or_else(|| {
                self.mcp_tools.iter().find_map(|(connection, requested)| {
                    let available_tools = available.mcp_tools.get(connection)?;
                    requested
                        .difference(available_tools)
                        .next()
                        .map(|tool| format!("MCP tool `{connection}/{tool}`"))
                })
            })
            .or_else(|| {
                self.mcp_tools
                    .keys()
                    .find(|connection| !available.mcp_tools.contains_key(*connection))
                    .map(|connection| format!("MCP connection `{connection}`"))
            })
            .or_else(|| {
                self.skills
                    .difference(&available.skills)
                    .next()
                    .map(|value| format!("skill `{value}`"))
            })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSelections {
    pub model: Option<String>,
    pub reasoning_mode: Option<String>,
    pub sandbox_mode: Option<SandboxMode>,
}

impl RuntimeSelections {
    pub fn validate(&self, owner: &str) -> Result<(), String> {
        if let Some(model) = &self.model {
            validate_identifier(owner, "selected model", model)?;
        }
        if let Some(reasoning_mode) = &self.reasoning_mode {
            validate_identifier(owner, "selected reasoning mode", reasoning_mode)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeProfileSnapshot {
    pub contract_version: u32,
    pub profile_ref: String,
    pub exposure: CapabilitySet,
    pub locked: RuntimeSelections,
    #[serde(default)]
    pub codex_personality: Option<CodexPersonality>,
}

impl RuntimeProfileSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        if self.contract_version != RUNTIME_PROFILE_CONTRACT_VERSION {
            return Err(format!(
                "Runtime profile contract version {} is unsupported",
                self.contract_version
            ));
        }
        validate_identifier("runtime profile", "profileRef", &self.profile_ref)?;
        self.exposure.validate("runtime profile exposure")?;
        self.locked.validate("runtime profile locked selections")?;
        validate_selection_availability(&self.locked, &self.exposure)
            .map_err(|capability| format!("Runtime profile locks unavailable {capability}"))
    }
}

pub fn validate_selection_availability(
    selections: &RuntimeSelections,
    capabilities: &CapabilitySet,
) -> Result<(), String> {
    if let Some(model) = &selections.model {
        if !capabilities.models.contains(model) {
            return Err(format!("model `{model}`"));
        }
    }
    if let Some(reasoning_mode) = &selections.reasoning_mode {
        if !capabilities.reasoning_modes.contains(reasoning_mode) {
            return Err(format!("reasoning mode `{reasoning_mode}`"));
        }
    }
    if let Some(sandbox_mode) = selections.sandbox_mode {
        if !capabilities.sandbox_modes.contains(&sandbox_mode) {
            return Err(format!("sandbox mode `{sandbox_mode:?}`"));
        }
    }
    Ok(())
}

pub fn validate_identifier(owner: &str, field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.trim() != value {
        Err(format!("{owner} {field} must be a non-empty trimmed value"))
    } else {
        Ok(())
    }
}

fn validate_identifiers<'a>(
    owner: &str,
    field: &str,
    values: impl Iterator<Item = &'a String>,
) -> Result<(), String> {
    for value in values {
        validate_identifier(owner, field, value)?;
    }
    Ok(())
}
