use super::*;
use std::sync::Arc;
use tauri::State;
pub(crate) struct AgentCliUpdateTauriState(pub(crate) Arc<AgentCliUpdateService>);

#[tauri::command]
pub(crate) fn agent_cli_update_overview(
    state: State<'_, AgentCliUpdateTauriState>,
) -> Result<CliUpdateOverview, String> {
    state.0.overview()
}
