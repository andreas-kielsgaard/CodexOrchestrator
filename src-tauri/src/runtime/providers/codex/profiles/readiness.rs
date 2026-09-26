//! Setup readiness for consumers that explicitly require product readiness evidence.
use super::*;
impl NativeProfileService {
    pub(crate) fn resolve_selected_home(&self) -> Result<ResolvedNativeCodexHome, String> {
        let resolved = self.resolve_session_home()?;
        self.reconcile_sandbox_adoption(&resolved.profile_id)?;
        let resolved = self.resolve_session_home()?;
        let readiness = &resolved.readiness;
        if readiness.authentication != "authenticated"
            || readiness.sandbox_initialization != "initialized"
            || readiness.workspace_write_canary != "passed"
            || readiness.mcp_reporting != "ready"
        {
            return Err(
                "The selected native Codex home is not ready for an application consumer".into(),
            );
        }
        Ok(resolved)
    }
}
