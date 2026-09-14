mod application;
mod domain;
pub(crate) mod repository;
pub(crate) mod device_locations;
pub(crate) mod transport;

pub(crate) use application::RepositoryCatalog;
pub(crate) use domain::{
    ResolvedBranchTarget, ResolvedRepoBranchWorktreeTarget, ResolvedRepositoryTarget,
    ResolvedWorktreeTarget,
};
pub(crate) use repository::REPOSITORY_CATALOG_SCHEMA;
