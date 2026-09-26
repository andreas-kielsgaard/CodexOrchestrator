//! Codex translation of Orchid's launch intent. No arbitrary process arguments cross the port.
//!
//! Managed MCP servers and native MCP suppression are thread configuration; the remaining
//! intents are process-level `-c` values. The test-only CLI runtime applies the same values.
use crate::contracts::ports::{
    RuntimeApprovalIntent, RuntimeLaunchExtension, RuntimeManagedMcpServer, RuntimePortError,
    RuntimePortErrorKind,
};
use serde_json::{json, Value};
use std::path::Path;

/// Environment variable carrying the bearer of the `index`th managed server that has one.
fn bearer_variable(index: usize) -> String {
    format!("ORCHID_MCP_BEARER_{index}")
}

fn bearer_variables(
    extension: &RuntimeLaunchExtension,
) -> impl Iterator<Item = (&RuntimeManagedMcpServer, String, &str)> {
    extension
        .managed_mcp_servers
        .iter()
        .filter_map(|server| server.bearer_token.as_deref().map(|bearer| (server, bearer)))
        .enumerate()
        .map(|(index, (server, bearer))| (server, bearer_variable(index), bearer))
}

/// Process environment for one launch: the provider-prepared environment plus managed bearers.
pub fn launch_environment(extension: Option<&RuntimeLaunchExtension>) -> Vec<(String, String)> {
    let Some(extension) = extension else {
        return Vec::new();
    };
    let mut environment = extension.environment.clone();
    environment.extend(
        bearer_variables(extension).map(|(_, variable, bearer)| (variable, bearer.to_owned())),
    );
    environment
}

/// Native configuration for one managed server. Managed tools never prompt for approval.
pub fn managed_server_config(
    server: &RuntimeManagedMcpServer,
    bearer_variable: Option<&str>,
) -> Value {
    let mut config = json!({
        "url": server.url,
        "required": server.required,
        "default_tools_approval_mode": "approve",
        "startup_timeout_sec": 10,
        "tool_timeout_sec": 300,
    });
    if let Some(variable) = bearer_variable {
        config["bearer_token_env_var"] = variable.into();
    }
    if let Some(tools) = &server.enabled_tools {
        config["enabled_tools"] = json!(tools);
    }
    config
}

/// Managed servers keyed by native name, each naming its bearer variable when it has one.
pub fn managed_servers(extension: &RuntimeLaunchExtension) -> Vec<(&str, Value)> {
    let bearers = bearer_variables(extension)
        .map(|(server, variable, _)| (server.name.as_str(), variable))
        .collect::<Vec<_>>();
    extension
        .managed_mcp_servers
        .iter()
        .map(|server| {
            let variable = bearers
                .iter()
                .find(|(name, _)| *name == server.name)
                .map(|(_, variable)| variable.as_str());
            (server.name.as_str(), managed_server_config(server, variable))
        })
        .collect()
}

pub(super) fn thread_configuration(
    connection: &super::connection::Connection,
    extension: Option<&RuntimeLaunchExtension>,
    cwd: Option<&str>,
) -> Result<Value, RuntimePortError> {
    let Some(extension) = extension.filter(|e| !e.managed_mcp_servers.is_empty() || e.native_mcp_enabled == Some(false)) else {
        return Ok(json!({}));
    };
    let native = connection.call("config/read", json!({"cwd":cwd,"includeLayers":false}))?;
    merge_managed_servers(&native["config"]["mcp_servers"], extension)
}

fn merge_managed_servers(
    native: &Value,
    extension: &RuntimeLaunchExtension,
) -> Result<Value, RuntimePortError> {
    let mut result = serde_json::Map::new();
    let mut names = std::collections::HashSet::new();
    if extension.native_mcp_enabled == Some(false) {
        if let Some(servers) = native.as_object() {
            for name in servers.keys() {
                if !valid_server_name(name) {
                    return Err(unsupported("Cannot safely mask a native MCP server with an unsupported name"));
                }
                result.insert(format!("mcp_servers.{name}.enabled"), json!(false));
            }
        }
    }
    for (name, config) in managed_servers(extension) {
        if !valid_server_name(name) {
            return Err(unsupported("Invalid product MCP server name"));
        }
        if native.get(name).is_some() || !names.insert(name) {
            return Err(unsupported(&format!("Product MCP server '{name}' conflicts with an existing native or product connection. Rename that connection before launching.")));
        }
        result.insert(format!("mcp_servers.{name}"), config);
    }
    Ok(Value::Object(result))
}

fn valid_server_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Process-level `-c` values for intents Codex resolves when the app-server starts.
pub fn process_overrides(extension: &RuntimeLaunchExtension, cwd: Option<&Path>) -> Vec<String> {
    let mut values = Vec::new();
    if extension.approval == RuntimeApprovalIntent::Unattended {
        values.push("approval_policy=\"never\"".to_owned());
    }
    if let Some(cwd) = cwd.filter(|_| extension.trusted_workspace) {
        values.push(workspace_trust(cwd));
    }
    if extension.sandbox_network_access {
        // Codex 0.144 reaches a token-protected loopback MCP transport from WorkspaceWrite only
        // through its network proxy.
        values.push("sandbox_workspace_write.network_access=true".to_owned());
        values.push("features.network_proxy=true".to_owned());
    }
    values
}

/// A private CODEX_HOME has no trust record for a just-created isolated worktree, and Codex then
/// reduces a requested WorkspaceWrite with approval `never` to read-only. This ephemeral
/// exact-project override neither persists trust nor widens the workspace boundary. Codex keys
/// Windows project trust by the lower-case path.
fn workspace_trust(cwd: &Path) -> String {
    let normalized = cwd.to_string_lossy().to_ascii_lowercase();
    let mut encoded = String::with_capacity(normalized.len());
    for character in normalized.chars() {
        match character {
            '\'' => encoded.push_str("''"),
            '\n' | '\r' | '\t' => encoded.push(' '),
            value => encoded.push(value),
        }
    }
    format!("projects.'{encoded}'.trust_level=\"trusted\"")
}

pub(super) fn apply_resume_reasoning(config: &mut Value, resolved_effort: &Value) {
    // Native defaults can return null. Codex 0.144 interprets a null config override as an
    // invalid empty effort; omission preserves native resolution or an explicit selection.
    if let Some(effort) = resolved_effort.as_str().filter(|value| !value.is_empty()) {
        config["model_reasoning_effort"] = effort.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(name: &str, url: &str) -> RuntimeManagedMcpServer {
        RuntimeManagedMcpServer {
            name: name.into(),
            url: url.into(),
            bearer_token: None,
            enabled_tools: None,
            required: true,
        }
    }

    #[test]
    fn managed_servers_carry_their_bearer_only_through_the_process_environment() {
        let extension = RuntimeLaunchExtension {
            managed_mcp_servers: vec![
                server("harness", "http://127.0.0.1:1/proxy/token/0"),
                RuntimeManagedMcpServer {
                    bearer_token: Some("secret".into()),
                    enabled_tools: Some(vec!["submit".into()]),
                    required: false,
                    ..server("plan_builder_1", "http://127.0.0.1:2/mcp")
                },
            ],
            environment: vec![("CODEX_HOME".into(), "home".into())],
            ..Default::default()
        };
        let config = merge_managed_servers(&json!({}), &extension).unwrap();
        assert_eq!(
            config["mcp_servers.plan_builder_1"],
            json!({
                "url": "http://127.0.0.1:2/mcp",
                "bearer_token_env_var": "ORCHID_MCP_BEARER_0",
                "enabled_tools": ["submit"],
                "required": false,
                "default_tools_approval_mode": "approve",
                "startup_timeout_sec": 10,
                "tool_timeout_sec": 300
            })
        );
        assert!(config["mcp_servers.harness"].get("bearer_token_env_var").is_none());
        assert!(config["mcp_servers.harness"].get("enabled_tools").is_none());
        assert!(!config.to_string().contains("secret"));
        assert_eq!(
            launch_environment(Some(&extension)),
            [
                ("CODEX_HOME".to_string(), "home".to_string()),
                ("ORCHID_MCP_BEARER_0".to_string(), "secret".to_string()),
            ]
        );
        assert!(!format!("{:?}", extension.managed_mcp_servers).contains("secret"));
    }

    #[test]
    fn process_overrides_translate_only_requested_intents() {
        assert!(process_overrides(&RuntimeLaunchExtension::default(), Some(Path::new("C:/w"))).is_empty());
        let extension = RuntimeLaunchExtension {
            approval: RuntimeApprovalIntent::Unattended,
            trusted_workspace: true,
            sandbox_network_access: true,
            ..Default::default()
        };
        assert_eq!(
            process_overrides(&extension, Some(Path::new(r"C:\Isolated\it's"))),
            [
                r#"approval_policy="never""#,
                r#"projects.'c:\isolated\it''s'.trust_level="trusted""#,
                "sandbox_workspace_write.network_access=true",
                "features.network_proxy=true",
            ]
        );
    }

    #[test]
    fn resume_omits_absent_native_effort_and_preserves_explicit_selection() {
        let mut inherited = json!({});
        apply_resume_reasoning(&mut inherited, &Value::Null);
        assert!(inherited.get("model_reasoning_effort").is_none());

        let mut explicit = json!({"model_reasoning_effort":"high"});
        apply_resume_reasoning(&mut explicit, &Value::Null);
        assert_eq!(explicit["model_reasoning_effort"], "high");

        apply_resume_reasoning(&mut inherited, &json!("medium"));
        assert_eq!(inherited["model_reasoning_effort"], "medium");
    }
    #[test]
    fn managed_mcp_is_additive_and_collisions_fail_without_exposing_connection_data() {
        let extension = RuntimeLaunchExtension {
            managed_mcp_servers: vec![server("workflow", "http://localhost/private-token")],
            ..Default::default()
        };
        let config =
            merge_managed_servers(&json!({"native":{"url":"secret"}}), &extension).unwrap();
        assert!(config.get("mcp_servers.workflow").is_some());
        assert!(config.get("mcp_servers").is_none());
        let error =
            merge_managed_servers(&json!({"workflow":{"url":"secret"}}), &extension).unwrap_err();
        assert!(error.message.contains("conflicts"));
        assert!(!error.message.contains("secret"));
        assert!(!error.message.contains("private-token"));
    }

    #[test]
    fn enabled_native_mcp_remains_codex_owned_while_managed_servers_are_additive() {
        let extension = RuntimeLaunchExtension {
            native_mcp_enabled: Some(true),
            managed_mcp_servers: vec![server("orchid", "http://localhost/owned")],
            ..Default::default()
        };

        let config =
            merge_managed_servers(&json!({"native":{"command":"provider-owned"}}), &extension)
                .unwrap();

        assert_eq!(config.as_object().map(|values| values.len()), Some(1));
        assert!(config.get("mcp_servers.orchid").is_some());
        assert!(config.get("mcp_servers.native").is_none());
        assert!(!config.to_string().contains("provider-owned"));
    }

    #[test]
    fn disabled_native_mcp_group_masks_native_servers_but_retains_managed_servers() {
        let extension = RuntimeLaunchExtension {
            native_mcp_enabled: Some(false),
            managed_mcp_servers: vec![server("orchid", "http://localhost/owned")],
            ..Default::default()
        };
        let config = merge_managed_servers(&json!({"native":{"command":"secret"}}), &extension).unwrap();
        assert_eq!(config["mcp_servers.native.enabled"], false);
        assert!(config.get("mcp_servers.orchid").is_some());
        assert!(!config.to_string().contains("secret"));
    }
}

pub(super) fn arguments(
    extension: Option<&RuntimeLaunchExtension>,
    cwd: &Path,
) -> Result<Vec<String>, RuntimePortError> {
    let mut args = vec!["app-server".into()];
    let Some(extension) = extension else {
        return Ok(args);
    };
    for value in process_overrides(extension, Some(cwd)) {
        args.extend(["-c".into(), value]);
    }
    if extension.ignore_user_rules {
        // CLI 0.144 exposes --ignore-rules only to exec. The existing Implementer contract can
        // run through app-server only when ignoring user/project rules is demonstrably a no-op.
        let home = extension
            .environment
            .iter()
            .find(|(name, _)| name == "CODEX_HOME")
            .map(|(_, path)| Path::new(path))
            .ok_or_else(|| unsupported("Rule discovery requires the attached native home"))?;
        let mut roots = vec![home.join("rules")];
        roots.extend(cwd.ancestors().map(|root| root.join(".codex/rules")));
        for root in roots {
            match std::fs::read_dir(&root) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = entry.map_err(|e| {
                            unsupported(&format!("Cannot inspect native rules: {e}"))
                        })?;
                        if entry
                            .path()
                            .extension()
                            .is_some_and(|extension| extension == "rules")
                        {
                            return Err(unsupported("This Harness requires ignoring user/project rules, but the selected Codex app-server cannot do that. Its execution policy has been preserved and this launch was not started."));
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(unsupported(&format!(
                        "Cannot inspect native rules: {error}"
                    )))
                }
            }
        }
    }
    Ok(args)
}

fn unsupported(message: &str) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::UnsupportedOptions, message)
}

pub(super) fn validate_effective_sandbox(
    requested: Option<crate::contracts::domain::RuntimeSandboxMode>,
    resolved: &Value,
) -> Result<(), RuntimePortError> {
    use crate::contracts::domain::RuntimeSandboxMode;
    let expected = match requested {
        None => return Ok(()),
        Some(RuntimeSandboxMode::ReadOnly) => "readOnly",
        Some(RuntimeSandboxMode::WorkspaceWrite) => "workspaceWrite",
        Some(RuntimeSandboxMode::DangerFullAccess) => "dangerFullAccess",
    };
    if resolved["type"].as_str() != Some(expected) {
        let setup_hint = if cfg!(windows)
            && expected == "workspaceWrite"
            && resolved["type"] == "readOnly"
        {
            " On Windows, complete Codex sandbox setup in the attached native profile, then resend the message."
        } else {
            " Check the sandbox configuration in the attached native profile, then resend the message."
        };
        return Err(unsupported(&format!(
            "Codex resolved the requested {expected} sandbox to {}. No turn was started.{setup_hint}",
            resolved["type"].as_str().unwrap_or("an unknown policy")
        )));
    }
    Ok(())
}

#[cfg(test)]
mod sandbox_tests {
    use super::*;
    #[test]
    fn explicit_sandbox_must_be_honored_but_inherited_native_policy_is_preserved() {
        use crate::contracts::domain::RuntimeSandboxMode::WorkspaceWrite;
        assert!(
            validate_effective_sandbox(Some(WorkspaceWrite), &json!({"type":"readOnly"})).is_err()
        );
        assert!(validate_effective_sandbox(
            Some(WorkspaceWrite),
            &json!({"type":"workspaceWrite"})
        )
        .is_ok());
        assert!(validate_effective_sandbox(None, &json!({"type":"customNativePolicy"})).is_ok());
    }
}
