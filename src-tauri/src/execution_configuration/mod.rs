#![allow(unused_imports)]

mod capability_profile;
mod configured_runtime;
mod creation_intent;
mod node_profile;
mod ports;
mod repository;
mod resolution;
mod runtime_profile;
mod service;
mod session_profile;
pub(crate) mod transport;

pub(crate) use capability_profile::{CapabilityProfile, CAPABILITY_PROFILE_CONTRACT_VERSION};
pub(crate) use configured_runtime::configured_runtime_profile;
pub(crate) use creation_intent::SessionCreationIntent;
pub(crate) use node_profile::{NodeProfile, NODE_PROFILE_CONTRACT_VERSION};
pub(crate) use ports::{CapabilityProfileRepository, CapabilityProfileRepositoryError};
pub(crate) use repository::{
    InMemoryCapabilityProfileRepository, SqliteCapabilityProfileRepository,
    CAPABILITY_PROFILE_SCHEMA,
};
pub(crate) use resolution::{
    DirectUserInvocationRequest, DirectUserInvocationResolution, ResolutionError,
    SessionCreationRequest, SessionCreationResolution, SessionProfileResolver,
    SESSION_CREATION_REQUEST_CONTRACT_VERSION,
};
pub(crate) use runtime_profile::{
    CapabilitySet, RuntimeProfileSnapshot, RuntimeSelections, SandboxMode,
};
pub(crate) use service::{CapabilityProfileService, CapabilityProfileServiceError};
pub(crate) use session_profile::SessionProfile;

#[cfg(test)]
mod tests;
