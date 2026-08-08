use crate::agent_sessions::ports::{
    AgentAccessCapabilities, AgentAccessCapabilityDiscovery, AgentAccessCapabilitySnapshot,
    CapabilityDiscoveryState, CapabilityProvenance, CapabilitySupport, InvocationCapabilities,
};
use chrono::{DateTime, Duration, Utc};
use std::process::Command;

const OBSERVED_FRESHNESS: Duration = Duration::minutes(30);
const UNAVAILABLE_FRESHNESS: Duration = Duration::minutes(1);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct CodexCliCapabilities {
    pub(crate) version: Option<String>,
    pub(crate) exec_json: Option<bool>,
    pub(crate) resume_json: Option<bool>,
    pub(crate) exec_model: Option<bool>,
    pub(crate) resume_model: Option<bool>,
    pub(crate) exec_sandbox: Option<bool>,
    pub(crate) resume_sandbox: Option<bool>,
}

/// Owns Codex executable/version/help discovery and translates it into product semantics.
///
/// Successful observations stay fresh for 30 minutes in the runtime's in-memory cache. A fully
/// unavailable probe is retried after one minute. The shorter failure lifetime supports recovery
/// after PATH or installation changes without probing on every invocation.
pub(crate) struct CodexCliCapabilityProbe {
    program: Result<String, String>,
}

impl CodexCliCapabilityProbe {
    pub(crate) fn new(program: impl Into<String>) -> Self {
        Self {
            program: resolve_program(program.into()),
        }
    }

    pub(super) fn from_resolved(program: Result<String, String>) -> Self {
        Self { program }
    }

    pub(crate) fn discover(&self) -> CodexCliCapabilities {
        self.discover_raw().0
    }

    fn discover_raw(&self) -> (CodexCliCapabilities, Vec<String>) {
        let program = match &self.program {
            Ok(program) => program,
            Err(error) => return (CodexCliCapabilities::default(), vec![error.clone()]),
        };
        let version = command_stdout(program, &["--version"]);
        let exec_help = command_stdout(program, &["exec", "--help"]);
        let resume_help = command_stdout(program, &["exec", "resume", "--help"]);
        let errors = [&version, &exec_help, &resume_help]
            .into_iter()
            .filter_map(|result| result.as_ref().err().cloned())
            .collect();
        let version = version
            .ok()
            .map(|output| output.trim().to_string())
            .filter(|output| !output.is_empty());
        let exec_help = exec_help.ok();
        let resume_help = resume_help.ok();

        (
            CodexCliCapabilities {
                version,
                exec_json: flag_support(exec_help.as_deref(), "--json"),
                resume_json: flag_support(resume_help.as_deref(), "--json"),
                exec_model: flag_support(exec_help.as_deref(), "--model"),
                resume_model: flag_support(resume_help.as_deref(), "--model"),
                exec_sandbox: flag_support(exec_help.as_deref(), "--sandbox"),
                // `exec resume` accepts the same `sandbox_mode` through its supported strict
                // `--config` surface even though it does not publish a dedicated `--sandbox` flag.
                resume_sandbox: any_flag_support(
                    resume_help.as_deref(),
                    &["--sandbox", "--config"],
                ),
            },
            errors,
        )
    }
}

impl AgentAccessCapabilityDiscovery for CodexCliCapabilityProbe {
    fn discover_capabilities(&self, observed_at: DateTime<Utc>) -> AgentAccessCapabilitySnapshot {
        let (raw, errors) = self.discover_raw();
        snapshot_from_raw(raw, errors, observed_at)
    }
}

pub(super) struct FixedCodexCapabilityDiscovery {
    capabilities: CodexCliCapabilities,
}

impl FixedCodexCapabilityDiscovery {
    pub(super) fn new(capabilities: CodexCliCapabilities) -> Self {
        Self { capabilities }
    }
}

impl AgentAccessCapabilityDiscovery for FixedCodexCapabilityDiscovery {
    fn discover_capabilities(&self, observed_at: DateTime<Utc>) -> AgentAccessCapabilitySnapshot {
        snapshot_from_raw(self.capabilities.clone(), Vec::new(), observed_at)
    }
}

fn snapshot_from_raw(
    raw: CodexCliCapabilities,
    errors: Vec<String>,
    observed_at: DateTime<Utc>,
) -> AgentAccessCapabilitySnapshot {
    let observed = raw.version.is_some()
        || [
            raw.exec_json,
            raw.resume_json,
            raw.exec_model,
            raw.resume_model,
            raw.exec_sandbox,
            raw.resume_sandbox,
        ]
        .into_iter()
        .any(|support| support.is_some());
    let discovery_state = if observed {
        CapabilityDiscoveryState::Observed
    } else {
        CapabilityDiscoveryState::Unavailable
    };
    let freshness = if observed {
        OBSERVED_FRESHNESS
    } else {
        UNAVAILABLE_FRESHNESS
    };
    let unavailable_reason = (!observed).then(|| {
        if errors.is_empty() {
            "Codex CLI capability probes returned no usable evidence".to_string()
        } else {
            errors.join("; ")
        }
    });

    AgentAccessCapabilitySnapshot {
        capabilities: AgentAccessCapabilities {
            start: InvocationCapabilities {
                structured_events: support(raw.exec_json),
                model_selection: support(raw.exec_model),
                sandbox_selection: support(raw.exec_sandbox),
            },
            resume: InvocationCapabilities {
                structured_events: support(raw.resume_json),
                model_selection: support(raw.resume_model),
                sandbox_selection: support(raw.resume_sandbox),
            },
        },
        discovery_state,
        provenance: CapabilityProvenance {
            source: "codex_cli_version_and_help".to_string(),
            runtime_version: raw.version,
        },
        observed_at,
        valid_until: observed_at + freshness,
        unavailable_reason,
    }
}

fn support(value: Option<bool>) -> CapabilitySupport {
    match value {
        Some(true) => CapabilitySupport::Supported,
        Some(false) => CapabilitySupport::Unsupported,
        None => CapabilitySupport::Unknown,
    }
}

pub(crate) fn resolve_program(program: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::path::Path;

        let explicit = Path::new(&program);
        if explicit.is_file() {
            let is_cmd = explicit
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("cmd"));
            if is_cmd {
                if let Some(native) = explicit.parent().and_then(npm_codex_native_binary) {
                    return Ok(native.to_string_lossy().into_owned());
                }
                return Err(format!(
                    "Codex CLI batch shim {} has no discoverable native npm binary",
                    explicit.display()
                ));
            }
            return Ok(program);
        }
        if !program.contains('/') && !program.contains('\\') {
            if let Some(path) = std::env::var_os("PATH") {
                let directories = std::env::split_paths(&path).collect::<Vec<_>>();
                if program == "codex" {
                    for directory in &directories {
                        if directory.join("codex.cmd").is_file() {
                            if let Some(native) = npm_codex_native_binary(directory) {
                                return Ok(native.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
                if let Some(path) = directories
                    .into_iter()
                    .map(|directory| directory.join(format!("{program}.exe")))
                    .find(|candidate| candidate.is_file())
                {
                    return Ok(path.to_string_lossy().into_owned());
                }
            }
        }
    }
    Ok(program)
}

#[cfg(windows)]
fn npm_codex_native_binary(npm_bin: &std::path::Path) -> Option<std::path::PathBuf> {
    #[cfg(target_arch = "x86_64")]
    let (package, target) = ("codex-win32-x64", "x86_64-pc-windows-msvc");
    #[cfg(target_arch = "aarch64")]
    let (package, target) = ("codex-win32-arm64", "aarch64-pc-windows-msvc");
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    return None;

    let candidate = npm_bin
        .join("node_modules")
        .join("@openai")
        .join("codex")
        .join("node_modules")
        .join("@openai")
        .join(package)
        .join("vendor")
        .join(target)
        .join("bin")
        .join("codex.exe");
    candidate.is_file().then_some(candidate)
}

fn command_stdout(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("{} {} probe failed: {error}", program, args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "{} {} probe exited with {}",
            program,
            args.join(" "),
            output.status
        ));
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
}

fn flag_support(help: Option<&str>, flag: &str) -> Option<bool> {
    help.map(|help| help.contains(flag))
}

fn any_flag_support(help: Option<&str>, flags: &[&str]) -> Option<bool> {
    help.map(|help| flags.iter().any(|flag| help.contains(flag)))
}
