#![allow(unused_imports)]

mod capability_profile;
mod defaults;
mod inventory;
mod model_catalogue;
pub(crate) use inventory::{NativeCapabilityEntry, NativeCapabilityInventory};
pub(crate) use model_catalogue::{ModelCatalogueView, StoredModelCatalogue};
mod creation_intent;
mod quick_features;
pub(crate) use quick_features::{QuickModel, QuickReasoningMode, QuickSkill, RuntimeQuickFeatures};
mod node_profile;
mod ports;
mod product_skills;
pub(crate) use product_skills::ProductSkillRoots;
pub(crate) mod skill_mentions;
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
    otp_skill_group, CapabilityProfile, ModelAllowance, ProfileRoutePolicy,
    CAPABILITY_PROFILE_CONTRACT_VERSION, NATIVE_MCP_GROUP, NATIVE_SKILL_GROUP, ORCHID_SKILL_GROUP,
};
pub(crate) use creation_intent::SessionCreationIntent;
pub(crate) use crate::runtime::providers::codex::configuration::CodexConfigurationSource;
pub(crate) use node_profile::{NodeProfile, NODE_PROFILE_CONTRACT_VERSION};
pub(crate) use ports::{
    CapabilityProfileRepository, CapabilityProfileRepositoryError, ProviderConfigurationSource,
    ProviderConfigurationSourceError, RuntimeSkillRoot,
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
    RUNTIME_PROFILE_CONTRACT_VERSION,
};
pub(crate) use service::{CapabilityProfileService, CapabilityProfileServiceError};
pub(crate) use session_profile::SessionProfile;
pub(crate) use session_skills::{
    compile_session_skill_inputs, pin_discovered_skill, validate_session_skill_inputs,
};

#[cfg(test)]
mod tests;
