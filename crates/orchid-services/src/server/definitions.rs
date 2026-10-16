use super::*;
use orchid_core::synchronization::definitions::Definition;

pub(super) async fn list(
    State(server): State<Server>,
    headers: HeaderMap,
) -> Result<Json<Vec<Definition>>, Failure> {
    server.authorize(&headers)?;
    server.definitions.list().map(Json).map_err(failure)
}
pub(super) async fn publish(
    State(server): State<Server>,
    Path(device): Path<String>,
    headers: HeaderMap,
    Json(definition): Json<Definition>,
) -> Result<Json<bool>, Failure> {
    server.authorize(&headers)?;
    server
        .definitions
        .accept_master(&device, &definition)
        .map(Json)
        .map_err(failure)
}
pub(super) async fn edit(
    State(server): State<Server>,
    headers: HeaderMap,
    Json(proposal): Json<Definition>,
) -> Result<Json<Definition>, Failure> {
    server.authorize(&headers)?;
    let current = server
        .definitions
        .get(&proposal.kind, &proposal.id)
        .map_err(failure)?
        .ok_or_else(|| failure("Definition is not synchronized"))?;
    if current.master_device_id != proposal.master_device_id {
        return Err(failure("Definition master mismatch"));
    }
    let request = HostCommand {
        method: "editDefinition".into(),
        input: serde_json::to_value(proposal).map_err(failure)?,
    };
    let Json(accepted) = command(
        State(server.clone()),
        Path(current.master_device_id.clone()),
        headers,
        Json(request),
    )
    .await?;
    let accepted: Definition = serde_json::from_value(accepted).map_err(failure)?;
    server
        .definitions
        .accept_master(&current.master_device_id, &accepted)
        .map_err(failure)?;
    Ok(Json(accepted))
}
pub(super) async fn skills(
    State(server): State<Server>,
    headers: HeaderMap,
) -> Result<Json<Vec<Value>>, Failure> {
    server.authorize(&headers)?;
    Ok(Json(server.definitions.list().map_err(failure)?.into_iter().filter(|d|d.kind=="skill"&&!d.content.is_null()).map(|d|serde_json::json!({"id":d.id,"masterDeviceId":d.master_device_id,"name":d.content["name"],"description":d.content["description"],"groupId":d.content["groupId"],"originDevice":d.content["originDevice"],"originPath":d.content["originPath"]})).collect()))
}
pub(super) async fn skill(
    State(server): State<Server>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Definition>, Failure> {
    server.authorize(&headers)?;
    server
        .definitions
        .get("skill", &id)
        .map_err(failure)?
        .filter(|definition| !definition.content.is_null())
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Skill is not retained".into()))
}
