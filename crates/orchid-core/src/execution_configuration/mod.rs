#![allow(unused_imports)]

mod capability_profile;
mod defaults;
mod inventory;
mod model_catalogue;
pub use inventory::{NativeCapabilityEntry, NativeCapabilityInventory};
pub use model_catalogue::{ModelCatalogueView, StoredModelCatalogue};
mod creation_intent;
mod quick_features;
pub use quick_features::{QuickModel, QuickReasoningMode, QuickSkill, RuntimeQuickFeatures};
mod node_profile;
mod ports;
mod product_skills;
pub use product_skills::ProductSkillRoots;
mod repository;
mod resolution;
mod runtime_profile;
mod service;
mod session_profile;
mod setups;
pub mod skill_mentions;
pub use setups::{ProviderSetup, ProviderSetupState};
mod session_skills;
mod skill_reader;
pub use capability_profile::{
    otp_skill_group, CapabilityProfile, ModelAllowance, ProfileRoutePolicy,
    CAPABILITY_PROFILE_CONTRACT_VERSION, NATIVE_MCP_GROUP, NATIVE_SKILL_GROUP, ORCHID_SKILL_GROUP,
};
pub use creation_intent::SessionCreationIntent;
pub use node_profile::{NodeProfile, NODE_PROFILE_CONTRACT_VERSION};
pub use ports::{
    CapabilityProfileRepository, CapabilityProfileRepositoryError, ProviderConfigurationSource,
    ProviderConfigurationSourceError, RuntimeSkillRoot,
};
pub use repository::{
    initialize_capability_profile_storage, migrate_route_model_catalogues,
    InMemoryCapabilityProfileRepository, SqliteCapabilityProfileRepository,
    CAPABILITY_PROFILE_SCHEMA,
};
pub use resolution::{
    DirectUserInvocationRequest, DirectUserInvocationResolution, ResolutionError,
    SessionCreationRequest, SessionCreationResolution, SessionProfileResolver,
    SESSION_CREATION_REQUEST_CONTRACT_VERSION,
};
pub use runtime_profile::{
    CapabilitySet, RuntimeProfileSnapshot, RuntimeSelections, SandboxMode,
    RUNTIME_PROFILE_CONTRACT_VERSION,
};
pub use service::{CapabilityProfileService, CapabilityProfileServiceError};
pub use session_profile::SessionProfile;
pub use session_skills::{
    compile_session_skill_inputs, pin_discovered_skill, validate_session_skill_inputs,
};
pub use skill_reader::SessionSkillReaderProvisioner;

pub mod definitions;
#[cfg(test)]
mod tests;
