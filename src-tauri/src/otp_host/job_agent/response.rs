use futures_util::StreamExt;
use serde_json::Value;

/// Keep the complete JSON-RPC envelope, including resource links and namespaced metadata.
pub(super) async fn read_rpc(response: reqwest::Response, id: &str) -> Result<Value, String> {
    let unreadable = || "Job Agent MCP returned an unreadable result.".to_string();
    let is_stream = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(';').next() == Some("text/event-stream"));
    if !is_stream {
        let value = response.json::<Value>().await.map_err(|_| unreadable())?;
        return matches_response(&value, id)
            .then_some(value)
            .ok_or_else(unreadable);
    }
    let events = sse_stream::SseStream::from_bytes_stream(response.bytes_stream());
    futures_util::pin_mut!(events);
    while let Some(event) = events.next().await {
        let event = event.map_err(|_| unreadable())?;
        let Some(data) = event.data.filter(|data| !data.is_empty()) else {
            continue;
        };
        let value: Value = serde_json::from_str(&data).map_err(|_| unreadable())?;
        if matches_response(&value, id) {
            return Ok(value);
        }
    }
    Err(unreadable())
}

fn matches_response(value: &Value, id: &str) -> bool {
    value.get("id").and_then(Value::as_str) == Some(id)
        && (value.get("result").is_some() || value.get("error").is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, routing::get, Router};
    use serde_json::json;

    #[tokio::test]
    async fn decodes_json_and_chunked_sse_without_losing_metadata() {
        let expected = json!({"jsonrpc":"2.0","id":"call","result":{
            "content":[{"type":"resource_link","uri":"file:///jobs.jsonl","name":"jobs"}],
            "_meta":{"vendor.example/stats":{"count":1}}
        }});
        let envelope = expected.to_string();
        let json_body = envelope.clone();
        let stream_body = format!(": heartbeat\r\n\r\ndata: {{\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}}\r\n\r\nevent: message\r\ndata: {envelope}\r\n\r\n");
        let app = Router::new()
            .route(
                "/json",
                get(move || async move { ([("content-type", "application/json")], json_body) }),
            )
            .route(
                "/sse",
                get(move || async move {
                    let chunks = stream_body
                        .as_bytes()
                        .chunks(7)
                        .map(|chunk| {
                            Ok::<_, std::convert::Infallible>(bytes::Bytes::copy_from_slice(chunk))
                        })
                        .collect::<Vec<_>>();
                    (
                        [("content-type", "text/event-stream; charset=utf-8")],
                        Body::from_stream(futures_util::stream::iter(chunks)),
                    )
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        for path in ["json", "sse"] {
            let response = reqwest::get(format!("http://{address}/{path}"))
                .await
                .unwrap();
            assert_eq!(read_rpc(response, "call").await.unwrap(), expected);
        }
        server.abort();
    }
}
