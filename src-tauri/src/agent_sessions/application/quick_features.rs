use super::AgentSessionApplication;
use crate::{
    agent_sessions::domain::AgentSessionId, execution_configuration::RuntimeQuickFeatures,
    execution_targets::domain::SessionExecutionTarget,
};

impl AgentSessionApplication {
    pub(crate) fn load_quick_features(
        &self,
        session_id: Option<&AgentSessionId>,
        working_directory: Option<&str>,
        execution_target: Option<&SessionExecutionTarget>,
    ) -> Result<RuntimeQuickFeatures, String> {
        let history = session_id
            .map(|id| self.load_session(id))
            .transpose()
            .map_err(|e| e.to_string())?;
        let target = match &history {
            Some(history) => history.session.execution_target.as_ref(),
            None => execution_target,
        };
        if target.is_some_and(|target| target.execution.is_remote()) {
            return Err(
                "Native quick-feature discovery is unavailable for remote sessions.".into(),
            );
        }
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
            let profiles = self
                .capability_profiles
                .as_ref()
                .ok_or("No Capability Profile service is available.")?;
            let capability = match target {
                Some(target) => {
                    let capability = profiles
                        .read(&target.capability_profile_id)
                        .map_err(|e| e.to_string())?;
                    if capability.revision != target.capability_profile_revision
                        || capability.execution != target.execution
                    {
                        return Err(
                            "The Capability Profile changed. Select the target again.".into()
                        );
                    }
                    capability
                }
                None => profiles.default_profile().map_err(|e| e.to_string())?,
            };
            if capability.execution.is_remote() {
                return Err(
                    "Native quick-feature discovery is unavailable for remote sessions.".into(),
                );
            }
            (
                target
                    .map(|target| target.path.as_str())
                    .or(working_directory.filter(|cwd| !cwd.trim().is_empty())),
                capability.defaults,
                None,
            )
        };
        let mut features = self
            .profile_source()
            .map_err(|e| e.to_string())?
            .quick_features_for_configuration(
                target
                    .map(|target| target.execution.configuration_ref.as_str())
                    .unwrap_or("selected"),
                cwd,
            )
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
