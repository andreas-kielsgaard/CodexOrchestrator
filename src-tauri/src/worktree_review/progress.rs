use crate::worktree_runtime::{
    TestActionProgress, TestActionProgressSink, TestActionStage, TestStartProgress,
    TestStartProgressSink, TestStartStage,
};
use serde::Serialize;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_OUTPUT_LINES: usize = 12;
const MAX_HISTORY_LINES: usize = 4_000;
const MAX_OUTPUT_LINE_CHARS: usize = 180;
const QUIET_AFTER_MS: u64 = 20_000;

pub(crate) trait ReviewClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

pub(crate) struct SystemReviewClock;

impl ReviewClock for SystemReviewClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewOperationProgressView {
    pub(crate) operation_ref: String,
    pub(crate) operation: String,
    pub(crate) state: String,
    pub(crate) stage: String,
    pub(crate) stage_label: String,
    pub(crate) activity: String,
    pub(crate) elapsed_ms: u64,
    pub(crate) evidence_age_ms: u64,
    pub(crate) recent_output: Vec<String>,
    pub(crate) condition: String,
    pub(crate) expected_wait: String,
    pub(crate) action_required: bool,
    pub(crate) action_guidance: String,
    pub(crate) reusable_summary: String,
    pub(crate) missing_readiness_fact: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewOperationStageView {
    pub(crate) stage: String,
    pub(crate) stage_label: String,
    pub(crate) observed_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewOperationHistoryView {
    pub(crate) operation_ref: String,
    pub(crate) operation: String,
    pub(crate) state: String,
    pub(crate) stage_label: String,
    pub(crate) started_at_ms: u64,
    pub(crate) updated_at_ms: u64,
    pub(crate) stage_history: Vec<ReviewOperationStageView>,
    pub(crate) output: Vec<String>,
    pub(crate) output_complete: bool,
}

#[derive(Clone)]
struct ProgressRecord {
    operation_ref: String,
    scope: String,
    operation: String,
    state: OperationState,
    stage: String,
    stage_label: String,
    started_at_ms: u64,
    updated_at_ms: u64,
    stage_history: Vec<ReviewOperationStageView>,
    recent_output: VecDeque<String>,
    full_output: VecDeque<String>,
    output_complete: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum OperationState {
    Pending,
    Succeeded,
    Failed,
}

pub(crate) struct ProgressRegistry {
    clock: Arc<dyn ReviewClock>,
    inner: Mutex<ProgressState>,
}

#[derive(Default)]
struct ProgressState {
    records: HashMap<String, ProgressRecord>,
    active_by_scope: HashMap<String, String>,
}

impl ProgressRegistry {
    pub(crate) fn system() -> Self {
        Self::new(Arc::new(SystemReviewClock))
    }

    fn new(clock: Arc<dyn ReviewClock>) -> Self {
        Self {
            clock,
            inner: Mutex::new(ProgressState::default()),
        }
    }

    pub(crate) fn begin(
        self: &Arc<Self>,
        operation_ref: &str,
        scope: String,
        operation: &str,
        stage: &str,
        stage_label: &str,
    ) -> Result<ProgressHandle, String> {
        validate_operation_ref(operation_ref)?;
        let now = self.clock.now_ms();
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "Review progress is unavailable.".to_string())?;
        if inner.records.contains_key(operation_ref) {
            return Err("The review operation reference was already used.".into());
        }
        inner
            .active_by_scope
            .insert(scope.clone(), operation_ref.to_owned());
        inner.records.insert(
            operation_ref.to_owned(),
            ProgressRecord {
                operation_ref: operation_ref.to_owned(),
                scope: scope.clone(),
                operation: operation.to_owned(),
                state: OperationState::Pending,
                stage: stage.to_owned(),
                stage_label: stage_label.to_owned(),
                started_at_ms: now,
                updated_at_ms: now,
                stage_history: vec![ReviewOperationStageView {
                    stage: stage.to_owned(),
                    stage_label: stage_label.to_owned(),
                    observed_at_ms: now,
                }],
                recent_output: VecDeque::new(),
                full_output: VecDeque::new(),
                output_complete: true,
            },
        );
        Ok(ProgressHandle {
            registry: self.clone(),
            operation_ref: operation_ref.to_owned(),
            scope,
        })
    }

    pub(crate) fn get(&self, operation_ref: &str) -> Result<ReviewOperationProgressView, String> {
        let now = self.clock.now_ms();
        let inner = self
            .inner
            .lock()
            .map_err(|_| "Review progress is unavailable.".to_string())?;
        let record = inner
            .records
            .get(operation_ref)
            .ok_or_else(|| "Review progress is not available yet.".to_string())?;
        Ok(progress_view(record, now))
    }

    pub(crate) fn list(&self) -> Vec<ReviewOperationProgressView> {
        let now = self.clock.now_ms();
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        let mut records = inner.records.values().collect::<Vec<_>>();
        records.sort_by(|left, right| right.started_at_ms.cmp(&left.started_at_ms));
        records
            .into_iter()
            .take(20)
            .map(|record| progress_view(record, now))
            .collect()
    }

    pub(crate) fn history_for_instance(
        &self,
        instance_ref: &str,
    ) -> Vec<ReviewOperationHistoryView> {
        let suffix = format!(":{instance_ref}");
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        let mut records = inner
            .records
            .values()
            .filter(|record| record.scope.ends_with(&suffix))
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.started_at_ms);
        records
            .into_iter()
            .map(|record| ReviewOperationHistoryView {
                operation_ref: record.operation_ref.clone(),
                operation: record.operation.clone(),
                state: state_name(record.state).into(),
                stage_label: record.stage_label.clone(),
                started_at_ms: record.started_at_ms,
                updated_at_ms: record.updated_at_ms,
                stage_history: record.stage_history.clone(),
                output: record.full_output.iter().cloned().collect(),
                output_complete: record.output_complete,
            })
            .collect()
    }

    pub(crate) fn history(
        &self,
        operation_ref: &str,
    ) -> Result<(Option<String>, ReviewOperationHistoryView), String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "Review progress is unavailable.".to_string())?;
        let record = inner
            .records
            .get(operation_ref)
            .ok_or_else(|| "Review operation evidence is unavailable.".to_string())?;
        let instance_ref = record
            .scope
            .split_once(':')
            .map(|(_, instance_ref)| instance_ref.to_owned());
        Ok((
            instance_ref,
            ReviewOperationHistoryView {
                operation_ref: record.operation_ref.clone(),
                operation: record.operation.clone(),
                state: state_name(record.state).into(),
                stage_label: record.stage_label.clone(),
                started_at_ms: record.started_at_ms,
                updated_at_ms: record.updated_at_ms,
                stage_history: record.stage_history.clone(),
                output: record.full_output.iter().cloned().collect(),
                output_complete: record.output_complete,
            },
        ))
    }

    pub(crate) fn fail_operation(&self, operation_ref: &str) {
        let now = self.clock.now_ms();
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        if let Some(record) = inner.records.get_mut(operation_ref) {
            if record.state == OperationState::Pending {
                record.state = OperationState::Failed;
                record.stage = "failed".into();
                record.stage_label = "Stopped with an error".into();
                record.updated_at_ms = now;
                record.stage_history.push(ReviewOperationStageView {
                    stage: "failed".into(),
                    stage_label: "Stopped with an error".into(),
                    observed_at_ms: now,
                });
            }
        }
        inner
            .active_by_scope
            .retain(|_, active| active != operation_ref);
    }
}

fn progress_view(record: &ProgressRecord, now: u64) -> ReviewOperationProgressView {
    let evidence_age_ms = now.saturating_sub(record.updated_at_ms);
    let activity = match record.state {
        OperationState::Pending if evidence_age_ms >= QUIET_AFTER_MS => "quiet",
        OperationState::Pending => "working",
        OperationState::Succeeded | OperationState::Failed => "finished",
    };
    let guidance = guidance(record);
    ReviewOperationProgressView {
        operation_ref: record.operation_ref.clone(),
        operation: record.operation.clone(),
        state: match record.state {
            OperationState::Pending => "pending",
            OperationState::Succeeded => "succeeded",
            OperationState::Failed => "failed",
        }
        .into(),
        stage: record.stage.clone(),
        stage_label: record.stage_label.clone(),
        activity: activity.into(),
        elapsed_ms: now.saturating_sub(record.started_at_ms),
        evidence_age_ms,
        recent_output: record.recent_output.iter().cloned().collect(),
        condition: guidance.condition,
        expected_wait: guidance.expected_wait,
        action_required: guidance.action_required,
        action_guidance: guidance.action_guidance,
        reusable_summary: guidance.reusable_summary,
        missing_readiness_fact: guidance.missing_readiness_fact,
    }
}

fn state_name(state: OperationState) -> &'static str {
    match state {
        OperationState::Pending => "pending",
        OperationState::Succeeded => "succeeded",
        OperationState::Failed => "failed",
    }
}

struct ProgressGuidance {
    condition: String,
    expected_wait: String,
    action_required: bool,
    action_guidance: String,
    reusable_summary: String,
    missing_readiness_fact: Option<String>,
}

fn guidance(record: &ProgressRecord) -> ProgressGuidance {
    let failed = record.state == OperationState::Failed;
    let completed = record.state == OperationState::Succeeded;
    let (condition, expected_wait, reusable, missing) = match record.stage.as_str() {
        "preparation" => (
            "Resolving the selected source and reserving isolated mutable resources.",
            "Usually under a minute.",
            "No build artifact exists yet; a successful Prepare remains reusable.",
            None,
        ),
        "compatibility" => (
            "The selected worktree does not declare the required Worktree Review child contract.",
            "No waiting will make this source compatible.",
            "Its retained isolated instance can be inspected, but it cannot be built or opened here.",
            Some("A versioned readiness and provenance contract in the selected source."),
        ),
        "typecheck" => (
            "The selected source is being checked before frontend and native compilation.",
            "Usually seconds; cold machines may take longer.",
            "The isolated reservation remains reusable if this step fails.",
            None,
        ),
        "frontend-build" => (
            "The application interface is being built into this instance's private output.",
            "Usually seconds to a minute.",
            "Typecheck has completed; a failed frontend build does not remove the prepared instance.",
            None,
        ),
        "tauri-compile-link" => (
            "Rust crates are compiling and linking the private Tauri executable.",
            "A cold native build can take several minutes; ongoing output is normal evidence.",
            "Completed TypeScript and frontend output remain in the isolated instance.",
            None,
        ),
        "build-reuse" => (
            "The exact previously verified private build is being reused.",
            "Usually a few seconds for identity and artifact verification.",
            "No mutable target or application data is shared with another instance.",
            None,
        ),
        "finalization" => (
            "The build result and verified artifact identity are being finalized.",
            "Usually under a minute after native linking completes.",
            "Successful private outputs remain retained for Open and later exact reuse.",
            None,
        ),
        "reservation" => (
            "The runtime is reserving lifecycle ownership before any child is resumed.",
            "Usually a few seconds.",
            "The verified private build remains reusable if Open cannot continue.",
            None,
        ),
        "supporting-services" => (
            "Private frontend and status services are starting inside the owned process tree.",
            "Usually several seconds.",
            "The verified private executable remains reusable.",
            Some("Both isolated supporting services must report ready."),
        ),
        "native-start" => (
            "The verified private executable is starting under exact process ownership.",
            "Usually several seconds; no second Rust build is expected.",
            "The verified private executable remains reusable.",
            Some("The owned native process must create the expected review window."),
        ),
        "waiting-for-window" => (
            "Supporting services and the native process exist; the application is waiting for the human-review surface, not merely a process or port.",
            "Normally under a minute after native start.",
            "The private build and prepared isolation remain reusable if window readiness fails.",
            Some("An exact owned, titled, visible, useful-size window plus the rendered application readiness marker."),
        ),
        "ready" | "complete" => (
            "The operation completed with its required evidence.",
            "No further waiting is required.",
            "The retained private build and isolated state remain available for the next safe action.",
            None,
        ),
        "failed" => (
            "The operation ended before all required evidence was established.",
            "No further progress is expected from this operation.",
            "Open Build details to see which prepared or built material remains reusable.",
            None,
        ),
        _ => (
            "The owned operation is reconciling its current stage.",
            "Continue waiting while evidence updates.",
            "Previously completed isolated material is retained.",
            None,
        ),
    };
    ProgressGuidance {
        condition: condition.into(),
        expected_wait: expected_wait.into(),
        action_required: failed,
        action_guidance: if failed {
            "Open Build details, keep reusable material, and follow the displayed recovery or compatibility guidance."
        } else if completed {
            "No action is required unless you want to continue to the next lifecycle step."
        } else {
            "No action is required while owned evidence continues or the quiet interval remains bounded."
        }
        .into(),
        reusable_summary: reusable.into(),
        missing_readiness_fact: missing.map(str::to_owned),
    }
}

#[derive(Clone)]
pub(crate) struct ProgressHandle {
    registry: Arc<ProgressRegistry>,
    operation_ref: String,
    scope: String,
}

impl ProgressHandle {
    pub(crate) fn update(&self, stage: &str, stage_label: &str, output: Option<&str>) {
        let now = self.registry.clock.now_ms();
        let output = output.and_then(sanitize_output);
        let Ok(mut inner) = self.registry.inner.lock() else {
            return;
        };
        if inner.active_by_scope.get(&self.scope) != Some(&self.operation_ref) {
            return;
        }
        let Some(record) = inner.records.get_mut(&self.operation_ref) else {
            return;
        };
        if record.state != OperationState::Pending {
            return;
        }
        let stage_changed = record.stage != stage || record.stage_label != stage_label;
        let mut meaningful = stage_changed;
        record.stage = stage.to_owned();
        record.stage_label = stage_label.to_owned();
        if stage_changed {
            record.stage_history.push(ReviewOperationStageView {
                stage: stage.to_owned(),
                stage_label: stage_label.to_owned(),
                observed_at_ms: now,
            });
        }
        if let Some(output) = output {
            if record.recent_output.back() != Some(&output) {
                meaningful = true;
                record.recent_output.push_back(output.clone());
                while record.recent_output.len() > MAX_OUTPUT_LINES {
                    record.recent_output.pop_front();
                }
                record.full_output.push_back(output);
                while record.full_output.len() > MAX_HISTORY_LINES {
                    record.full_output.pop_front();
                    record.output_complete = false;
                }
            }
        }
        if meaningful {
            record.updated_at_ms = now;
        }
    }

    pub(crate) fn succeed(&self) {
        self.finish(OperationState::Succeeded, "complete", "Finished");
    }

    pub(crate) fn fail(&self) {
        self.finish(OperationState::Failed, "failed", "Stopped with an error");
    }

    pub(crate) fn fail_with(&self, stage: &str, label: &str, output: Option<&str>) {
        self.update(stage, label, output);
        self.finish(OperationState::Failed, stage, label);
    }

    fn finish(&self, state: OperationState, stage: &str, label: &str) {
        let now = self.registry.clock.now_ms();
        let Ok(mut inner) = self.registry.inner.lock() else {
            return;
        };
        let active = inner.active_by_scope.get(&self.scope) == Some(&self.operation_ref);
        if let Some(record) = inner.records.get_mut(&self.operation_ref) {
            if record.state == OperationState::Pending {
                record.state = state;
                record.stage = stage.into();
                record.stage_label = label.into();
                record.updated_at_ms = now;
                if record
                    .stage_history
                    .last()
                    .is_none_or(|last| last.stage != stage || last.stage_label != label)
                {
                    record.stage_history.push(ReviewOperationStageView {
                        stage: stage.into(),
                        stage_label: label.into(),
                        observed_at_ms: now,
                    });
                }
            }
        }
        if active {
            inner.active_by_scope.remove(&self.scope);
        }
    }
}

impl TestActionProgressSink for ProgressHandle {
    fn progress(&self, progress: TestActionProgress<'_>) {
        let (stage, label) = match progress.stage {
            TestActionStage::SourceInspection => {
                ("preparation", "Checking source and build inputs")
            }
            TestActionStage::Typecheck => ("typecheck", "Checking TypeScript"),
            TestActionStage::FrontendBuild => {
                ("frontend-build", "Building the application interface")
            }
            TestActionStage::TauriCompileLink => (
                "tauri-compile-link",
                "Compiling and linking the Tauri application",
            ),
            TestActionStage::BuildReuse => ("build-reuse", "Reusing the verified private build"),
            TestActionStage::Finalizing => ("finalization", "Finalizing the isolated build"),
        };
        self.update(stage, label, progress.output);
    }
}

impl TestStartProgressSink for ProgressHandle {
    fn progress(&self, progress: TestStartProgress<'_>) {
        let (stage, label) = match progress.stage {
            TestStartStage::Reservation => ("reservation", "Reserving the review instance"),
            TestStartStage::SupportingServices => (
                "supporting-services",
                "Starting isolated supporting services",
            ),
            TestStartStage::NativeStart => ("native-start", "Starting the verified worktree build"),
            TestStartStage::WaitingForWindow => (
                "waiting-for-window",
                "Waiting for a usable worktree-build window",
            ),
            TestStartStage::Ready => ("ready", "Worktree-build window ready"),
        };
        self.update(stage, label, progress.output);
    }
}

fn validate_operation_ref(value: &str) -> Result<(), String> {
    if !(8..=80).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("The review operation reference is invalid.".into());
    }
    Ok(())
}

pub(super) fn sanitize_output(input: &str) -> Option<String> {
    let mut text = strip_ansi(input).trim().to_owned();
    if text.is_empty() {
        return None;
    }
    text = redact_assignments(text);
    text = redact_urls(text);
    text = redact_paths(text);
    text = redact_ports(text);
    text = redact_process_identity(text);
    text = redact_long_identifiers(text);
    let bounded = if text.chars().count() > MAX_OUTPUT_LINE_CHARS {
        let prefix: String = text
            .chars()
            .take(MAX_OUTPUT_LINE_CHARS.saturating_sub(1))
            .collect();
        format!("{prefix}â€¦")
    } else {
        text
    };
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
        } else if !ch.is_control() || ch == '\t' {
            output.push(ch);
        }
    }
    output
}

fn redact_assignments(mut text: String) -> String {
    for marker in [
        "TOKEN=",
        "SECRET=",
        "PASSWORD=",
        "API_KEY=",
        "AUTHORITY=",
        "BEARER ",
        "GHP_",
        "SK-",
    ] {
        while let Some(index) = text.to_ascii_uppercase().find(marker) {
            let end = text[index..]
                .find(char::is_whitespace)
                .map(|offset| index + offset)
                .unwrap_or(text.len());
            text.replace_range(index..end, "[redacted credential]");
        }
    }
    text
}

fn redact_urls(mut text: String) -> String {
    for prefix in [
        "http://127.0.0.1:",
        "http://localhost:",
        "https://127.0.0.1:",
    ] {
        while let Some(index) = text.to_ascii_lowercase().find(prefix) {
            let end = text[index..]
                .find(char::is_whitespace)
                .map(|offset| index + offset)
                .unwrap_or(text.len());
            text.replace_range(index..end, "[private local endpoint]");
        }
    }
    text
}

fn redact_paths(text: String) -> String {
    let mut text = text;
    loop {
        let bytes = text.as_bytes();
        let start = (0..bytes.len()).find(|index| {
            let drive = index + 2 < bytes.len()
                && bytes[*index].is_ascii_alphabetic()
                && bytes[*index + 1] == b':'
                && matches!(bytes[*index + 2], b'\\' | b'/');
            let unc =
                index + 1 < bytes.len() && bytes[*index] == b'\\' && bytes[*index + 1] == b'\\';
            let unix =
                bytes[*index] == b'/' && (*index == 0 || bytes[*index - 1].is_ascii_whitespace());
            let relative = index + 2 < bytes.len()
                && bytes[*index] == b'.'
                && bytes[*index + 1] == b'.'
                && matches!(bytes[*index + 2], b'\\' | b'/')
                && (*index == 0
                    || bytes[*index - 1].is_ascii_whitespace()
                    || matches!(bytes[*index - 1], b'"' | b'\''));
            drive || unc || unix || relative
        });
        let Some(start) = start else {
            break;
        };
        let quoted = start > 0 && matches!(bytes[start - 1], b'"' | b'\'');
        let quote = if quoted { Some(bytes[start - 1]) } else { None };
        let end = (start..bytes.len())
            .find(|index| {
                quote
                    .map(|quote| bytes[*index] == quote)
                    .unwrap_or_else(|| bytes[*index].is_ascii_whitespace())
            })
            .unwrap_or(bytes.len());
        text.replace_range(start..end, "[private path]");
    }
    text
}

fn redact_ports(mut text: String) -> String {
    for marker in ["PORT=", "PORT ", "PORT:", "LISTENING ON "] {
        loop {
            let upper = text.to_ascii_uppercase();
            let Some(index) = upper.find(marker) else {
                break;
            };
            let digits = index + marker.len();
            let end = text[digits..]
                .find(|ch: char| !ch.is_ascii_digit())
                .map(|offset| digits + offset)
                .unwrap_or(text.len());
            if end == digits {
                break;
            }
            text.replace_range(index..end, "[private port]");
        }
    }
    text
}

fn redact_process_identity(mut text: String) -> String {
    while let Some(index) = text.find(r"Local\CodexOrchestrator.") {
        let end = text[index..]
            .find(char::is_whitespace)
            .map(|offset| index + offset)
            .unwrap_or(text.len());
        text.replace_range(index..end, "[private process identity]");
    }
    text
}

fn redact_long_identifiers(text: String) -> String {
    text.split_whitespace()
        .map(|token| {
            let candidate = token.trim_matches(|ch: char| !ch.is_ascii_hexdigit());
            if candidate.len() >= 20 && candidate.chars().all(|ch| ch.is_ascii_hexdigit()) {
                "[identifier]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct FakeClock(AtomicU64);

    impl ReviewClock for FakeClock {
        fn now_ms(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    #[test]
    fn bounds_and_redacts_safe_output() {
        let clock = Arc::new(FakeClock(AtomicU64::new(1_000)));
        let registry = Arc::new(ProgressRegistry::new(clock));
        let handle = registry
            .begin(
                "operation-safe-1",
                "build:instance".into(),
                "build",
                "typecheck",
                "Checking TypeScript",
            )
            .unwrap();
        for index in 0..20 {
            handle.update(
                "tauri-compile-link",
                "Compiling and linking the Tauri application",
                Some(&format!(
                    "TOKEN=secret-{index} \"C:\\private runtime\\target\" http://127.0.0.1:18200 PORT=18201 safe-line-{index} {}",
                    "a".repeat(64)
                )),
            );
        }
        let view = registry.get("operation-safe-1").unwrap();
        assert_eq!(view.recent_output.len(), MAX_OUTPUT_LINES);
        assert!(view
            .recent_output
            .iter()
            .all(|line| line.chars().count() <= MAX_OUTPUT_LINE_CHARS));
        let joined = view.recent_output.join("\n");
        assert!(!joined.contains("secret"));
        assert!(!joined.contains("C:\\private"));
        assert!(!joined.contains("18200"));
        assert!(!joined.contains("18201"));
        assert!(!joined.contains("runtime\\target"));
        assert!(joined.contains("[redacted credential]"));
        assert!(joined.contains("[private path]"));
        assert!(joined.contains("[private local endpoint]"));
        assert!(joined.contains("[identifier]"));
        let history = registry.history_for_instance("instance");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].output.len(), 20);
        assert!(history[0].output_complete);
        assert_eq!(
            history[0]
                .stage_history
                .iter()
                .map(|stage| stage.stage.as_str())
                .collect::<Vec<_>>(),
            ["typecheck", "tauri-compile-link"]
        );
    }

    #[test]
    fn redacts_relative_private_paths() {
        let sanitized = sanitize_output(
            "failed ../../../../AppData/Roaming/private/target and '..\\..\\private runtime\\log'",
        )
        .unwrap();
        assert_eq!(sanitized, "failed [private path] and '[private path]'");
        assert!(!sanitized.contains("AppData"));
        assert!(!sanitized.contains("private runtime"));
    }

    #[test]
    fn sanitizes_unicode_vite_output_without_poisoning_progress() {
        let sanitized = sanitize_output(
            "../../../../../../crp/private/dist/assets/app-C84RI57Y.js 160.08 kB â”‚ gzip: 41.05 kB",
        )
        .expect("safe Vite output");
        assert_eq!(sanitized, "[private path] 160.08 kB â”‚ gzip: 41.05 kB");
    }

    #[test]
    fn quiet_is_evidence_age_not_a_stall_claim() {
        let clock = Arc::new(FakeClock(AtomicU64::new(1_000)));
        let registry = Arc::new(ProgressRegistry::new(clock.clone()));
        registry
            .begin(
                "operation-quiet-1",
                "build:instance".into(),
                "build",
                "typecheck",
                "Checking TypeScript",
            )
            .unwrap();
        assert_eq!(
            registry.get("operation-quiet-1").unwrap().activity,
            "working"
        );
        clock.0.store(1_000 + QUIET_AFTER_MS, Ordering::SeqCst);
        let quiet = registry.get("operation-quiet-1").unwrap();
        assert_eq!(quiet.activity, "quiet");
        assert_eq!(quiet.state, "pending");
        assert!(!quiet.action_required);
        assert!(quiet.action_guidance.contains("No action is required"));
    }

    #[test]
    fn repeated_identical_observation_is_not_new_meaningful_evidence() {
        let clock = Arc::new(FakeClock(AtomicU64::new(1_000)));
        let registry = Arc::new(ProgressRegistry::new(clock.clone()));
        let handle = registry
            .begin(
                "operation-repeat-1",
                "start:instance".into(),
                "start",
                "waiting-for-window",
                "Waiting for a usable worktree-build window",
            )
            .unwrap();
        handle.update(
            "waiting-for-window",
            "Waiting for a usable worktree-build window",
            Some("The owned window exists but is not yet usable at review size."),
        );
        clock.0.store(1_000 + QUIET_AFTER_MS, Ordering::SeqCst);
        handle.update(
            "waiting-for-window",
            "Waiting for a usable worktree-build window",
            Some("The owned window exists but is not yet usable at review size."),
        );

        let view = registry.get("operation-repeat-1").unwrap();
        assert_eq!(view.recent_output.len(), 1);
        assert_eq!(view.activity, "quiet");
        assert_eq!(view.evidence_age_ms, QUIET_AFTER_MS);
    }

    #[test]
    fn waiting_window_and_failure_expose_exact_missing_fact_and_recovery_guidance() {
        let clock = Arc::new(FakeClock(AtomicU64::new(1_000)));
        let registry = Arc::new(ProgressRegistry::new(clock));
        let handle = registry
            .begin(
                "operation-window-1",
                "start:instance".into(),
                "start",
                "waiting-for-window",
                "Waiting for a usable worktree-build window",
            )
            .unwrap();
        let waiting = registry.get("operation-window-1").unwrap();
        assert!(waiting.condition.contains("human-review surface"));
        assert!(waiting
            .missing_readiness_fact
            .as_deref()
            .is_some_and(|fact| fact.contains("readiness marker")));
        handle.fail_with(
            "failed",
            "Owned window readiness was not established",
            Some("The exact titled rendered window did not appear."),
        );
        let failed = registry.get("operation-window-1").unwrap();
        assert!(failed.action_required);
        assert!(failed.action_guidance.contains("Build details"));
        assert_eq!(
            failed.stage_label,
            "Owned window readiness was not established"
        );
    }

    #[test]
    fn older_scope_updates_cannot_replace_newer_operation() {
        let clock = Arc::new(FakeClock(AtomicU64::new(1_000)));
        let registry = Arc::new(ProgressRegistry::new(clock));
        let older = registry
            .begin(
                "operation-old-1",
                "build:instance".into(),
                "build",
                "typecheck",
                "Checking TypeScript",
            )
            .unwrap();
        let newer = registry
            .begin(
                "operation-new-1",
                "build:instance".into(),
                "build",
                "frontend-build",
                "Building the application interface",
            )
            .unwrap();
        older.update("failed", "Wrong stale update", Some("stale"));
        newer.update(
            "tauri-compile-link",
            "Compiling and linking the Tauri application",
            Some("Compiling application"),
        );
        assert_eq!(registry.get("operation-old-1").unwrap().stage, "typecheck");
        assert_eq!(
            registry.get("operation-new-1").unwrap().stage,
            "tauri-compile-link"
        );
    }
}
