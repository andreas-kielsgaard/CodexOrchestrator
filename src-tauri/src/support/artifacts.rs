use super::*;

pub(crate) fn create_artifact(
    conn: &Connection,
    task_id: Option<&str>,
    task_run_id: Option<&str>,
    conversation_id: Option<&str>,
    kind: &str,
    title: &str,
    content: Option<&str>,
) -> Result<String, String> {
    let artifact_id = Uuid::new_v4().to_string();
    let created_at = now_iso();

    conn.execute(
        "
INSERT INTO artifacts (
  id, task_id, task_run_id, conversation_id, kind, title, uri, content, created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)
",
        params![
            artifact_id,
            task_id,
            task_run_id,
            conversation_id,
            kind,
            title,
            content,
            created_at
        ],
    )
    .map_err(sql_error("create artifact"))?;

    Ok(artifact_id)
}
