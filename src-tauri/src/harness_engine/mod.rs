pub(crate) mod domain;
pub(crate) mod proxy;
pub(crate) mod repository;
pub(crate) mod service;
pub(crate) mod sidecar;

pub(crate) use domain::ManagedMcpUpstreamDescriptor;
pub(crate) use service::{
    HarnessEngineService, HarnessEngineTauriState, ManagedMcpUpstreamOwner,
    ManagedMcpUpstreamRegistry,
};
