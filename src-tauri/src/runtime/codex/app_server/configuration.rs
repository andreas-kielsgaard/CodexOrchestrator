//! Provider launch configuration and support checks. No arbitrary process arguments cross the port.
use crate::agent_sessions::ports::{
    RuntimeLaunchExtension, RuntimePortError, RuntimePortErrorKind,
};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn thread_configuration(
    connection: &super::connection::Connection,
    extension: Option<&RuntimeLaunchExtension>,
    cwd: Option<&str>,
) -> Result<Value, RuntimePortError> {
    let Some(extension) = extension.filter(|e| !e.managed_mcp_servers.is_empty()) else {
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
    for server in &extension.managed_mcp_servers {
        if server.name.is_empty()
            || !server
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(unsupported("Invalid product MCP server name"));
        }
        if native.get(&server.name).is_some() || !names.insert(&server.name) {
            return Err(unsupported(&format!("Product MCP server '{}' conflicts with an existing native or product connection. Rename that connection before launching.",server.name)));
        }
        result.insert(
            format!("mcp_servers.{}", server.name),
            managed_server_config(server),
        );
    }
    Ok(Value::Object(result))
}

pub(in crate::runtime::codex) fn managed_server_config(
    server: &crate::agent_sessions::ports::RuntimeManagedMcpServer,
) -> Value {
    json!({"url":server.url,"required":true,"default_tools_approval_mode":"approve","startup_timeout_sec":10,"tool_timeout_sec":300})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::ports::RuntimeManagedMcpServer;
    #[test]
    fn managed_mcp_is_additive_and_collisions_fail_without_exposing_connection_data() {
        let extension = RuntimeLaunchExtension {
            managed_mcp_servers: vec![RuntimeManagedMcpServer {
                name: "workflow".into(),
                url: "http://localhost/private-token".into(),
            }],
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
}

pub(super) fn arguments(
    extension: Option<&RuntimeLaunchExtension>,
    cwd: &Path,
) -> Result<Vec<String>, RuntimePortError> {
    let mut args = vec!["app-server".into()];
    let Some(extension) = extension else {
        return Ok(args);
    };
    for value in &extension.config_overrides {
        if value.starts_with('-')
            || !value
                .split_once('=')
                .is_some_and(|(key, _)| !key.trim().is_empty())
        {
            return Err(unsupported("Invalid native configuration assignment"));
        }
        args.extend(["-c".into(), value.clone()]);
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
    requested: Option<crate::agent_sessions::domain::RuntimeSandboxMode>,
    resolved: &Value,
) -> Result<(), RuntimePortError> {
    use crate::agent_sessions::domain::RuntimeSandboxMode;
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
        use crate::agent_sessions::domain::RuntimeSandboxMode::WorkspaceWrite;
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
