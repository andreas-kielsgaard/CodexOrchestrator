//! Reusable identity definitions and Session-owned assigned identity values.

pub(crate) mod domain;
pub(crate) mod repository;
pub(crate) mod service;
pub(crate) mod transport;

pub(crate) use domain::{AssignedAgentIdentity, IdentityId, IdentityShape};
