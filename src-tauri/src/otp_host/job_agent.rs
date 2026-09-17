use super::installations::{JobAgentInstallation, OtpInstallationService};
use crate::{
    agent_sessions::domain::AgentSessionId,
    execution_configuration::SessionCreationResolution,
    harness_engine::{
        ManagedMcpUpstreamDescriptor, ManagedMcpUpstreamOwner, ManagedMcpUpstreamRegistry,
    },
    otp_host::OtpRegistry,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::Arc,
};

pub(crate) trait AgentMcpUpstreamProvisioner: Send + Sync {
    fn provision(
        &self,
        session_id: &AgentSessionId,
        profile: &SessionCreationResolution,
        upstreams: &ManagedMcpUpstreamRegistry,
    ) -> Result<(), String>;
}

pub(crate) struct JobAgentMcpProvisioner {
    registry: Arc<OtpRegistry>,
    installations: Arc<OtpInstallationService>,
}

impl JobAgentMcpProvisioner {
    pub(crate) fn new(
        registry: Arc<OtpRegistry>,
        installations: Arc<OtpInstallationService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            registry,
            installations,
        })
    }
}

impl AgentMcpUpstreamProvisioner for JobAgentMcpProvisioner {
    fn provision(
        &self,
        session_id: &AgentSessionId,
        profile: &SessionCreationResolution,
        upstreams: &ManagedMcpUpstreamRegistry,
    ) -> Result<(), String> {
        let tools = profile
            .session_profile()
            .node_capabilities()
            .mcp_tools
            .get("job_agent")
            .cloned()
            .unwrap_or_default();
        if tools.is_empty() {
            return Ok(());
        }
        if upstreams
            .resolve_scoped(session_id.as_str(), "job_agent")
            .is_ok()
        {
            return Ok(());
        }
        let installation = self.installations.verified_installation()?;
        let (_, descriptor) = self.registry.agent_mcp_server("job_agent")?;
        let known = descriptor
            .tools
            .iter()
            .map(|tool| (tool.id.as_str(), tool.capability.as_str()))
            .collect::<BTreeMap<_, _>>();
        let capabilities = tools
            .iter()
            .map(|tool| {
                known
                    .get(tool.as_str())
                    .copied()
                    .ok_or_else(|| format!("Unknown Job Agent MCP tool {tool}"))
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let configuration = profile
            .session_profile()
            .agent_mcp_configuration()
            .get("job_agent")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get("job_agent"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        self.registry
            .validate_agent_mcp_configuration("job_agent", "job_agent", &configuration)?;
        let grants = crate::otp_packages::job_agent::grants_from_configuration(&configuration)?;
        let upstream = start_host(&installation, capabilities, grants)?;
        let registration = upstreams.register_for_session(session_id.as_str(), upstream.0)?;
        if let Err(owner) = upstreams.retain_owner(&registration, upstream.1) {
            owner.stop();
            upstreams.unregister(&registration);
            return Err("Unable to retain Job Agent session MCP host.".into());
        }
        Ok(())
    }
}

fn start_host(
    installation: &JobAgentInstallation,
    capabilities: BTreeSet<&str>,
    grants: Vec<(String, String)>,
) -> Result<
    (
        ManagedMcpUpstreamDescriptor,
        Box<dyn ManagedMcpUpstreamOwner>,
    ),
    String,
> {
    let root = installation.root.trim();
    let mut child = Command::new(installation.python.trim())
        .args(["-m", "job_agent.mcp.orchid_host", "serve"])
        .current_dir(root)
        .env(
            "PYTHONPATH",
            std::path::Path::new(root).join("app").join("code"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Unable to start Job Agent MCP host: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("Job Agent MCP host has no input.")?;
    let mutation_grants = grants
        .into_iter()
        .fold(
            BTreeMap::<String, Vec<String>>::new(),
            |mut values, (capability, transition)| {
                values.entry(capability).or_default().push(transition);
                values
            },
        )
        .into_iter()
        .map(|(capability, transitions)| json!({"capability":capability,"transitions":transitions}))
        .collect::<Vec<_>>();
    writeln!(
        stdin,
        "{}",
        json!({
            "root":root,
            "capabilities":capabilities,
            "mutationGrants":mutation_grants,
        })
    )
    .map_err(|error| format!("Unable to configure Job Agent MCP host: {error}"))?;
    stdin.flush().map_err(|error| error.to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Job Agent MCP host has no output.")?;
    let mut ready = String::new();
    BufReader::new(stdout)
        .read_line(&mut ready)
        .map_err(|error| format!("Unable to read Job Agent MCP host: {error}"))?;
    let value: Value = serde_json::from_str(&ready)
        .map_err(|error| format!("Job Agent MCP host returned invalid startup data: {error}"))?;
    if value.get("kind").and_then(Value::as_str) != Some("ready")
        || value.get("serverName").and_then(Value::as_str) != Some("job_agent")
    {
        return Err(value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Job Agent MCP host did not start.")
            .into());
    }
    let descriptor = ManagedMcpUpstreamDescriptor {
        name: "job_agent".into(),
        url: value
            .get("url")
            .and_then(Value::as_str)
            .ok_or("Job Agent MCP host did not provide its URL.")?
            .into(),
        bearer_token: value
            .get("bearerToken")
            .and_then(Value::as_str)
            .ok_or("Job Agent MCP host did not provide its bearer token.")?
            .into(),
        caller_context: false,
    };
    Ok((
        descriptor,
        Box::new(JobAgentHostOwner {
            child,
            stdin: Some(stdin),
        }),
    ))
}

struct JobAgentHostOwner {
    child: Child,
    stdin: Option<ChildStdin>,
}

impl ManagedMcpUpstreamOwner for JobAgentHostOwner {
    fn stop(mut self: Box<Self>) {
        self.stdin.take();
        let _ = self.child.wait();
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
    }
}
