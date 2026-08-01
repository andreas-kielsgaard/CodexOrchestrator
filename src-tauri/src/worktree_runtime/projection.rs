use super::domain::{
    CacheProjection, CacheReuse, InstanceId, InstanceProjection, PortProjection,
    RuntimeContractError, RuntimePaths,
};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectionRequest {
    pub(crate) instance_id: InstanceId,
    pub(crate) instances_root: PathBuf,
    pub(crate) node_cache_root: PathBuf,
    pub(crate) rust_cache_root: PathBuf,
    pub(crate) node_cache_key: String,
    pub(crate) rust_cache_key: String,
    pub(crate) node_cache_reuse: CacheReuse,
    pub(crate) rust_cache_reuse: CacheReuse,
    pub(crate) ports: PortProjection,
}

/// Projects paths and cache routes without claiming that any directory or cache entry exists.
pub(crate) fn project_instance(
    request: ProjectionRequest,
) -> Result<InstanceProjection, RuntimeContractError> {
    require_absolute(&request.instances_root, "instances root")?;
    require_absolute(&request.node_cache_root, "node cache root")?;
    require_absolute(&request.rust_cache_root, "Rust cache root")?;
    let root = request.instances_root.join(request.instance_id.as_str());
    let projection = InstanceProjection {
        caches: CacheProjection {
            node_path: request.node_cache_root.join(&request.node_cache_key),
            node_reuse: request.node_cache_reuse,
            rust_path: request.rust_cache_root.join(&request.rust_cache_key),
            rust_reuse: request.rust_cache_reuse,
            node_key: request.node_cache_key,
            rust_key: request.rust_cache_key,
        },
        paths: RuntimePaths {
            frontend_dist: root.join("dist"),
            cargo_target: root.join("cargo-target"),
            app_data: root.join("app-data"),
            credentials_home: root.join("credentials"),
            temp: root.join("temp"),
            logs: root.join("logs"),
            screenshots: root.join("screenshots"),
            recordings: root.join("recordings"),
            evidence: root.join("evidence"),
            instance_root: root,
        },
        ports: request.ports,
    };
    projection.validate()?;
    Ok(projection)
}

fn require_absolute(path: &Path, label: &str) -> Result<(), RuntimeContractError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(RuntimeContractError::new(format!(
            "{label} must be absolute and normalized"
        )));
    }
    Ok(())
}
