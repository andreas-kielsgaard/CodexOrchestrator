use super::AgentSessionApplication;
use crate::{
    agent_sessions::domain::AgentSessionId,
    execution_configuration::RuntimeQuickFeatures,
    execution_targets::domain::{ExecutionBinding, SessionExecutionTarget},
};
use orchid_engine::contracts::ProviderConfigurationRef;
use std::collections::BTreeSet;

impl AgentSessionApplication {
    pub(crate) fn load_quick_features(
        &self,
        session_id: Option<&AgentSessionId>,
        working_directory: Option<&str>,
        execution_target: Option<&SessionExecutionTarget>,
    ) -> Result<RuntimeQuickFeatures, String> {
        self.load_quick_features_for_configuration(
            session_id,
            working_directory,
            execution_target,
            None,
        )
    }

    pub(crate) fn load_quick_features_for_configuration(
        &self,
        session_id: Option<&AgentSessionId>,
        working_directory: Option<&str>,
        execution_target: Option<&SessionExecutionTarget>,
        configuration: Option<&ProviderConfigurationRef>,
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
        let (
            cwd,
            defaults,
            expected_profile,
            reference,
            allowed_groups,
            allowed_paths,
            allowed_names,
        ) = if let Some(history) = &history {
            let pinned = history.session.session_profile.as_ref();
            if let Some(pinned) = pinned {
                pinned.verify_digest().map_err(|e| e.to_string())?;
            }
            let expected = pinned.map(|pinned| pinned.session_profile().configuration());
            let legacy_provider = ExecutionBinding::default().provider;
            let reference = target
                .map(|target| target.execution.configuration())
                .or_else(|| expected.cloned())
                .or_else(|| {
                    self.configuration_source(&legacy_provider)
                        .ok()
                        .and_then(|source| source.configuration_ref_for_session(history.session.id.as_str()).ok().flatten())
                        .map(|id| ProviderConfigurationRef::new(&legacy_provider, id))
                })
                .ok_or("This older Session has no provider configuration binding; its original skill catalogue cannot be identified.")?;
            (
                history.session.working_directory.as_deref(),
                pinned
                    .map(|value| value.session_profile().pinned_defaults().clone())
                    .unwrap_or_default(),
                expected,
                reference,
                BTreeSet::new(),
                pinned
                    .map(|value| {
                        value
                            .session_profile()
                            .session_skill_inputs()
                            .iter()
                            .map(|skill| skill.path.clone())
                            .collect()
                    })
                    .unwrap_or_default(),
                BTreeSet::new(),
            )
        } else {
            let capability = match target {
                Some(target) => {
                    let profiles = self
                        .capability_profiles
                        .as_ref()
                        .ok_or("No Capability Profile service is available.")?;
                    let profile = profiles
                        .resolve_draft_selection(&target.capability_profile_id, &target.execution)
                        .map_err(|error| error.to_string())?;
                    Some(profile)
                }
                None => self
                    .capability_profiles
                    .as_ref()
                    .and_then(|profiles| profiles.default_profile().ok()),
            };
            let reference = target
                .map(|target| target.execution.configuration())
                .or_else(|| configuration.cloned())
                .unwrap_or_else(|| ExecutionBinding::default().configuration());
            let allowed_groups = capability
                .as_ref()
                .and_then(|profile| {
                    target
                        .and_then(|selected| profile.route_for_execution(&selected.execution))
                        .or_else(|| profile.default_route())
                })
                .map(|route| route.skill_groups.clone())
                .unwrap_or_default();
            let allowed_names = capability
                .as_ref()
                .filter(|profile| profile.route_policies.is_empty())
                .map(|profile| profile.allowed_capabilities.skills.clone())
                .unwrap_or_default();
            (
                target
                    .map(|target| target.path.as_str())
                    .or(working_directory.filter(|cwd| !cwd.trim().is_empty())),
                capability
                    .map(|profile| profile.defaults)
                    .unwrap_or_default(),
                None,
                reference,
                allowed_groups,
                BTreeSet::new(),
                allowed_names,
            )
        };
        let mut features = self
            .configuration_source(&reference.provider)
            .map_err(|e| e.to_string())?
            .quick_features_for_configuration(&reference.configuration_id, cwd)
            .map_err(|e| e.to_string())?;
        if expected_profile.is_some_and(|expected| features.configuration.as_ref() != Some(expected)) {
            return Err("The selected runtime profile no longer matches this Session.".into());
        }
        self.product_skills.append_quick_skills(&mut features);
        features.skills.retain(|skill| {
            skill.group_id == crate::execution_configuration::NATIVE_SKILL_GROUP
                || allowed_groups.contains(&skill.group_id)
                || allowed_names.contains(&skill.name)
                || std::path::Path::new(&skill.id)
                    .canonicalize()
                    .ok()
                    .is_some_and(|path| {
                        allowed_paths.contains(&path.to_string_lossy().into_owned())
                    })
        });
        features.defaults.model = defaults.model.or(features.defaults.model);
        features.defaults.reasoning_mode =
            defaults.reasoning_mode.or(features.defaults.reasoning_mode);
        Ok(features)
    }
}
