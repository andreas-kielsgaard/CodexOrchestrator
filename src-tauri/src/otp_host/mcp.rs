use super::OtpRegistry;
use crate::{
    harness_engine::{ManagedMcpUpstreamDescriptor, ManagedMcpUpstreamOwner},
    otp_api::{CapabilityRef, Entrypoint},
    workflows::execution::WorkflowExecutionService,
};
use axum::body::Body;
use http_body_util::BodyExt;
use hyper::{header, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use std::{
    convert::Infallible,
    sync::{Arc, Weak},
    thread,
};
use tokio_util::sync::CancellationToken;

struct ServerOwner {
    cancellation: CancellationToken,
    thread: Option<thread::JoinHandle<()>>,
}
impl Drop for ServerOwner {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl ManagedMcpUpstreamOwner for ServerOwner {
    fn stop(self: Box<Self>) {
        drop(self)
    }
}

pub(crate) fn start_server(
    registry: Arc<OtpRegistry>,
    application: Weak<WorkflowExecutionService>,
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
    let descriptors = registry
        .mcp_tools()
        .keys()
        .map(|package| ManagedMcpUpstreamDescriptor {
            name: package.clone(),
            url: format!("http://{address}/mcp/{package}"),
            bearer_token: bearer.clone(),
            workflow_tool_name: None,
            workflow_prepare_url: None,
            caller_context: true,
        })
        .collect();
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();
    let thread=thread::Builder::new().name("otp-mcp".into()).spawn(move||{
        let runtime=tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().expect("OTP MCP runtime");
        runtime.block_on(async move{
            let listener=tokio::net::TcpListener::from_std(listener).expect("OTP MCP listener");
            loop{
                let accepted=tokio::select!{_ = cancel.cancelled()=>break,accepted=listener.accept()=>accepted};
                let Ok((stream,_))=accepted else{continue};
                let registry=registry.clone();let application=application.clone();let bearer=bearer.clone();
                tokio::spawn(async move{
                    let service=service_fn(move|request|{
                        let registry=registry.clone();let application=application.clone();let bearer=bearer.clone();
                        async move{Ok::<_,Infallible>(handle(request,registry,application,&bearer).await)}
                    });
                    let _=http1::Builder::new().serve_connection(TokioIo::new(stream),service).await;
                });
            }
        });
    }).map_err(|e|e.to_string())?;
    Ok((
        descriptors,
        Box::new(ServerOwner {
            cancellation,
            thread: Some(thread),
        }),
    ))
}

async fn handle(
    request: Request<hyper::body::Incoming>,
    registry: Arc<OtpRegistry>,
    application: Weak<WorkflowExecutionService>,
    bearer: &str,
) -> Response<Body> {
    if request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        != Some(format!("Bearer {bearer}").as_str())
    {
        return text(StatusCode::UNAUTHORIZED, "OTP host authorization failed");
    }
    let Some(package) = request
        .uri()
        .path()
        .strip_prefix("/mcp/")
        .map(str::to_string)
    else {
        return text(StatusCode::NOT_FOUND, "Unknown OTP endpoint");
    };
    if !registry.mcp_tools().contains_key(&package) {
        return text(StatusCode::NOT_FOUND, "OTP is not imported");
    }
    let headers = request.headers().clone();
    let body = match request.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return text(StatusCode::BAD_REQUEST, "Unreadable MCP request"),
    };
    let value: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return rpc_error(Value::Null, e.to_string()),
    };
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    match value.get("method").and_then(Value::as_str) {
        Some("initialize") => response(
            json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":package,"version":"1"}}}),
        ),
        Some("notifications/initialized") => text(StatusCode::ACCEPTED, ""),
        Some("tools/list") => {
            let tools=registry.catalogue().into_iter().filter(|p|p.id==package).flat_map(|p|p.tools).filter_map(|tool|{
                let Entrypoint::Mcp{ref input_schema}=tool.entrypoint else{return None};
                Some(json!({"name":tool.id,"title":tool.name,"description":tool.description,"inputSchema":input_schema,"annotations":tool.annotations,"_meta":{"otp":{"package":package,"tool":tool}}}))
            }).collect::<Vec<_>>();
            response(json!({"jsonrpc":"2.0","id":id,"result":{"tools":tools}}))
        }
        Some("tools/call") => {
            let result = (|| -> Result<String, String> {
                let tool = value
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .ok_or("Missing tool name")?;
                let descriptor = registry.tool(&CapabilityRef {
                    package: package.clone(),
                    tool: tool.into(),
                })?;
                if !matches!(descriptor.entrypoint, Entrypoint::Mcp { .. }) {
                    return Err("This capability is not an MCP tool".into());
                }
                let session = headers
                    .get("x-otp-session-id")
                    .and_then(|h| h.to_str().ok())
                    .ok_or("Trusted Session context is absent")?;
                let invocation = headers
                    .get("x-otp-invocation-id")
                    .and_then(|h| h.to_str().ok())
                    .ok_or("Trusted invocation context is absent")?;
                let application = application
                    .upgrade()
                    .ok_or("OTP product host is unavailable")?;
                let result = application.invoke_mcp(
                    &package,
                    tool,
                    session,
                    invocation,
                    value
                        .pointer("/params/arguments")
                        .cloned()
                        .unwrap_or(json!({})),
                )?;
                Ok(result.text)
            })();
            let (text, is_error) = match result {
                Ok(text) => (text, false),
                Err(error) => (error, true),
            };
            response(
                json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":text}],"isError":is_error}}),
            )
        }
        _ => rpc_error(id, "Unsupported MCP method".into()),
    }
}
fn response(value: Value) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}
fn rpc_error(id: Value, message: String) -> Response<Body> {
    response(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":message}}))
}
fn text(status: StatusCode, value: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(value.to_string()))
        .unwrap()
}
