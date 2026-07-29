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
}

#[derive(Clone)]
struct ProgressRecord {
    operation_ref: String,
    operation: String,
    state: OperationState,
    stage: String,
    stage_label: String,
    started_at_ms: u64,
    updated_at_ms: u64,
    recent_output: VecDeque<String>,
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
                operation: operation.to_owned(),
                state: OperationState::Pending,
                stage: stage.to_owned(),
                stage_label: stage_label.to_owned(),
                started_at_ms: now,
                updated_at_ms: now,
                recent_output: VecDeque::new(),
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
        record.stage = stage.to_owned();
        record.stage_label = stage_label.to_owned();
        if let Some(output) = output {
            record.recent_output.push_back(output);
            while record.recent_output.len() > MAX_OUTPUT_LINES {
                record.recent_output.pop_front();
            }
        }
        record.updated_at_ms = now;
    }

    pub(crate) fn succeed(&self) {
        self.finish(OperationState::Succeeded, "complete", "Finished");
    }

    pub(crate) fn fail(&self) {
        self.finish(OperationState::Failed, "failed", "Stopped with an error");
    }

    fn finish(&self, state: OperationState, stage: &str, label: &str) {
        let now = self.registry.clock.now_ms();
        let Ok(mut inner) = self.registry.inner.lock() else {
            return;
        };
        let active = inner.active_by_scope.get(&self.scope) == Some(&self.operation_ref);
        if let Some(record) = inner.records.get_mut(&self.operation_ref) {
            record.state = state;
            record.stage = stage.into();
            record.stage_label = label.into();
            record.updated_at_ms = now;
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

fn sanitize_output(input: &str) -> Option<String> {
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
        format!("{prefix}…")
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
                    "TOKEN=secret-{index} \"C:\\private runtime\\target\" http://127.0.0.1:18200 PORT=18201 {}",
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
            "../../../../../../crp/private/dist/assets/app-C84RI57Y.js 160.08 kB │ gzip: 41.05 kB",
        )
        .expect("safe Vite output");
        assert_eq!(sanitized, "[private path] 160.08 kB │ gzip: 41.05 kB");
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
