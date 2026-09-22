#![allow(unused_imports)]

mod capability_profile;
mod defaults;
mod inventory;
mod model_catalogue;
pub(crate) use model_catalogue::{ModelCatalogueView, StoredModelCatalogue};
pub(crate) use inventory::{NativeCapabilityEntry, NativeCapabilityInventory};
mod creation_intent;
mod native_codex;
mod quick_features;
pub(crate) use quick_features::{QuickModel, QuickReasoningMode, QuickSkill, RuntimeQuickFeatures};
mod node_profile;
mod ports;
mod repository;
mod resolution;
mod runtime_profile;
mod service;
mod session_profile;
mod session_skills;
mod skill_reader;
pub(crate) use skill_reader::SessionSkillReaderProvisioner;
pub(crate) mod transport;

pub(crate) use capability_profile::{
    CapabilityProfile, ModelAllowance, ProfileRoutePolicy, CAPABILITY_PROFILE_CONTRACT_VERSION,
};
pub(crate) use creation_intent::SessionCreationIntent;
pub(crate) use native_codex::NativeCodexSelectedRuntimeProfileSource;
pub(crate) use node_profile::{NodeProfile, NODE_PROFILE_CONTRACT_VERSION};
pub(crate) use ports::{
    CapabilityProfileRepository, CapabilityProfileRepositoryError, RuntimeSkillRoot,
    SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError, WorkingContextProfileSource,
    PinnedConfigurationProfileSource,
};
pub(crate) use repository::{
    initialize_capability_profile_storage, InMemoryCapabilityProfileRepository,
    SqliteCapabilityProfileRepository, CAPABILITY_PROFILE_SCHEMA,
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
pub(crate) use session_skills::{compile_session_skill_inputs, validate_session_skill_inputs};

#[cfg(test)]
mod tests;
