mod active_app;
mod agent_sessions;
mod execution_configuration;
mod git_process;
mod harness_engine;
mod identities;
mod native_profiles;
mod product_home;
// The semantic save command is intentionally dormant until the later MCP adapter owns its input.
#[allow(dead_code)]
mod orchestration;
mod persistence;
mod product_database;
mod product_decisions;
mod repository_catalog;
mod repository_context;
mod repository_discovery;
mod runtime;
mod session_events;
mod storage;
mod workflows;
pub(crate) mod worktree_application;
mod worktree_review;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    active_app::run();
}

/// Runs the private Harness Engine child mode before Tauri initializes.
pub fn run_harness_engine_sidecar_if_requested() -> bool {
    harness_engine::sidecar::run_if_requested()
}
