//! Claude launch preparation: points the launch at the setup's folder and CLI.
use super::setups::ClaudeSetups;
use crate::agent_sessions::{
    application::ProviderLaunchPreparation,
    domain::{AgentInvocationId, AgentSessionId},
    ports::RuntimeLaunchExtension,
};
use orchid_engine::{contracts::ProviderConfigurationRef, providers::claude::PROVIDER};
use std::sync::Arc;

pub(crate) struct ClaudeLaunchPreparation(pub(crate) Arc<ClaudeSetups>);

impl ProviderLaunchPreparation for ClaudeLaunchPreparation {
    fn prepare_launch(
        &self,
        configuration: &ProviderConfigurationRef,
        _session_id: &AgentSessionId,
        _invocation_id: &AgentInvocationId,
        _resuming: bool,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<RuntimeLaunchExtension, String> {
        if configuration.provider != PROVIDER {
            return Err(format!(
                "Claude cannot prepare a launch for agent provider `{}`",
                configuration.provider
            ));
        }
        let setup = self.0.resolve(&configuration.configuration_id)?;
        let mut extension = extension.unwrap_or_default();
        extension.environment.extend(self.0.environment(&setup));
        extension.executable = Some(setup.executable);
        Ok(extension)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::providers::claude::setups::test_setups;

    #[test]
    fn launches_use_the_setup_folder_and_cli() {
        let folder = tempfile::tempdir().unwrap();
        let setups = Arc::new(test_setups(None));
        let setup = setups.add(folder.path(), Some("/opt/claude".into())).unwrap();
        let preparation = ClaudeLaunchPreparation(setups);
        let session = AgentSessionId::new("session").unwrap();
        let invocation = AgentInvocationId::new("invocation").unwrap();
        let extension = preparation
            .prepare_launch(&ProviderConfigurationRef::new("claude", &setup.id), &session, &invocation, false, None)
            .unwrap();
        assert_eq!(extension.executable.as_deref(), Some("/opt/claude"));
        assert_eq!(extension.environment, [("CLAUDE_CONFIG_DIR".to_string(), setup.folder)]);
        assert!(preparation
            .prepare_launch(&ProviderConfigurationRef::new("codex", &setup.id), &session, &invocation, false, None)
            .is_err());
    }
}
