//! Background updates for application-supported local agent CLIs.
use chrono::Utc;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io,
    process::{Command, Stdio},
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::State;

const UPDATE_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CliUpdatePhase {
    Checking,
    Updating,
    Current,
    Updated,
    ControllerManaged,
    Failed,
    Unavailable,
}

impl CliUpdatePhase {
    fn active(self) -> bool {
        matches!(self, Self::Checking | Self::Updating)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliUpdateStatus {
    provider: String,
    label: String,
    phase: CliUpdatePhase,
    channel: String,
    previous_version: Option<String>,
    installed_version: Option<String>,
    checked_at: Option<String>,
    summary: String,
    notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliUpdateOverview {
    providers: Vec<CliUpdateStatus>,
}

#[derive(Clone)]
struct CliSpec {
    provider: &'static str,
    label: &'static str,
    program: String,
    update_args: &'static [&'static str],
}

trait CliCommandRunner: Send + Sync {
    fn version(&self, spec: &CliSpec) -> Result<String, String>;
    fn update(&self, spec: &CliSpec) -> Result<String, String>;
}

struct SystemCliCommandRunner;

impl CliCommandRunner for SystemCliCommandRunner {
    fn version(&self, spec: &CliSpec) -> Result<String, String> {
        let output = run_command(&spec.program, &["--version"], Duration::from_secs(20))?;
        Ok(first_nonempty_line(&output).unwrap_or_else(|| output.trim().to_string()))
    }

    fn update(&self, spec: &CliSpec) -> Result<String, String> {
        run_command(&spec.program, spec.update_args, UPDATE_TIMEOUT)
    }
}

struct UpdateState {
    providers: BTreeMap<String, CliUpdateStatus>,
}

pub(crate) struct AgentCliUpdateService {
    state: Mutex<UpdateState>,
    changed: Condvar,
    runner: Arc<dyn CliCommandRunner>,
    specs: Vec<CliSpec>,
}

impl AgentCliUpdateService {
    pub(crate) fn system() -> Arc<Self> {
        Self::new(
            Arc::new(SystemCliCommandRunner),
            vec![
                CliSpec {
                    provider: "codex",
                    label: "Codex CLI",
                    program: crate::runtime::providers::codex::resolve_program("codex".into())
                        .unwrap_or_else(|_| "codex".into()),
                    update_args: &["update"],
                },
                CliSpec {
                    provider: "claude",
                    label: "Claude Code",
                    program: crate::runtime::providers::claude::setups::effective_executable(
                        "claude",
                    ),
                    update_args: &["update"],
                },
            ],
        )
    }

    fn new(runner: Arc<dyn CliCommandRunner>, specs: Vec<CliSpec>) -> Arc<Self> {
        let providers = specs
            .iter()
            .map(|spec| {
                (
                    spec.provider.into(),
                    CliUpdateStatus {
                        provider: spec.provider.into(),
                        label: spec.label.into(),
                        phase: CliUpdatePhase::Checking,
                        channel: "Stable".into(),
                        previous_version: None,
                        installed_version: None,
                        checked_at: None,
                        summary: "Waiting for the background update check.".into(),
                        notes: Vec::new(),
                    },
                )
            })
            .collect();
        Arc::new(Self {
            state: Mutex::new(UpdateState { providers }),
            changed: Condvar::new(),
            runner,
            specs,
        })
    }

    pub(crate) fn start(self: &Arc<Self>) {
        for spec in self.specs.clone() {
            let service = self.clone();
            thread::spawn(move || service.update_one(spec));
        }
    }

    pub(crate) fn mark_controller_managed(&self) {
        let checked_at = Utc::now().to_rfc3339();
        if let Ok(mut state) = self.state.lock() {
            for status in state.providers.values_mut() {
                status.phase = CliUpdatePhase::ControllerManaged;
                status.checked_at = Some(checked_at.clone());
                status.summary =
                    "Updates are managed by the primary Orchid application instance.".into();
            }
            self.changed.notify_all();
        }
    }

    pub(crate) fn overview(&self) -> Result<CliUpdateOverview, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "CLI update status is unavailable".to_string())?;
        Ok(CliUpdateOverview {
            providers: state.providers.values().cloned().collect(),
        })
    }

    fn update_one(&self, spec: CliSpec) {
        let _update_lock = match provider_update_lock(spec.provider) {
            Ok(file) => file,
            Err(error) => {
                self.finish(
                    &spec,
                    CliUpdatePhase::Failed,
                    None,
                    None,
                    format!("{} update coordination failed: {error}", spec.label),
                    Vec::new(),
                );
                return;
            }
        };
        let before = match self.runner.version(&spec) {
            Ok(version) => version,
            Err(error) => {
                self.finish(
                    &spec,
                    CliUpdatePhase::Unavailable,
                    None,
                    None,
                    format!("{} is unavailable: {error}", spec.label),
                    Vec::new(),
                );
                return;
            }
        };
        self.mutate(&spec, |status| {
            status.phase = CliUpdatePhase::Updating;
            status.previous_version = Some(before.clone());
            status.installed_version = Some(before.clone());
            status.summary = format!(
                "Checking {} for a stable update in the background.",
                spec.label
            );
        });
        let update_output = match self.runner.update(&spec) {
            Ok(output) => output,
            Err(error) => {
                self.finish(
                    &spec,
                    CliUpdatePhase::Failed,
                    Some(before.clone()),
                    Some(before),
                    format!("{} could not be updated: {error}", spec.label),
                    Vec::new(),
                );
                return;
            }
        };
        let after = self
            .runner
            .version(&spec)
            .unwrap_or_else(|_| before.clone());
        let updated = after != before;
        self.finish(
            &spec,
            if updated {
                CliUpdatePhase::Updated
            } else {
                CliUpdatePhase::Current
            },
            Some(before.clone()),
            Some(after.clone()),
            if updated {
                format!("{} updated from {before} to {after}.", spec.label)
            } else {
                format!("{} is current at {after}.", spec.label)
            },
            updated
                .then(|| update_notes(&update_output))
                .unwrap_or_default(),
        );
    }

    fn mutate(&self, spec: &CliSpec, update: impl FnOnce(&mut CliUpdateStatus)) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(status) = state.providers.get_mut(spec.provider) {
                update(status);
            }
            self.changed.notify_all();
        }
    }

    fn finish(
        &self,
        spec: &CliSpec,
        phase: CliUpdatePhase,
        previous_version: Option<String>,
        installed_version: Option<String>,
        summary: String,
        notes: Vec<String>,
    ) {
        self.mutate(spec, |status| {
            status.phase = phase;
            status.previous_version = previous_version;
            status.installed_version = installed_version;
            status.checked_at = Some(Utc::now().to_rfc3339());
            status.summary = summary;
            status.notes = notes;
        });
    }
}

fn provider_update_lock(provider: &str) -> Result<File, String> {
    let path = std::env::temp_dir().join(format!("orchid-{provider}-cli-update.lock"));
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("unable to open {}: {error}", path.display()))?;
    file.lock()
        .map_err(|error| format!("unable to lock {}: {error}", path.display()))?;
    Ok(file)
}

impl crate::agent_sessions::application::ProviderUpdateReadiness for AgentCliUpdateService {
    fn is_updating(&self, provider: &str) -> bool {
        self.state
            .lock()
            .ok()
            .and_then(|state| {
                state
                    .providers
                    .get(provider)
                    .map(|status| status.phase.active())
            })
            .unwrap_or(false)
    }

    fn wait_until_ready(&self, provider: &str) -> Result<bool, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "CLI update status is unavailable".to_string())?;
        let waited = state
            .providers
            .get(provider)
            .is_some_and(|status| status.phase.active());
        while state
            .providers
            .get(provider)
            .is_some_and(|status| status.phase.active())
        {
            state = self
                .changed
                .wait(state)
                .map_err(|_| "CLI update status is unavailable".to_string())?;
        }
        Ok(waited)
    }
}

pub(crate) struct AgentCliUpdateTauriState(pub(crate) Arc<AgentCliUpdateService>);

#[tauri::command]
pub(crate) fn agent_cli_update_overview(
    state: State<'_, AgentCliUpdateTauriState>,
) -> Result<CliUpdateOverview, String> {
    state.0.overview()
}

fn run_command(program: &str, args: &[&str], timeout: Duration) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| command_error(program, args, error))?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| command_error(program, args, error))?;
                let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    if !text.ends_with('\n') && !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&stderr);
                }
                if output.status.success() {
                    return Ok(text);
                }
                return Err(format!(
                    "{} exited with {}{}",
                    command_label(program, args),
                    output.status,
                    first_nonempty_line(&text)
                        .map(|line| format!(": {line}"))
                        .unwrap_or_default()
                ));
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{} timed out", command_label(program, args)));
            }
            Err(error) => return Err(command_error(program, args, error)),
        }
    }
}

fn update_notes(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(12)
        .map(|line| line.chars().take(500).collect())
        .collect()
}

fn first_nonempty_line(value: &str) -> Option<String> {
    value
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn command_label(program: &str, args: &[&str]) -> String {
    format!("{program} {}", args.join(" "))
}

fn command_error(program: &str, args: &[&str], error: io::Error) -> String {
    format!("{} could not start: {error}", command_label(program, args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::application::ProviderUpdateReadiness;

    struct ImmediateRunner;
    impl CliCommandRunner for ImmediateRunner {
        fn version(&self, _: &CliSpec) -> Result<String, String> {
            Ok("1.2.3".into())
        }
        fn update(&self, _: &CliSpec) -> Result<String, String> {
            Ok("Release notes\n- fixed startup".into())
        }
    }

    #[test]
    fn background_check_reaches_current_and_releases_waiters() {
        let service = AgentCliUpdateService::new(
            Arc::new(ImmediateRunner),
            vec![CliSpec {
                provider: "codex",
                label: "Codex CLI",
                program: "codex".into(),
                update_args: &["update"],
            }],
        );
        service.start();
        service.wait_until_ready("codex").unwrap();
        let overview = service.overview().unwrap();
        assert_eq!(overview.providers[0].phase, CliUpdatePhase::Current);
        assert_eq!(
            overview.providers[0].installed_version.as_deref(),
            Some("1.2.3")
        );
    }

    #[test]
    fn update_notes_are_bounded_and_omit_empty_lines() {
        assert_eq!(
            update_notes("\n Release notes \n\n- one\n"),
            vec!["Release notes", "- one"]
        );
    }
}
