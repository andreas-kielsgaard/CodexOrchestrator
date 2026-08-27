use super::{
    application::WorkflowApplication,
    domain::{WorkflowInvocation, WorkflowMcpComponent, WorkflowMcpOutput},
};
use crate::harness_engine::{ManagedMcpUpstreamDescriptor, ManagedMcpUpstreamOwner};
use axum::body::Body;
use http_body_util::BodyExt;
use hyper::{header, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{convert::Infallible, sync::Weak, thread};
use tokio_util::sync::CancellationToken;

pub(crate) const SERVER_NAME: &str = "workflow_handoff";
pub(crate) const TOOL_NAME: &str = "handoff_to_agent";
pub(crate) const INTERFACE_ID: &str = "prompt_agent_files_and_text/v1";
pub(crate) const RESERVED_ARGUMENT: &str = "_workflowInvocation";

pub(crate) fn component() -> WorkflowMcpComponent {
    WorkflowMcpComponent {
        server_name: SERVER_NAME.to_string(),
        tool_name: TOOL_NAME.to_string(),
        title: "Handoff to agent".to_string(),
        participation_mode: "native".to_string(),
        interface_id: INTERFACE_ID.to_string(),
    }
}

pub(crate) struct WorkflowMcpServerOwner {
    cancellation: CancellationToken,
    thread: Option<thread::JoinHandle<()>>,
}

impl ManagedMcpUpstreamOwner for WorkflowMcpServerOwner {
    fn stop(mut self: Box<Self>) {
        self.cancellation.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) fn start_sample_server(
    application: Weak<WorkflowApplication>,
) -> Result<
    (
        ManagedMcpUpstreamDescriptor,
        Box<dyn ManagedMcpUpstreamOwner>,
    ),
    String,
> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("Unable to bind Workflow MCP server: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Unable to configure Workflow MCP server: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("Unable to identify Workflow MCP server: {error}"))?;
    let bearer = uuid::Uuid::new_v4().simple().to_string();
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();
    let server_bearer = bearer.clone();
    let thread = thread::Builder::new()
        .name("workflow-native-mcp".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("Workflow MCP runtime");
            runtime.block_on(async move {
                let listener =
                    tokio::net::TcpListener::from_std(listener).expect("Workflow MCP listener");
                run(listener, application, server_bearer, cancel).await;
            });
        })
        .map_err(|error| format!("Unable to start Workflow MCP server: {error}"))?;
    Ok((
        ManagedMcpUpstreamDescriptor {
            name: SERVER_NAME.to_string(),
            url: format!("http://{address}/mcp"),
            bearer_token: bearer,
            workflow_tool_name: Some(TOOL_NAME.to_string()),
            workflow_prepare_url: Some(format!("http://{address}/prepare")),
        },
        Box::new(WorkflowMcpServerOwner {
            cancellation,
            thread: Some(thread),
        }),
    ))
}

async fn run(
    listener: tokio::net::TcpListener,
    application: Weak<WorkflowApplication>,
    bearer: String,
    cancellation: CancellationToken,
) {
    loop {
        let accepted = tokio::select! {
            _ = cancellation.cancelled() => break,
            accepted = listener.accept() => accepted,
        };
        let Ok((stream, _)) = accepted else { continue };
        let application = application.clone();
        let bearer = bearer.clone();
        tokio::spawn(async move {
            let service = service_fn(move |request| {
                let application = application.clone();
                let bearer = bearer.clone();
                async move { Ok::<_, Infallible>(handle(request, application, &bearer).await) }
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

async fn handle(
    request: Request<hyper::body::Incoming>,
    application: Weak<WorkflowApplication>,
    bearer: &str,
) -> Response<Body> {
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        == Some(format!("Bearer {bearer}").as_str());
    if !authorized {
        return text(
            StatusCode::UNAUTHORIZED,
            "Workflow MCP authorization failed.",
        );
    }
    let path = request.uri().path().to_string();
    let headers = request.headers().clone();
    let body = match request.into_body().collect().await {
        Ok(body) => body.to_bytes(),
        Err(_) => {
            return text(
                StatusCode::BAD_REQUEST,
                "Workflow MCP request is unreadable.",
            )
        }
    };
    let Some(application) = application.upgrade() else {
        return text(
            StatusCode::SERVICE_UNAVAILABLE,
            "Workflow Engine is unavailable.",
        );
    };
    if path == "/prepare" {
        let request = match serde_json::from_slice::<PrepareRequest>(&body) {
            Ok(request) => request,
            Err(error) => {
                return text(
                    StatusCode::BAD_REQUEST,
                    &format!("Invalid Workflow preparation: {error}"),
                )
            }
        };
        return match application.prepare_mcp_native_handoff(
            &request.workflow_instance_id,
            &request.sender_node_id,
            &request.source_session_id,
            &request.source_invocation_id,
            &request.server_name,
            &request.tool_name,
        ) {
            Ok(handoff) => json_response(json!({ "handoff": handoff })),
            Err(error) => text(StatusCode::CONFLICT, &error),
        };
    }
    if path != "/mcp" {
        return text(
            StatusCode::NOT_FOUND,
            "Workflow MCP endpoint was not found.",
        );
    }
    let request = match serde_json::from_slice::<Value>(&body) {
        Ok(request) => request,
        Err(error) => return rpc_error(Value::Null, format!("Invalid MCP request: {error}")),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    match request.get("method").and_then(Value::as_str) {
        Some("initialize") => json_response(json!({
            "jsonrpc":"2.0","id":id,"result":{
                "protocolVersion":"2025-06-18",
                "capabilities":{"tools":{}},
                "serverInfo":{"name":"workflow-handoff","version":"1"}
            }
        })),
        Some("notifications/initialized") => Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(Body::empty())
            .expect("empty response"),
        Some("tools/list") => json_response(json!({
            "jsonrpc":"2.0","id":id,"result":{"tools":[tool_metadata()]}
        })),
        Some("tools/call") => handle_tool_call(id, &request, &headers, &application),
        _ => rpc_error(id, "Unsupported Workflow MCP method.".to_string()),
    }
}

fn handle_tool_call(
    id: Value,
    request: &Value,
    headers: &hyper::HeaderMap,
    application: &WorkflowApplication,
) -> Response<Body> {
    if request.pointer("/params/name").and_then(Value::as_str) != Some(TOOL_NAME) {
        return rpc_error(id, "Unknown Workflow MCP tool.".to_string());
    }
    let arguments = request
        .pointer("/params/arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let workflow_context = arguments
        .get(RESERVED_ARGUMENT)
        .map(|value| serde_json::from_value::<WorkflowInvocation>(value.clone()));
    let workflow_instance_id = header_value(headers, "x-workflow-instance-id");
    let source_session_id = header_value(headers, "x-workflow-session-id");
    let file_paths = arguments
        .get("filePaths")
        .and_then(Value::as_array)
        .map(|paths| {
            paths
                .iter()
                .map(|path| path.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        })
        .flatten();
    let output = match (
        file_paths,
        arguments.get("promptText").and_then(Value::as_str),
    ) {
        (Some(file_paths), Some(prompt_text)) => Ok(WorkflowMcpOutput {
            file_paths,
            prompt_text: prompt_text.to_string(),
        }),
        _ => Err("filePaths and promptText are required.".to_string()),
    };
    if let Some(workflow_context) = workflow_context {
        let invocation = match workflow_context {
            Ok(invocation) => invocation,
            Err(error) => {
                return tool_error(id, &format!("Workflow invocation is invalid: {error}"))
            }
        };
        let (Some(workflow_instance_id), Some(source_session_id)) =
            (workflow_instance_id, source_session_id)
        else {
            return tool_error(id, "Trusted Workflow routing context is absent.");
        };
        if let Err(error) = application.settle_mcp_native_handoff(
            workflow_instance_id,
            source_session_id,
            &invocation,
            output.clone(),
        ) {
            return tool_error(id, &error);
        }
    }
    let output = match output {
        Ok(output) => output,
        Err(error) => return tool_error(id, &error),
    };
    json_response(json!({
        "jsonrpc":"2.0","id":id,"result":{
            "content":[{"type":"text","text":"Workflow handoff output accepted."}],
            "structuredContent":output,
            "isError":false
        }
    }))
}

fn tool_metadata() -> Value {
    json!({
        "name":TOOL_NAME,
        "title":"Handoff to agent",
        "description":"Provide files and prompt text to a configured Workflow connection.",
        "inputSchema":{
            "type":"object",
            "properties":{
                "filePaths":{"type":"array","items":{"type":"string"}},
                "promptText":{"type":"string"}
            },
            "required":["filePaths","promptText"],
            "additionalProperties":false
        },
        "outputSchema":{
            "type":"object",
            "properties":{
                "filePaths":{"type":"array","items":{"type":"string"}},
                "promptText":{"type":"string"}
            },
            "required":["filePaths","promptText"]
        },
        "_meta":{
            "contractVersion":"workflow-tool-participation/v1",
            "interface":INTERFACE_ID,
            "trustedArgument":RESERVED_ARGUMENT
        }
    })
}

fn header_value<'a>(headers: &'a hyper::HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn json_response(value: Value) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .expect("JSON response")
}

fn rpc_error(id: Value, message: String) -> Response<Body> {
    json_response(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":message}}))
}

fn tool_error(id: Value, message: &str) -> Response<Body> {
    json_response(json!({
        "jsonrpc":"2.0","id":id,"result":{
            "content":[{"type":"text","text":message}],"isError":true
        }
    }))
}

fn text(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(message.to_string()))
        .expect("text response")
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrepareRequest {
    workflow_instance_id: String,
    sender_node_id: String,
    source_session_id: String,
    source_invocation_id: String,
    server_name: String,
    tool_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_declares_native_contract_without_trusted_argument_in_input_schema() {
        let tool = tool_metadata();
        assert_eq!(
            tool["_meta"]["contractVersion"],
            "workflow-tool-participation/v1"
        );
        assert_eq!(tool["_meta"]["interface"], INTERFACE_ID);
        assert_eq!(tool["_meta"]["trustedArgument"], RESERVED_ARGUMENT);
        assert!(tool["inputSchema"]["properties"]
            .get(RESERVED_ARGUMENT)
            .is_none());
        assert_eq!(
            tool["outputSchema"]["properties"]["promptText"]["type"],
            "string"
        );
    }
}
