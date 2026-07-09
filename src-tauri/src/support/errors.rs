pub(crate) fn open_task_not_found(task_id: &str) -> String {
    format!("Open task not found: {task_id}")
}

pub(crate) fn task_detail_not_found(task_id: &str) -> String {
    format!("Task not found: {task_id}")
}

pub(crate) fn truncate(value: &str, max_length: usize) -> String {
    if value.len() <= max_length {
        return value.to_string();
    }

    let truncated = value
        .chars()
        .take(max_length.saturating_sub(3))
        .collect::<String>();
    format!("{truncated}...")
}

pub(crate) fn sql_error(context: &str) -> impl FnOnce(rusqlite::Error) -> String + '_ {
    move |error| format!("Unable to {context}: {error}")
}
