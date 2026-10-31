use std::sync::Arc;
pub(super) fn compose(
    database: Arc<crate::persistence::ActiveDatabase>,
    agents: Arc<crate::agent_sessions::application::AgentSessionApplication>,
    profiles: Arc<crate::execution_configuration::CapabilityProfileService>,
    repositories: Arc<crate::repository_catalog::RepositoryCatalog>,
    upstreams: Arc<crate::harness_engine::ManagedMcpUpstreamRegistry>,
    workflows: Arc<crate::workflows::execution::WorkflowExecutionService>,
) -> Result<Arc<crate::voice_sessions::VoiceSessionApplication>, String> {
    let speech_root = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .ok_or("LOCALAPPDATA is required for local speech")?
        .join("Orchid/voice-speech");
    let voice = crate::voice_sessions::VoiceSessionApplication::new(
        crate::voice_sessions::repository::VoiceSessionRepository(database),
        agents.clone(),
        profiles.clone(),
        speech_root,
        workflows.clone(),
    )?;
    voice.register_workflow_runtime()?;
    let control = Arc::new(crate::orchid_control::OrchidControl {
        agents,
        profiles,
        repositories,
    });
    let (descriptors, owner) = crate::orchid_control::mcp::start(control, Arc::downgrade(&voice))?;
    let mut descriptors = descriptors.into_iter();
    let registration = upstreams.register(descriptors.next().ok_or("No Orchid MCP endpoint")?)?;
    if let Err(owner) = upstreams.retain_owner(&registration, owner) {
        owner.stop();
        upstreams.unregister(&registration);
        return Err("Unable to retain Orchid tools".into());
    }
    for descriptor in descriptors {
        upstreams.register(descriptor)?;
    }
    Ok(voice)
}
