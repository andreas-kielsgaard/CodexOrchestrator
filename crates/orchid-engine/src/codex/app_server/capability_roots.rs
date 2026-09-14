//! Connection-local native discovery roots; neither global configuration nor workspace files change.
use super::connection::Connection;
use crate::contracts::ports::{RuntimePortError, RuntimePortErrorKind};

pub(super) fn apply_skill_roots(
    connection: &Connection,
    roots: &[String],
) -> Result<(), RuntimePortError> {
    if roots.is_empty() {
        return Ok(());
    }
    if roots
        .iter()
        .any(|root| !std::path::Path::new(root).is_absolute())
    {
        return Err(RuntimePortError::new(
            RuntimePortErrorKind::UnsupportedOptions,
            "Skill roots must be absolute",
        ));
    }
    connection.call(
        "skills/extraRoots/set",
        serde_json::json!({"extraRoots":roots}),
    )?;
    Ok(())
}
