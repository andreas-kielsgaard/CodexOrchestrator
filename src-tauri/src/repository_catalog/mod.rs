mod application;
pub(crate) mod device_locations;
mod domain;
pub(crate) mod repository;
pub(crate) mod transport;

pub(crate) use application::RepositoryCatalog;
pub(crate) use domain::{
    RegisteredRepository, ResolvedBranchTarget, ResolvedRepoBranchWorktreeTarget,
    ResolvedRepositoryTarget, ResolvedWorktreeTarget,
};
pub(crate) use repository::REPOSITORY_CATALOG_SCHEMA;
