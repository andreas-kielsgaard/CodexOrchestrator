use crate::{protocol::*, service::Service};
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
#[derive(Clone)]
struct Context {
    service: Arc<Service>,
    token: Arc<String>,
}
struct Error(String);
impl From<String> for Error {
    fn from(e: String) -> Self {
        Self(e)
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(ApiError { message: self.0 })).into_response()
    }
}
async fn auth(State(c): State<Context>, request: Request, next: Next) -> Response {
    let actual = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or("");
    // Check the whole token without an early byte mismatch.
    let mut difference = actual.len() ^ c.token.len();
    for (i, b) in c.token.bytes().enumerate() {
        difference |= (b ^ actual.as_bytes().get(i).copied().unwrap_or(0)) as usize;
    }
    if difference != 0 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                message: "Coordinator authentication failed".into(),
            }),
        )
            .into_response();
    }
    next.run(request).await
}
pub fn router(service: Arc<Service>, token: String) -> Router {
    let context = Context {
        service,
        token: Arc::new(token),
    };
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/devices", get(list).post(register))
        .route("/api/v1/hetzner/connect", post(connect))
        .route("/api/v1/devices/{id}", get(detail).patch(rename))
        .route("/api/v1/devices/{id}/refresh", post(refresh))
        .route("/api/v1/devices/{id}/power", post(power))
        .layer(DefaultBodyLimit::max(16384))
        .layer(middleware::from_fn_with_state(context.clone(), auth))
        .with_state(context)
}
async fn health(State(c): State<Context>) -> Result<Json<Health>, Error> {
    Ok(Json(c.service.health()?))
}
async fn list(State(c): State<Context>) -> Result<Json<Vec<Device>>, Error> {
    Ok(Json(c.service.repository.list()?))
}
async fn connect(
    State(c): State<Context>,
    Json(i): Json<Connect>,
) -> Result<Json<Discovery>, Error> {
    Ok(Json(c.service.connect(i).await?))
}
async fn register(
    State(c): State<Context>,
    Json(i): Json<Register>,
) -> Result<Json<Device>, Error> {
    Ok(Json(c.service.register(i).await?))
}
async fn detail(State(c): State<Context>, Path(id): Path<String>) -> Result<Json<Device>, Error> {
    Ok(Json(c.service.repository.device(&id)?.0))
}
async fn refresh(State(c): State<Context>, Path(id): Path<String>) -> Result<Json<Device>, Error> {
    Ok(Json(c.service.refresh(&id).await?))
}
async fn rename(
    State(c): State<Context>,
    Path(id): Path<String>,
    Json(i): Json<Rename>,
) -> Result<Json<Device>, Error> {
    Ok(Json(c.service.rename(&id, &i.name).await?))
}
async fn power(
    State(c): State<Context>,
    Path(id): Path<String>,
    Json(i): Json<PowerRequest>,
) -> Result<Json<Operation>, Error> {
    Ok(Json(c.service.power(&id, i).await?))
}
