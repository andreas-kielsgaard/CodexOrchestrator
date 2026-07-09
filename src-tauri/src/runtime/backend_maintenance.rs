use super::*;

pub(crate) fn check_and_reopen_backend(app: AppHandle) -> Result<BackendMaintenanceResult, String> {
    let checked_at = now_iso();
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let newest_source = newest_backend_source_modified_at(&source_root)?;
    let executable_path = current_executable_path()?;
    let executable_modified_at = executable_modified_at(&executable_path);
    let stale = match (newest_source.as_ref(), executable_modified_at.as_ref()) {
        (Some((_, source_modified_at)), Some(exe_modified_at)) => {
            source_modified_at > exe_modified_at
        }
        (Some(_), None) => true,
        _ => false,
    };

    if !stale {
        return Ok(BackendMaintenanceResult {
            status: "current".to_string(),
            stale: false,
            checked_at,
            newest_source_path: newest_source
                .as_ref()
                .map(|(path, _)| path.to_string_lossy().to_string()),
            newest_source_modified_at: newest_source
                .as_ref()
                .map(|(_, modified_at)| system_time_to_rfc3339(*modified_at)),
            executable_modified_at: executable_modified_at.map(system_time_to_rfc3339),
            message: "Rust backend is current.".to_string(),
        });
    }

    spawn_backend_rebuild_and_reopen(&source_root, &manifest_path, &executable_path)?;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        app.exit(0);
    });

    Ok(BackendMaintenanceResult {
        status: "restarting".to_string(),
        stale: true,
        checked_at,
        newest_source_path: newest_source
            .as_ref()
            .map(|(path, _)| path.to_string_lossy().to_string()),
        newest_source_modified_at: newest_source
            .as_ref()
            .map(|(_, modified_at)| system_time_to_rfc3339(*modified_at)),
        executable_modified_at: executable_modified_at.map(system_time_to_rfc3339),
        message: "Rust backend is stale. Closing, rebuilding, and reopening...".to_string(),
    })
}

fn newest_backend_source_modified_at(
    root: &Path,
) -> Result<Option<(PathBuf, std::time::SystemTime)>, String> {
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    collect_newest_backend_source_modified_at(root, &mut newest)?;
    Ok(newest)
}

fn collect_newest_backend_source_modified_at(
    directory: &Path,
    newest: &mut Option<(PathBuf, std::time::SystemTime)>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Unable to scan backend source directory: {error}"))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if should_scan_backend_source_dir(&path) {
                collect_newest_backend_source_modified_at(&path, newest)?;
            }
            continue;
        }

        if !is_backend_source_file(&path) {
            continue;
        }

        let modified_at = match entry.metadata().and_then(|metadata| metadata.modified()) {
            Ok(modified_at) => modified_at,
            Err(_) => continue,
        };

        if newest
            .as_ref()
            .is_none_or(|(_, current_modified_at)| modified_at > *current_modified_at)
        {
            *newest = Some((path, modified_at));
        }
    }

    Ok(())
}

fn current_executable_path() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| format!("Unable to resolve current executable path: {error}"))
}

fn executable_modified_at(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn spawn_backend_rebuild_and_reopen(
    source_root: &Path,
    manifest_path: &Path,
    executable_path: &Path,
) -> Result<(), String> {
    let script_path = std::env::temp_dir().join("codex-orchestrator-backend-reopen.ps1");
    let log_path = std::env::temp_dir().join("codex-orchestrator-backend-reopen.log");
    let script = r#"
param(
    [int]$ParentProcessId,
    [string]$SourceRoot,
    [string]$ManifestPath,
    [string]$ExecutablePath,
    [string]$LogPath
)

$ErrorActionPreference = 'Continue'
try {
    Wait-Process -Id $ParentProcessId -ErrorAction SilentlyContinue
} catch {}

Set-Location -LiteralPath $SourceRoot
$cargoOutput = & cargo build --manifest-path $ManifestPath 2>&1
$exitCode = $LASTEXITCODE
$timestamp = Get-Date -Format o
"[$timestamp] cargo build exit $exitCode" | Out-File -FilePath $LogPath -Encoding utf8
$cargoOutput | Out-File -FilePath $LogPath -Encoding utf8 -Append
Start-Process -FilePath $ExecutablePath
"#;

    fs::write(&script_path, script)
        .map_err(|error| format!("Unable to write backend rebuild helper: {error}"))?;

    let mut command = Command::new("powershell");
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .arg("-ParentProcessId")
        .arg(std::process::id().to_string())
        .arg("-SourceRoot")
        .arg(source_root)
        .arg("-ManifestPath")
        .arg(manifest_path)
        .arg("-ExecutablePath")
        .arg(executable_path)
        .arg("-LogPath")
        .arg(log_path);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0208);
    }

    command
        .spawn()
        .map_err(|error| format!("Unable to launch backend rebuild helper: {error}"))?;

    Ok(())
}

fn should_scan_backend_source_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    !matches!(name, "target" | ".git")
}

fn is_backend_source_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|value| value.to_str()),
        Some("Cargo.toml" | "Cargo.lock" | "build.rs" | "tauri.conf.json")
    ) || path.extension().and_then(|value| value.to_str()) == Some("rs")
}

fn system_time_to_rfc3339(value: std::time::SystemTime) -> String {
    chrono::DateTime::<Utc>::from(value).to_rfc3339()
}
