use std::sync::{Arc, Weak};
pub(crate) fn start(
    control: Arc<super::OrchidControl>,
    _voice: Weak<crate::voice_sessions::VoiceSessionApplication>,
) -> Result<
    (
        Vec<crate::harness_engine::ManagedMcpUpstreamDescriptor>,
        Box<dyn crate::harness_engine::ManagedMcpUpstreamOwner>,
    ),
    String,
> {
    orchid_core::orchid_control::mcp::start(control, None)
}
