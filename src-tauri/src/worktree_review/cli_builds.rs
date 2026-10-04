//! Lightweight discovery of shared CLI output; no source freshness contract.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliBuild {
    worktree_root: PathBuf,
    attempt_root: PathBuf,
    output_root: PathBuf,
    executable: PathBuf,
    profile: String,
    application_schema_version: Option<i64>,
}

fn cache_root() -> Option<PathBuf> {
    std::env::var_os("ORCHESTRATOR_BUILD_CACHE_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(|root| PathBuf::from(root).join("CodexOrchestrator/build-cache"))
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|root| PathBuf::from(root).join(".cache/CodexOrchestrator/build-cache"))
        })
}

fn children(root: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

pub(crate) fn latest(worktree: &Path) -> Option<CliBuild> {
    latest_in(worktree, &cache_root()?)
}

fn latest_in(worktree: &Path, cache: &Path) -> Option<CliBuild> {
    let worktree = worktree.canonicalize().ok()?;
    let mut receipts: Vec<PathBuf> = children(cache)
        .into_iter()
        .map(|owner| owner.join("latest-build.json"))
        .collect();
    // Existing builds predate automatic registration; their publication receipts suffice.
    for owner in children(&cache.parent()?.join("builds")) {
        for attempt in children(&owner) {
            receipts.push(attempt.join("output/build.json"));
        }
    }
    receipts
        .into_iter()
        .filter_map(|receipt| {
            let build: CliBuild = serde_json::from_slice(&std::fs::read(&receipt).ok()?).ok()?;
            if build.worktree_root.canonicalize().ok()? != worktree {
                return None;
            }
            let output = build.output_root.canonicalize().ok()?;
            let executable = build.executable.canonicalize().ok()?;
            if !executable.is_file() || !executable.starts_with(&output) {
                return None;
            }
            let date = std::fs::metadata(output.join("build.json"))
                .ok()?
                .modified()
                .ok()?;
            Some((date, build))
        })
        .max_by_key(|(date, _)| *date)
        .map(|(_, build)| build)
}

pub(crate) fn open(worktree: &Path) -> Result<(), String> {
    use crate::worktree_application::{
        PhysicalWorktreeApplication, PhysicalWorktreeBuildResult, WorktreeApplicationLaunchContext,
    };
    let build = latest(worktree).ok_or("No successful CLI build is available for this worktree")?;
    let data = build.attempt_root.join("review-data");
    let webview = build.attempt_root.join("review-webview");
    std::fs::create_dir_all(&data).map_err(|error| error.to_string())?;
    let context = WorktreeApplicationLaunchContext::new([
        (
            "CODEX_ORCHESTRATOR_APP_DATA_DIR",
            data.to_string_lossy().into_owned(),
        ),
        (
            "WEBVIEW2_USER_DATA_FOLDER",
            webview.to_string_lossy().into_owned(),
        ),
        (
            "CODEX_ORCHESTRATOR_REVIEW_INSTANCE_LABEL",
            "Worktree CLI build".into(),
        ),
    ])
    .map_err(|error| error.message)?;
    let physical = PhysicalWorktreeBuildResult {
        worktree_root: build.worktree_root,
        log_path: build.attempt_root.join("build.log"),
        attempt_root: build.attempt_root,
        output_root: build.output_root,
        executable: build.executable,
        application_schema_version: build.application_schema_version,
    };
    PhysicalWorktreeApplication
        .open(&physical, &context)
        .map_err(|error| error.message)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_existing_and_registered_builds_but_not_missing_or_other_worktree_outputs() {
        let root = tempfile::tempdir().unwrap();
        let worktree = root.path().join("worktree");
        let other = root.path().join("other");
        let cache = root.path().join("build-cache");
        let output = root.path().join("builds/owner/attempt/output");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::create_dir_all(cache.join("owner")).unwrap();
        let build = CliBuild {
            worktree_root: worktree.clone(),
            attempt_root: output.parent().unwrap().to_path_buf(),
            executable: output.join("app.exe"),
            output_root: output.clone(),
            profile: "debug".into(),
            application_schema_version: None,
        };
        std::fs::write(&build.executable, b"app").unwrap();
        let receipt = serde_json::to_vec(&build).unwrap();
        std::fs::write(output.join("build.json"), &receipt).unwrap();
        assert_eq!(
            latest_in(&worktree, &cache).unwrap().executable,
            build.executable
        );
        assert!(latest_in(&other, &cache).is_none());
        std::fs::write(cache.join("owner/latest-build.json"), &receipt).unwrap();
        assert!(latest_in(&worktree, &cache).is_some());
        let newer_output = root.path().join("builds/owner/newer/output");
        std::fs::create_dir_all(&newer_output).unwrap();
        let mut newer = build.clone();
        newer.output_root = newer_output.clone();
        newer.attempt_root = newer_output.parent().unwrap().to_path_buf();
        newer.executable = newer_output.join("app.exe");
        std::fs::write(&newer.executable, b"newer").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(25));
        std::fs::write(
            newer_output.join("build.json"),
            serde_json::to_vec(&newer).unwrap(),
        )
        .unwrap();
        assert_eq!(
            latest_in(&worktree, &cache).unwrap().executable,
            newer.executable
        );
        std::fs::remove_file(&newer.executable).unwrap();
        assert_eq!(
            latest_in(&worktree, &cache).unwrap().executable,
            build.executable
        );
        std::fs::remove_file(&build.executable).unwrap();
        assert!(latest_in(&worktree, &cache).is_none());
    }
}
