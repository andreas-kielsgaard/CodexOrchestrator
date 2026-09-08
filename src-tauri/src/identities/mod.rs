//! Reusable identity definitions and Session-owned assigned identity values.

pub(crate) mod domain;
pub(crate) mod repository;
pub(crate) mod service;
pub(crate) mod transport;

#[cfg(test)]
pub(crate) use domain::IdentityShape;
pub(crate) use domain::{AssignedAgentIdentity, IdentityId};
