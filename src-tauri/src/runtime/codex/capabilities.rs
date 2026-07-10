use std::process::Command;

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

/// Probes each CLI surface independently. A failed optional probe leaves only that capability
/// unknown; it does not erase successfully discovered version or help data.
pub(crate) struct CodexCliCapabilityProbe {
    program: String,
}

impl CodexCliCapabilityProbe {
    pub(crate) fn new(program: impl Into<String>) -> Self {
        Self {
            program: resolve_program(program.into()),
        }
    }

    pub(crate) fn discover(&self) -> CodexCliCapabilities {
        let version = command_stdout(&self.program, &["--version"])
            .map(|output| output.trim().to_string())
            .filter(|output| !output.is_empty());
        let exec_help = command_stdout(&self.program, &["exec", "--help"]);
        let resume_help = command_stdout(&self.program, &["exec", "resume", "--help"]);

        CodexCliCapabilities {
            version,
            exec_json: flag_support(&exec_help, "--json"),
            resume_json: flag_support(&resume_help, "--json"),
            exec_model: flag_support(&exec_help, "--model"),
            resume_model: flag_support(&resume_help, "--model"),
            exec_sandbox: flag_support(&exec_help, "--sandbox"),
            resume_sandbox: flag_support(&resume_help, "--sandbox"),
        }
    }
}

pub(super) fn resolve_program(program: String) -> String {
    #[cfg(windows)]
    {
        use std::path::Path;

        if Path::new(&program).is_file() {
            return program;
        }
        if !program.contains('/') && !program.contains('\\') {
            if let Some(path) = std::env::var_os("PATH") {
                let directories = std::env::split_paths(&path).collect::<Vec<_>>();
                if program == "codex" {
                    for directory in &directories {
                        if directory.join("codex.cmd").is_file() {
                            if let Some(native) = npm_codex_native_binary(directory) {
                                return native.to_string_lossy().into_owned();
                            }
                        }
                    }
                }
                if let Some(path) = directories
                    .into_iter()
                    .map(|directory| directory.join(format!("{program}.exe")))
                    .find(|candidate| candidate.is_file())
                {
                    return path.to_string_lossy().into_owned();
                }
            }
        }
    }
    program
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

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output.status.success().then(|| {
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        text
    })
}

fn flag_support(help: &Option<String>, flag: &str) -> Option<bool> {
    help.as_ref().map(|help| help.contains(flag))
}
