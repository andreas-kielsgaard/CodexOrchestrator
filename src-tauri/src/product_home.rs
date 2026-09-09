//! Product-owned files; independent of the selected provider home and application database.
use std::path::PathBuf;

pub(crate) struct OrchestrationHomePaths {
    pub(crate) root: PathBuf,
}
impl OrchestrationHomePaths {
    pub(crate) fn system() -> Result<Self, String> {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .ok_or("Unable to resolve the user's home directory")?;
        Ok(Self {
            root: PathBuf::from(home).join(".codex-orchestrator"),
        })
    }
    pub(crate) fn skills(&self) -> PathBuf {
        self.root.join("skills")
    }
}
