//! Reusable identity definitions and Session-owned assigned identity values.

pub mod domain;
pub mod repository;
pub mod service;
#[cfg(test)]
pub use domain::IdentityShape;
pub use domain::{AssignedAgentIdentity, IdentityId};
pub mod definitions;
