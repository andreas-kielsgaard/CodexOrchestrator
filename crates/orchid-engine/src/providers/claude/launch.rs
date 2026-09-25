//! Claude translation of Orchid's launch intent. No arbitrary process arguments cross the port.
use crate::contracts::{
    domain::RuntimeSandboxMode,
    ports::{
        RuntimeApprovalIntent, RuntimeInvocationRequest, RuntimeLaunchExtension, RuntimePortError,
        RuntimePortErrorKind,
    },
};
use serde_json::{json, Map, Value};

/// Which native conversation the process continues.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Conversation {
    /// A new conversation under an Orchid-chosen session ID.
    New(String),
    Resume(String),
}

impl Conversation {
    pub fn session_id(&self) -> &str {
        match self {
            Self::New(id) | Self::Resume(id) => id,
        }
    }
}

/// Arguments for one invocation's `claude` process.
pub fn arguments(
    request: &RuntimeInvocationRequest,
    conversation: &Conversation,
) -> Result<Vec<String>, RuntimePortError> {
    let mut args: Vec<String> = [
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        // Echo each user message when Claude takes it into the conversation, so steering knows
        // which turn answers it.
        "--replay-user-messages",
        "--permission-prompt-tool",
        "stdio",
    ]
    .map(String::from)
    .into();
    match conversation {
        Conversation::New(id) => args.extend(["--session-id".into(), id.clone()]),
        Conversation::Resume(id) => args.extend(["--resume".into(), id.clone()]),
    }
    if let Some(model) = &request.options.model {
        args.extend(["--model".into(), model.clone()]);
    }
    match request.options.sandbox {
        None => {}
        Some(RuntimeSandboxMode::DangerFullAccess) => {
            args.extend(["--permission-mode".into(), "bypassPermissions".into()])
        }
        Some(mode) => {
            return Err(unsupported(&format!(
                "Claude Code offers full access only; the requested {mode:?} access mode is unavailable."
            )))
        }
    }
    let Some(extension) = request.launch_extension.as_ref() else {
        return Ok(args);
    };
    if let Some(effort) = &extension.reasoning_mode {
        args.extend(["--effort".into(), effort.clone()]);
    }
    if extension.approval == RuntimeApprovalIntent::Unattended {
        args.extend(["--permission-prompts".into(), "none".into()]);
    }
    if extension.native_mcp_enabled == Some(false) {
        args.push("--strict-mcp-config".into());
    }
    if extension.ignore_user_rules {
        // No settings files: neither user nor project permission rules apply.
        args.extend(["--setting-sources".into(), String::new()]);
    }
    if !extension.managed_mcp_servers.is_empty() {
        args.extend(["--mcp-config".into(), mcp_config(extension).to_string()]);
        args.push("--allowedTools".into());
        args.push(managed_tools(extension).join(","));
    }
    Ok(args)
}

/// Process environment: the setup's environment plus managed server bearers.
pub fn environment(extension: Option<&RuntimeLaunchExtension>) -> Vec<(String, String)> {
    let Some(extension) = extension else {
        return Vec::new();
    };
    let mut environment = extension.environment.clone();
    environment
        .extend(bearers(extension).map(|(_, variable, bearer)| (variable, bearer.to_owned())));
    environment
}

/// A process started from inside a Claude Code session would otherwise attach to it.
pub fn parent_session_variables() -> Vec<String> {
    [
        "CLAUDECODE",
        "CLAUDE_CODE_ENTRYPOINT",
        "CLAUDE_CODE_SESSION_ID",
        "CLAUDE_CODE_CHILD_SESSION",
        "CLAUDE_CODE_SSE_PORT",
    ]
    .map(String::from)
    .into()
}

/// Managed servers that carry a bearer, with the variable holding it. Claude expands the variable
/// in the server's header, so the secret never appears in the process arguments.
fn bearers(extension: &RuntimeLaunchExtension) -> impl Iterator<Item = (&str, String, &str)> {
    extension
        .managed_mcp_servers
        .iter()
        .filter_map(|server| Some((server.name.as_str(), server.bearer_token.as_deref()?)))
        .enumerate()
        .map(|(index, (name, bearer))| (name, format!("ORCHID_MCP_BEARER_{index}"), bearer))
}

fn mcp_config(extension: &RuntimeLaunchExtension) -> Value {
    let variables: Vec<_> = bearers(extension)
        .map(|(name, variable, _)| (name, variable))
        .collect();
    let mut servers = Map::new();
    for server in &extension.managed_mcp_servers {
        let mut config = json!({"type": "http", "url": server.url});
        if let Some((_, variable)) = variables.iter().find(|(name, _)| *name == server.name) {
            config["headers"] = json!({"Authorization": format!("Bearer ${{{variable}}}")});
        }
        servers.insert(server.name.clone(), config);
    }
    json!({ "mcpServers": servers })
}

/// Managed tools never ask for approval. A server without a tool list is allowed whole.
fn managed_tools(extension: &RuntimeLaunchExtension) -> Vec<String> {
    extension
        .managed_mcp_servers
        .iter()
        .flat_map(|server| match &server.enabled_tools {
            Some(tools) => tools
                .iter()
                .map(|tool| format!("mcp__{}__{tool}", server.name))
                .collect(),
            None => vec![format!("mcp__{}", server.name)],
        })
        .collect()
}

/// Managed servers the invocation cannot run without.
pub(super) fn required_servers(request: &RuntimeInvocationRequest) -> Vec<String> {
    request
        .launch_extension
        .iter()
        .flat_map(|extension| &extension.managed_mcp_servers)
        .filter(|server| server.required)
        .map(|server| server.name.clone())
        .collect()
}

fn unsupported(message: &str) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::UnsupportedOptions, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        domain::{AgentInvocationId, AgentRuntimeOptions, AgentSessionId},
        ports::RuntimeManagedMcpServer,
    };

    fn request(extension: Option<RuntimeLaunchExtension>) -> RuntimeInvocationRequest {
        RuntimeInvocationRequest {
            session_id: AgentSessionId::new("session").unwrap(),
            invocation_id: AgentInvocationId::new("invocation").unwrap(),
            submitted_text: "hello".into(),
            working_directory: Some("/work".into()),
            options: AgentRuntimeOptions {
                model: Some("sonnet".into()),
                sandbox: Some(RuntimeSandboxMode::DangerFullAccess),
            },
            launch_extension: extension,
        }
    }

    #[test]
    fn translates_each_requested_intent() {
        let extension = RuntimeLaunchExtension {
            reasoning_mode: Some("high".into()),
            approval: RuntimeApprovalIntent::Unattended,
            native_mcp_enabled: Some(false),
            ignore_user_rules: true,
            ..Default::default()
        };
        let args = arguments(
            &request(Some(extension)),
            &Conversation::Resume("id".into()),
        )
        .unwrap();
        let joined = args.join(" ");
        for expected in [
            "--resume id",
            "--model sonnet",
            "--permission-mode bypassPermissions",
            "--effort high",
            "--permission-prompts none",
            "--strict-mcp-config",
        ] {
            assert!(
                joined.contains(expected),
                "{expected} missing from {joined}"
            );
        }
        let sources = args
            .iter()
            .position(|arg| arg == "--setting-sources")
            .unwrap();
        assert_eq!(args[sources + 1], "");
        assert!(!joined.contains("--session-id"));
    }

    #[test]
    fn restricted_access_modes_are_unsupported() {
        let mut request = request(None);
        request.options.sandbox = Some(RuntimeSandboxMode::WorkspaceWrite);
        let error = arguments(&request, &Conversation::New("id".into())).unwrap_err();
        assert_eq!(error.kind, RuntimePortErrorKind::UnsupportedOptions);
    }

    #[test]
    fn managed_servers_carry_their_bearer_only_through_the_environment() {
        let server = |name: &str, bearer: Option<&str>, tools: Option<Vec<String>>| {
            RuntimeManagedMcpServer {
                name: name.into(),
                url: format!("http://127.0.0.1/{name}"),
                bearer_token: bearer.map(Into::into),
                enabled_tools: tools,
                required: true,
            }
        };
        let extension = RuntimeLaunchExtension {
            managed_mcp_servers: vec![
                server("harness", None, None),
                server(
                    "plan_builder_1",
                    Some("secret"),
                    Some(vec!["submit".into()]),
                ),
            ],
            environment: vec![("CLAUDE_CONFIG_DIR".into(), "/config".into())],
            ..Default::default()
        };
        let args = arguments(
            &request(Some(extension.clone())),
            &Conversation::New("id".into()),
        )
        .unwrap();
        let config: Value =
            serde_json::from_str(&args[args.iter().position(|a| a == "--mcp-config").unwrap() + 1])
                .unwrap();
        assert_eq!(
            config["mcpServers"]["plan_builder_1"],
            json!({"type":"http","url":"http://127.0.0.1/plan_builder_1","headers":{"Authorization":"Bearer ${ORCHID_MCP_BEARER_0}"}})
        );
        assert!(config["mcpServers"]["harness"].get("headers").is_none());
        let allowed = &args[args.iter().position(|a| a == "--allowedTools").unwrap() + 1];
        assert_eq!(allowed, "mcp__harness,mcp__plan_builder_1__submit");
        assert!(!args.join(" ").contains("secret"));
        assert_eq!(
            environment(Some(&extension)),
            [
                ("CLAUDE_CONFIG_DIR".to_string(), "/config".to_string()),
                ("ORCHID_MCP_BEARER_0".to_string(), "secret".to_string()),
            ]
        );
    }
}
