use super::AgentSessionApplication;
use crate::{
    agent_sessions::domain::AgentSessionId, execution_configuration::RuntimeQuickFeatures,
};

impl AgentSessionApplication {
    pub(crate) fn load_quick_features(
        &self,
        session_id: Option<&AgentSessionId>,
        working_directory: Option<&str>,
    ) -> Result<RuntimeQuickFeatures, String> {
        let history = session_id
            .map(|id| self.load_session(id))
            .transpose()
            .map_err(|e| e.to_string())?;
        let (cwd, defaults, expected_profile) = if let Some(history) = &history {
            let pinned = history
                .session
                .session_profile
                .as_ref()
                .ok_or("This Session has no pinned configuration.")?;
            pinned.verify_digest().map_err(|e| e.to_string())?;
            (
                history.session.working_directory.as_deref(),
                pinned.session_profile().pinned_defaults().clone(),
                Some(pinned.session_profile().runtime_profile_ref()),
            )
        } else {
            let capability = self
                .capability_profiles
                .as_ref()
                .ok_or("No Capability Profile service is available.")?
                .default_profile()
                .map_err(|e| e.to_string())?;
            (
                working_directory.filter(|cwd| !cwd.trim().is_empty()),
                capability.defaults,
                None,
            )
        };
        let mut features = self
            .profile_source()
            .map_err(|e| e.to_string())?
            .quick_features_at(cwd)
            .map_err(|e| e.to_string())?;
        if expected_profile.is_some_and(|expected| expected != features.profile_ref) {
            return Err("The selected runtime profile no longer matches this Session.".into());
        }
        features.defaults.model = defaults.model.or(features.defaults.model);
        features.defaults.reasoning_mode =
            defaults.reasoning_mode.or(features.defaults.reasoning_mode);
        Ok(features)
    }
}
