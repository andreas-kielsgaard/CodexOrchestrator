//! Codex CLI implementation of the provider-neutral Agent Runtime port.
//!
//! Argument/capability mapping, JSONL normalization, and process coordination are intentionally
//! separate. The process supervisor remains the sole owner of child processes.

pub(crate) mod app_server;
pub(crate) mod configuration;
pub(crate) mod continuation;
pub(crate) mod profiles;
#[cfg(test)]
mod arguments;
mod capabilities;
#[cfg(test)]
mod protocol;
#[cfg(test)]
mod runtime;

pub(crate) use capabilities::resolve_program;
#[cfg(all(test, feature = "live-tests"))]
pub(crate) use capabilities::{CodexCliCapabilities, CodexCliCapabilityProbe};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use runtime::CodexCliRuntime;

#[cfg(test)]
mod tests;
