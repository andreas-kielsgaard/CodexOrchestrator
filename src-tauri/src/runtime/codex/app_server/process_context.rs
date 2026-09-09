//! A newly owned runtime must not attach to the launching Codex task's desktop bridge.
pub(super) fn parent_session_variables() -> Vec<String> {
    [
        "CODEX_APP_TOOLS_PIPE_PATH",
        "CODEX_SESSION_ID",
        "CODEX_THREAD_ID",
        "CODEX_INTERNAL_ORIGINATOR_OVERRIDE",
        "CODEX_PERMISSION_PROFILE",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
