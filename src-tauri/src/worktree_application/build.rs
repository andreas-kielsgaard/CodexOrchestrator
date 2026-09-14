use super::{
    build_tool,
    domain::{
        PhysicalWorktreeBuildRequest, PhysicalWorktreeBuildResult,
        PhysicalWorktreeDependencyPolicy, WorktreeApplicationError, WorktreeApplicationErrorKind,
    },
};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Deserialize)]
struct ToolReply {
    ok: bool,
    result: Option<ToolOutput>,
    error: Option<ToolError>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolOutput {
    output_root: PathBuf,
    executable: PathBuf,
}
#[derive(Deserialize)]
struct ToolError {
    kind: String,
    message: String,
}

pub(super) fn build(
    request: &PhysicalWorktreeBuildRequest,
) -> Result<PhysicalWorktreeBuildResult, WorktreeApplicationError> {
    let worktree = request
        .worktree_root
        .canonicalize()
        .map_err(|_| error("worktree_unavailable", "The worktree is unavailable."))?;
    if request.attempt_root.starts_with(&worktree) {
        return Err(error(
            "invalid_request",
            "Application storage must be outside the worktree.",
        ));
    }
    fs::create_dir_all(&request.attempt_root).map_err(|_| unavailable())?;
    let attempt = request
        .attempt_root
        .canonicalize()
        .map_err(|_| unavailable())?;
    if attempt.starts_with(&worktree) {
        return Err(error(
            "invalid_request",
            "Application storage resolves inside the worktree.",
        ));
    }
    let tool = build_tool::materialize()?;
    let input = attempt.join("request.json");
    let reply = attempt.join("result.json");
    let log_path = attempt.join("build.log");
    let (policy, npm_cache) = match &request.dependency_policy {
        PhysicalWorktreeDependencyPolicy::UseExisting => ("use-existing", None),
        PhysicalWorktreeDependencyPolicy::Install { cache_root } => ("install", Some(cache_root)),
    };
    fs::write(&input, serde_json::to_vec(&json!({
        "action": "app", "worktreeRoot": external(&worktree), "attemptRoot": external(&attempt),
        "profile": request.profile, "cache": "auto", "dependencyPolicy": policy,
        "npmCache": npm_cache.map(|path| external(path)), "cargoBinaryName": request.cargo_binary_name,
    })).map_err(|_| unavailable())?).map_err(|_| unavailable())?;
    let log = fs::File::create(&log_path).map_err(|_| unavailable())?;
    let stderr = log.try_clone().map_err(|_| unavailable())?;
    let mut command = Command::new("node");
    command
        .arg(tool)
        .args(["app", "--request"])
        .arg(external(&input))
        .arg("--result")
        .arg(external(&reply))
        .current_dir(&worktree)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let status = command.status().map_err(|_| {
        error(
            "toolchain_unavailable",
            "Node.js is required to build applications.",
        )
    })?;
    let reply: ToolReply = fs::read(&reply)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .ok_or_else(|| {
            error(
                "build_failed",
                "The build tool did not return a result. See build.log.",
            )
        })?;
    if !status.success() || !reply.ok {
        return Err(reply
            .error
            .map(|failure| error(&failure.kind, &failure.message))
            .unwrap_or_else(|| error("build_failed", "Application compilation failed.")));
    }
    let result = reply.result.ok_or_else(unavailable)?;
    let output_root = result
        .output_root
        .canonicalize()
        .map_err(|_| unavailable())?;
    let executable = result
        .executable
        .canonicalize()
        .map_err(|_| unavailable())?;
    if output_root != attempt.join("output")
        || !executable.starts_with(&output_root)
        || !executable.is_file()
    {
        return Err(error(
            "output_unavailable",
            "The build tool returned an invalid application output.",
        ));
    }
    Ok(PhysicalWorktreeBuildResult {
        worktree_root: worktree,
        attempt_root: attempt,
        output_root,
        log_path,
        executable,
    })
}

fn external(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", unc));
    }
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
}
fn unavailable() -> WorktreeApplicationError {
    error(
        "output_unavailable",
        "Application build storage is unavailable.",
    )
}
fn error(kind: &str, message: &str) -> WorktreeApplicationError {
    let kind = match kind {
        "invalid_request" => WorktreeApplicationErrorKind::InvalidRequest,
        "worktree_unavailable" => WorktreeApplicationErrorKind::WorktreeUnavailable,
        "toolchain_unavailable" => WorktreeApplicationErrorKind::ToolchainUnavailable,
        "output_unavailable" => WorktreeApplicationErrorKind::OutputUnavailable,
        _ => WorktreeApplicationErrorKind::BuildFailed,
    };
    WorktreeApplicationError::new(kind, message)
}
