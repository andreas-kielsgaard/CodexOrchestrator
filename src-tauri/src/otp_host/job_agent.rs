use super::installations::{JobAgentInstallation, OtpInstallationService};
use crate::{
    agent_sessions::domain::AgentSessionId,
    execution_configuration::SessionCreationResolution,
    harness_engine::{
        AgentMcpUpstreamProvisioner, ManagedMcpUpstreamDescriptor, ManagedMcpUpstreamOwner,
        ManagedMcpUpstreamRegistry,
    },
    otp_api::AgentMcpToolDescriptor,
    otp_host::OtpRegistry,
};
use axum::body::Body;
use http_body_util::BodyExt;
use hyper::{header, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use tokio_util::sync::CancellationToken;

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
        let selected_tools = profile
            .session_profile()
            .node_capabilities()
            .mcp_tools
            .get("job_agent")
            .cloned()
            .unwrap_or_default();
        if selected_tools.is_empty()
            || upstreams
                .resolve_scoped(session_id.as_str(), "job_agent")
                .is_ok()
        {
            return Ok(());
        }

        let (_, descriptor) = self.registry.agent_mcp_server("job_agent")?;
        let known = descriptor
            .tools
            .iter()
            .map(|tool| (tool.id.as_str(), tool.capability.as_str()))
            .collect::<BTreeMap<_, _>>();
        let capabilities = selected_tools
            .iter()
            .map(|tool| {
                known
                    .get(tool.as_str())
                    .map(|capability| (*capability).to_owned())
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
        let tools = descriptor
            .tools
            .into_iter()
            .filter(|tool| selected_tools.contains(&tool.id))
            .collect::<Vec<_>>();
        let facade =
            JobAgentMcpFacade::start(self.installations.clone(), tools, capabilities, grants)?;
        let registration =
            upstreams.register_for_session(session_id.as_str(), facade.descriptor)?;
        if let Err(owner) = upstreams.retain_owner(&registration, facade.owner) {
            owner.stop();
            upstreams.unregister(&registration);
            return Err("Unable to retain Job Agent session MCP facade.".into());
        }
        Ok(())
    }
}

struct JobAgentMcpFacade {
    descriptor: ManagedMcpUpstreamDescriptor,
    owner: Box<dyn ManagedMcpUpstreamOwner>,
}

impl JobAgentMcpFacade {
    fn start(
        installations: Arc<OtpInstallationService>,
        tools: Vec<AgentMcpToolDescriptor>,
        capabilities: BTreeSet<String>,
        grants: Vec<(String, String)>,
    ) -> Result<Self, String> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("Unable to start Job Agent MCP facade: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("Unable to configure Job Agent MCP facade: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("Unable to identify Job Agent MCP facade: {error}"))?;
        let bearer = uuid::Uuid::new_v4().simple().to_string();
        let state = Arc::new(JobAgentFacadeState {
            installations,
            tools: tools
                .into_iter()
                .map(|tool| (tool.id.clone(), tool))
                .collect(),
            capabilities,
            grants,
            backend: Mutex::new(None),
        });
        let cancellation = CancellationToken::new();
        let cancel = cancellation.clone();
        let thread_state = state.clone();
        let thread_bearer = bearer.clone();
        let thread = thread::Builder::new()
            .name("job-agent-mcp-facade".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .expect("Job Agent MCP facade runtime");
                runtime.block_on(async move {
                    let listener = tokio::net::TcpListener::from_std(listener)
                        .expect("Job Agent MCP facade listener");
                    loop {
                        let accepted = tokio::select! {
                            _ = cancel.cancelled() => break,
                            accepted = listener.accept() => accepted,
                        };
                        let Ok((stream, _)) = accepted else { continue };
                        let state = thread_state.clone();
                        let bearer = thread_bearer.clone();
                        tokio::spawn(async move {
                            let service = service_fn(move |request| {
                                let state = state.clone();
                                let bearer = bearer.clone();
                                async move {
                                    Ok::<_, Infallible>(
                                        handle_facade(request, state, &bearer).await,
                                    )
                                }
                            });
                            let _ = http1::Builder::new()
                                .serve_connection(TokioIo::new(stream), service)
                                .await;
                        });
                    }
                });
            })
            .map_err(|error| format!("Unable to start Job Agent MCP facade: {error}"))?;
        Ok(Self {
            descriptor: ManagedMcpUpstreamDescriptor {
                name: "job_agent".into(),
                url: format!("http://{address}/mcp"),
                bearer_token: bearer,
                workflow_tool_name: None,
                workflow_prepare_url: None,
                caller_context: false,
            },
            owner: Box::new(JobAgentFacadeOwner {
                cancellation,
                thread: Some(thread),
                state,
            }),
        })
    }
}

struct JobAgentFacadeState {
    installations: Arc<OtpInstallationService>,
    tools: BTreeMap<String, AgentMcpToolDescriptor>,
    capabilities: BTreeSet<String>,
    grants: Vec<(String, String)>,
    backend: Mutex<Option<JobAgentBackend>>,
}

struct JobAgentBackend {
    upstream: ManagedMcpUpstreamDescriptor,
    owner: Box<dyn ManagedMcpUpstreamOwner>,
    session_id: Option<String>,
}

impl JobAgentFacadeState {
    fn start_backend_if_needed(&self) -> Result<ManagedMcpUpstreamDescriptor, String> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| "Job Agent MCP facade is unavailable.".to_string())?;
        if let Some(existing) = backend.as_ref() {
            return Ok(existing.upstream.clone());
        }
        let installation = self
            .installations
            .configured_installation()?
            .ok_or("Job Agent is not configured. Save its local installation configuration and retry this tool call.")?;
        let (upstream, owner) = start_host(
            &installation,
            self.capabilities.clone(),
            self.grants.clone(),
        )?;
        let descriptor = upstream.clone();
        *backend = Some(JobAgentBackend {
            upstream,
            owner,
            session_id: None,
        });
        Ok(descriptor)
    }

    async fn backend_session(
        &self,
    ) -> Result<(ManagedMcpUpstreamDescriptor, Option<String>), String> {
        let upstream = self.start_backend_if_needed()?;
        if let Some(session_id) = self
            .backend
            .lock()
            .map_err(|_| "Job Agent MCP facade is unavailable.".to_string())?
            .as_ref()
            .and_then(|backend| backend.session_id.clone())
        {
            return Ok((upstream, Some(session_id)));
        }
        let client = reqwest::Client::new();
        let response = client
            .post(&upstream.url)
            .bearer_auth(&upstream.bearer_token)
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .json(&json!({
                "jsonrpc": "2.0", "id": "orchid-job-agent-facade", "method": "initialize",
                "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                    "clientInfo": {"name": "Orchid Job Agent facade", "version": "1"}}
            }))
            .send()
            .await
            .map_err(|_| "Job Agent MCP is unavailable for this tool call.".to_string())?;
        if !response.status().is_success() {
            return Err("Job Agent MCP rejected initialization for this tool call.".into());
        }
        let session_id = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let mut initialized = client
            .post(&upstream.url)
            .bearer_auth(&upstream.bearer_token)
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        if let Some(session_id) = &session_id {
            initialized = initialized.header("mcp-session-id", session_id);
        }
        initialized
            .send()
            .await
            .map_err(|_| "Job Agent MCP is unavailable for this tool call.".to_string())?;
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| "Job Agent MCP facade is unavailable.".to_string())?;
        let backend = backend
            .as_mut()
            .ok_or("Job Agent MCP backend was released before this tool call.")?;
        backend.session_id = session_id.clone();
        Ok((backend.upstream.clone(), session_id))
    }

    async fn call(&self, name: &str, arguments: Value) -> Result<Value, String> {
        if !self.tools.contains_key(name) {
            return Err("Job Agent tool is not exposed to this session.".into());
        }
        let (upstream, session_id) = self.backend_session().await?;
        let client = reqwest::Client::new();
        let mut request = client
            .post(&upstream.url)
            .bearer_auth(&upstream.bearer_token)
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .json(&json!({
                "jsonrpc":"2.0", "id":"orchid-job-agent-call", "method":"tools/call",
                "params":{"name":name,"arguments":arguments}
            }));
        if let Some(session_id) = session_id {
            request = request.header("mcp-session-id", session_id);
        }
        let response = request
            .send()
            .await
            .map_err(|_| "Job Agent MCP is unavailable for this tool call.".to_string())?;
        if !response.status().is_success() {
            return Err("Job Agent MCP rejected this tool call.".into());
        }
        response
            .json::<Value>()
            .await
            .map_err(|_| "Job Agent MCP returned an unreadable result.".to_string())
    }
}

struct JobAgentFacadeOwner {
    cancellation: CancellationToken,
    thread: Option<thread::JoinHandle<()>>,
    state: Arc<JobAgentFacadeState>,
}

impl ManagedMcpUpstreamOwner for JobAgentFacadeOwner {
    fn stop(mut self: Box<Self>) {
        self.cancellation.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if let Ok(mut backend) = self.state.backend.lock() {
            if let Some(backend) = backend.take() {
                backend.owner.stop();
            }
        }
    }
}

async fn handle_facade<B>(
    request: Request<B>,
    state: Arc<JobAgentFacadeState>,
    bearer: &str,
) -> Response<Body>
where
    B: hyper::body::Body<Data = bytes::Bytes> + Send + 'static,
{
    if request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        != Some(format!("Bearer {bearer}").as_str())
    {
        return text(
            StatusCode::UNAUTHORIZED,
            "Job Agent MCP facade authorization failed",
        );
    }
    let body = match request.into_body().collect().await {
        Ok(body) => body.to_bytes(),
        Err(_) => return text(StatusCode::BAD_REQUEST, "Unreadable MCP request"),
    };
    let value: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(error) => return rpc_error(Value::Null, error.to_string()),
    };
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    match value.get("method").and_then(Value::as_str) {
        Some("initialize") => response(json!({
            "jsonrpc":"2.0", "id":id,
            "result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},
            "serverInfo":{"name":"job_agent","version":"1"}}
        })),
        Some("notifications/initialized") => text(StatusCode::ACCEPTED, ""),
        Some("tools/list") => response(json!({
            "jsonrpc":"2.0", "id":id,
            "result":{"tools": state.tools.values().map(|tool| json!({
                "name":tool.id,"title":tool.name,"description":tool.description,
                "inputSchema":tool.input_schema
            })).collect::<Vec<_>>()}
        })),
        Some("tools/call") => {
            let Some(name) = value.pointer("/params/name").and_then(Value::as_str) else {
                return tool_error(id, "Job Agent MCP call is missing a tool name.");
            };
            match state
                .call(
                    name,
                    value
                        .pointer("/params/arguments")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                )
                .await
            {
                Ok(mut result) => {
                    result["id"] = id;
                    response(result)
                }
                Err(error) => tool_error(id, &error),
            }
        }
        _ => rpc_error(id, "Unsupported MCP method".into()),
    }
}

fn response(value: Value) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .expect("JSON response")
}

fn rpc_error(id: Value, message: String) -> Response<Body> {
    response(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":message}}))
}

fn tool_error(id: Value, message: &str) -> Response<Body> {
    response(json!({
        "jsonrpc":"2.0", "id":id,
        "result":{"content":[{"type":"text","text":message}],"isError":true}
    }))
}

fn text(status: StatusCode, value: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(value.to_owned()))
        .expect("text response")
}

fn start_host(
    installation: &JobAgentInstallation,
    capabilities: BTreeSet<String>,
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
        .map_err(|_| "Unable to start Job Agent MCP for this tool call.".to_string())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("Job Agent MCP has no configuration input.")?;
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
        json!({"root":root,"capabilities":capabilities,"mutationGrants":mutation_grants})
    )
    .map_err(|_| "Unable to configure Job Agent MCP for this tool call.".to_string())?;
    stdin
        .flush()
        .map_err(|_| "Unable to configure Job Agent MCP for this tool call.".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Job Agent MCP has no startup output.")?;
    let mut ready = String::new();
    BufReader::new(stdout)
        .read_line(&mut ready)
        .map_err(|_| "Unable to read Job Agent MCP startup output.".to_string())?;
    let value: Value = serde_json::from_str(&ready)
        .map_err(|_| "Job Agent MCP returned invalid startup data.".to_string())?;
    if value.get("kind").and_then(Value::as_str) != Some("ready")
        || value.get("serverName").and_then(Value::as_str) != Some("job_agent")
    {
        return Err("Job Agent MCP did not start for this tool call.".into());
    }
    let descriptor = ManagedMcpUpstreamDescriptor {
        name: "job_agent".into(),
        url: value
            .get("url")
            .and_then(Value::as_str)
            .ok_or("Job Agent MCP did not provide an endpoint.")?
            .into(),
        bearer_token: value
            .get("bearerToken")
            .and_then(Value::as_str)
            .ok_or("Job Agent MCP did not provide endpoint authorization.")?
            .into(),
        workflow_tool_name: None,
        workflow_prepare_url: None,
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
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_configuration::{
        CapabilityProfile, CapabilitySet, NodeProfile, RuntimeProfileSnapshot, RuntimeSelections,
        SessionCreationRequest, SessionProfileResolver,
    };

    fn state() -> Arc<JobAgentFacadeState> {
        let installations = OtpInstallationService::in_memory();
        Arc::new(JobAgentFacadeState {
            installations,
            tools: [(
                "get_source_catalog".into(),
                AgentMcpToolDescriptor {
                    id: "get_source_catalog".into(),
                    name: "Get source catalog".into(),
                    description: "Read source summaries.".into(),
                    capability: "source_discovery".into(),
                    input_schema: json!({"type":"object"}),
                    output_schema: json!({"type":"object"}),
                    expected_behavior: "Reads source summaries.".into(),
                    recommended_usage: "Use for source discovery.".into(),
                    required_grants: vec![],
                },
            )]
            .into_iter()
            .collect(),
            capabilities: ["source_discovery".into()].into_iter().collect(),
            grants: vec![],
            backend: Mutex::new(None),
        })
    }

    fn selected_tool_profile() -> SessionCreationResolution {
        let capabilities = CapabilitySet {
            mcp_tools: [(
                "job_agent".into(),
                ["get_source_catalog".into()].into_iter().collect(),
            )]
            .into_iter()
            .collect(),
            ..CapabilitySet::default()
        };
        SessionProfileResolver::resolve_snapshot(
            RuntimeProfileSnapshot {
                contract_version: 1,
                profile_ref: "configured-runtime".into(),
                exposure: capabilities.clone(),
                locked: RuntimeSelections::default(),
            },
            SessionCreationRequest {
                contract_version: 1,
                capability_profile: CapabilityProfile {
                    execution: Default::default(),
                    contract_version: 1,
                    defaults: Default::default(),
                    capability_profile_id: "profile".into(),
                    name: "Profile".into(),
                    revision: 1,
                    allowed_capabilities: capabilities.clone(),
                },
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: capabilities,
                    pinned_defaults: RuntimeSelections::default(),
                },
                agent_mcp_configuration: [("job_agent".into(), json!({}))].into_iter().collect(),
            },
        )
        .unwrap()
    }

    #[test]
    fn provisioning_selected_tools_never_requires_job_agent_configuration() {
        let registry = OtpRegistry::import(&["job_agent"]).unwrap();
        let provisioner =
            JobAgentMcpProvisioner::new(registry, OtpInstallationService::in_memory());
        let upstreams = ManagedMcpUpstreamRegistry::default();
        let session = AgentSessionId::new("session-1").unwrap();

        provisioner
            .provision(&session, &selected_tool_profile(), &upstreams)
            .unwrap();

        assert_eq!(
            upstreams
                .resolve_scoped(session.as_str(), "job_agent")
                .unwrap()
                .name,
            "job_agent"
        );
        upstreams.shutdown();
    }

    #[test]
    fn tools_list_is_static_and_does_not_start_the_job_agent_backend() {
        let state = state();
        let response = tokio::runtime::Runtime::new().unwrap().block_on(async {
            let request = Request::builder()
                .header(header::AUTHORIZATION, "Bearer facade-test")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
                ))
                .unwrap();
            handle_facade(request, state.clone(), "facade-test").await
        });
        let body = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { response.into_body().collect().await.unwrap().to_bytes() });
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value
                .pointer("/result/tools/0/name")
                .and_then(Value::as_str),
            Some("get_source_catalog")
        );
        assert!(state.backend.lock().unwrap().is_none());
    }

    #[test]
    fn unavailable_job_agent_is_a_tool_result_not_a_session_setup_failure() {
        let state = state();
        let response = tokio::runtime::Runtime::new().unwrap().block_on(async {
            let request = Request::builder()
                .header(header::AUTHORIZATION, "Bearer facade-test")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_source_catalog","arguments":{}}}).to_string(),
                ))
                .unwrap();
            handle_facade(request, state.clone(), "facade-test").await
        });
        let body = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { response.into_body().collect().await.unwrap().to_bytes() });
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value.pointer("/result/isError").and_then(Value::as_bool),
            Some(true)
        );
        assert!(state.backend.lock().unwrap().is_none());
    }
}
