//! Provider-neutral Agent Session domain and application contracts.
//!
//! This module owns durable identity, lifecycle invariants, repository operations, and the runtime
//! port used by later slices. It may depend on serialization and general-purpose value/time types,
//! but it must not depend on Tauri, SQLite, operating-system process APIs, React, or Codex protocol
//! types. Provider adapters and process ownership stay under `runtime`; transport DTOs remain a
//! separate boundary.

pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod ports;
pub(crate) mod repository;
pub(crate) mod transport;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod live_smoke;
