use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_event(
    conn: &Connection,
    kind: &str,
    occurred_at: &str,
    project_id: Option<&str>,
    task_id: Option<&str>,
    task_run_id: Option<&str>,
    conversation_id: Option<&str>,
    artifact_id: Option<&str>,
    validation_run_id: Option<&str>,
    payload: Map<String, Value>,
) -> Result<String, String> {
    let event_id = Uuid::new_v4().to_string();
    let payload_json = Value::Object(payload).to_string();

    conn.execute(
        "
INSERT INTO events (
  id, kind, occurred_at, project_id, task_id, task_run_id, conversation_id, artifact_id,
  validation_run_id, payload_json
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
",
        params![
            event_id,
            kind,
            occurred_at,
            project_id,
            task_id,
            task_run_id,
            conversation_id,
            artifact_id,
            validation_run_id,
            payload_json
        ],
    )
    .map_err(sql_error("create event"))?;

    Ok(event_id)
}

pub(crate) fn insert_string(payload: &mut Map<String, Value>, key: &str, value: &str) {
    payload.insert(key.to_string(), Value::String(value.to_string()));
}

pub(crate) fn insert_i64(payload: &mut Map<String, Value>, key: &str, value: i64) {
    payload.insert(key.to_string(), Value::Number(value.into()));
}

pub(crate) fn insert_nullable_i64(payload: &mut Map<String, Value>, key: &str, value: Option<i64>) {
    match value {
        Some(value) => insert_i64(payload, key, value),
        None => {
            payload.insert(key.to_string(), Value::Null);
        }
    }
}

pub(crate) fn insert_nullable_string(
    payload: &mut Map<String, Value>,
    key: &str,
    value: Option<&str>,
) {
    match value {
        Some(value) => insert_string(payload, key, value),
        None => {
            payload.insert(key.to_string(), Value::Null);
        }
    }
}

pub(crate) fn insert_bool(payload: &mut Map<String, Value>, key: &str, value: bool) {
    payload.insert(key.to_string(), Value::Bool(value));
}

pub(crate) fn insert_string_array(payload: &mut Map<String, Value>, key: &str, values: &[String]) {
    payload.insert(
        key.to_string(),
        Value::Array(
            values
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        ),
    );
}

pub(crate) fn parse_event_payload(
    event_id: &str,
    payload_json: &str,
) -> Result<Map<String, Value>, String> {
    match serde_json::from_str::<Value>(payload_json)
        .map_err(|error| format!("Invalid JSON payload for event {event_id}: {error}"))?
    {
        Value::Object(payload) => Ok(payload),
        _ => Err(format!(
            "Invalid JSON payload for event {event_id}: expected a JSON object"
        )),
    }
}
