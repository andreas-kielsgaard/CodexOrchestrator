//! Runtime infrastructure boundaries used by Agent Sessions.
//!
//! Runtime code must not own Agent Session persistence or Tauri presentation contracts.

pub(crate) mod capabilities;
pub(crate) mod codex;
pub(crate) mod instance;
pub(crate) mod processes;
