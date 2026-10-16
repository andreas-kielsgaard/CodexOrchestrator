//! Provider-neutral Agent Session domain and application contracts.
//!
//! This module owns durable identity, lifecycle invariants, repository operations, and the runtime
//! port used by later slices. It may depend on serialization and general-purpose value/time types,
//! but it must not depend on Tauri, SQLite, operating-system process APIs, React, or Codex protocol
//! types. Provider adapters and process ownership stay under `runtime`; transport DTOs remain a
//! separate boundary.

pub mod application;
pub mod domain;
pub mod file_history;
pub mod notifications;
pub mod ports;
pub mod references;
pub mod repository;
pub mod session_event_adapter;
#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "live-tests"))]
mod live_smoke;

pub mod workspace;

pub mod interactions;

pub mod organization;

pub mod imports;

pub mod preparation;

pub mod queue;

pub mod target_operation;

pub mod history;

pub mod api;
pub mod client;
pub mod queries;
