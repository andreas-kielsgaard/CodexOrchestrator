//! One shared MCP reader; each session receives a distinct bearer-scoped manifest.

use crate::{
    agent_sessions::{domain::AgentSessionId, ports::RuntimeSkillInput},
    execution_configuration::SessionCreationResolution,
    harness_engine::{
        AgentMcpUpstreamProvisioner, ManagedMcpUpstreamDescriptor, ManagedMcpUpstreamOwner,
        ManagedMcpUpstreamRegistry,
    },
};
use axum::body::Body;
use http_body_util::BodyExt;
use hyper::{header, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    convert::Infallible,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};
use tokio_util::sync::CancellationToken;

pub(crate) struct SessionSkillReaderProvisioner {
    url: String,
    manifests: Arc<Mutex<BTreeMap<String, Vec<RuntimeSkillInput>>>>,
    cancellation: CancellationToken,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

impl SessionSkillReaderProvisioner {
    pub(crate) fn start() -> Result<Arc<Self>, String> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("Unable to bind Orchid skill reader: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let url = format!(
            "http://{}/mcp",
            listener.local_addr().map_err(|error| error.to_string())?
        );
        let manifests = Arc::new(Mutex::new(BTreeMap::new()));
        let cancellation = CancellationToken::new();
        let thread_manifests = manifests.clone();
        let cancel = cancellation.clone();
        let worker = thread::Builder::new()
            .name("orchid-skill-reader".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .expect("skill reader runtime");
                runtime.block_on(async move {
                    let listener =
                        tokio::net::TcpListener::from_std(listener).expect("skill reader listener");
                    loop {
                        let accepted = tokio::select! {
                            _ = cancel.cancelled() => break,
                            accepted = listener.accept() => accepted,
                        };
                        let Ok((stream, _)) = accepted else { continue };
                        let manifests = thread_manifests.clone();
                        tokio::spawn(async move {
                            let service = service_fn(move |request| {
                                let manifests = manifests.clone();
                                async move {
                                    Ok::<_, Infallible>(handle_request(request, manifests).await)
                                }
                            });
                            let _ = http1::Builder::new()
                                .serve_connection(TokioIo::new(stream), service)
                                .await;
                        });
                    }
                });
            })
            .map_err(|error| format!("Unable to start Orchid skill reader: {error}"))?;
        Ok(Arc::new(Self {
            url,
            manifests,
            cancellation,
            worker: Mutex::new(Some(worker)),
        }))
    }
}

impl Drop for SessionSkillReaderProvisioner {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Ok(worker) = self.worker.get_mut() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl AgentMcpUpstreamProvisioner for SessionSkillReaderProvisioner {
    fn provision(
        &self,
        session_id: &AgentSessionId,
        profile: &SessionCreationResolution,
        upstreams: &ManagedMcpUpstreamRegistry,
    ) -> Result<(), String> {
        let skills = profile.session_profile().session_skill_inputs();
        if skills.is_empty()
            || upstreams
                .resolve_scoped(session_id.as_str(), "orchid_skills")
                .is_ok()
        {
            return Ok(());
        }
        if !profile
            .session_profile()
            .node_capabilities()
            .mcp_tools
            .get("orchid_skills")
            .is_some_and(|tools| tools.contains("read_skill"))
        {
            return Err("Pinned skill manifest has no authorized reader exposure".into());
        }
        let token = uuid::Uuid::new_v4().simple().to_string();
        self.manifests
            .lock()
            .map_err(|_| "Skill reader unavailable")?
            .insert(token.clone(), skills.to_vec());
        let descriptor = ManagedMcpUpstreamDescriptor {
            name: "orchid_skills".into(),
            url: self.url.clone(),
            bearer_token: token.clone(),
            workflow_tool_name: None,
            workflow_prepare_url: None,
            caller_context: false,
        };
        let registration = match upstreams.register_for_session(session_id.as_str(), descriptor) {
            Ok(registration) => registration,
            Err(error) => {
                self.manifests
                    .lock()
                    .map_err(|_| "Skill reader unavailable")?
                    .remove(&token);
                return Err(error);
            }
        };
        let owner: Box<dyn ManagedMcpUpstreamOwner> = Box::new(SkillTokenOwner {
            token,
            manifests: self.manifests.clone(),
        });
        if let Err(owner) = upstreams.retain_owner(&registration, owner) {
            owner.stop();
            upstreams.unregister(&registration);
            return Err("Unable to retain session skill reader".into());
        }
        Ok(())
    }
}

struct SkillTokenOwner {
    token: String,
    manifests: Arc<Mutex<BTreeMap<String, Vec<RuntimeSkillInput>>>>,
}

impl ManagedMcpUpstreamOwner for SkillTokenOwner {
    fn stop(self: Box<Self>) {
        if let Ok(mut manifests) = self.manifests.lock() {
            manifests.remove(&self.token);
        }
    }
}

async fn handle_request<B>(
    request: Request<B>,
    manifests: Arc<Mutex<BTreeMap<String, Vec<RuntimeSkillInput>>>>,
) -> Response<Body>
where
    B: hyper::body::Body<Data = bytes::Bytes> + Send + 'static,
{
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or("");
    let skills = manifests
        .lock()
        .ok()
        .and_then(|entries| entries.get(token).cloned());
    let Some(skills) = skills else {
        return text(StatusCode::UNAUTHORIZED, "Unauthorized skill reader");
    };
    let body = match request.into_body().collect().await {
        Ok(body) => body.to_bytes(),
        Err(_) => return text(StatusCode::BAD_REQUEST, "Invalid MCP request"),
    };
    if body.len() > 64 * 1024 {
        return text(StatusCode::BAD_REQUEST, "MCP request is too large");
    }
    let value: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return text(StatusCode::BAD_REQUEST, "Invalid JSON"),
    };
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    match value.get("method").and_then(Value::as_str) {
        Some("initialize") => response(
            json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"orchid_skills","version":"1"}}}),
        ),
        Some("notifications/initialized") => text(StatusCode::ACCEPTED, ""),
        Some("tools/list") => response(
            json!({"jsonrpc":"2.0","id":id,"result":{"tools":[{"name":"read_skill","description":"Read one skill in this session's Orchid-approved manifest by name.","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false}}]}}),
        ),
        Some("tools/call")
            if value.pointer("/params/name").and_then(Value::as_str) == Some("read_skill") =>
        {
            let name = value
                .pointer("/params/arguments/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            match read_skill(&skills, name) {
                Ok(contents) => response(
                    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":contents}]}}),
                ),
                Err(message) => response(
                    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":message}],"isError":true}}),
                ),
            }
        }
        _ => response(
            json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Unsupported MCP method"}}),
        ),
    }
}

fn read_skill(skills: &[RuntimeSkillInput], name: &str) -> Result<String, String> {
    let skill = skills
        .iter()
        .find(|skill| skill.name == name)
        .ok_or("Skill is not in this session manifest")?;
    let path = Path::new(&skill.path);
    if !path.is_absolute() || path.file_name().is_none_or(|name| name != "SKILL.md") {
        return Err("Pinned skill path is invalid".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "Pinned skill is unavailable")?;
    if bytes.len() > 256 * 1024 {
        return Err("Pinned skill exceeds the reader limit".into());
    }
    if format!("{:x}", Sha256::digest(&bytes)) != skill.content_sha256 {
        return Err("Pinned skill changed since session creation".into());
    }
    String::from_utf8(bytes).map_err(|_| "Pinned skill is not UTF-8 text".into())
}

fn response(value: Value) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .expect("MCP JSON")
}

fn text(status: StatusCode, value: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(value.to_owned()))
        .expect("MCP text")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reader_rejects_unlisted_names_and_mutated_sources() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("SKILL.md");
        std::fs::write(&path, "first").unwrap();
        let skill = RuntimeSkillInput {
            id: path.to_string_lossy().into(),
            name: "one".into(),
            path: path.to_string_lossy().into(),
            content_sha256: format!("{:x}", Sha256::digest(b"first")),
            description: "".into(),
        };
        assert_eq!(read_skill(&[skill.clone()], "one").unwrap(), "first");
        assert!(read_skill(&[skill.clone()], "two").is_err());
        std::fs::write(&path, "changed").unwrap();
        assert!(read_skill(&[skill], "one").is_err());
    }
}
