//! Direct local-device API remains usable without a synchronization server.
use crate::host::CommandHandler;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use orchid_core::agent_sessions::{
    api::{AgentSessionDetailsDto, AgentSessionSummaryDto},
    client::HostCommand,
    domain::AgentSessionId,
    queries::AgentSessionQueries,
};
use std::sync::Arc;
#[derive(Clone)]
pub struct HostApi {
    pub token: String,
    pub handler: Arc<CommandHandler>,
    pub queries: AgentSessionQueries,
}
type Failure = (StatusCode, String);
impl HostApi {
    pub fn router(self) -> Router {
        Router::new()
            .route("/v1/commands", post(command))
            .route("/v1/sessions", get(sessions))
            .route("/v1/sessions/{id}", get(history))
            .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))
            .with_state(self)
    }
    fn authorize(&self, headers: &HeaderMap) -> Result<(), Failure> {
        if headers.get("authorization").and_then(|v| v.to_str().ok())
            == Some(&format!("Bearer {}", self.token))
        {
            Ok(())
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                "Device authentication required".into(),
            ))
        }
    }
}
fn failure(error: impl ToString) -> Failure {
    (StatusCode::BAD_REQUEST, error.to_string())
}
async fn command(
    State(api): State<HostApi>,
    headers: HeaderMap,
    Json(command): Json<HostCommand>,
) -> Result<Json<serde_json::Value>, Failure> {
    api.authorize(&headers)?;
    tokio::task::spawn_blocking(move || (api.handler)(command))
        .await
        .map_err(failure)?
        .map(Json)
        .map_err(failure)
}
async fn sessions(
    State(api): State<HostApi>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentSessionSummaryDto>>, Failure> {
    api.authorize(&headers)?;
    tokio::task::spawn_blocking(move || api.queries.list_sessions(Default::default()))
        .await
        .map_err(failure)?
        .map(|rows| Json(rows.into_iter().map(Into::into).collect()))
        .map_err(failure)
}
async fn history(
    State(api): State<HostApi>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AgentSessionDetailsDto>, Failure> {
    api.authorize(&headers)?;
    let id = AgentSessionId::new(id).map_err(failure)?;
    tokio::task::spawn_blocking(move || api.queries.load_session(&id))
        .await
        .map_err(failure)?
        .map(|h| Json(AgentSessionDetailsDto::from_history(h)))
        .map_err(failure)
}
