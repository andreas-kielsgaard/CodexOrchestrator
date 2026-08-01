use super::progress::{ReviewOperationHistoryView, ReviewOperationStageView};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBuildOperationEvidenceView {
    pub(crate) operation_ref: String,
    pub(crate) operation: String,
    pub(crate) state: String,
    pub(crate) started_at_ms: u64,
    pub(crate) ended_at_ms: Option<u64>,
    pub(crate) duration_ms: u64,
    pub(crate) stage_history: Vec<ReviewOperationStageView>,
    pub(crate) build_reuse: bool,
    pub(crate) no_compilation: bool,
    pub(crate) instance_ref: String,
    pub(crate) git_commit: String,
    pub(crate) source_fingerprint: String,
    pub(crate) receipt: ReviewBuildReceiptEvidenceView,
    pub(crate) isolation: ReviewBuildIsolationEvidenceView,
    pub(crate) artifact_paths: ReviewBuildArtifactPathsView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBuildReceiptEvidenceView {
    pub(crate) artifact_key: String,
    pub(crate) receipt_hash: String,
    pub(crate) executable_hash: String,
    pub(crate) frontend_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBuildIsolationEvidenceView {
    pub(crate) mutable_paths_instance_private: bool,
    pub(crate) cargo_target_instance_private: bool,
    pub(crate) frontend_output_instance_private: bool,
    pub(crate) app_data_instance_private: bool,
    pub(crate) logs_instance_private: bool,
    pub(crate) node_cache_instance_private: bool,
    pub(crate) rust_cache_instance_private: bool,
    pub(crate) node_cache_mode: String,
    pub(crate) node_cache_key: String,
    pub(crate) rust_cache_mode: String,
    pub(crate) rust_cache_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBuildArtifactPathsView {
    pub(crate) instance_root: PathBuf,
    pub(crate) executable: PathBuf,
    pub(crate) frontend_output: PathBuf,
    pub(crate) app_data: PathBuf,
    pub(crate) logs: PathBuf,
    pub(crate) receipt: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredIdentity {
    git_commit: String,
    source_fingerprint: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProjection {
    caches: StoredCaches,
    paths: StoredPaths,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCaches {
    node_key: String,
    node_path: PathBuf,
    node_reuse: String,
    rust_key: String,
    rust_path: PathBuf,
    rust_reuse: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredPaths {
    instance_root: PathBuf,
    frontend_dist: PathBuf,
    cargo_target: PathBuf,
    app_data: PathBuf,
    credentials_home: PathBuf,
    temp: PathBuf,
    logs: PathBuf,
    screenshots: PathBuf,
    recordings: PathBuf,
    evidence: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredReceipt {
    version: u8,
    artifact_key: String,
    executable_hash: String,
    frontend_hash: String,
}

pub(crate) fn assemble(
    registry_path: PathBuf,
    instance_ref: String,
    operation: ReviewOperationHistoryView,
) -> Result<ReviewBuildOperationEvidenceView, String> {
    if operation.operation != "build" {
        return Err("Build evidence is available only for a Build operation.".into());
    }
    let connection = Connection::open_with_flags(
        registry_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "The isolated build registry evidence is unavailable.".to_string())?;
    let stored = connection
        .query_row(
            "SELECT identity_json, projection_json FROM worktree_runtime_instances WHERE instance_id=?1",
            [&instance_ref],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| "The isolated build registry evidence is unavailable.".to_string())?
        .ok_or_else(|| "The isolated build identity is unavailable.".to_string())?;
    let identity: StoredIdentity = serde_json::from_str(&stored.0)
        .map_err(|_| "The isolated build identity is invalid.".to_string())?;
    let projection: StoredProjection = serde_json::from_str(&stored.1)
        .map_err(|_| "The isolated build projection is invalid.".to_string())?;
    let receipt_path = projection
        .paths
        .evidence
        .join("verified-build-receipt.json");
    let receipt_bytes = fs::read(&receipt_path)
        .map_err(|_| "The verified private build receipt is unavailable.".to_string())?;
    if receipt_bytes.len() > 16_384 {
        return Err("The verified private build receipt is invalid.".into());
    }
    let receipt: StoredReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|_| "The verified private build receipt is invalid.".to_string())?;
    if receipt.version != 1 {
        return Err("The verified private build receipt version is unsupported.".into());
    }
    let private_paths = [
        &projection.paths.frontend_dist,
        &projection.paths.cargo_target,
        &projection.paths.app_data,
        &projection.paths.credentials_home,
        &projection.paths.temp,
        &projection.paths.logs,
        &projection.paths.screenshots,
        &projection.paths.recordings,
        &projection.paths.evidence,
    ];
    let mutable_paths_instance_private = private_paths
        .iter()
        .all(|path| path.starts_with(&projection.paths.instance_root));
    let stages = &operation.stage_history;
    let build_reuse = stages.iter().any(|stage| stage.stage == "build-reuse");
    let no_compilation = build_reuse
        && !stages.iter().any(|stage| {
            matches!(
                stage.stage.as_str(),
                "typecheck" | "frontend-build" | "tauri-compile-link"
            )
        });
    let ended_at_ms = (operation.state != "pending").then_some(operation.updated_at_ms);
    Ok(ReviewBuildOperationEvidenceView {
        operation_ref: operation.operation_ref,
        operation: operation.operation,
        state: operation.state,
        started_at_ms: operation.started_at_ms,
        ended_at_ms,
        duration_ms: operation
            .updated_at_ms
            .saturating_sub(operation.started_at_ms),
        stage_history: operation.stage_history,
        build_reuse,
        no_compilation,
        instance_ref,
        git_commit: identity.git_commit,
        source_fingerprint: identity.source_fingerprint,
        receipt: ReviewBuildReceiptEvidenceView {
            artifact_key: receipt.artifact_key,
            receipt_hash: format!("{:x}", Sha256::digest(&receipt_bytes)),
            executable_hash: receipt.executable_hash,
            frontend_hash: receipt.frontend_hash,
        },
        isolation: ReviewBuildIsolationEvidenceView {
            mutable_paths_instance_private,
            cargo_target_instance_private: projection
                .paths
                .cargo_target
                .starts_with(&projection.paths.instance_root),
            frontend_output_instance_private: projection
                .paths
                .frontend_dist
                .starts_with(&projection.paths.instance_root),
            app_data_instance_private: projection
                .paths
                .app_data
                .starts_with(&projection.paths.instance_root),
            logs_instance_private: projection
                .paths
                .logs
                .starts_with(&projection.paths.instance_root),
            node_cache_instance_private: projection
                .caches
                .node_path
                .starts_with(&projection.paths.instance_root),
            rust_cache_instance_private: projection
                .caches
                .rust_path
                .starts_with(&projection.paths.instance_root),
            node_cache_mode: projection.caches.node_reuse,
            node_cache_key: projection.caches.node_key,
            rust_cache_mode: projection.caches.rust_reuse,
            rust_cache_key: projection.caches.rust_key,
        },
        artifact_paths: ReviewBuildArtifactPathsView {
            instance_root: projection.paths.instance_root,
            executable: projection
                .paths
                .cargo_target
                .join("debug/codex-orchestrator.exe"),
            frontend_output: projection.paths.frontend_dist,
            app_data: projection.paths.app_data,
            logs: projection.paths.logs,
            receipt: receipt_path,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use tempfile::tempdir;

    #[test]
    fn evidence_distinguishes_cold_compile_from_exact_reuse_and_keeps_paths_instance_private() {
        let directory = tempdir().expect("directory");
        let instance_root = directory.path().join("instances/wt-proof");
        let evidence = instance_root.join("evidence");
        fs::create_dir_all(&evidence).expect("evidence");
        let receipt = serde_json::json!({
            "version": 1,
            "artifactKey": "artifact-proof",
            "executableHash": "exe-proof",
            "frontendHash": "frontend-proof"
        });
        fs::write(
            evidence.join("verified-build-receipt.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .expect("receipt");
        let registry = directory.path().join("registry.sqlite");
        let connection = Connection::open(&registry).expect("registry");
        connection
            .execute_batch(
                "CREATE TABLE worktree_runtime_instances (
                    instance_id TEXT PRIMARY KEY,
                    identity_json TEXT NOT NULL,
                    projection_json TEXT NOT NULL
                );",
            )
            .expect("schema");
        let path = |name: &str| instance_root.join(name);
        let identity = serde_json::json!({
            "gitCommit": "abcdef0123456789",
            "sourceFingerprint": "a".repeat(64)
        });
        let projection = serde_json::json!({
            "caches": {
                "nodeKey": "node-proof",
                "nodePath": directory.path().join("shared-cache/npm/node-proof"),
                "nodeReuse": "shared_keyed",
                "rustKey": "rust-proof",
                "rustPath": path("cache/rust"),
                "rustReuse": "isolated_fallback"
            },
            "paths": {
                "instanceRoot": instance_root,
                "frontendDist": path("dist"),
                "cargoTarget": path("cargo-target"),
                "appData": path("app-data"),
                "credentialsHome": path("credentials"),
                "temp": path("temp"),
                "logs": path("logs"),
                "screenshots": path("screenshots"),
                "recordings": path("recordings"),
                "evidence": evidence
            }
        });
        connection
            .execute(
                "INSERT INTO worktree_runtime_instances VALUES (?1, ?2, ?3)",
                params!["wt-proof", identity.to_string(), projection.to_string()],
            )
            .expect("instance");
        drop(connection);

        let operation = ReviewOperationHistoryView {
            operation_ref: "review-operation-proof".into(),
            operation: "build".into(),
            state: "succeeded".into(),
            stage_label: "Finished".into(),
            started_at_ms: 100,
            updated_at_ms: 125,
            stage_history: vec![
                stage("preparation", 100),
                stage("build-reuse", 110),
                stage("complete", 125),
            ],
            output: vec!["Verified private build".into()],
            output_complete: true,
        };
        let view = assemble(registry, "wt-proof".into(), operation).expect("evidence");
        assert!(view.build_reuse);
        assert!(view.no_compilation);
        assert_eq!(view.duration_ms, 25);
        assert!(view.isolation.mutable_paths_instance_private);
        assert!(view.isolation.rust_cache_instance_private);
        assert!(!view.isolation.node_cache_instance_private);
        assert_eq!(view.isolation.node_cache_mode, "shared_keyed");
        assert!(view
            .artifact_paths
            .executable
            .starts_with(&view.artifact_paths.instance_root));
    }

    fn stage(stage: &str, observed_at_ms: u64) -> ReviewOperationStageView {
        ReviewOperationStageView {
            stage: stage.into(),
            stage_label: stage.into(),
            observed_at_ms,
        }
    }
}
