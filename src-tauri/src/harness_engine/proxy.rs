use super::domain::{HarnessMcpExposurePlan, HarnessMediationPlan, SidecarBindingRegistration};
use axum::{body::Body, http::header};
use bytes::Bytes;
use futures_util::{stream, Stream, StreamExt};
use http_body_util::BodyExt;
use hyper::{server::conn::http1, service::service_fn, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    convert::Infallible,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, RwLock},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct ProxyBinding {
    pub(crate) registration: SidecarBindingRegistration,
    pub(crate) plan: HarnessMediationPlan,
    pub(crate) current_invocation_id: Option<String>,
}

#[derive(Default)]
pub(crate) struct ProxyBindings {
    pub(super) by_token: BTreeMap<String, ProxyBinding>,
    token_by_binding: BTreeMap<String, String>,
}

impl ProxyBindings {
    pub(crate) fn register(
        &mut self,
        mut registration: SidecarBindingRegistration,
    ) -> Result<String, String> {
        registration.verify_digest()?;
        let plan = serde_json::from_str::<HarnessMediationPlan>(&registration.mediation_plan)
            .map_err(|error| format!("Harness mediation plan is invalid: {error}"))?;
        if plan.contract_version != super::domain::MEDIATION_PLAN_VERSION {
            return Err(format!(
                "Unsupported Harness mediation plan {}.",
                plan.contract_version
            ));
        }
        if let Some(existing_token) = self.token_by_binding.get(&registration.binding_id) {
            let existing = self
                .by_token
                .get_mut(existing_token)
                .ok_or_else(|| "Harness proxy binding index is inconsistent.".to_string())?;
            if existing.registration == registration
                || (registration.harness_token.as_deref() == Some(existing_token)
                    && registrations_match_without_token(&existing.registration, &registration))
            {
                registration.harness_token = Some(existing_token.clone());
                existing.registration = registration;
                existing.plan = plan;
                return Ok(existing_token.clone());
            }
            return Err(format!(
                "Harness binding {} was already registered with different immutable bytes.",
                registration.binding_id
            ));
        }
        let token = registration
            .harness_token
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());
        if self.by_token.contains_key(&token) {
            return Err("Harness token is already assigned to another binding.".to_string());
        }
        registration.harness_token = Some(token.clone());
        self.token_by_binding
            .insert(registration.binding_id.clone(), token.clone());
        self.by_token.insert(
            token.clone(),
            ProxyBinding {
                registration,
                plan,
                current_invocation_id: None,
            },
        );
        Ok(token)
    }

    pub(crate) fn retire(&mut self, binding_id: &str) {
        if let Some(token) = self.token_by_binding.remove(binding_id) {
            self.by_token.remove(&token);
        }
    }

    pub(crate) fn prepare_invocation(
        &mut self,
        binding_id: &str,
        invocation_id: &str,
    ) -> Result<(), String> {
        if invocation_id.trim().is_empty() {
            return Err("Harness invocation ID is required.".to_string());
        }
        let token = self
            .token_by_binding
            .get(binding_id)
            .cloned()
            .ok_or_else(|| {
                "Harness binding is unavailable for invocation preparation.".to_string()
            })?;
        let binding = self
            .by_token
            .get_mut(&token)
            .ok_or_else(|| "Harness proxy binding index is inconsistent.".to_string())?;
        binding.current_invocation_id = Some(invocation_id.to_string());
        Ok(())
    }

    fn exposure(
        &self,
        token: &str,
        index: usize,
    ) -> Option<(ProxyBinding, HarnessMcpExposurePlan)> {
        let binding = self.by_token.get(token)?.clone();
        let exposure = binding.plan.exposures.get(index)?.clone();
        Some((binding, exposure))
    }
}

fn registrations_match_without_token(
    left: &SidecarBindingRegistration,
    right: &SidecarBindingRegistration,
) -> bool {
    left.binding_id == right.binding_id
        && left.session_id == right.session_id
        && left.runtime_instance_id == right.runtime_instance_id
        && left.session_instance_token == right.session_instance_token
        && left.harness_snapshot == right.harness_snapshot
        && same_tool_policy(&left.mediation_plan, &right.mediation_plan)
        && left.source_workflow_instance_id == right.source_workflow_instance_id
        && left.source_node_id == right.source_node_id
}

fn same_tool_policy(left: &str, right: &str) -> bool {
    let parse = |value: &str| {
        serde_json::from_str::<HarnessMediationPlan>(value)
            .ok()
            .map(|p| super::exposure::HarnessExposurePolicy::from_resolved(&p))
    };
    matches!((parse(left), parse(right)), (Some(left), Some(right)) if left == right)
}

pub(crate) async fn run_proxy_listener(
    listener: tokio::net::TcpListener,
    bindings: Arc<RwLock<ProxyBindings>>,
    cancellation: CancellationToken,
) {
    loop {
        let accepted = tokio::select! {
            _ = cancellation.cancelled() => break,
            accepted = listener.accept() => accepted,
        };
        let Ok((stream, _)) = accepted else {
            continue;
        };
        let bindings = bindings.clone();
        tokio::spawn(async move {
            let service = service_fn(move |request| {
                let bindings = bindings.clone();
                async move { Ok::<_, Infallible>(handle_proxy_request(request, bindings).await) }
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

pub(crate) fn proxy_url(address: SocketAddr, token: &str, exposure_index: usize) -> String {
    format!("http://{address}/bindings/{token}/servers/{exposure_index}/mcp")
}

async fn handle_proxy_request(
    request: Request<hyper::body::Incoming>,
    bindings: Arc<RwLock<ProxyBindings>>,
) -> Response<Body> {
    let Some((token, exposure_index)) = parse_proxy_path(request.uri().path()) else {
        return text_response(
            StatusCode::NOT_FOUND,
            "Harness proxy endpoint was not found.",
        );
    };
    let exposure = bindings
        .read()
        .ok()
        .and_then(|bindings| bindings.exposure(token, exposure_index));
    let Some((binding, exposure)) = exposure else {
        return text_response(
            StatusCode::UNAUTHORIZED,
            "Harness binding is unavailable or retired.",
        );
    };
    let method = request.method().clone();
    let headers = request.headers().clone();
    let body = match request.into_body().collect().await {
        Ok(body) => body.to_bytes(),
        Err(error) => {
            return json_rpc_failure(
                Value::Null,
                format!("Harness proxy could not read the MCP request: {error}"),
            )
        }
    };
    let request_json = serde_json::from_slice::<Value>(&body).ok();
    if let Some((id, tool_name)) = tool_call(&request_json) {
        if !exposure.access.allows(tool_name) {
            return denied_tool_result(
                id,
                format!(
                    "Harness denied MCP tool {tool_name}; it is not exposed by {}.",
                    exposure.configured_server_name
                ),
            );
        }
    }

    let client = match reqwest::Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            return json_rpc_failure(
                request_id(&request_json),
                format!("Harness proxy HTTP client is unavailable: {error}"),
            )
        }
    };
    let mut forwarded_body = body.clone();
    let mut workflow_warning = None;
    if let Some((id, tool_name)) = tool_call(&request_json) {
        if exposure.upstream.caller_context
            && (headers.contains_key("x-otp-session-id")
                || headers.contains_key("x-otp-invocation-id"))
        {
            return denied_tool_result(
                id,
                "Harness rejected caller-supplied OTP routing context.".to_string(),
            );
        }
        if exposure.upstream.workflow_tool_name.as_deref() == Some(tool_name) {
            if has_reserved_workflow_argument(&request_json) {
                return denied_tool_result(
                    id,
                    "Harness rejected caller-supplied Workflow routing context.".to_string(),
                );
            }
            let invocation_id = match binding.current_invocation_id.as_deref() {
                Some(invocation_id) => invocation_id,
                None => {
                    return json_rpc_failure(
                        id,
                        "Harness has no current invocation for Workflow routing.".to_string(),
                    )
                }
            };
            let Some(prepare_url) = exposure.upstream.workflow_prepare_url.as_deref() else {
                return json_rpc_failure(
                    id,
                    "Managed Workflow MCP preparation is unavailable.".to_string(),
                );
            };
            let mut preparation = client.post(prepare_url).json(&json!({
                "workflowInstanceId": binding.registration.source_workflow_instance_id,
                "senderNodeId": binding.registration.source_node_id,
                "sourceSessionId": binding.registration.session_id,
                "sourceInvocationId": invocation_id,
                "serverName": exposure.configured_server_name,
                "toolName": tool_name,
            }));
            if !exposure.upstream.bearer_token.is_empty() {
                preparation = preparation.bearer_auth(&exposure.upstream.bearer_token);
            }
            let preparation = match preparation.send().await {
                Ok(response) => response,
                Err(error) => {
                    eprintln!("Harness Workflow MCP preparation failed: {error}");
                    return json_rpc_failure(
                        id,
                        "Workflow routing preparation failed.".to_string(),
                    );
                }
            };
            if !preparation.status().is_success() {
                let message = preparation
                    .text()
                    .await
                    .unwrap_or_else(|_| "Workflow routing preparation failed.".to_string());
                return json_rpc_failure(id, message);
            }
            let preparation = match preparation.json::<Value>().await {
                Ok(preparation) => preparation,
                Err(error) => {
                    eprintln!("Harness Workflow MCP preparation response was invalid: {error}");
                    return json_rpc_failure(
                        id,
                        "Workflow routing preparation was invalid.".to_string(),
                    );
                }
            };
            if let Some(handoff) = preparation.get("handoff").filter(|value| !value.is_null()) {
                let Some(invocation) = handoff.get("invocation").cloned() else {
                    return json_rpc_failure(
                        id,
                        "Workflow routing context was incomplete.".to_string(),
                    );
                };
                let mut forwarded = request_json.clone().unwrap_or(Value::Null);
                let Some(params) = forwarded.get_mut("params").and_then(Value::as_object_mut)
                else {
                    return json_rpc_failure(
                        id,
                        "MCP tool parameters must be an object.".to_string(),
                    );
                };
                let arguments = params
                    .entry("arguments")
                    .or_insert_with(|| Value::Object(serde_json::Map::new()));
                let Some(arguments) = arguments.as_object_mut() else {
                    return json_rpc_failure(
                        id,
                        "MCP tool arguments must be an object.".to_string(),
                    );
                };
                arguments.insert("_workflowInvocation".to_string(), invocation);
                forwarded_body = match serde_json::to_vec(&forwarded) {
                    Ok(body) => Bytes::from(body),
                    Err(error) => {
                        return json_rpc_failure(
                            id,
                            format!("Unable to mediate Workflow MCP call: {error}"),
                        );
                    }
                };
                workflow_warning = Some(
                    handoff
                        .get("warningText")
                        .and_then(Value::as_str)
                        .unwrap_or("This tool call also activated a Workflow connection.")
                        .to_string(),
                );
            }
        }
    }
    let mut upstream = client
        .request(method, &exposure.upstream.url)
        .body(forwarded_body);
    for name in [
        header::ACCEPT,
        header::CONTENT_TYPE,
        header::HeaderName::from_static("mcp-session-id"),
        header::HeaderName::from_static("mcp-protocol-version"),
        header::HeaderName::from_static("last-event-id"),
    ] {
        if let Some(value) = headers.get(&name) {
            upstream = upstream.header(name, value);
        }
    }
    if !exposure.upstream.bearer_token.is_empty() {
        upstream = upstream.bearer_auth(&exposure.upstream.bearer_token);
    }
    if exposure.upstream.caller_context && tool_call(&request_json).is_some() {
        let Some(invocation_id) = binding.current_invocation_id.as_deref() else {
            return json_rpc_failure(
                request_id(&request_json),
                "Harness has no current invocation for OTP routing.".to_string(),
            );
        };
        upstream = upstream
            .header("x-otp-session-id", &binding.registration.session_id)
            .header("x-otp-invocation-id", invocation_id);
    }
    if workflow_warning.is_some() {
        upstream = upstream
            .header(
                "x-workflow-instance-id",
                &binding.registration.source_workflow_instance_id,
            )
            .header("x-workflow-session-id", &binding.registration.session_id);
        if let Some(invocation_id) = &binding.current_invocation_id {
            upstream = upstream.header("x-workflow-invocation-id", invocation_id);
        }
    }
    let upstream = match upstream.send().await {
        Ok(response) => response,
        Err(error) => {
            eprintln!(
                "Harness managed MCP request failed for {}: {error}",
                exposure.configured_server_name
            );
            return json_rpc_failure(
                request_id(&request_json),
                format!(
                    "Harness could not reach managed MCP server {}.",
                    exposure.configured_server_name
                ),
            );
        }
    };
    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let mut response = Response::builder().status(status);
    for name in [
        header::CONTENT_TYPE,
        header::HeaderName::from_static("mcp-session-id"),
        header::RETRY_AFTER,
    ] {
        if let Some(value) = upstream_headers.get(&name) {
            response = response.header(name, value);
        }
    }
    if is_tools_list(&request_json) {
        if let Some(selected) = exposure.access.selected_tools() {
            let is_event_stream = upstream_headers
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.starts_with("text/event-stream"));
            if is_event_stream {
                return response
                    .body(Body::from_stream(filter_tools_list_sse_stream(
                        upstream.bytes_stream(),
                        selected.to_vec(),
                    )))
                    .unwrap_or_else(|_| {
                        text_response(StatusCode::INTERNAL_SERVER_ERROR, "Harness proxy failed.")
                    });
            }
            let response_body = match upstream.bytes().await {
                Ok(body) => body,
                Err(error) => {
                    eprintln!(
                        "Harness managed MCP response failed for {}: {error}",
                        exposure.configured_server_name
                    );
                    return json_rpc_failure(
                        request_id(&request_json),
                        "Harness could not read the managed MCP response.".to_string(),
                    );
                }
            };
            let response_body = match filter_tools_list_response(&response_body, selected) {
                Ok(body) => body,
                Err(error) => {
                    return json_rpc_failure(request_id(&request_json), error);
                }
            };
            return response
                .body(Body::from(response_body))
                .unwrap_or_else(|_| {
                    text_response(StatusCode::INTERNAL_SERVER_ERROR, "Harness proxy failed.")
                });
        }
    }
    if let Some(warning) = workflow_warning {
        let response_body = match upstream.bytes().await {
            Ok(body) => body,
            Err(error) => {
                eprintln!("Harness Workflow MCP response failed: {error}");
                return json_rpc_failure(
                    request_id(&request_json),
                    "Harness could not read the Workflow MCP response.".to_string(),
                );
            }
        };
        let response_body = match append_workflow_warning(&response_body, &warning) {
            Ok(body) => body,
            Err(error) => return json_rpc_failure(request_id(&request_json), error),
        };
        return response
            .body(Body::from(response_body))
            .unwrap_or_else(|_| {
                text_response(StatusCode::INTERNAL_SERVER_ERROR, "Harness proxy failed.")
            });
    }
    response
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap_or_else(|_| {
            text_response(StatusCode::INTERNAL_SERVER_ERROR, "Harness proxy failed.")
        })
}

fn append_workflow_warning(body: &Bytes, warning: &str) -> Result<Bytes, String> {
    let mut value = serde_json::from_slice::<Value>(body)
        .map_err(|_| "Harness received an invalid Workflow MCP response.".to_string())?;
    if value.pointer("/result/isError").and_then(Value::as_bool) == Some(true) {
        return Ok(body.clone());
    }
    let content = value
        .pointer_mut("/result/content")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "Workflow MCP response did not contain result content.".to_string())?;
    content.push(json!({"type":"text","text":warning}));
    serde_json::to_vec(&value)
        .map(Bytes::from)
        .map_err(|error| format!("Harness could not encode Workflow MCP response: {error}"))
}

fn parse_proxy_path(path: &str) -> Option<(&str, usize)> {
    let segments = path.trim_matches('/').split('/').collect::<Vec<_>>();
    if segments.len() != 5
        || segments[0] != "bindings"
        || segments[2] != "servers"
        || segments[4] != "mcp"
    {
        return None;
    }
    Some((segments[1], segments[3].parse().ok()?))
}

fn tool_call(request: &Option<Value>) -> Option<(Value, &str)> {
    let request = request.as_ref()?;
    if request.get("method")?.as_str()? != "tools/call" {
        return None;
    }
    Some((
        request.get("id").cloned().unwrap_or(Value::Null),
        request.get("params")?.get("name")?.as_str()?,
    ))
}

fn has_reserved_workflow_argument(request: &Option<Value>) -> bool {
    request
        .as_ref()
        .and_then(|request| request.pointer("/params/arguments/_workflowInvocation"))
        .is_some()
}

fn is_tools_list(request: &Option<Value>) -> bool {
    request
        .as_ref()
        .and_then(|request| request.get("method"))
        .and_then(Value::as_str)
        == Some("tools/list")
}

fn request_id(request: &Option<Value>) -> Value {
    request
        .as_ref()
        .and_then(|request| request.get("id"))
        .cloned()
        .unwrap_or(Value::Null)
}

fn denied_tool_result(id: Value, message: String) -> Response<Body> {
    json_response(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{"type": "text", "text": message}],
            "isError": true
        }
    }))
}

fn json_rpc_failure(id: Value, message: String) -> Response<Body> {
    json_response(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": -32000, "message": message}
    }))
}

fn json_response(value: Value) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .expect("JSON response")
}

fn text_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(message.to_string()))
        .expect("text response")
}

fn filter_tools_list_response(body: &Bytes, selected: &[String]) -> Result<Bytes, String> {
    if let Ok(mut value) = serde_json::from_slice::<Value>(body) {
        filter_tools(&mut value, selected)?;
        return serde_json::to_vec(&value)
            .map(Bytes::from)
            .map_err(|error| format!("Harness could not encode filtered tools/list: {error}"));
    }
    let text = std::str::from_utf8(body)
        .map_err(|_| "Harness received an unreadable tools/list response.".to_string())?;
    let mut filtered = String::new();
    let mut found = false;
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data.is_empty() {
                filtered.push_str(line);
                filtered.push('\n');
                continue;
            }
            let mut value = serde_json::from_str::<Value>(data).map_err(|error| {
                format!("Harness received an invalid streamed tools/list response: {error}")
            })?;
            filter_tools(&mut value, selected)?;
            filtered.push_str("data: ");
            filtered.push_str(&value.to_string());
            filtered.push('\n');
            found = true;
        } else {
            filtered.push_str(line);
            filtered.push('\n');
        }
    }
    if !found {
        return Err("Harness could not interpret the managed tools/list response.".to_string());
    }
    Ok(Bytes::from(filtered))
}

struct ToolsListSseState<S> {
    upstream: Pin<Box<S>>,
    buffer: Vec<u8>,
    selected: Vec<String>,
    finished: bool,
}

fn filter_tools_list_sse_stream<S, E>(
    upstream: S,
    selected: Vec<String>,
) -> impl Stream<Item = Result<Bytes, io::Error>>
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    stream::unfold(
        ToolsListSseState {
            upstream: Box::pin(upstream),
            buffer: Vec::new(),
            selected,
            finished: false,
        },
        |mut state| async move {
            loop {
                if let Some(event) = take_sse_event(&mut state.buffer) {
                    let filtered = filter_tools_list_sse_event(&event, &state.selected)
                        .map_err(io::Error::other);
                    return Some((filtered, state));
                }
                if state.finished {
                    if state.buffer.is_empty() {
                        return None;
                    }
                    let remaining = std::mem::take(&mut state.buffer);
                    let filtered = filter_tools_list_sse_event(&remaining, &state.selected)
                        .map_err(io::Error::other);
                    return Some((filtered, state));
                }
                match state.upstream.as_mut().next().await {
                    Some(Ok(chunk)) => state.buffer.extend_from_slice(&chunk),
                    Some(Err(error)) => {
                        eprintln!("Harness managed MCP response stream failed: {error}");
                        state.finished = true;
                        return Some((
                            Err(io::Error::other(
                                "Harness managed MCP response stream failed.",
                            )),
                            state,
                        ));
                    }
                    None => state.finished = true,
                }
            }
        },
    )
}

fn take_sse_event(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let lf = buffer.windows(2).position(|window| window == b"\n\n");
    let crlf = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    let end = match (lf, crlf) {
        (Some(lf), Some(crlf)) if crlf < lf => crlf + 4,
        (Some(lf), _) => lf + 2,
        (None, Some(crlf)) => crlf + 4,
        (None, None) => return None,
    };
    Some(buffer.drain(..end).collect())
}

fn filter_tools_list_sse_event(event: &[u8], selected: &[String]) -> Result<Bytes, String> {
    let text = std::str::from_utf8(event)
        .map_err(|_| "Harness received an unreadable streamed tools/list response.".to_string())?;
    let mut filtered = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let (content, ending) = if let Some(content) = line.strip_suffix("\r\n") {
            (content, "\r\n")
        } else if let Some(content) = line.strip_suffix('\n') {
            (content, "\n")
        } else {
            (line, "")
        };
        let Some(data) = content.strip_prefix("data:") else {
            filtered.push_str(line);
            continue;
        };
        let data = data.trim();
        if data.is_empty() {
            filtered.push_str(line);
            continue;
        }
        let mut value = serde_json::from_str::<Value>(data).map_err(|error| {
            format!("Harness received an invalid streamed tools/list response: {error}")
        })?;
        if value
            .get("result")
            .and_then(|result| result.get("tools"))
            .is_some_and(Value::is_array)
        {
            filter_tools(&mut value, selected)?;
        }
        filtered.push_str("data: ");
        filtered.push_str(&value.to_string());
        filtered.push_str(ending);
    }
    Ok(Bytes::from(filtered))
}

fn filter_tools(value: &mut Value, selected: &[String]) -> Result<(), String> {
    let tools = value
        .get_mut("result")
        .and_then(|result| result.get_mut("tools"))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            "Managed MCP tools/list response did not contain a tool list.".to_string()
        })?;
    tools.retain(|tool| {
        tool.get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| selected.iter().any(|selected| selected == name))
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness_engine::domain::{
        binding_digest, HarnessToolAccess, ManagedMcpUpstreamDescriptor, MEDIATION_PLAN_VERSION,
    };
    use std::sync::Mutex;
    use std::time::Duration;

    struct UpstreamState {
        tools: Mutex<Vec<Value>>,
        calls: Mutex<Vec<String>>,
        authorizations: Mutex<Vec<String>>,
    }

    struct WorkflowUpstreamState {
        preparations: Mutex<Vec<Value>>,
        calls: Mutex<Vec<Value>>,
    }

    #[test]
    fn selected_tool_filter_preserves_upstream_annotations_and_omissions() {
        let input = Bytes::from(
            json!({"jsonrpc":"2.0","id":1,"result":{"tools":[
                {"name":"annotated","annotations":{"readOnlyHint":true,"openWorldHint":false},"x-provider":{"parallelClass":"safe"}},
                {"name":"unannotated"},
                {"name":"blocked","annotations":{"destructiveHint":true}}
            ]}}).to_string(),
        );
        let filtered =
            filter_tools_list_response(&input, &["annotated".into(), "unannotated".into()])
                .unwrap();
        let value: Value = serde_json::from_slice(&filtered).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);
        assert_eq!(tools[0]["annotations"]["openWorldHint"], false);
        assert_eq!(tools[0]["x-provider"]["parallelClass"], "safe");
        assert!(tools[1].get("annotations").is_none());
    }

    #[test]
    fn streamed_tool_filter_preserves_upstream_annotations() {
        let event = format!(
            "event: message\ndata: {}\n\n",
            json!({"jsonrpc":"2.0","id":1,"result":{"tools":[
                {"name":"kept","annotations":{"readOnlyHint":true,"idempotentHint":true}},
                {"name":"blocked","annotations":{"readOnlyHint":false}}
            ]}})
        );
        let filtered = filter_tools_list_sse_event(event.as_bytes(), &["kept".into()]).unwrap();
        let text = std::str::from_utf8(&filtered).unwrap();
        let data = text
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .unwrap();
        let value: Value = serde_json::from_str(data).unwrap();
        assert_eq!(
            value["result"]["tools"][0]["annotations"]["readOnlyHint"],
            true
        );
        assert_eq!(
            value["result"]["tools"][0]["annotations"]["idempotentHint"],
            true
        );
    }

    #[tokio::test]
    async fn chunk_split_stream_preserves_annotations_and_extensions() {
        let upstream = stream::iter(vec![
            Ok::<_, io::Error>(Bytes::from_static(
                b"data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[{\"name\":\"kept\",\"annot",
            )),
            Ok(Bytes::from_static(
                b"ations\":{\"readOnlyHint\":true},\"x-provider\":{\"class\":\"safe\"}},{\"name\":\"hidden\"}]}}\n",
            )),
            Ok(Bytes::from_static(b"\n")),
        ]);
        let chunks = filter_tools_list_sse_stream(upstream, vec!["kept".into()])
            .collect::<Vec<_>>()
            .await;
        let bytes = chunks
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .concat();
        let text = std::str::from_utf8(&bytes).unwrap();
        let data = text
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .unwrap();
        let value: Value = serde_json::from_str(data).unwrap();
        assert_eq!(
            value["result"]["tools"][0]["annotations"]["readOnlyHint"],
            true
        );
        assert_eq!(value["result"]["tools"][0]["x-provider"]["class"], "safe");
        assert_eq!(value["result"]["tools"].as_array().unwrap().len(), 1);
    }

    async fn start_workflow_upstream(
        state: Arc<WorkflowUpstreamState>,
    ) -> (SocketAddr, CancellationToken) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let cancellation = CancellationToken::new();
        let cancel = cancellation.clone();
        tokio::spawn(async move {
            loop {
                let accepted = tokio::select! {
                    _ = cancel.cancelled() => break,
                    accepted = listener.accept() => accepted,
                };
                let Ok((stream, _)) = accepted else { continue };
                let state = state.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |request: Request<hyper::body::Incoming>| {
                        let state = state.clone();
                        async move {
                            let path = request.uri().path().to_string();
                            let body = request.into_body().collect().await.unwrap().to_bytes();
                            let request = serde_json::from_slice::<Value>(&body).unwrap();
                            let response = if path == "/prepare" {
                                state.preparations.lock().unwrap().push(request);
                                json!({"handoff":{
                                    "invocation":{
                                        "contractVersion":"workflow-invocation/v1",
                                        "connectionActivationReference":"activation-1",
                                        "recipeReference":"recipe-live",
                                        "connectionReference":"edge",
                                        "senderNodeReference":"sender",
                                        "senderActivationReference":"invocation-current"
                                    },
                                    "warningText":null
                                }})
                            } else {
                                let id = request.get("id").cloned().unwrap_or(Value::Null);
                                state.calls.lock().unwrap().push(request);
                                json!({"jsonrpc":"2.0","id":id,"result":{
                                    "content":[{"type":"text","text":"original response"}],
                                    "structuredContent":{"filePaths":["a.md"],"promptText":"Review"},
                                    "isError":false
                                }})
                            };
                            Ok::<_, Infallible>(json_response(response))
                        }
                    });
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        (address, cancellation)
    }

    async fn start_upstream(state: Arc<UpstreamState>) -> (SocketAddr, CancellationToken) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let cancellation = CancellationToken::new();
        let cancel = cancellation.clone();
        tokio::spawn(async move {
            loop {
                let accepted = tokio::select! {
                    _ = cancel.cancelled() => break,
                    accepted = listener.accept() => accepted,
                };
                let Ok((stream, _)) = accepted else { continue };
                let state = state.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |request: Request<hyper::body::Incoming>| {
                        let state = state.clone();
                        async move {
                            let authorization = request
                                .headers()
                                .get(header::AUTHORIZATION)
                                .and_then(|value| value.to_str().ok())
                                .unwrap_or_default()
                                .to_string();
                            state.authorizations.lock().unwrap().push(authorization);
                            let body = request.into_body().collect().await.unwrap().to_bytes();
                            let request = serde_json::from_slice::<Value>(&body).unwrap();
                            let id = request.get("id").cloned().unwrap_or(Value::Null);
                            let response = match request.get("method").and_then(Value::as_str) {
                                Some("tools/list") => {
                                    let tools = state
                                        .tools
                                        .lock()
                                        .unwrap()
                                        .iter()
                                        .cloned()
                                        .collect::<Vec<_>>();
                                    json!({"jsonrpc":"2.0","id":id,"result":{"tools":tools}})
                                }
                                Some("tools/call") => {
                                    let name =
                                        request["params"]["name"].as_str().unwrap().to_string();
                                    state.calls.lock().unwrap().push(name.clone());
                                    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":format!("called {name}")}],"isError":false}})
                                }
                                _ => json!({"jsonrpc":"2.0","id":id,"result":{}}),
                            };
                            Ok::<_, Infallible>(
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(response.to_string()))
                                    .unwrap(),
                            )
                        }
                    });
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        (address, cancellation)
    }

    #[tokio::test]
    async fn selected_tools_filter_emits_a_complete_sse_event_before_upstream_closes() {
        let event = Bytes::from(
            "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[{\"name\":\"allowed\"},{\"name\":\"hidden\"}]}}\n\n",
        );
        let upstream = stream::once(async move { Ok::<_, io::Error>(event) })
            .chain(stream::pending::<Result<Bytes, io::Error>>());
        let mut filtered = Box::pin(filter_tools_list_sse_stream(
            upstream,
            vec!["allowed".to_string()],
        ));

        let first = tokio::time::timeout(Duration::from_millis(100), filtered.next())
            .await
            .expect("filtered event must not wait for upstream close")
            .expect("one filtered event")
            .expect("valid filtered event");
        let text = std::str::from_utf8(&first).unwrap();
        assert!(text.contains("allowed"));
        assert!(!text.contains("hidden"));
    }

    #[tokio::test]
    async fn participating_call_injects_current_context_and_appends_warning_after_upstream() {
        let state = Arc::new(WorkflowUpstreamState {
            preparations: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
        });
        let (upstream, upstream_cancel) = start_workflow_upstream(state.clone()).await;
        let snapshot = "{}".to_string();
        let plan = serde_json::to_string(&HarnessMediationPlan {
            contract_version: MEDIATION_PLAN_VERSION.to_string(),
            exposures: vec![HarnessMcpExposurePlan {
                configured_server_name: "workflow_handoff".into(),
                proxy_server_name: "workflow_harness_1".into(),
                upstream: ManagedMcpUpstreamDescriptor {
                    name: "workflow_handoff".into(),
                    url: format!("http://{upstream}/mcp"),
                    bearer_token: "secret".into(),
                    workflow_tool_name: Some("handoff_to_agent".into()),
                    workflow_prepare_url: Some(format!("http://{upstream}/prepare")),
                    caller_context: false,
                },
                access: HarnessToolAccess::EntireServer,
            }],
        })
        .unwrap();
        let mut bindings = ProxyBindings::default();
        let token = bindings
            .register(SidecarBindingRegistration {
                binding_id: "binding".into(),
                session_id: "session".into(),
                runtime_instance_id: "invocation-initial".into(),
                session_instance_token: "session-token".into(),
                configuration_digest: binding_digest(&snapshot, &plan),
                harness_snapshot: snapshot,
                mediation_plan: plan,
                harness_token: None,
                source_workflow_instance_id: "instance".into(),
                source_node_id: "sender".into(),
            })
            .unwrap();
        bindings
            .prepare_invocation("binding", "invocation-current")
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = listener.local_addr().unwrap();
        let proxy_cancel = CancellationToken::new();
        tokio::spawn(run_proxy_listener(
            listener,
            Arc::new(RwLock::new(bindings)),
            proxy_cancel.clone(),
        ));
        let url = proxy_url(proxy, &token, 0);

        let result = rpc(
            &url,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
                "name":"handoff_to_agent",
                "arguments":{"filePaths":["a.md"],"promptText":"Review"}
            }}),
        )
        .await;
        assert_eq!(result["result"]["content"].as_array().unwrap().len(), 2);
        assert_eq!(result["result"]["content"][0]["text"], "original response");
        assert_eq!(
            result["result"]["content"][1]["text"],
            "This tool call also activated a Workflow connection."
        );
        let preparation = state.preparations.lock().unwrap().remove(0);
        assert_eq!(preparation["sourceInvocationId"], "invocation-current");
        let call = state.calls.lock().unwrap().remove(0);
        assert_eq!(
            call["params"]["arguments"]["_workflowInvocation"]["senderActivationReference"],
            "invocation-current"
        );

        let denied = rpc(
            &url,
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"handoff_to_agent",
                "arguments":{
                    "filePaths":[],"promptText":"Attack",
                    "_workflowInvocation":{"connectionReference":"attacker"}
                }
            }}),
        )
        .await;
        assert_eq!(denied["result"]["isError"], true);
        assert!(state.calls.lock().unwrap().is_empty());
        proxy_cancel.cancel();
        upstream_cancel.cancel();
    }

    #[test]
    fn workflow_warning_preserves_original_content_and_is_appended_once() {
        let body = Bytes::from(
            json!({
                "jsonrpc":"2.0","id":1,"result":{
                    "content":[{"type":"text","text":"original"}],
                    "structuredContent":{"filePaths":["handoff.md"],"promptText":"Review."},
                    "isError":false
                }
            })
            .to_string(),
        );
        let mediated = append_workflow_warning(&body, "Workflow activated.").unwrap();
        let value: Value = serde_json::from_slice(&mediated).unwrap();
        assert_eq!(value["result"]["content"].as_array().unwrap().len(), 2);
        assert_eq!(value["result"]["content"][0]["text"], "original");
        assert_eq!(value["result"]["content"][1]["text"], "Workflow activated.");
        assert_eq!(
            value["result"]["structuredContent"]["filePaths"][0],
            "handoff.md"
        );
        let error = Bytes::from(
            json!({"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"failed"}],"isError":true}}).to_string(),
        );
        assert_eq!(
            append_workflow_warning(&error, "Workflow activated.").unwrap(),
            error
        );
    }

    #[test]
    fn caller_supplied_workflow_invocation_is_detected_before_forwarding() {
        let request = Some(json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call","params":{
                "name":"handoff_to_agent",
                "arguments":{"_workflowInvocation":{"connectionReference":"attacker"}}
            }
        }));
        assert!(has_reserved_workflow_argument(&request));
        assert!(!has_reserved_workflow_argument(&Some(json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"handoff_to_agent","arguments":{"promptText":"safe"}
            }
        }))));
    }

    #[test]
    fn prepared_invocation_replaces_only_runtime_context_on_immutable_binding() {
        let snapshot = "{}".to_string();
        let plan = serde_json::to_string(&HarnessMediationPlan {
            contract_version: MEDIATION_PLAN_VERSION.to_string(),
            exposures: Vec::new(),
        })
        .unwrap();
        let mut bindings = ProxyBindings::default();
        let token = bindings
            .register(SidecarBindingRegistration {
                binding_id: "binding".into(),
                session_id: "session".into(),
                runtime_instance_id: "runtime-initial".into(),
                session_instance_token: "session-token".into(),
                configuration_digest: binding_digest(&snapshot, &plan),
                harness_snapshot: snapshot,
                mediation_plan: plan,
                harness_token: None,
                source_workflow_instance_id: "instance".into(),
                source_node_id: "node".into(),
            })
            .unwrap();
        bindings
            .prepare_invocation("binding", "runtime-current")
            .unwrap();
        let binding = &bindings.by_token[&token];
        assert_eq!(binding.registration.runtime_instance_id, "runtime-initial");
        assert_eq!(
            binding.current_invocation_id.as_deref(),
            Some("runtime-current")
        );
        assert_eq!(binding.registration.source_workflow_instance_id, "instance");
    }

    #[tokio::test]
    async fn upstream_connection_failure_does_not_expose_private_endpoint_details() {
        let unavailable = "127.0.0.1:1".parse().unwrap();
        let (proxy, token, cancel) =
            start_proxy(unavailable, HarnessToolAccess::EntireServer).await;
        let value = rpc(
            &proxy_url(proxy, &token, 0),
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}),
        )
        .await;
        let message = value["error"]["message"].as_str().unwrap();

        assert!(message.contains("plan_builder"));
        assert!(!message.contains("127.0.0.1"));
        assert!(!message.contains(":1"));
        cancel.cancel();
    }

    async fn start_proxy(
        upstream: SocketAddr,
        access: HarnessToolAccess,
    ) -> (SocketAddr, String, CancellationToken) {
        let snapshot = "{\"harnessName\":\"Review\"}".to_string();
        let plan = serde_json::to_string(&HarnessMediationPlan {
            contract_version: MEDIATION_PLAN_VERSION.to_string(),
            exposures: vec![HarnessMcpExposurePlan {
                configured_server_name: "plan_builder".into(),
                proxy_server_name: "workflow_harness_1".into(),
                upstream: ManagedMcpUpstreamDescriptor {
                    name: "plan_builder".into(),
                    url: format!("http://{upstream}/mcp"),
                    bearer_token: "managed-secret".into(),
                    workflow_tool_name: None,
                    workflow_prepare_url: None,
                    caller_context: false,
                },
                access,
            }],
        })
        .unwrap();
        let mut bindings = ProxyBindings::default();
        let token = bindings
            .register(SidecarBindingRegistration {
                binding_id: "binding-1".into(),
                session_id: "session-1".into(),
                runtime_instance_id: "runtime-1".into(),
                session_instance_token: "session-token".into(),
                configuration_digest: binding_digest(&snapshot, &plan),
                harness_snapshot: snapshot,
                mediation_plan: plan,
                harness_token: None,
                source_workflow_instance_id: "workflow-instance-1".into(),
                source_node_id: "node-1".into(),
            })
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let cancellation = CancellationToken::new();
        tokio::spawn(run_proxy_listener(
            listener,
            Arc::new(RwLock::new(bindings)),
            cancellation.clone(),
        ));
        (address, token, cancellation)
    }

    async fn rpc(url: &str, value: Value) -> Value {
        reqwest::Client::new()
            .post(url)
            .header(header::ACCEPT, "application/json")
            .json(&value)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn selected_tools_are_filtered_and_gated_before_managed_forwarding() {
        let state = Arc::new(UpstreamState {
            tools: Mutex::new(vec![
                json!({"name":"allowed","description":"allowed","inputSchema":{"type":"object"}}),
                json!({"name":"blocked","description":"blocked","inputSchema":{"type":"object"}}),
            ]),
            calls: Mutex::new(Vec::new()),
            authorizations: Mutex::new(Vec::new()),
        });
        let (upstream, upstream_cancel) = start_upstream(state.clone()).await;
        let (proxy, token, proxy_cancel) = start_proxy(
            upstream,
            HarnessToolAccess::SelectedTools {
                tool_names: vec!["allowed".into()],
            },
        )
        .await;
        let url = proxy_url(proxy, &token, 0);
        let listed = rpc(
            &url,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}),
        )
        .await;
        assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 1);
        assert_eq!(listed["result"]["tools"][0]["name"], "allowed");

        let denied = rpc(&url, json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"blocked","arguments":{}}})).await;
        assert_eq!(denied["result"]["isError"], true);
        assert!(denied["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Harness denied"));
        assert!(state.calls.lock().unwrap().is_empty());

        let allowed = rpc(&url, json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"allowed","arguments":{}}})).await;
        assert_eq!(allowed["result"]["isError"], false);
        assert_eq!(state.calls.lock().unwrap().as_slice(), ["allowed"]);
        assert!(state
            .authorizations
            .lock()
            .unwrap()
            .iter()
            .all(|value| value == "Bearer managed-secret"));
        proxy_cancel.cancel();
        upstream_cancel.cancel();
    }

    #[tokio::test]
    async fn whole_server_tools_list_reflects_current_upstream_advertisement() {
        let state = Arc::new(UpstreamState {
            tools: Mutex::new(vec![json!({
                "name":"first",
                "description":"first",
                "inputSchema":{"type":"object"},
                "annotations":{"readOnlyHint":true,"openWorldHint":false},
                "x-provider":{"parallelClass":"safe"}
            })]),
            calls: Mutex::new(Vec::new()),
            authorizations: Mutex::new(Vec::new()),
        });
        let (upstream, upstream_cancel) = start_upstream(state.clone()).await;
        let (proxy, token, proxy_cancel) =
            start_proxy(upstream, HarnessToolAccess::EntireServer).await;
        let url = proxy_url(proxy, &token, 0);
        let first = rpc(
            &url,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}),
        )
        .await;
        assert_eq!(first["result"]["tools"].as_array().unwrap().len(), 1);
        assert_eq!(
            first["result"]["tools"][0]["annotations"]["readOnlyHint"],
            true
        );
        assert_eq!(
            first["result"]["tools"][0]["x-provider"]["parallelClass"],
            "safe"
        );
        state.tools.lock().unwrap().push(json!({
            "name":"newest","description":"newest","inputSchema":{"type":"object"}
        }));
        let second = rpc(
            &url,
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        )
        .await;
        assert_eq!(second["result"]["tools"].as_array().unwrap().len(), 2);
        assert_eq!(second["result"]["tools"][1]["name"], "newest");
        proxy_cancel.cancel();
        upstream_cancel.cancel();
    }
}
