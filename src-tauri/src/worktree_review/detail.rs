use super::{
    progress::{sanitize_output, ReviewOperationHistoryView},
    worktree_build::{WorktreeBuildContextView, WorktreeScope},
};
use serde::Serialize;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_DETAIL_OUTPUT_LINES: usize = 10_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewInstanceDetailView {
    pub(crate) instance_ref: String,
    pub(crate) name: String,
    pub(crate) source_label: String,
    pub(crate) purpose: String,
    pub(crate) phase: String,
    pub(crate) health: String,
    pub(crate) stale: bool,
    pub(crate) build: String,
    pub(crate) compatibility: String,
    pub(crate) compatibility_message: String,
    pub(crate) orientation: String,
    pub(crate) prepare_produced: String,
    pub(crate) build_produced: String,
    pub(crate) open_produced: String,
    pub(crate) current_condition: String,
    pub(crate) action_required: bool,
    pub(crate) action_summary: String,
    pub(crate) reusable_summary: String,
    pub(crate) retention: ReviewRetentionView,
    pub(crate) artifacts: Vec<ReviewArtifactView>,
    pub(crate) lifecycle_history: Vec<ReviewLifecycleEventView>,
    pub(crate) operations: Vec<ReviewOperationHistoryView>,
    pub(crate) context: WorktreeBuildContextView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewRetentionView {
    pub(crate) policy: String,
    pub(crate) cleanup: String,
    pub(crate) automatic: bool,
    pub(crate) action_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewArtifactView {
    pub(crate) label: String,
    pub(crate) state: String,
    pub(crate) summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewLifecycleEventView {
    pub(crate) occurred_at_ms: u64,
    pub(crate) kind: String,
    pub(crate) summary: String,
}

pub(super) struct DetailInput {
    pub(super) instance_ref: String,
    pub(super) name: String,
    pub(super) source_label: String,
    pub(super) phase: String,
    pub(super) health: String,
    pub(super) stale: bool,
    pub(super) build: String,
    pub(super) compatibility: String,
    pub(super) compatibility_message: String,
    pub(super) context: WorktreeBuildContextView,
    pub(super) instance_root: PathBuf,
    pub(super) lifecycle_history: Vec<ReviewLifecycleEventView>,
    pub(super) operations: Vec<ReviewOperationHistoryView>,
}

pub(super) fn assemble(mut input: DetailInput) -> ReviewInstanceDetailView {
    append_retained_logs(&input.instance_root, &mut input.operations);
    let running_ready = input.phase == "running" && input.health == "healthy" && !input.stale;
    let action_required = input.compatibility == "incompatible"
        || input.stale
        || input.health == "unhealthy"
        || matches!(
            input.build.as_str(),
            "failed" | "superseded" | "rebuild-required"
        );
    let current_condition = if input.compatibility == "incompatible" {
        "This retained instance cannot be opened because its selected source lacks the required child readiness and provenance contract."
    } else if input.build == "superseded" {
        "The selected worktree changed after this instance was prepared. Its history remains inspectable, but it is not a build of the current source."
    } else if input.build == "rebuild-required" {
        "The source identity is current, but the private build receipt or artifact hashes no longer establish an exact reusable build."
    } else if running_ready {
        "The exact owned worktree-build window is ready for human review."
    } else if input.stale {
        "Recorded lifecycle ownership and observed processes no longer agree; recovery is required before reuse."
    } else if input.phase == "stopped" || input.phase == "recovered" {
        "No owned child is running. Retained build material and isolated data remain available."
    } else if input.build == "passed" {
        "A verified private build is retained and can be opened without recompiling when its identity remains exact."
    } else {
        "The instance is prepared, but no verified private application build is available yet."
    };
    let action_summary = if input.compatibility == "incompatible" {
        "Choose or update to a compatible worktree lineage; retrying Open would only wait for evidence this source cannot produce."
    } else if input.build == "superseded" {
        "Prepare a fresh named instance for the current worktree state. Do not Open or rebuild this historical identity."
    } else if input.build == "rebuild-required" {
        "Run Build again for this instance. Open remains unavailable until its private outputs are verified."
    } else if input.stale || input.health == "unhealthy" {
        "Use Recover to reconcile only this instance's owned process tree, then Build or Open again as indicated."
    } else if running_ready {
        "No action is required. Review the separate window, Focus it when desired, or Stop it independently."
    } else if input.build == "passed" {
        "Open the retained verified build, or Build again to verify exact reuse before opening."
    } else {
        "Run Build. Open remains unavailable until the private executable and frontend output are verified."
    };
    let reusable_summary = if input.build == "passed" {
        "The private executable and frontend output are reusable only while source, toolchain, launch identity, and artifact hashes remain exact."
    } else if input.build == "superseded" {
        "Only the history and isolated retained data remain inspectable; the prior executable is not reusable for current source."
    } else if input.build == "rebuild-required" {
        "The prepared isolation and history remain reusable, but no executable is currently verified."
    } else {
        "The isolated reservation, data root, logs, and ownership record remain reusable; no executable is claimed."
    };
    ReviewInstanceDetailView {
        instance_ref: input.instance_ref,
        name: input.name,
        source_label: input.source_label,
        purpose: "A human-requested, isolated build for comparing and interacting with one selected Git worktree while the main application stays open.".into(),
        phase: input.phase,
        health: input.health,
        stale: input.stale,
        build: input.build.clone(),
        compatibility: input.compatibility,
        compatibility_message: input.compatibility_message,
        orientation: "This page describes one retained source identity, its private build outputs, its owned review-window lifecycle, and what the human can safely do next.".into(),
        prepare_produced: "Prepare reserved an opaque instance, isolated mutable roots, ports, application data, logs, and exact process ownership.".into(),
        build_produced: if input.build == "passed" {
            "Build produced and verified a private frontend bundle and Tauri executable for this exact source and toolchain identity.".into()
        } else if input.build == "superseded" {
            "A historical private build is retained, but the selected worktree now has a different source identity.".into()
        } else if input.build == "rebuild-required" {
            "Private output may remain, but its receipt or artifact hashes require Build to verify it again.".into()
        } else {
            "Build has not produced a currently verified private executable.".into()
        },
        open_produced: if running_ready {
            "Open started only required supporting services plus the verified executable and established an exact usable review window.".into()
        } else {
            "Open is not currently backed by a ready, usable owned window.".into()
        },
        current_condition: current_condition.into(),
        action_required,
        action_summary: action_summary.into(),
        reusable_summary: reusable_summary.into(),
        retention: ReviewRetentionView {
            policy: "Retained until deliberate developer cleanup".into(),
            cleanup: "Stop removes only the owned process tree. Generated outputs and isolated data remain for review or exact reuse; automatic pruning is not implemented.".into(),
            automatic: false,
            action_required: input.stale,
        },
        artifacts: artifacts(&input.instance_root, &input.build),
        lifecycle_history: input.lifecycle_history,
        operations: input.operations,
        context: input.context,
    }
}

#[tauri::command]
pub(crate) fn worktree_build_detail() -> Result<ReviewInstanceDetailView, String> {
    let context = WorktreeScope::from_environment()?.context()?;
    let instance_ref = std::env::var("RUNTIME_INSTANCE_ID")
        .map_err(|_| "Worktree-build instance identity is unavailable.".to_string())?;
    let instance_root = std::env::var_os("CODEX_ORCHESTRATOR_REVIEW_INSTANCE_ROOT")
        .map(PathBuf::from)
        .ok_or_else(|| "Worktree-build detail storage is unavailable.".to_string())?;
    let name = context.name.clone();
    Ok(assemble(DetailInput {
        instance_ref,
        name,
        source_label: context
            .branch
            .clone()
            .unwrap_or_else(|| format!("Detached {}", context.head.abbreviated_id)),
        phase: "running".into(),
        health: if rendered_marker(&instance_root) {
            "healthy".into()
        } else {
            "unhealthy".into()
        },
        stale: false,
        build: "passed".into(),
        compatibility: "compatible".into(),
        compatibility_message:
            "This child is running the Worktree Review readiness and provenance contract.".into(),
        context,
        instance_root: instance_root.clone(),
        lifecycle_history: child_history(&instance_root),
        operations: Vec::new(),
    }))
}

fn rendered_marker(root: &Path) -> bool {
    root.join("app-data/review-window-ready").is_file()
}

fn artifacts(root: &Path, build: &str) -> Vec<ReviewArtifactView> {
    let artifact = |label: &str, exists: bool, available: &str, missing: &str| ReviewArtifactView {
        label: label.into(),
        state: if exists {
            if build == "passed" {
                "available"
            } else {
                "retained"
            }
        } else {
            "not-produced"
        }
        .into(),
        summary: if exists { available } else { missing }.into(),
    };
    vec![
        artifact(
            "Private application executable",
            root.join("cargo-target/debug/codex-orchestrator.exe").is_file(),
            "Verified native output exists inside this instance's isolated Cargo target.",
            "No private native executable is present.",
        ),
        artifact(
            "Private frontend bundle",
            root.join("dist/index.html").is_file(),
            "Frontend output exists inside this instance's isolated distribution root.",
            "No private frontend bundle is present.",
        ),
        artifact(
            "Isolated application state",
            root.join("app-data").is_dir(),
            "Application data and database state remain private to this instance.",
            "No isolated application state has been observed.",
        ),
        artifact(
            "Sanitized operation history",
            root.join("logs").is_dir(),
            "Build and launch output is retained below the private instance and exposed only through the sanitized read model.",
            "No retained operation output is available.",
        ),
    ]
}

fn child_history(root: &Path) -> Vec<ReviewLifecycleEventView> {
    [
        (
            "Prepared",
            root.join("app-data"),
            "Isolated application state was prepared.",
        ),
        (
            "Built",
            root.join("cargo-target/debug/codex-orchestrator.exe"),
            "The private executable was produced.",
        ),
        (
            "Opened",
            root.join("app-data/review-window-ready"),
            "The rendered worktree-build window reported ready.",
        ),
    ]
    .into_iter()
    .filter_map(|(kind, path, summary)| {
        modified_ms(&path).map(|occurred_at_ms| ReviewLifecycleEventView {
            occurred_at_ms,
            kind: kind.into(),
            summary: summary.into(),
        })
    })
    .collect()
}

fn append_retained_logs(root: &Path, operations: &mut Vec<ReviewOperationHistoryView>) {
    for (file, operation, label) in [
        ("build.log", "build", "Retained build output"),
        ("vite.log", "start", "Retained frontend-service output"),
        ("status.log", "start", "Retained status-service output"),
        ("tauri.log", "start", "Retained native-application output"),
    ] {
        let path = root.join("logs").join(file);
        if !path.is_file() {
            continue;
        }
        let (output, output_complete) = read_safe_lines(&path);
        if output.is_empty() {
            continue;
        }
        let timestamp = modified_ms(&path).unwrap_or(0);
        operations.push(ReviewOperationHistoryView {
            operation_ref: format!("retained-{}", file.trim_end_matches(".log")),
            operation: operation.into(),
            state: "succeeded".into(),
            stage_label: label.into(),
            started_at_ms: timestamp,
            updated_at_ms: timestamp,
            stage_history: Vec::new(),
            output,
            output_complete,
        });
    }
}

fn read_safe_lines(path: &Path) -> (Vec<String>, bool) {
    let Ok(file) = fs::File::open(path) else {
        return (Vec::new(), false);
    };
    let mut output = Vec::new();
    let mut complete = true;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            complete = false;
            break;
        };
        if let Some(line) = sanitize_output(&line) {
            output.push(line);
            if output.len() > MAX_DETAIL_OUTPUT_LINES {
                output.remove(0);
                complete = false;
            }
        }
    }
    (output, complete)
}

fn modified_ms(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .ok()
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn retained_output_is_complete_beyond_summary_and_safely_sanitized() {
        let root = tempdir().expect("temporary root");
        let logs = root.path().join("logs");
        fs::create_dir_all(&logs).expect("create logs");
        let mut lines = (0..30)
            .map(|index| format!("safe build line {index}"))
            .collect::<Vec<_>>();
        lines.push("TOKEN=top-secret C:\\private\\target http://127.0.0.1:34123".into());
        fs::write(logs.join("build.log"), lines.join("\n")).expect("write build log");

        let mut operations = Vec::new();
        append_retained_logs(root.path(), &mut operations);

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].output.len(), 31);
        assert!(operations[0].output_complete);
        let output = operations[0].output.join("\n");
        assert!(output.contains("safe build line 29"));
        assert!(!output.contains("top-secret"));
        assert!(!output.contains("C:\\private"));
        assert!(!output.contains("34123"));
        assert!(output.contains("[redacted credential]"));
        assert!(output.contains("[private path]"));
        assert!(output.contains("[private local endpoint]"));
    }
}
