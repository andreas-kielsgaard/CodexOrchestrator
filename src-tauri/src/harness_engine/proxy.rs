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
                .get(existing_token)
                .ok_or_else(|| "Harness proxy binding index is inconsistent.".to_string())?;
            if existing.registration == registration
                || (registration.harness_token.as_deref() == Some(existing_token)
                    && registrations_match_without_token(&existing.registration, &registration))
            {
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
        self.by_token
            .insert(token.clone(), ProxyBinding { registration, plan });
        Ok(token)
    }

    pub(crate) fn retire(&mut self, binding_id: &str) {
        if let Some(token) = self.token_by_binding.remove(binding_id) {
            self.by_token.remove(&token);
        }
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
        && left.mediation_plan == right.mediation_plan
        && left.configuration_digest == right.configuration_digest
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
    let Some((_binding, exposure)) = exposure else {
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
    let mut upstream = client
        .request(method, &exposure.upstream.url)
        .body(body.clone());
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
    response
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap_or_else(|_| {
            text_response(StatusCode::INTERNAL_SERVER_ERROR, "Harness proxy failed.")
        })
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
        tools: Mutex<Vec<String>>,
        calls: Mutex<Vec<String>>,
        authorizations: Mutex<Vec<String>>,
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
                                        .map(|name| json!({"name": name, "description": name, "inputSchema": {"type": "object"}}))
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
            tools: Mutex::new(vec!["allowed".into(), "blocked".into()]),
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
            tools: Mutex::new(vec!["first".into()]),
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
        state.tools.lock().unwrap().push("newest".into());
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
