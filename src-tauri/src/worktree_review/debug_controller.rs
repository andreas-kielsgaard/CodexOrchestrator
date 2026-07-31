use super::{
    comparison::WorktreeComparisonView,
    detail::ReviewInstanceDetailView,
    service::{
        AcceptedReviewOperationView, HumanReviewLauncherService, ReviewInstanceView,
        ReviewOperationStatusView, ReviewSourceView,
    },
    worktree_build::WorktreeBuildContextView,
};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};
use tokio::sync::oneshot;
use uuid::Uuid;

const ENABLE_SETTING: &str = "CODEX_ORCHESTRATOR_REVIEW_CONTROLLER";
const ENABLE_VALUE: &str = "enabled";
const TOKEN_HEADER: &str = "x-codex-review-capability";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProofNavigationView {
    pub(crate) route: String,
    pub(crate) sequence: String,
}

#[tauri::command]
pub(crate) fn worktree_review_proof_navigation() -> Result<Option<ProofNavigationView>, String> {
    let path = std::env::var_os("CODEX_ORCHESTRATOR_REVIEW_NAVIGATION_PATH")
        .map(PathBuf::from)
        .ok_or_else(|| "This is not an isolated worktree-build proof surface.".to_string())?;
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "Proof navigation state is unavailable.".to_string())?;
    if !metadata.file_type().is_file() || metadata.len() > 4_096 {
        return Err("Proof navigation state is invalid.".into());
    }
    let value: ProofNavigationView = serde_json::from_slice(
        &fs::read(path).map_err(|_| "Proof navigation state is unavailable.".to_string())?,
    )
    .map_err(|_| "Proof navigation state is invalid.".to_string())?;
    if !matches!(
        value.route.as_str(),
        "application" | "worktree-details" | "file-review"
    ) || value.sequence.len() != 32
        || !value.sequence.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Proof navigation state is invalid.".into());
    }
    Ok(Some(value))
}

pub(crate) struct DebugReviewController {
    descriptor_path: PathBuf,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

impl Drop for DebugReviewController {
    fn drop(&mut self) {
        if let Ok(mut shutdown) = self.shutdown.lock() {
            if let Some(shutdown) = shutdown.take() {
                let _ = shutdown.send(());
            }
        }
        let _ = fs::remove_file(&self.descriptor_path);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ControllerDescriptor<'a> {
    version: u8,
    address: String,
    protected_capability: &'a str,
    process_id: u32,
}

pub(crate) fn start_if_enabled(
    service: Arc<HumanReviewLauncherService>,
    review_root: &Path,
) -> Result<Option<DebugReviewController>, String> {
    if !controller_enabled(
        cfg!(debug_assertions),
        std::env::var(ENABLE_SETTING).ok().as_deref(),
    ) {
        return Ok(None);
    }
    if !review_root.is_absolute() {
        return Err("The isolated review controller root must be absolute.".into());
    }
    let controller_root = review_root.join("debug-controller");
    fs::create_dir_all(&controller_root)
        .map_err(|error| format!("create isolated debug controller root: {error}"))?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| format!("bind isolated debug controller: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("read isolated debug controller address: {error}"))?;
    if address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err("The debug review controller did not bind to loopback.".into());
    }
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("configure isolated debug controller: {error}"))?;
    let capability = fresh_capability();
    let descriptor_path = controller_root.join(format!("controller-{}.json", std::process::id()));
    write_descriptor(&descriptor_path, address, &capability)?;
    let state = Arc::new(ControllerState::new(
        Arc::new(ServiceBackend { service }),
        capability,
    ));
    let (shutdown_send, shutdown_receive) = oneshot::channel();
    let thread_descriptor = descriptor_path.clone();
    let spawn = thread::Builder::new()
        .name("worktree-review-debug-controller".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let Ok(runtime) = runtime else {
                let _ = fs::remove_file(thread_descriptor);
                return;
            };
            runtime.block_on(async move {
                let listener = match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => listener,
                    Err(_) => return,
                };
                let server = axum::serve(listener, router(state)).with_graceful_shutdown(async {
                    let _ = shutdown_receive.await;
                });
                let _ = server.await;
            });
        });
    if let Err(error) = spawn {
        let _ = fs::remove_file(&descriptor_path);
        return Err(format!("start isolated debug controller: {error}"));
    }
    Ok(Some(DebugReviewController {
        descriptor_path,
        shutdown: Mutex::new(Some(shutdown_send)),
    }))
}

fn controller_enabled(debug_build: bool, value: Option<&str>) -> bool {
    debug_build && value == Some(ENABLE_VALUE)
}

fn fresh_capability() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn write_descriptor(path: &Path, address: SocketAddr, capability: &str) -> Result<(), String> {
    let protected_capability = protect_capability(capability)?;
    let bytes = serde_json::to_vec(&ControllerDescriptor {
        version: 1,
        address: format!("http://{address}"),
        protected_capability: &protected_capability,
        process_id: std::process::id(),
    })
    .map_err(|_| "encode isolated debug controller descriptor".to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create isolated debug controller descriptor: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("write isolated debug controller descriptor: {error}"))
}

#[cfg(windows)]
fn protect_capability(value: &str) -> Result<String, String> {
    use std::{ptr::null, slice};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB},
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: value.len() as u32,
        pbData: value.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    if unsafe {
        CryptProtectData(
            &input,
            null(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    } == 0
    {
        return Err("Protect the isolated debug controller capability.".into());
    }
    let bytes = unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let encoded = hex_encode(bytes);
    unsafe {
        LocalFree(output.pbData as _);
    }
    Ok(encoded)
}

#[cfg(not(windows))]
fn protect_capability(value: &str) -> Result<String, String> {
    Ok(hex_encode(value.as_bytes()))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0xf) as usize] as char);
    }
    output
}

fn router(state: Arc<ControllerState>) -> Router {
    Router::new()
        .route("/v1/command", post(command))
        .with_state(state)
}

async fn command(
    State(state): State<Arc<ControllerState>>,
    headers: HeaderMap,
    input: Result<Json<serde_json::Value>, JsonRejection>,
) -> Response {
    if !authorized(&headers, &state.capability) {
        return controller_error(StatusCode::UNAUTHORIZED, "Authentication failed.");
    }
    let Json(value) = match input {
        Ok(value) => value,
        Err(_) => {
            return controller_error(
                StatusCode::BAD_REQUEST,
                "The controller request schema is invalid.",
            )
        }
    };
    let input = match parse_envelope(value) {
        Ok(input) => input,
        Err(error) => return controller_error(error.status, &error.message),
    };
    match state.dispatch(input) {
        Ok(output) => (StatusCode::OK, Json(output)).into_response(),
        Err(error) => controller_error(error.status, &error.message),
    }
}

fn authorized(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| constant_time_equal(value.as_bytes(), expected.as_bytes()))
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn controller_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message,
        })),
    )
        .into_response()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommandEnvelope {
    request_ref: String,
    command: ControllerCommand,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum ControllerCommand {
    ListSources,
    ListInstances,
    BeginPrepare {
        source_ref: String,
        name: String,
    },
    BeginBuild {
        instance_ref: String,
    },
    BeginOpen {
        instance_ref: String,
        activation: OpenActivation,
    },
    Operation {
        operation_ref: String,
    },
    Status {
        instance_ref: String,
    },
    Stop {
        instance_ref: String,
    },
    Recover {
        instance_ref: String,
    },
    NavigateLauncher,
    NavigateLauncherDetail {
        instance_ref: String,
    },
    Navigate {
        instance_ref: String,
        route: ProofRoute,
    },
    WorktreeContext {
        instance_ref: String,
    },
    FileReview {
        instance_ref: String,
    },
    BuildDetail {
        instance_ref: String,
    },
}

fn parse_envelope(value: serde_json::Value) -> Result<CommandEnvelope, DispatchError> {
    let envelope = value
        .as_object()
        .ok_or_else(|| DispatchError::bad_request("The controller request must be an object."))?;
    ensure_keys(envelope, &["requestRef", "command"])?;
    let command = envelope
        .get("command")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| DispatchError::bad_request("The controller command must be an object."))?;
    let kind = command
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| DispatchError::bad_request("The controller command kind is required."))?;
    let allowed = match kind {
        "list_sources" | "list_instances" | "navigate_launcher" => &["kind"][..],
        "begin_prepare" => &["kind", "sourceRef", "name"],
        "begin_build"
        | "operation"
        | "status"
        | "stop"
        | "recover"
        | "worktree_context"
        | "file_review"
        | "build_detail"
        | "navigate_launcher_detail" => {
            if kind == "operation" {
                &["kind", "operationRef"][..]
            } else {
                &["kind", "instanceRef"][..]
            }
        }
        "begin_open" => &["kind", "instanceRef", "activation"],
        "navigate" => &["kind", "instanceRef", "route"],
        _ => {
            return Err(DispatchError::bad_request(
                "The controller command is unavailable.",
            ))
        }
    };
    ensure_keys(command, allowed)?;
    serde_json::from_value(value)
        .map_err(|_| DispatchError::bad_request("The controller request schema is invalid."))
}

fn ensure_keys(
    object: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
) -> Result<(), DispatchError> {
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(DispatchError::bad_request(
            "The controller request schema is invalid.",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum OpenActivation {
    BackgroundProof,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ProofRoute {
    Application,
    WorktreeDetails,
    FileReview,
}

impl ProofRoute {
    fn as_str(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::WorktreeDetails => "worktree-details",
            Self::FileReview => "file-review",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum ControllerOutput {
    Sources(Vec<ReviewSourceView>),
    Instances(Vec<ReviewInstanceView>),
    Accepted(AcceptedReviewOperationView),
    Operation(ReviewOperationStatusView),
    Instance(ReviewInstanceView),
    Navigated { route: String },
    WorktreeContext(WorktreeBuildContextView),
    FileReview(WorktreeComparisonView),
    BuildDetail(ReviewInstanceDetailView),
}

#[derive(Clone)]
enum ReplayEntry {
    Pending(String),
    Complete(String, Result<ControllerOutput, DispatchError>),
}

struct ControllerState {
    backend: Arc<dyn ControllerBackend>,
    capability: String,
    replay: Mutex<HashMap<String, ReplayEntry>>,
}

impl ControllerState {
    fn new(backend: Arc<dyn ControllerBackend>, capability: String) -> Self {
        Self {
            backend,
            capability,
            replay: Mutex::new(HashMap::new()),
        }
    }

    fn dispatch(&self, input: CommandEnvelope) -> Result<ControllerOutput, DispatchError> {
        validate_request_ref(&input.request_ref)?;
        let fingerprint = command_fingerprint(&input.command)?;
        {
            let mut replay = self.replay.lock().map_err(|_| {
                DispatchError::unavailable("The controller request registry is unavailable.")
            })?;
            match replay.get(&input.request_ref) {
                Some(ReplayEntry::Pending(existing)) if existing == &fingerprint => {
                    return Err(DispatchError::conflict(
                        "The matching controller request is still in progress.",
                    ));
                }
                Some(ReplayEntry::Complete(existing, output)) if existing == &fingerprint => {
                    return output.clone();
                }
                Some(_) => {
                    return Err(DispatchError::conflict(
                        "The controller request reference was already used for different semantics.",
                    ));
                }
                None => {
                    replay.insert(
                        input.request_ref.clone(),
                        ReplayEntry::Pending(fingerprint.clone()),
                    );
                }
            }
        }
        let result = self.execute(input.command);
        if let Ok(mut replay) = self.replay.lock() {
            replay.insert(
                input.request_ref,
                ReplayEntry::Complete(fingerprint, result.clone()),
            );
        }
        result
    }

    fn execute(&self, command: ControllerCommand) -> Result<ControllerOutput, DispatchError> {
        let result = match command {
            ControllerCommand::ListSources => Ok(ControllerOutput::Sources(self.backend.sources())),
            ControllerCommand::ListInstances => {
                Ok(ControllerOutput::Instances(self.backend.instances()))
            }
            ControllerCommand::BeginPrepare { source_ref, name } => self
                .backend
                .begin_prepare(source_ref, name)
                .map(ControllerOutput::Accepted),
            ControllerCommand::BeginBuild { instance_ref } => self
                .backend
                .begin_build(instance_ref)
                .map(ControllerOutput::Accepted),
            ControllerCommand::BeginOpen {
                instance_ref,
                activation: OpenActivation::BackgroundProof,
            } => self
                .backend
                .begin_open(instance_ref)
                .map(ControllerOutput::Accepted),
            ControllerCommand::Operation { operation_ref } => self
                .backend
                .operation(operation_ref)
                .map(ControllerOutput::Operation),
            ControllerCommand::Status { instance_ref } => self
                .backend
                .status(instance_ref)
                .map(ControllerOutput::Instance),
            ControllerCommand::Stop { instance_ref } => self
                .backend
                .stop(instance_ref)
                .map(ControllerOutput::Instance),
            ControllerCommand::Recover { instance_ref } => self
                .backend
                .recover(instance_ref)
                .map(ControllerOutput::Instance),
            ControllerCommand::NavigateLauncher => {
                self.backend
                    .navigate_launcher()
                    .map(|()| ControllerOutput::Navigated {
                        route: "worktree-review".into(),
                    })
            }
            ControllerCommand::NavigateLauncherDetail { instance_ref } => self
                .backend
                .navigate_launcher_detail(instance_ref)
                .map(|()| ControllerOutput::Navigated {
                    route: "worktree-review-detail".into(),
                }),
            ControllerCommand::Navigate {
                instance_ref,
                route,
            } => self
                .backend
                .navigate(instance_ref, route)
                .map(|()| ControllerOutput::Navigated {
                    route: route.as_str().into(),
                }),
            ControllerCommand::WorktreeContext { instance_ref } => self
                .backend
                .context(instance_ref)
                .map(ControllerOutput::WorktreeContext),
            ControllerCommand::FileReview { instance_ref } => self
                .backend
                .file_review(instance_ref)
                .map(ControllerOutput::FileReview),
            ControllerCommand::BuildDetail { instance_ref } => self
                .backend
                .detail(instance_ref)
                .map(ControllerOutput::BuildDetail),
        };
        result.map_err(DispatchError::safe)
    }
}

fn validate_request_ref(value: &str) -> Result<(), DispatchError> {
    if !(16..=80).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(DispatchError::bad_request(
            "The controller request reference is invalid.",
        ));
    }
    Ok(())
}

fn command_fingerprint(command: &ControllerCommand) -> Result<String, DispatchError> {
    serde_json::to_vec(command)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| DispatchError::bad_request("The controller command is invalid."))
}

trait ControllerBackend: Send + Sync {
    fn sources(&self) -> Vec<ReviewSourceView>;
    fn instances(&self) -> Vec<ReviewInstanceView>;
    fn begin_prepare(
        &self,
        source_ref: String,
        name: String,
    ) -> Result<AcceptedReviewOperationView, String>;
    fn begin_build(&self, instance_ref: String) -> Result<AcceptedReviewOperationView, String>;
    fn begin_open(&self, instance_ref: String) -> Result<AcceptedReviewOperationView, String>;
    fn operation(&self, operation_ref: String) -> Result<ReviewOperationStatusView, String>;
    fn status(&self, instance_ref: String) -> Result<ReviewInstanceView, String>;
    fn stop(&self, instance_ref: String) -> Result<ReviewInstanceView, String>;
    fn recover(&self, instance_ref: String) -> Result<ReviewInstanceView, String>;
    fn navigate_launcher(&self) -> Result<(), String>;
    fn navigate_launcher_detail(&self, instance_ref: String) -> Result<(), String>;
    fn navigate(&self, instance_ref: String, route: ProofRoute) -> Result<(), String>;
    fn context(&self, instance_ref: String) -> Result<WorktreeBuildContextView, String>;
    fn file_review(&self, instance_ref: String) -> Result<WorktreeComparisonView, String>;
    fn detail(&self, instance_ref: String) -> Result<ReviewInstanceDetailView, String>;
}

struct ServiceBackend {
    service: Arc<HumanReviewLauncherService>,
}

impl ControllerBackend for ServiceBackend {
    fn sources(&self) -> Vec<ReviewSourceView> {
        self.service.sources()
    }

    fn instances(&self) -> Vec<ReviewInstanceView> {
        self.service.instances()
    }

    fn begin_prepare(
        &self,
        source_ref: String,
        name: String,
    ) -> Result<AcceptedReviewOperationView, String> {
        self.service.begin_prepare(source_ref, name)
    }

    fn begin_build(&self, instance_ref: String) -> Result<AcceptedReviewOperationView, String> {
        self.service.begin_build(instance_ref)
    }

    fn begin_open(&self, instance_ref: String) -> Result<AcceptedReviewOperationView, String> {
        self.service.begin_open(instance_ref, false)
    }

    fn operation(&self, operation_ref: String) -> Result<ReviewOperationStatusView, String> {
        self.service.operation_status(operation_ref)
    }

    fn status(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.service.status(instance_ref)
    }

    fn stop(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.service.stop(instance_ref)
    }

    fn recover(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.service.recover(instance_ref)
    }

    fn navigate_launcher(&self) -> Result<(), String> {
        self.service.proof_navigate_launcher()
    }

    fn navigate_launcher_detail(&self, instance_ref: String) -> Result<(), String> {
        self.service.proof_navigate_launcher_detail(instance_ref)
    }

    fn navigate(&self, instance_ref: String, route: ProofRoute) -> Result<(), String> {
        self.service.proof_navigate(instance_ref, route.as_str())
    }

    fn context(&self, instance_ref: String) -> Result<WorktreeBuildContextView, String> {
        self.service.context(instance_ref)
    }

    fn file_review(&self, instance_ref: String) -> Result<WorktreeComparisonView, String> {
        self.service.comparison(instance_ref)
    }

    fn detail(&self, instance_ref: String) -> Result<ReviewInstanceDetailView, String> {
        self.service.detail(instance_ref)
    }
}

#[derive(Clone, Debug)]
struct DispatchError {
    status: StatusCode,
    message: String,
}

impl DispatchError {
    fn bad_request(message: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn conflict(message: &str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    fn unavailable(message: &str) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: message.into(),
        }
    }

    fn safe(message: String) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;

    #[derive(Default)]
    struct FakeBackend {
        calls: Mutex<Vec<String>>,
    }

    impl ControllerBackend for FakeBackend {
        fn sources(&self) -> Vec<ReviewSourceView> {
            self.calls.lock().unwrap().push("sources".into());
            Vec::new()
        }
        fn instances(&self) -> Vec<ReviewInstanceView> {
            Vec::new()
        }
        fn begin_prepare(
            &self,
            _source_ref: String,
            _name: String,
        ) -> Result<AcceptedReviewOperationView, String> {
            unreachable!()
        }
        fn begin_build(
            &self,
            _instance_ref: String,
        ) -> Result<AcceptedReviewOperationView, String> {
            unreachable!()
        }
        fn begin_open(&self, _instance_ref: String) -> Result<AcceptedReviewOperationView, String> {
            unreachable!()
        }
        fn operation(&self, _operation_ref: String) -> Result<ReviewOperationStatusView, String> {
            unreachable!()
        }
        fn status(&self, _instance_ref: String) -> Result<ReviewInstanceView, String> {
            unreachable!()
        }
        fn stop(&self, _instance_ref: String) -> Result<ReviewInstanceView, String> {
            unreachable!()
        }
        fn recover(&self, _instance_ref: String) -> Result<ReviewInstanceView, String> {
            unreachable!()
        }
        fn navigate_launcher(&self) -> Result<(), String> {
            self.calls.lock().unwrap().push("navigate-launcher".into());
            Ok(())
        }
        fn navigate_launcher_detail(&self, _instance_ref: String) -> Result<(), String> {
            unreachable!()
        }
        fn navigate(&self, _instance_ref: String, _route: ProofRoute) -> Result<(), String> {
            unreachable!()
        }
        fn context(&self, _instance_ref: String) -> Result<WorktreeBuildContextView, String> {
            unreachable!()
        }
        fn file_review(&self, _instance_ref: String) -> Result<WorktreeComparisonView, String> {
            unreachable!()
        }
        fn detail(&self, _instance_ref: String) -> Result<ReviewInstanceDetailView, String> {
            unreachable!()
        }
    }

    #[test]
    fn controller_is_absent_without_debug_and_explicit_enablement() {
        assert!(!controller_enabled(false, Some(ENABLE_VALUE)));
        assert!(!controller_enabled(true, None));
        assert!(!controller_enabled(true, Some("true")));
        assert!(controller_enabled(true, Some(ENABLE_VALUE)));
        let first = fresh_capability();
        let second = fresh_capability();
        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
        let protected = protect_capability(&first).expect("protect capability");
        assert!(!protected.contains(&first));
    }

    #[tokio::test]
    async fn rejects_invalid_capability_and_schema_without_leaking_the_capability() {
        let state = Arc::new(ControllerState::new(
            Arc::new(FakeBackend::default()),
            "not-logged-controller-secret".into(),
        ));
        let invalid = request(
            state.clone(),
            Some("wrong"),
            r#"{"requestRef":"request-reference-1","command":{"kind":"list_sources"}}"#,
        )
        .await;
        assert_eq!(invalid.0, StatusCode::UNAUTHORIZED);
        assert!(!invalid.1.contains("not-logged-controller-secret"));

        let malformed = request(
            state,
            Some("not-logged-controller-secret"),
            r#"{"requestRef":"request-reference-2","command":{"kind":"list_sources","extra":"rejected"}}"#,
        )
        .await;
        assert_eq!(malformed.0, StatusCode::BAD_REQUEST);
        assert!(!malformed.1.contains("not-logged-controller-secret"));
    }

    #[test]
    fn exact_replay_converges_and_collision_fails_before_execution() {
        let backend = Arc::new(FakeBackend::default());
        let state = ControllerState::new(backend.clone(), "secret".into());
        let request = CommandEnvelope {
            request_ref: "request-reference-3".into(),
            command: ControllerCommand::ListSources,
        };
        assert!(matches!(
            state.dispatch(request.clone()).unwrap(),
            ControllerOutput::Sources(_)
        ));
        assert!(matches!(
            state.dispatch(request).unwrap(),
            ControllerOutput::Sources(_)
        ));
        let collision = state
            .dispatch(CommandEnvelope {
                request_ref: "request-reference-3".into(),
                command: ControllerCommand::ListInstances,
            })
            .expect_err("collision");
        assert_eq!(collision.status, StatusCode::CONFLICT);
        assert_eq!(backend.calls.lock().unwrap().as_slice(), ["sources"]);
        assert!(matches!(
            state
                .dispatch(CommandEnvelope {
                    request_ref: "request-reference-4".into(),
                    command: ControllerCommand::NavigateLauncher,
                })
                .unwrap(),
            ControllerOutput::Navigated { route } if route == "worktree-review"
        ));
        assert_eq!(
            backend.calls.lock().unwrap().as_slice(),
            ["sources", "navigate-launcher"]
        );
    }

    async fn request(
        state: Arc<ControllerState>,
        token: Option<&str>,
        body: &str,
    ) -> (StatusCode, String) {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/v1/command")
            .header("content-type", "application/json");
        if let Some(token) = token {
            builder = builder.header(TOKEN_HEADER, token);
        }
        let response = router(state)
            .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }
}
