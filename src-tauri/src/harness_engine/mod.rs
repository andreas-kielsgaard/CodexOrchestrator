pub(crate) mod catalog;
pub(crate) mod catalog_repository;
pub(crate) mod catalog_service;
pub(crate) mod configuration;
pub(crate) mod domain;
pub(crate) mod proxy;
pub(crate) mod repository;
pub(crate) mod resolution;
pub(crate) mod service;
pub(crate) mod sidecar;
pub(crate) mod transport;
pub(crate) mod workflow_adapter;

pub(crate) use domain::ManagedMcpUpstreamDescriptor;
pub(crate) use service::{
    HarnessEngineService, HarnessEngineTauriState, ManagedMcpUpstreamOwner,
    ManagedMcpUpstreamRegistry,
};
