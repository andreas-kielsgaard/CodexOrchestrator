use super::{tool_map, OrchidControl};
use crate::{
    harness_engine::{ManagedMcpUpstreamDescriptor, ManagedMcpUpstreamOwner},
    voice_sessions::VoiceSessionApplication,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use std::{
    sync::{Arc, Weak},
    thread,
};
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
struct Host {
    control: Arc<OrchidControl>,
    voice: Weak<VoiceSessionApplication>,
    bearer: String,
}
struct Owner {
    cancel: CancellationToken,
    thread: Option<thread::JoinHandle<()>>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl ManagedMcpUpstreamOwner for Owner {
    fn stop(self: Box<Self>) {
        drop(self);
    }
}
pub(crate) fn start(
    control: Arc<OrchidControl>,
    voice: Weak<VoiceSessionApplication>,
) -> Result<
    (
        Vec<ManagedMcpUpstreamDescriptor>,
        Box<dyn ManagedMcpUpstreamOwner>,
    ),
    String,
> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    let bearer = uuid::Uuid::new_v4().simple().to_string();
    let descriptors = tool_map()
        .keys()
        .map(|name| ManagedMcpUpstreamDescriptor {
            name: name.clone(),
            url: format!("http://{address}/mcp/{name}"),
            bearer_token: bearer.clone(),
            workflow_tool_name: None,
            workflow_prepare_url: None,
            caller_context: true,
        })
        .collect();
    let host = Host {
        control,
        voice,
        bearer,
    };
    let cancel = CancellationToken::new();
    let cancelled = cancel.clone();
    let thread = thread::Builder::new()
        .name("orchid-control-mcp".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Orchid MCP runtime");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).expect("MCP listener");
                let _ = axum::serve(
                    listener,
                    Router::new()
                        .route("/mcp/{package}", post(handle))
                        .with_state(host),
                )
                .with_graceful_shutdown(cancelled.cancelled_owned())
                .await;
            });
            runtime.shutdown_timeout(std::time::Duration::from_secs(2));
        })
        .map_err(|e| e.to_string())?;
    Ok((
        descriptors,
        Box::new(Owner {
            cancel,
            thread: Some(thread),
        }),
    ))
}
async fn handle(
    State(host): State<Host>,
    Path(package): Path<String>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    if headers.get("authorization").and_then(|v| v.to_str().ok())
        != Some(format!("Bearer {}", host.bearer).as_str())
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !tool_map().contains_key(&package) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let result = match request["method"].as_str() {
        Some("initialize") => {
            json!({"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":package,"version":"1"}})
        }
        Some("notifications/initialized") => return StatusCode::ACCEPTED.into_response(),
        Some("tools/list") => json!({"tools":definitions(&package)}),
        Some("tools/call") => {
            let caller = headers
                .get("x-otp-session-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let invocation = headers
                .get("x-otp-invocation-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let tool = request
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let args = request
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or(json!({}));
            let result = tokio::task::spawn_blocking(move || -> Result<Value, String> {
                let caller = caller.ok_or("Trusted caller context is absent")?;
                let invocation = invocation.ok_or("Trusted invocation context is absent")?;
                let session = crate::agent_sessions::domain::AgentSessionId::new(caller.clone())
                    .map_err(|e| e.to_string())?;
                let history = host
                    .control
                    .agents
                    .load_session(&session)
                    .map_err(|e| e.to_string())?;
                if !history.invocations.iter().any(|h| {
                    h.invocation.id.as_str() == invocation && !h.invocation.status.is_terminal()
                }) {
                    return Err("Caller invocation is no longer active".into());
                }
                if !tool_map()[&package].contains(&tool) {
                    return Err("Unknown tool".into());
                }
                if package == "orchid_control" {
                    host.control.invoke(&tool, args)
                } else {
                    let voice = host.voice.upgrade().ok_or("Voice service unavailable")?;
                    match tool.as_str() {
                        "publish" => voice.publish(&caller, args),
                        "delegate_work" => voice.delegate(&caller, args),
                        "observe_sessions" => voice.observe(&caller, args),
                        _ => Err("Unknown voice tool".into()),
                    }
                }
            })
            .await;
            match result {
                Ok(Ok(value)) => {
                    json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                }
                Ok(Err(error)) => json!({"content":[{"type":"text","text":error}],"isError":true}),
                Err(error) => {
                    json!({"content":[{"type":"text","text":error.to_string()}],"isError":true})
                }
            }
        }
        _ => {
            return Json(
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Unknown method"}}),
            )
            .into_response()
        }
    };
    Json(json!({"jsonrpc":"2.0","id":id,"result":result})).into_response()
}
fn definition(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required}})
}
fn definitions(package: &str) -> Vec<Value> {
    let string = json!({"type":"string"});
    if package == "voice_session" {
        return vec![
        definition("delegate_work","Hand off independent work and return acceptance immediately. New recipients inherit the voice setup profile and directory unless supplied. Result observations resume the caller after its current turn finishes. Finish the turn while waiting rather than sleeping or polling.",json!({"task":string,"title":string,"sessionId":string,"profileId":string,"workingDirectory":string}),&["task"]),
        definition("publish","Publish deliberate speech. Retain report material with pendingTopic {id,title,material}; remove a finished topic with completedTopicId. Publication is not proof of playback.",json!({"kind":{"type":"string","enum":["acknowledgement","announcement","explanation"]},"text":string,"topicId":string,"completedTopicId":string,"pendingTopic":{"type":"object","properties":{"id":string,"title":string,"material":string},"required":["id","title","material"]}}),&["kind","text"]),
        definition("observe_sessions","Observe terminal results from selected sessions anywhere in Orchid. Replaces the explicit observation selection; delegated sessions remain observed.",json!({"sessionIds":{"type":"array","items":string}}),&["sessionIds"]),
    ];
    }
    vec![
        definition("list_sessions","Discover ordinary Orchid sessions across the application.",json!({"limit":{"type":"integer"}}),&[]),
        definition("read_session","Read recent invocation events, final replies, and pinned runtime requests.",json!({"sessionId":string,"limit":{"type":"integer"}}),&["sessionId"]),
        definition("list_profiles","Discover available Capability Profiles and execution settings.",json!({}),&[]),
        definition("list_workspaces","Discover registered repositories and workspace roots.",json!({}),&[]),
        definition("start_session","Create an ordinary independent session and send application work. Use delegate_work for voice result routing.",json!({"profileId":string,"workingDirectory":string,"title":string,"task":string}),&["profileId","task"]),
        definition("send_work","Send application work using existing session delivery and queueing.",json!({"sessionId":string,"text":string}),&["sessionId","text"]),
        definition("steer_session","Steer a running invocation through Orchid's existing steering service.",json!({"sessionId":string,"invocationId":string,"inputId":string,"text":string}),&["sessionId","invocationId","inputId","text"]),
        definition("cancel_invocation","Request cancellation of an invocation.",json!({"invocationId":string}),&["invocationId"]),
        definition("respond_to_request","Respond to a pinned runtime request with the provider's existing typed response. Inspect read_session interactions for its request ID and response encoding.",json!({"sessionId":string,"invocationId":string,"requestId":string,"response":{"type":"object"}}),&["sessionId","invocationId","requestId","response"]),
    ]
}
