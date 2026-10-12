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
mod phone_connections;
use phone_connections::PhoneConnections;

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
    phones: Arc<PhoneConnections>,
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
        phones: Arc::new(PhoneConnections::default()),
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
    if !authorized(&headers, &state.token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
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
    use super::{
        continuous_audio::{AudioWindow, ContinuousAudio},
        query_schedule::QuerySchedule,
    };
    use std::time::{Duration, Instant};
    let connection_cancel = state.cancel.child_token();
    let Ok(connection_id) = state.phones.open(connection_cancel.clone(), || {
        state.app.update(|s| { s.phone_connected = true; Ok(()) })
    }) else { return; };
    let (mut sink, mut source) = socket.split();
    let mut events = state.app.events.subscribe();
    let conversation = state.app.snapshot().ok().and_then(|s| s.conversation);
    let classifications: Vec<Value> = conversation
        .as_ref()
        .map(|c| {
            c.inputs
                .iter()
                .rev()
                .take(20)
                .filter_map(|input| {
                    input
                        .relevance
                        .as_ref()
                        .map(|relevance| json!({"id":input.id,"relevance":relevance,"revision":input.evaluated_revision}))
                })
                .collect()
        })
        .unwrap_or_default();
    if sink.send(Message::Text(json!({"type":"hello","version":2,"conversationId":conversation.as_ref().map(|c|&c.id),"mainSessionId":conversation.map(|c|c.main_session_id),"inputClassifications":classifications}).to_string().into())).await.is_err() {
        let _=state.phones.close(connection_id, || state.app.update(|s|{s.phone_connected=false;Ok(())})); return;
    }
    let (windows, mut latest) = tokio::sync::watch::channel::<Option<AudioWindow>>(None);
    let app = state.app.clone();
    let cancelled = connection_cancel.clone();
    let decoder = tokio::spawn(async move {
        loop {
            tokio::select! { _=cancelled.cancelled()=>break, changed=latest.changed()=>{if changed.is_err(){break;}} }
            let Some(window) = latest.borrow_and_update().clone() else {
                continue;
            };
            let decode_app = app.clone();
            let decode_window = window.clone();
            let _ = app.audio_pending(1);
            let result = tokio::task::spawn_blocking(move || {
                decode_app
                    .speech
                    .transcribe(&decode_window.audio, decode_window.start_ms)
            })
            .await;
            let _ = app.audio_pending(-1);
            if cancelled.is_cancelled() {
                break;
            }
            if latest
                .borrow()
                .as_ref()
                .is_some_and(|current| current.stream_id != window.stream_id)
            {
                continue;
            }
            match result {
                Ok(Ok(transcript)) => {
                    let _ = app.audio_transcribed(Ok(&transcript.text));
                    if let Err(error) = app.transcript_window(&window, transcript) {
                        app.set_error(error);
                    }
                }
                Ok(Err(error)) => {
                    let _ = app.audio_transcribed(Err(&error));
                    app.set_error(error);
                }
                Err(error) => app.set_error(format!("Continuous transcription failed: {error}")),
            }
        }
    });
    let mut audio = ContinuousAudio::default();
    let mut schedule = QuerySchedule::default();
    let mut active_stream = String::new();
    let mut last_voice = Instant::now();
    let mut last_voice_ms = 0;
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    loop {
        tokio::select! {
            _=connection_cancel.cancelled()=>break,
            _=tick.tick()=>{
                if let Ok(snapshot)=state.app.snapshot() {
                    if let Some(c)=snapshot.conversation {
                        let blocks:Vec<_>=c.inputs.iter().filter(|i|i.stream_id.as_deref()==Some(&active_stream)&&!i.text.is_empty()).cloned().collect();
                        let quiet=if snapshot.speaking {0} else {last_voice.elapsed().as_millis() as u64};
                        if schedule.due(&blocks,&snapshot.input_policy,quiet,last_voice_ms,c.inputs.iter().any(|i|i.filter_pending),c.playback.as_ref().is_some_and(|p|p.state=="playing")) {
                            if let Err(error)=state.app.request_filter(){state.app.set_error(error);}
                        }
                    }
                }
            },
            event=events.recv()=>match event {
                Ok(value)=>{if value["type"]!="changed" && sink.send(Message::Text(value.to_string().into())).await.is_err(){break;}}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>{let _=sink.send(Message::Text(json!({"type":"error","message":"Voice connection fell behind; reactivate listening"}).to_string().into())).await;}
                Err(_)=>break,
            },
            message=source.next()=>{
                let Some(Ok(message))=message else{break};
                match message {
                    Message::Text(text)=>{
                        let Ok(value)=serde_json::from_str::<Value>(&text) else{continue};
                        match value["type"].as_str() {
                            Some("input_settings")=>{
                                match serde_json::from_value::<super::domain::VoiceInputPolicy>(value["settings"].clone()) {
                                    Ok(policy)=>match policy.validate(){
                                        Ok(())=>{let _=state.app.update(|s|{s.input_policy=policy;Ok(())});}
                                        Err(error)=>state.app.set_error(error),
                                    },
                                    Err(error)=>state.app.set_error(error.to_string()),
                                }
                            }
                            Some("audio_stream")=>{
                                let policy=state.app.snapshot().map(|s|s.input_policy).unwrap_or_default();
                                let stream=value["streamId"].as_str().unwrap_or("");
                                if stream.is_empty(){state.app.set_error("Audio stream ID missing".into());continue;}
                                match audio.push(stream,value["epochMs"].as_i64().unwrap_or(0),value["startSample"].as_u64().unwrap_or(0),value["pcm"].as_str().unwrap_or(""),policy.context_seconds) {
                                    Ok(window)=>{
                                        if active_stream!=stream {active_stream=stream.into();schedule=QuerySchedule::default();}
                                        if value["hasSpeech"].as_bool().unwrap_or(false) {last_voice=Instant::now();last_voice_ms=window.end_ms;}
                                        let _=state.app.audio_received(); windows.send_replace(Some(window));
                                    }
                                    Err(error)=>state.app.set_error(error),
                                }
                            }
                            Some("text")=>{
                                let app=state.app.clone();
                                tokio::task::spawn_blocking(move || {
                                    if let (Some(id),Some(text))=(value["id"].as_str(),value["text"].as_str()) {if let Err(error)=app.input(id.into(),text.into()){app.set_error(error);}}
                                });
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
    connection_cancel.cancel();
    decoder.abort();
    let _ = state.phones.close(connection_id, || state.app.update(|s| {
        s.phone_connected = false;
        s.speaking = false;
        s.pending_audio = 0;
        s.capture = None;
        if let Some(c) = &mut s.conversation {
            c.input_buffer.complete = false;
        }
        Ok(())
    }));
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
