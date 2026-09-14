use crate::agent_sessions::imports::*;
pub(crate) trait CodexHistorySource: Send + Sync {
    fn read(&self, home: &ImportHome, thread_id: &str) -> Result<ImportedThread, String>;
    fn fork(
        &self,
        home: &ImportHome,
        thread_id: &str,
        last_turn_id: &str,
        cwd: &str,
    ) -> Result<ImportedThread, String>;
}
pub(crate) trait ImportHomeSource: Send + Sync {
    fn selected_import_home(&self) -> Result<ImportHome, String>;
}
pub(crate) trait AgentSessionImportStore: Send + Sync {
    fn receipt(&self, request_id: &str) -> Result<Option<ImportReceipt>, String>;
    fn claim(&self, receipt: &ImportReceipt) -> Result<(), String>;
    fn record_fork(&self, receipt: &ImportReceipt) -> Result<(), String>;
    fn materialize(&self, receipt: &ImportReceipt) -> Result<(), String>;
}
