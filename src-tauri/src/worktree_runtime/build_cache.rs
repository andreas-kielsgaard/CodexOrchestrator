use super::domain::{InstanceIdentity, InstanceProjection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

const RECEIPT_VERSION: u8 = 1;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildReceipt {
    version: u8,
    artifact_key: String,
    executable_hash: String,
    frontend_hash: String,
}

pub(super) fn reusable_build(
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
    tauri_identifier: &str,
) -> bool {
    let receipt = receipt_path(projection);
    let Ok(metadata) = fs::symlink_metadata(&receipt) else {
        return false;
    };
    if !metadata.file_type().is_file() || metadata.len() > 16_384 {
        return false;
    }
    let Ok(receipt): Result<BuildReceipt, _> = fs::read(&receipt)
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(std::io::Error::other))
    else {
        return false;
    };
    if receipt.version != RECEIPT_VERSION
        || receipt.artifact_key != artifact_key(identity, projection, tauri_identifier)
    {
        return false;
    }
    let executable = executable_path(projection);
    file_hash(&executable).is_some_and(|hash| hash == receipt.executable_hash)
        && tree_hash(&projection.paths.frontend_dist)
            .is_some_and(|hash| hash == receipt.frontend_hash)
}

pub(super) fn record_build(
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
    tauri_identifier: &str,
) -> Result<(), String> {
    let executable_hash = file_hash(&executable_path(projection))
        .ok_or_else(|| "The private application executable could not be verified.".to_string())?;
    let frontend_hash = tree_hash(&projection.paths.frontend_dist)
        .ok_or_else(|| "The private frontend output could not be verified.".to_string())?;
    let receipt = BuildReceipt {
        version: RECEIPT_VERSION,
        artifact_key: artifact_key(identity, projection, tauri_identifier),
        executable_hash,
        frontend_hash,
    };
    let path = receipt_path(projection);
    let parent = path
        .parent()
        .ok_or_else(|| "The private build receipt location is invalid.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|_| "The private build receipt location is unavailable.".to_string())?;
    let temporary = path.with_extension("pending");
    fs::write(
        &temporary,
        serde_json::to_vec(&receipt)
            .map_err(|_| "The private build receipt could not be encoded.".to_string())?,
    )
    .and_then(|_| fs::rename(&temporary, &path))
    .map_err(|_| "The private build receipt could not be recorded.".to_string())
}

fn artifact_key(
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
    tauri_identifier: &str,
) -> String {
    let mut hash = Sha256::new();
    for value in [
        identity.source_fingerprint.as_str(),
        identity.git_commit.as_str(),
        identity.build_id.as_str(),
        projection.caches.node_key.as_str(),
        projection.caches.rust_key.as_str(),
        tauri_identifier,
        "tauri-debug-no-bundle-v1",
    ] {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn receipt_path(projection: &InstanceProjection) -> PathBuf {
    projection
        .paths
        .evidence
        .join("verified-build-receipt.json")
}

fn executable_path(projection: &InstanceProjection) -> PathBuf {
    projection
        .paths
        .cargo_target
        .join("debug/codex-orchestrator.exe")
}

fn file_hash(path: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(bytes)))
}

fn tree_hash(root: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(root).ok()?;
    if !metadata.file_type().is_dir() {
        return None;
    }
    let mut files = Vec::new();
    collect_regular_files(root, root, &mut files)?;
    if files.is_empty() {
        return None;
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = Sha256::new();
    for (relative, path) in files {
        let bytes = fs::read(path).ok()?;
        let relative = relative.to_string_lossy();
        hash.update((relative.len() as u64).to_le_bytes());
        hash.update(relative.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    Some(format!("{:x}", hash.finalize()))
}

fn collect_regular_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Option<()> {
    for entry in fs::read_dir(directory).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).ok()?;
        if metadata.file_type().is_dir() {
            collect_regular_files(root, &path, files)?;
        } else if metadata.file_type().is_file() {
            files.push((path.strip_prefix(root).ok()?.to_path_buf(), path));
        } else {
            return None;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_runtime::domain::{
        BuildId, CacheProjection, CacheReuse, InstanceId, PortProjection, RuntimePaths, SessionLink,
    };

    #[test]
    fn exact_private_artifacts_reuse_and_identity_or_content_changes_invalidate() {
        let directory = tempfile::tempdir().expect("directory");
        let first = fixture(directory.path().join("first"), "source-a");
        write_artifacts(&first.1);
        record_build(&first.0, &first.1, "dev.fixture.a").expect("receipt");
        assert!(reusable_build(&first.0, &first.1, "dev.fixture.a"));

        let changed = fixture(directory.path().join("first"), "source-b");
        assert!(!reusable_build(&changed.0, &changed.1, "dev.fixture.a"));

        fs::write(executable_path(&first.1), "changed executable").expect("change executable");
        assert!(!reusable_build(&first.0, &first.1, "dev.fixture.a"));
    }

    #[test]
    fn sibling_instances_never_reuse_each_others_mutable_artifacts() {
        let directory = tempfile::tempdir().expect("directory");
        let first = fixture(directory.path().join("first"), "source-a");
        let second = fixture(directory.path().join("second"), "source-a");
        write_artifacts(&first.1);
        record_build(&first.0, &first.1, "dev.fixture.a").expect("receipt");
        assert!(reusable_build(&first.0, &first.1, "dev.fixture.a"));
        assert!(!reusable_build(&second.0, &second.1, "dev.fixture.a"));
        assert_ne!(first.1.paths.cargo_target, second.1.paths.cargo_target);
        assert_ne!(first.1.paths.frontend_dist, second.1.paths.frontend_dist);
    }

    fn fixture(root: PathBuf, source: &str) -> (InstanceIdentity, InstanceProjection) {
        let instance_id = InstanceId::new(format!("wt-{source}")).expect("instance");
        let identity = InstanceIdentity {
            instance_id,
            review_name: "Fixture".into(),
            worktree_path: root.join("source"),
            git_commit: "abcdef0123456789".into(),
            source_fingerprint: format!("{source:0<64}"),
            build_id: BuildId::new(format!("build-{source}")).expect("build"),
            session_link: SessionLink::new(format!("session-{source}")).expect("session"),
        };
        let projection = InstanceProjection {
            caches: CacheProjection {
                node_key: "node-key".into(),
                node_path: root.join("cache/node"),
                node_reuse: CacheReuse::IsolatedFallback,
                rust_key: "rust-key".into(),
                rust_path: root.join("cache/rust"),
                rust_reuse: CacheReuse::IsolatedFallback,
            },
            paths: RuntimePaths {
                instance_root: root.clone(),
                frontend_dist: root.join("dist"),
                cargo_target: root.join("cargo-target"),
                app_data: root.join("app-data"),
                credentials_home: root.join("credentials"),
                temp: root.join("temp"),
                logs: root.join("logs"),
                screenshots: root.join("screenshots"),
                recordings: root.join("recordings"),
                evidence: root.join("evidence"),
            },
            ports: PortProjection { vite: 1, status: 2 },
        };
        (identity, projection)
    }

    fn write_artifacts(projection: &InstanceProjection) {
        fs::create_dir_all(projection.paths.cargo_target.join("debug")).expect("target");
        fs::create_dir_all(&projection.paths.frontend_dist).expect("dist");
        fs::write(executable_path(projection), "executable").expect("executable");
        fs::write(
            projection.paths.frontend_dist.join("index.html"),
            "frontend",
        )
        .expect("frontend");
    }
}
