use super::{
    domain::{PhoneCapture, Playback, VoiceSettings},
    VoiceSessionApplication,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc, thread};
use tokio_util::sync::CancellationToken;

pub(crate) struct GatewayOwner {
    cancellation: CancellationToken,
    thread: Option<thread::JoinHandle<()>>,
}
impl Drop for GatewayOwner {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[derive(Clone)]
struct GatewayState {
    app: Arc<VoiceSessionApplication>,
    token: String,
    cancel: CancellationToken,
}
pub(crate) fn validate_address(value: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = value
        .parse()
        .map_err(|_| "Use a numeric Tailscale address and port, such as 100.x.x.x:4818")?;
    let private = match address.ip() {
        std::net::IpAddr::V4(ip) => {
            ip.is_loopback() || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        }
        std::net::IpAddr::V6(ip) => ip.is_loopback(),
    };
    if !private || address.port() == 0 {
        return Err(
            "Bind to the laptop Tailscale IPv4 address (or loopback for local testing)".into(),
        );
    }
    Ok(address)
}
pub(crate) fn start(
    app: Arc<VoiceSessionApplication>,
    settings: &VoiceSettings,
) -> Result<GatewayOwner, String> {
    let listener = std::net::TcpListener::bind(validate_address(&settings.bind_address)?)
        .map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let cancel = CancellationToken::new();
    let cancellation = cancel.clone();
    let state = GatewayState {
        app,
        token: settings.token.clone(),
        cancel: cancel.clone(),
    };
    let thread = thread::Builder::new()
        .name("voice-phone".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("voice gateway runtime");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).expect("voice listener");
                let router = Router::new()
                    .route("/phone", get(upgrade))
                .route("/sessions", get(session_list))
                .route("/status", get(live_status))
                    .route("/sessions/{id}", get(session_history))
                    .with_state(state);
                let _ = axum::serve(listener, router)
                    .with_graceful_shutdown(cancel.cancelled_owned())
                    .await;
            });
            runtime.shutdown_timeout(std::time::Duration::from_secs(2));
        })
        .map_err(|e| e.to_string())?;
    Ok(GatewayOwner {
        cancellation,
        thread: Some(thread),
    })
}
async fn upgrade(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !authorized(&headers, &state.token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let accepted = state.app.update(|snapshot| {
        if snapshot.phone_connected {
            return Err("Phone already connected".into());
        }
        snapshot.phone_connected = true;
        Ok(())
    });
    if accepted.is_err() {
        return StatusCode::CONFLICT.into_response();
    }
    ws.max_message_size(5_500_000)
        .on_upgrade(move |socket| connection(socket, state))
}
fn authorized(headers: &HeaderMap, token: &str) -> bool {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        == Some(format!("Bearer {token}").as_str())
}
async fn session_list(State(state): State<GatewayState>, headers: HeaderMap) -> Response {
    if !authorized(&headers, &state.token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let result = tokio::task::spawn_blocking(move || state.app.phone_sessions()).await;
    history_response(result)
}
async fn live_status(State(state): State<GatewayState>, headers: HeaderMap) -> Response {
    if !authorized(&headers, &state.token) { return StatusCode::UNAUTHORIZED.into_response(); }
    history_response(tokio::task::spawn_blocking(move || state.app.diagnostics()).await)
}
async fn session_history(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    if !authorized(&headers, &state.token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let result = tokio::task::spawn_blocking(move || {
        state
            .app
            .phone_history(&id, query.get("before").map(String::as_str))
    })
    .await;
    history_response(result)
}
fn history_response(result: Result<Result<Value, String>, tokio::task::JoinError>) -> Response {
    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(message)) => (StatusCode::NOT_FOUND, Json(json!({"error":message}))).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
async fn connection(socket: WebSocket, state: GatewayState) {
    let (mut sink, mut source) = socket.split();
    let mut events = state.app.events.subscribe();
    let conversation = state.app.snapshot().ok().and_then(|s| s.conversation);
    let classifications: Vec<Value> = conversation.as_ref().map(|c| c.inputs.iter().rev().take(20).filter_map(|input| input.relevance.as_ref().map(|relevance| json!({"id":input.id,"relevance":relevance}))).collect()).unwrap_or_default();
    if sink.send(Message::Text(json!({"type":"hello","version":1,"conversationId":conversation.as_ref().map(|c|&c.id),"mainSessionId":conversation.map(|c|c.main_session_id),"inputClassifications":classifications}).to_string().into())).await.is_err() { let _=state.app.update(|s|{s.phone_connected=false;Ok(())});return; }
    // Drafts are display-only and replace queued drafts instead of delaying final inputs.
    let preview_cancel = state.cancel.child_token();
    let (previews, mut latest_preview) = tokio::sync::watch::channel::<Option<Value>>(None);
    let preview_app = state.app.clone();
    let cancelled = preview_cancel.clone();
    let preview_processor = tokio::spawn(async move {
        loop {
            tokio::select! { _ = cancelled.cancelled() => break, changed = latest_preview.changed() => { if changed.is_err() { break; } } }
            let Some(value) = latest_preview.borrow_and_update().clone() else { continue };
            let app = preview_app.clone();
            let result = tokio::task::spawn_blocking(move || {
                app.speech.transcribe_preview(value["audio"].as_str().ok_or("Audio is missing")?)
                    .map(|text| json!({"type":"transcript_preview","id":value["id"],"text":text}))
            }).await;
            if cancelled.is_cancelled() { break; }
            if let Ok(Ok(value)) = result {
                if value["text"].as_str().is_some_and(|text| !text.trim().is_empty()) {
                    let _ = preview_app.events.send(value);
                }
            }
        }
    });
    let (inputs, mut pending) = tokio::sync::mpsc::channel::<Value>(4);
    let app = state.app.clone();
    let cancelled = state.cancel.clone();
    let processor = tokio::spawn(async move {
        loop {
            let value =
                tokio::select! {_ = cancelled.cancelled()=>break, value=pending.recv()=>value};
            let Some(value) = value else { break };
            let app = app.clone();
            let error_app = app.clone();
            let result = tokio::task::spawn_blocking(move || {
                let text = if value["type"] == "audio" {
                    let result = app.speech.transcribe(value["audio"].as_str().ok_or("Audio is missing")?);
                    let _ = app.audio_transcribed(result.as_deref().map_err(String::as_str));
                    result?
                } else {
                    value["text"].as_str().ok_or("Text is missing")?.to_string()
                };
                if text.trim().is_empty() {
                    let _ = app.events.send(json!({"type":"transcript","id":value["id"],"text":""}));
                }
                app.input(
                    value["id"]
                        .as_str()
                        .ok_or("Input ID is missing")?
                        .to_string(),
                    text,
                )
            })
            .await;
            if let Ok(Err(error)) = result {
                let _ = error_app.update(|s| {
                    if let Some(c) = &mut s.conversation {
                        c.input_buffer.complete = false;
                    }
                    Ok(())
                });
                error_app.set_error(error);
            }
            let _ = error_app.audio_pending(-1);
            let _ = error_app.release_input();
        }
    });
    loop {
        tokio::select! {
            _=state.cancel.cancelled()=>break,
            event=events.recv()=>match event {
                Ok(value)=>{if value["type"]!="changed" && sink.send(Message::Text(value.to_string().into())).await.is_err(){break;}}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>{let _=sink.send(Message::Text(json!({"type":"error","message":"Output connection fell behind; ask for the report again"}).to_string().into())).await;}
                Err(_)=>break,
            },
            message=source.next()=>{
                let Some(Ok(message))=message else{break};
                match message {
                    Message::Text(text)=>{
                        let Ok(value)=serde_json::from_str::<Value>(&text) else{continue};
                        match value["type"].as_str() {
                            Some("audio_preview")=>{previews.send_replace(Some(value));}
                            Some("audio"|"text")=>{
                                let _=state.app.audio_pending(1);
                                if value["type"] == "audio" { let _=state.app.audio_received(); }
                                if inputs.try_send(value).is_err(){
                                    let _=state.app.audio_pending(-1);
                                    state.app.set_error("Input queue is busy; please repeat shortly".into());
                                }
                            }
                            Some("speech_state")=>{let _=state.app.speech_activity(value["speaking"].as_bool().unwrap_or(false));}
                            Some("capture_status")=>{if let Ok(capture)=serde_json::from_value::<PhoneCapture>(value["capture"].clone()){let _=state.app.capture_report(capture);}}
                            Some("playback"|"interrupt")=>{if let Ok(playback)=serde_json::from_value::<Playback>(value["playback"].clone()){let _=state.app.playback(playback);}}
                            Some("ping")=>{let _=sink.send(Message::Text(json!({"type":"pong"}).to_string().into())).await;}
                            _=>{}
                        }
                    }
                    Message::Ping(data)=>{let _=sink.send(Message::Pong(data)).await;}
                    Message::Close(_)=>break,
                    _=>{}
                }
            }
        }
    }
    processor.abort();
    preview_cancel.cancel();
    preview_processor.abort();
    let _ = state.app.update(|s| {
        s.phone_connected = false;
        s.speaking = false;
        s.pending_audio = 0;
        s.capture = None;
        if let Some(c) = &mut s.conversation {
            c.input_buffer.complete = false;
        }
        Ok(())
    });
    let _ = sink.close().await;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phone_history_requires_the_configured_bearer_token() {
        let mut headers = HeaderMap::new();
        assert!(!authorized(&headers, "phone-token"));
        headers.insert("authorization", "Bearer wrong-token".parse().unwrap());
        assert!(!authorized(&headers, "phone-token"));
        headers.insert("authorization", "Bearer phone-token".parse().unwrap());
        assert!(authorized(&headers, "phone-token"));
    }
    #[test]
    fn gateway_cannot_bind_public_or_lan_interfaces() {
        for address in [
            "0.0.0.0:4818",
            "192.168.0.139:4818",
            "8.8.8.8:4818",
            "100.128.1.1:4818",
        ] {
            assert!(validate_address(address).is_err());
        }
        for address in ["127.0.0.1:4818", "100.105.52.29:4818"] {
            assert!(validate_address(address).is_ok());
        }
    }
}
