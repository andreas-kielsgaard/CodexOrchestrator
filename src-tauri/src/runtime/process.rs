use super::*;

impl CodexCommandRunner for SystemCodexCommandRunner {
    fn run(&self, input: CodexCommandRunInput) -> Result<CodexCommandRunResult, String> {
        let mut command = Command::new(&input.command);
        command.args(&input.args);

        if let Some(cwd) = &input.cwd {
            command.current_dir(cwd);
        }

        if let Some(env) = &input.env {
            for (key, value) in env {
                match value {
                    Some(value) => {
                        command.env(key, value);
                    }
                    None => {
                        command.env_remove(key);
                    }
                }
            }
        }

        let output = command
            .output()
            .map_err(|error| format!("Unable to launch Codex: {error}"))?;

        Ok(CodexCommandRunResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().map(i64::from),
            signal: process_exit_signal(&output.status),
        })
    }
}

impl GitDiffRunner for SystemGitDiffRunner {
    fn collect_tracked_diff(&self, input: GitDiffRunInput) -> Result<GitDiffRunResult, String> {
        let output = Command::new("git")
            .args(["diff", "--binary", "HEAD", "--"])
            .current_dir(&input.worktree_path)
            .output()
            .map_err(|error| format!("Unable to launch Git diff: {error}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code().map(i64::from);
        let signal = process_exit_signal(&output.status);

        if exit_code != Some(0) || signal.is_some() {
            return Err(format!(
                "Git diff failed {}: {}",
                process_failure_reason(exit_code, signal.as_deref()),
                stderr
            ));
        }

        Ok(GitDiffRunResult { diff: stdout })
    }
}

impl ValidationCommandRunner for SystemValidationCommandRunner {
    fn run(&self, input: ValidationCommandRunInput) -> Result<ValidationCommandRunResult, String> {
        let mut command = Command::new(&input.command);
        command.args(&input.args).current_dir(&input.cwd);

        if let Some(env) = &input.env {
            for (key, value) in env {
                match value {
                    Some(value) => {
                        command.env(key, value);
                    }
                    None => {
                        command.env_remove(key);
                    }
                }
            }
        }

        let output = command
            .output()
            .map_err(|error| format!("Unable to launch validation command: {error}"))?;

        Ok(ValidationCommandRunResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().map(i64::from),
            signal: process_exit_signal(&output.status),
        })
    }
}

pub(crate) fn process_failure_reason(exit_code: Option<i64>, signal: Option<&str>) -> String {
    if let Some(signal) = signal {
        return format!("on signal {signal}");
    }

    match exit_code {
        Some(exit_code) => format!("with exit code {exit_code}"),
        None => "without an exit code".to_string(),
    }
}

#[cfg(unix)]
pub(crate) fn process_exit_signal(status: &std::process::ExitStatus) -> Option<String> {
    use std::os::unix::process::ExitStatusExt;

    status.signal().map(|signal| signal.to_string())
}

#[cfg(not(unix))]
pub(crate) fn process_exit_signal(_status: &std::process::ExitStatus) -> Option<String> {
    None
}
