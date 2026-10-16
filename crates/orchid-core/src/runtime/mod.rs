//! Runtime infrastructure boundaries used by Agent Sessions.
//!
//! Runtime code must not own Agent Session persistence or Tauri presentation contracts.

pub mod capabilities;
pub mod instance;
pub mod processes;
pub mod providers;
