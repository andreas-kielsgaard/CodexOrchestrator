//! Codex-owned skill discovery.
use super::connection::Connection;
use crate::contracts::ports::RuntimePortError;

pub(super) fn apply_skill_roots(
    _connection: &Connection,
    _roots: &[String],
) -> Result<(), RuntimePortError> {
    // CLI 0.154's active app-server protocol no longer accepts this request.
    // `skills/config/write` configures named skills, not discovery roots, so it
    // is not a compatible substitute. Native Codex configuration remains the
    // source of discovery and an optional product root must not block launch.
    Ok(())
}
