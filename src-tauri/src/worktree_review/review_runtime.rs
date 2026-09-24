use super::domain::{
    CleanupStorageKey, ReviewBuild, ReviewBuildId, ReviewSourceSelection, ReviewWorkspace,
};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) const REVIEW_INSTANCE_LABEL_ENV: &str = "CODEX_ORCHESTRATOR_REVIEW_INSTANCE_LABEL";
const RUNTIME_DIRECTORY: &str = "review-runtimes";
const SEED_MANIFEST: &str = "seed.json";
const SEED_FORMAT_VERSION: u32 = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReviewApplicationIdentity {
    pub(crate) identifier: String,
    pub(crate) label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreparedReviewRuntime {
    pub(crate) app_data: PathBuf,
    pub(crate) webview_data: PathBuf,
    pub(crate) identity: ReviewApplicationIdentity,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SeedManifest<'a> {
    seed_format_version: u32,
    build_id: &'a str,
    branch_or_commit: &'a str,
    worktree_id: &'a str,
    source_app_data: String,
    seeded_at: String,
    controller_schema_version: i64,
    target_schema_version: Option<i64>,
    database_seeded: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExistingSeedManifest {
    seed_format_version: u32,
    target_schema_version: Option<i64>,
    database_seeded: bool,
}

pub(crate) fn identity(
    build: &ReviewBuild,
    workspace: &ReviewWorkspace,
) -> ReviewApplicationIdentity {
    let source = build
        .source
        .branch_ref
        .as_ref()
        .map(|value| value.as_str().trim_start_matches("refs/heads/"))
        .unwrap_or_else(|| source_object(&build.source.selection));
    let source_slug = slug(source, 42);
    let worktree_slug = short_slug(workspace.worktree_id.as_str(), 18);
    let build_slug = short_slug(build.id.as_str(), 18);
    ReviewApplicationIdentity {
        identifier: format!(
            "dev.codex-orchestrator.review.{source_slug}.wt-{worktree_slug}.bld-{build_slug}"
        ),
        label: format!(
            "{source} · {} · {}",
            workspace.worktree_id.as_str(),
            build.id.as_str()
        ),
    }
}

pub(crate) fn runtime_storage_key(build_id: &ReviewBuildId) -> Result<CleanupStorageKey, String> {
    CleanupStorageKey::new(format!("{RUNTIME_DIRECTORY}/{}", build_id.as_str()))
        .map_err(|error| error.to_string())
}

pub(crate) fn prepare(
    review_root: &Path,
    source_app_data: &Path,
    build: &ReviewBuild,
    workspace: &ReviewWorkspace,
    compiled_schema_version: Option<i64>,
) -> Result<PreparedReviewRuntime, String> {
    let review_root = review_root
        .canonicalize()
        .map_err(|error| format!("Worktree Review storage is unavailable: {error}"))?;
    let source_app_data = source_app_data
        .canonicalize()
        .map_err(|error| format!("Controller AppData is unavailable: {error}"))?;
    let storage_key = runtime_storage_key(&build.id)?;
    let root = review_root.join(storage_key.as_str());
    let manifest = root.join(SEED_MANIFEST);
    let identity = identity(build, workspace);
    let target_schema_version = compiled_schema_version
        .or_else(|| target_schema_version(Path::new(workspace.location.as_str())));
    let database_seeded = target_schema_version == Some(crate::storage::ACTIVE_SCHEMA_VERSION);
    if existing_seed_is_compatible(&manifest, target_schema_version, database_seeded)
        && root.join("app-data").is_dir()
        && root.join("webview-data").is_dir()
    {
        return Ok(PreparedReviewRuntime {
            app_data: root.join("app-data"),
            webview_data: root.join("webview-data"),
            identity,
        });
    }

    let parent = root
        .parent()
        .ok_or_else(|| "Review runtime storage is invalid.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Review runtime storage could not be created: {error}"))?;
    if root.exists() {
        fs::remove_dir_all(&root)
            .map_err(|error| format!("Incomplete review runtime could not be reset: {error}"))?;
    }
    let staging = parent.join(format!(".{}.staging", build.id.as_str()));
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|error| format!("Review runtime staging could not be reset: {error}"))?;
    }
    let app_data = staging.join("app-data");
    let webview_data = staging.join("webview-data");
    fs::create_dir_all(&app_data)
        .and_then(|_| fs::create_dir_all(&webview_data))
        .map_err(|error| format!("Review runtime staging could not be created: {error}"))?;
    if let Err(error) = mirror_app_data(&source_app_data, &app_data, database_seeded) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let source_label = build
        .source
        .branch_ref
        .as_ref()
        .map(|value| value.as_str())
        .unwrap_or_else(|| source_object(&build.source.selection));
    let seed = SeedManifest {
        seed_format_version: SEED_FORMAT_VERSION,
        build_id: build.id.as_str(),
        branch_or_commit: source_label,
        worktree_id: workspace.worktree_id.as_str(),
        source_app_data: source_app_data.to_string_lossy().into_owned(),
        seeded_at: chrono::Utc::now().to_rfc3339(),
        controller_schema_version: crate::storage::ACTIVE_SCHEMA_VERSION,
        target_schema_version,
        database_seeded,
    };
    fs::write(
        staging.join(SEED_MANIFEST),
        serde_json::to_vec_pretty(&seed).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("Review runtime seed receipt could not be written: {error}"))?;
    fs::rename(&staging, &root)
        .map_err(|error| format!("Review runtime seed could not be published: {error}"))?;
    Ok(PreparedReviewRuntime {
        app_data: root.join("app-data"),
        webview_data: root.join("webview-data"),
        identity,
    })
}

fn existing_seed_is_compatible(
    manifest: &Path,
    target_schema_version: Option<i64>,
    database_seeded: bool,
) -> bool {
    let Ok(bytes) = fs::read(manifest) else {
        return false;
    };
    let Ok(existing) = serde_json::from_slice::<ExistingSeedManifest>(&bytes) else {
        return false;
    };
    existing.seed_format_version == SEED_FORMAT_VERSION
        && existing.target_schema_version == target_schema_version
        && existing.database_seeded == database_seeded
}

fn target_schema_version(worktree: &Path) -> Option<i64> {
    const PREFIX: &str = "pub(crate) const ACTIVE_SCHEMA_VERSION: i64 =";
    fs::read_to_string(worktree.join("src-tauri").join("src").join("storage.rs"))
        .ok()?
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix(PREFIX)
                .and_then(|value| value.trim().trim_end_matches(';').parse().ok())
        })
}

fn mirror_app_data(source: &Path, destination: &Path, copy_databases: bool) -> Result<(), String> {
    for entry in fs::read_dir(source)
        .map_err(|error| format!("Controller AppData could not be read: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Controller AppData could not be read: {error}"))?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if excluded_root_entry(&name_text) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Controller AppData entry could not be inspected: {error}"))?;
        mirror_entry(
            &entry.path(),
            destination.join(&name),
            &name_text,
            file_type,
            copy_databases,
        )?;
    }
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path, copy_databases: bool) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("Review AppData directory could not be created: {error}"))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("Controller AppData directory could not be read: {error}"))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if excluded_root_entry(&name_text) {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        mirror_entry(
            &entry.path(),
            destination.join(&name),
            &name_text,
            file_type,
            copy_databases,
        )?;
    }
    Ok(())
}

fn mirror_entry(
    source: &Path,
    destination: PathBuf,
    name: &str,
    file_type: fs::FileType,
    copy_databases: bool,
) -> Result<(), String> {
    if file_type.is_symlink() {
        return Ok(());
    }
    if file_type.is_dir() {
        return copy_directory(source, &destination, copy_databases);
    }
    if !file_type.is_file() {
        return Ok(());
    }
    if name.to_ascii_lowercase().ends_with(".sqlite") {
        if copy_databases {
            snapshot_sqlite(source, &destination)
        } else {
            Ok(())
        }
    } else {
        fs::copy(source, destination)
            .map(|_| ())
            .map_err(|error| format!("Controller AppData file could not be copied: {error}"))
    }
}

fn snapshot_sqlite(source: &Path, destination: &Path) -> Result<(), String> {
    let connection = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("AppData database could not be opened for snapshot: {error}"))?;
    connection
        .execute("VACUUM INTO ?1", [destination.to_string_lossy().as_ref()])
        .map_err(|error| format!("AppData database snapshot failed: {error}"))?;
    Ok(())
}

fn excluded_root_entry(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "worktree-review"
        || lower == "ebwebview"
        || lower == "native-codex-profile-probes"
        || lower == "agent-session-navigation.json"
        || lower.ends_with(".sqlite-wal")
        || lower.ends_with(".sqlite-shm")
        || lower.ends_with(".sqlite-journal")
        || lower.ends_with(".lock")
        || matches!(lower.as_str(), "cache" | "caches" | "logs" | "temp" | "tmp")
}

fn source_object(selection: &ReviewSourceSelection) -> &str {
    match selection {
        ReviewSourceSelection::PhysicalWorktree {
            captured_object_id, ..
        }
        | ReviewSourceSelection::WorktreeSnapshot {
            captured_object_id, ..
        } => captured_object_id.as_str(),
        ReviewSourceSelection::ExactCommit {
            selected_object, ..
        }
        | ReviewSourceSelection::BranchCommit { selected_object } => selected_object.as_str(),
        ReviewSourceSelection::LiveWorktree {
            trigger_virtual_commit_id,
            trigger_head_object_id,
            ..
        } => trigger_virtual_commit_id
            .as_ref()
            .unwrap_or(trigger_head_object_id)
            .as_str(),
    }
}

fn short_slug(value: &str, maximum: usize) -> String {
    let compact = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    bounded_slug(&compact, value, maximum)
}

fn slug(value: &str, maximum: usize) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !output.is_empty() {
                output.push('-');
            }
            separator = false;
            output.push(character.to_ascii_lowercase());
        } else {
            separator = true;
        }
    }
    bounded_slug(output.trim_matches('-'), value, maximum)
}

fn bounded_slug(normalized: &str, original: &str, maximum: usize) -> String {
    let hash = format!("{:x}", Sha256::digest(original.as_bytes()));
    if normalized.is_empty() {
        return format!("item-{}", &hash[..8]);
    }
    if normalized.len() <= maximum {
        return normalized.to_owned();
    }
    let suffix = &hash[..8];
    let prefix_length = maximum.saturating_sub(suffix.len() + 1);
    let prefix = normalized[..prefix_length].trim_end_matches('-');
    format!("{prefix}-{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::domain::{
        BranchRef, BuildLifecycle, GitObjectId, RepositoryId, RetentionKey, ReviewBuildName,
        SourceBinding, WorkspaceId, WorkspaceLifecycle, WorkspaceOwnership, WorktreeId,
        WorktreeLocation,
    };
    use chrono::Utc;

    #[test]
    fn slug_is_identifier_safe() {
        assert_eq!(slug("refinement/usability", 42), "refinement-usability");
        assert_eq!(slug("  Strange ++ Name  ", 42), "strange-name");
        assert!(slug("+++", 42).starts_with("item-"));
        assert_ne!(
            slug("a-very-long-shared-prefix-with-one-ending", 24),
            slug("a-very-long-shared-prefix-with-two-ending", 24)
        );
    }

    #[test]
    fn excludes_live_and_recursive_appdata() {
        for name in [
            "worktree-review",
            "EBWebView",
            "agent-session-navigation.json",
            "data.sqlite-wal",
            "data.sqlite-shm",
        ] {
            assert!(excluded_root_entry(name), "{name}");
        }
        assert!(!excluded_root_entry("orchestration-materials"));
        assert!(!excluded_root_entry("codex-orchestrator-active-v3.sqlite"));
    }

    #[test]
    fn identity_names_the_source_worktree_and_build() {
        let (build, workspace) = fixture("build-one", "worktree-alpha");
        let value = identity(&build, &workspace);
        assert_eq!(
            value.identifier,
            "dev.codex-orchestrator.review.refinement-usability.wt-worktreealpha.bld-buildone"
        );
        assert_eq!(
            value.label,
            "refinement/usability · worktree-alpha · build-one"
        );

        let (other, _) = fixture("build-two", "worktree-alpha");
        assert_ne!(value.identifier, identity(&other, &workspace).identifier);
    }

    #[test]
    fn first_launch_seeds_durable_data_once_with_consistent_sqlite() {
        let source = tempfile::tempdir().unwrap();
        let review = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        write_target_schema(target.path(), crate::storage::ACTIVE_SCHEMA_VERSION);
        fs::write(source.path().join("settings.json"), "controller").unwrap();
        fs::create_dir(source.path().join("EBWebView")).unwrap();
        fs::write(source.path().join("EBWebView").join("cookies"), "private").unwrap();
        fs::create_dir(source.path().join("nested")).unwrap();
        fs::write(
            source.path().join("nested").join("session.sqlite-wal"),
            "live",
        )
        .unwrap();

        let database_path = source.path().join("state.sqlite");
        let database = Connection::open(&database_path).unwrap();
        database
            .execute_batch(
                "PRAGMA journal_mode=WAL; CREATE TABLE facts(value TEXT); INSERT INTO facts VALUES ('ready');",
            )
            .unwrap();

        let (build, mut workspace) = fixture("build-seed", "worktree-seed");
        workspace.location =
            WorktreeLocation::new(target.path().to_string_lossy().into_owned()).unwrap();
        let prepared = prepare(
            review.path(),
            source.path(),
            &build,
            &workspace,
            Some(crate::storage::ACTIVE_SCHEMA_VERSION),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(prepared.app_data.join("settings.json")).unwrap(),
            "controller"
        );
        assert!(!prepared.app_data.join("EBWebView").exists());
        assert!(!prepared
            .app_data
            .join("nested")
            .join("session.sqlite-wal")
            .exists());
        let snapshot = Connection::open(prepared.app_data.join("state.sqlite")).unwrap();
        assert_eq!(
            snapshot
                .query_row("SELECT value FROM facts", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ready"
        );
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(prepared.app_data.parent().unwrap().join(SEED_MANIFEST)).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["buildId"], "build-seed");
        assert_eq!(manifest["databaseSeeded"], true);
        assert_eq!(
            manifest["targetSchemaVersion"],
            crate::storage::ACTIVE_SCHEMA_VERSION
        );

        fs::write(prepared.app_data.join("settings.json"), "reviewed").unwrap();
        fs::write(source.path().join("settings.json"), "changed-controller").unwrap();
        let relaunched = prepare(
            review.path(),
            source.path(),
            &build,
            &workspace,
            Some(crate::storage::ACTIVE_SCHEMA_VERSION),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(relaunched.app_data.join("settings.json")).unwrap(),
            "reviewed"
        );
    }

    #[test]
    fn incompatible_target_schema_keeps_safe_files_but_not_controller_databases() {
        let source = tempfile::tempdir().unwrap();
        let review = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        // The checkout may have changed after compilation; the build receipt remains authoritative.
        write_target_schema(target.path(), crate::storage::ACTIVE_SCHEMA_VERSION);
        fs::write(source.path().join("settings.json"), "controller").unwrap();
        let database = Connection::open(source.path().join("state.sqlite")).unwrap();
        database
            .execute_batch("CREATE TABLE facts(value TEXT); INSERT INTO facts VALUES ('ready');")
            .unwrap();

        let (build, mut workspace) = fixture("build-older", "worktree-older");
        workspace.location =
            WorktreeLocation::new(target.path().to_string_lossy().into_owned()).unwrap();
        let prepared = prepare(
            review.path(),
            source.path(),
            &build,
            &workspace,
            Some(crate::storage::ACTIVE_SCHEMA_VERSION - 1),
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(prepared.app_data.join("settings.json")).unwrap(),
            "controller"
        );
        assert!(!prepared.app_data.join("state.sqlite").exists());
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(prepared.app_data.parent().unwrap().join(SEED_MANIFEST)).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["databaseSeeded"], false);
        assert_eq!(
            manifest["targetSchemaVersion"],
            crate::storage::ACTIVE_SCHEMA_VERSION - 1
        );
    }

    fn write_target_schema(root: &Path, version: i64) {
        let source = root.join("src-tauri").join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("storage.rs"),
            format!("pub(crate) const ACTIVE_SCHEMA_VERSION: i64 = {version};\n"),
        )
        .unwrap();
    }

    fn fixture(build_id: &str, worktree_id: &str) -> (ReviewBuild, ReviewWorkspace) {
        let repository_id = RepositoryId::new("repository-one").unwrap();
        let workspace_id = WorkspaceId::new("workspace-one").unwrap();
        let build_id = ReviewBuildId::new(build_id).unwrap();
        let now = Utc::now();
        let build = ReviewBuild {
            profile: None,
            id: build_id.clone(),
            name: ReviewBuildName::new("Review build").unwrap(),
            source: SourceBinding {
                repository_id: repository_id.clone(),
                branch_ref: Some(BranchRef::new("refs/heads/refinement/usability").unwrap()),
                selection: ReviewSourceSelection::BranchCommit {
                    selected_object: GitObjectId::new("a".repeat(40)).unwrap(),
                },
                workspace_id: workspace_id.clone(),
            },
            workspace_id: workspace_id.clone(),
            retention_key: RetentionKey::new("source-one").unwrap(),
            current_output_id: None,
            lifecycle: BuildLifecycle::Active,
            created_at: now,
            updated_at: now,
        };
        let workspace = ReviewWorkspace {
            id: workspace_id,
            repository_id,
            worktree_id: WorktreeId::new(worktree_id).unwrap(),
            location: WorktreeLocation::new("C:/review/worktree").unwrap(),
            ownership: WorkspaceOwnership::BorrowedPhysicalWorktree,
            lifecycle: WorkspaceLifecycle::Ready,
            created_at: now,
            updated_at: now,
        };
        (build, workspace)
    }
}
