use super::{
    application::{WorkflowApplication, WorkflowRepository},
    domain::{
        WorkflowConnectionConfig, WorkflowDefinition, WorkflowElementKind, WorkflowElementRef,
        WorkflowNodeConfig,
    },
    instance_domain::{
        ResolvedRepoBranchWorktreeTarget, WorkflowBranchTarget, WorkflowRepositoryTarget,
        WorkflowWorktreeTarget,
    },
    repository::SqliteWorkflowRepository,
};
use crate::{
    agent_sessions::{
        application::{
            AgentSessionApplication, AgentSessionNotification, AgentSessionNotifier,
            SendAgentSessionMessageCommand, SendAgentSessionMessageResult,
            SystemAgentSessionProviders,
        },
        domain::{AgentInvocationStatus, AgentSessionId, NormalizedRuntimeEventKind},
        repository::SqliteAgentSessionRepository,
    },
    harness_engine::{HarnessEngineService, ManagedMcpUpstreamRegistry},
    native_profiles::NativeProfileService,
    runtime::codex::CodexCliRuntime,
    storage,
    worktree_targets_temp::DiscoveredWorktreeTargetSource,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    thread,
    time::{Duration, Instant},
};

const COMMAND: &str = "workflow-cli";
const CATALOG_CONTRACT: &str = "workflow-cli-catalog/v1";
const DEFAULT_WAIT_SECONDS: u64 = 600;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkflowCatalog {
    contract_version: String,
    workflows: Vec<WorkflowBlueprint>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkflowBlueprint {
    name: String,
    nodes: Vec<WorkflowNodeConfig>,
    connections: Vec<WorkflowConnectionConfig>,
}

#[derive(Default)]
struct CliNotifier {
    workflow: Mutex<Option<Weak<WorkflowApplication>>>,
}

impl AgentSessionNotifier for CliNotifier {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
        let workflow = self.workflow.lock().ok().and_then(|slot| slot.clone());
        if let Some(workflow) = workflow.and_then(|workflow| workflow.upgrade()) {
            // Routing is best effort after the terminal Agent Session fact is durable, matching
            // the desktop composition. Connection failures remain recorded on their activation.
            let _ = workflow.on_agent_notification(&notification);
        }
        Ok(())
    }
}

struct CliRuntimeContext {
    workflow: Arc<WorkflowApplication>,
    sessions: Arc<AgentSessionApplication>,
    harnesses: Arc<HarnessEngineService>,
    database_path: PathBuf,
}

impl Drop for CliRuntimeContext {
    fn drop(&mut self) {
        let _ = self.harnesses.shutdown();
    }
}

pub(crate) fn run_if_requested() -> Option<Result<(), String>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.first().map(String::as_str) != Some(COMMAND) {
        return None;
    }
    Some(run(&arguments[1..]))
}

fn run(arguments: &[String]) -> Result<(), String> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(usage());
    };
    let options = parse_options(&arguments[1..])?;
    let app_data_dir = absolute_path(required_option(&options, "app-data-dir")?, "app-data-dir")?;
    let database_path = storage::active_database_path(&app_data_dir);
    match command {
        "import" => {
            let file = absolute_path(required_option(&options, "file")?, "file")?;
            let catalog = read_catalog(&file)?;
            let installed = import_catalog(&database_path, catalog)?;
            print_json(json!({
                "command": "import",
                "workflowTypes": installed.iter().map(|definition| json!({
                    "id": definition.workflow_type.id,
                    "name": definition.workflow_type.name,
                    "activeRecipeId": definition.workflow_type.active_recipe_id,
                })).collect::<Vec<_>>()
            }))
        }
        "instantiate" => {
            let workflow_reference = required_option(&options, "workflow")?;
            let name = required_option(&options, "name")?;
            let worktree = absolute_path(required_option(&options, "worktree")?, "worktree")?;
            let context = compose_runtime(&app_data_dir)?;
            let definition = find_workflow(&context.workflow, workflow_reference)?;
            let target = resolve_target(&app_data_dir, &worktree)?;
            let instance = context.workflow.create_workflow_instance(
                &definition.workflow_type.id,
                name,
                target,
            )?;
            print_json(json!({
                "command": "instantiate",
                "instanceId": instance.summary.id,
                "instanceName": instance.summary.name,
                "workflowTypeId": instance.summary.workflow_type_id,
                "workflowTypeName": instance.summary.workflow_type_name,
                "recipeId": instance.summary.recipe_id,
                "worktree": instance.target.worktree.path,
            }))
        }
        "send" => {
            let instance_reference = required_option(&options, "instance")?;
            let node_reference = required_option(&options, "node")?;
            let message = required_option(&options, "message")?.to_string();
            let wait_seconds = options
                .get("wait-seconds")
                .map(|value| {
                    value
                        .parse::<u64>()
                        .map_err(|_| "--wait-seconds must be a positive integer".to_string())
                })
                .transpose()?
                .unwrap_or(DEFAULT_WAIT_SECONDS);
            let context = compose_runtime(&app_data_dir)?;
            let instance = find_instance(&context.workflow, instance_reference)?;
            let node = instance
                .recipe
                .nodes
                .iter()
                .find(|node| node.id == node_reference || node.name == node_reference)
                .ok_or_else(|| format!("Workflow node {node_reference} was not found."))?;
            let acknowledgement = send_to_node(&context, &instance, &node.id, message)?;
            let settled = wait_until_settled(
                &context,
                &instance.summary.id,
                &acknowledgement,
                Duration::from_secs(wait_seconds),
            )?;
            print_json(json!({
                "command": "send",
                "sourceSessionId": acknowledgement.session_id.as_str(),
                "sourceInvocationId": acknowledgement.invocation_id.as_str(),
                "instance": settled,
            }))
        }
        "show" => {
            let instance_reference = required_option(&options, "instance")?;
            let context = compose_runtime(&app_data_dir)?;
            let instance = find_instance(&context.workflow, instance_reference)?;
            print_json(instance_report(&context, &instance.summary.id)?)
        }
        _ => Err(usage()),
    }
}

fn parse_options(arguments: &[String]) -> Result<HashMap<String, String>, String> {
    let mut options = HashMap::new();
    let mut index = 0;
    while index < arguments.len() {
        let key = arguments[index].strip_prefix("--").ok_or_else(usage)?;
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("--{key} requires a value"))?;
        if value.starts_with("--") {
            return Err(format!("--{key} requires a value"));
        }
        if options.insert(key.to_string(), value.clone()).is_some() {
            return Err(format!("--{key} was supplied more than once"));
        }
        index += 2;
    }
    Ok(options)
}

fn required_option<'a>(
    options: &'a HashMap<String, String>,
    name: &str,
) -> Result<&'a str, String> {
    options
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("--{name} is required"))
}

fn absolute_path(value: &str, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("--{name} must be an absolute path"));
    }
    Ok(path)
}

fn read_catalog(path: &Path) -> Result<WorkflowCatalog, String> {
    let content = fs::read_to_string(path).map_err(|error| {
        format!(
            "Unable to read Workflow catalog {}: {error}",
            path.display()
        )
    })?;
    let catalog = serde_json::from_str::<WorkflowCatalog>(&content).map_err(|error| {
        format!(
            "Unable to parse Workflow catalog {}: {error}",
            path.display()
        )
    })?;
    if catalog.contract_version != CATALOG_CONTRACT {
        return Err(format!(
            "Unsupported Workflow catalog contract {}.",
            catalog.contract_version
        ));
    }
    validate_catalog(&catalog)?;
    Ok(catalog)
}

fn validate_catalog(catalog: &WorkflowCatalog) -> Result<(), String> {
    if catalog.workflows.is_empty() {
        return Err("A Workflow catalog must contain at least one Workflow.".to_string());
    }
    let mut names = HashSet::new();
    let mut element_ids = HashSet::new();
    for workflow in &catalog.workflows {
        if workflow.name.trim().is_empty() {
            return Err("Workflow names cannot be blank.".to_string());
        }
        if !names.insert(workflow.name.trim().to_lowercase()) {
            return Err(format!(
                "Workflow {} appears more than once.",
                workflow.name
            ));
        }
        for id in workflow.nodes.iter().map(|node| node.id.as_str()).chain(
            workflow
                .connections
                .iter()
                .map(|connection| connection.id.as_str()),
        ) {
            if !element_ids.insert(id.to_string()) {
                return Err(format!("Workflow element id {id} appears more than once."));
            }
        }
    }
    Ok(())
}

fn import_catalog(
    database_path: &Path,
    catalog: WorkflowCatalog,
) -> Result<Vec<WorkflowDefinition>, String> {
    let repository = SqliteWorkflowRepository::open(database_path)?;
    let existing = repository.list_workflow_types()?;
    for workflow in &catalog.workflows {
        if existing
            .iter()
            .any(|candidate| candidate.name == workflow.name)
        {
            return Err(format!(
                "Workflow {} already exists; catalog import never overwrites definitions.",
                workflow.name
            ));
        }
    }
    catalog
        .workflows
        .into_iter()
        .map(|workflow| install_workflow(&repository, workflow))
        .collect()
}

fn install_workflow(
    repository: &dyn WorkflowRepository,
    workflow: WorkflowBlueprint,
) -> Result<WorkflowDefinition, String> {
    let definition = repository.create_workflow_type(&workflow.name)?;
    let workflow_type_id = definition.workflow_type.id;
    let mut elements = Vec::with_capacity(workflow.nodes.len() + workflow.connections.len());
    for node in workflow.nodes {
        elements.push(WorkflowElementRef {
            kind: WorkflowElementKind::Node,
            id: node.id.clone(),
        });
        repository.save_node_draft(&workflow_type_id, node)?;
    }
    for connection in workflow.connections {
        elements.push(WorkflowElementRef {
            kind: WorkflowElementKind::Connection,
            id: connection.id.clone(),
        });
        repository.save_connection_draft(&workflow_type_id, connection)?;
    }
    repository.activate_changes(&workflow_type_id, &elements)
}

fn compose_runtime(app_data_dir: &Path) -> Result<CliRuntimeContext, String> {
    fs::create_dir_all(app_data_dir).map_err(|error| {
        format!(
            "Unable to create application data directory {}: {error}",
            app_data_dir.display()
        )
    })?;
    let database_path = storage::active_database_path(app_data_dir);
    let connection = storage::open_active_database(&database_path)?;
    let session_repository =
        Arc::new(SqliteAgentSessionRepository::new(connection).map_err(|error| error.to_string())?);
    let native_profiles = Arc::new(NativeProfileService::open(
        database_path.clone(),
        app_data_dir.to_path_buf(),
    )?);
    let upstreams = Arc::new(ManagedMcpUpstreamRegistry::default());
    let harnesses = HarnessEngineService::open_system(&database_path, upstreams.clone())?;
    let notifier = Arc::new(CliNotifier::default());
    let providers = Arc::new(SystemAgentSessionProviders);
    let sessions = Arc::new(
        AgentSessionApplication::new(
            session_repository,
            Arc::new(CodexCliRuntime::system("codex", None)),
            notifier.clone(),
            providers.clone(),
            providers,
            None,
        )
        .with_native_profile_launch_authority(native_profiles)
        .with_session_harness_launch_authority(harnesses.clone()),
    );
    let workflow = Arc::new(WorkflowApplication::new(
        Arc::new(SqliteWorkflowRepository::open(&database_path)?),
        sessions.clone(),
        harnesses.clone(),
    ));
    let (descriptor, owner) = super::mcp::start_sample_server(Arc::downgrade(&workflow))?;
    let registration = upstreams.register(descriptor)?;
    if let Err(owner) = upstreams.retain_owner(&registration, owner) {
        owner.stop();
        upstreams.unregister(&registration);
        return Err("Unable to retain the Workflow MCP server.".to_string());
    }
    *notifier
        .workflow
        .lock()
        .map_err(|_| "Workflow CLI notification registry is unavailable.".to_string())? =
        Some(Arc::downgrade(&workflow));
    Ok(CliRuntimeContext {
        workflow,
        sessions,
        harnesses,
        database_path,
    })
}

fn find_workflow(
    workflow: &WorkflowApplication,
    reference: &str,
) -> Result<WorkflowDefinition, String> {
    let matches = workflow
        .list_workflow_types()?
        .into_iter()
        .filter(|candidate| candidate.id == reference || candidate.name == reference)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [candidate] => workflow.load_workflow_type(&candidate.id),
        [] => Err(format!("Workflow {reference} was not found.")),
        _ => Err(format!(
            "Workflow name {reference} is ambiguous; use its id."
        )),
    }
}

fn find_instance(
    workflow: &WorkflowApplication,
    reference: &str,
) -> Result<super::instance_domain::WorkflowInstance, String> {
    let matches = workflow
        .list_workflow_instances()?
        .into_iter()
        .filter(|candidate| candidate.id == reference || candidate.name == reference)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [candidate] => workflow.load_workflow_instance(&candidate.id),
        [] => Err(format!("Workflow instance {reference} was not found.")),
        _ => Err(format!(
            "Workflow instance name {reference} is ambiguous; use its id."
        )),
    }
}

fn resolve_target(
    app_data_dir: &Path,
    requested_worktree: &Path,
) -> Result<ResolvedRepoBranchWorktreeTarget, String> {
    let requested = fs::canonicalize(requested_worktree).map_err(|error| {
        format!(
            "Unable to resolve worktree {}: {error}",
            requested_worktree.display()
        )
    })?;
    let source =
        DiscoveredWorktreeTargetSource::new(app_data_dir.join("codex-orchestrator.sqlite"));
    let targets = source.list()?;
    let target = targets
        .into_iter()
        .find(|target| {
            fs::canonicalize(&target.worktree.path)
                .map(|path| path == requested)
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            format!(
                "Worktree {} is not a currently discovered temporary Workflow target.",
                requested.display()
            )
        })?;
    Ok(ResolvedRepoBranchWorktreeTarget {
        repository: WorkflowRepositoryTarget {
            id: target.repository.id,
            name: target.repository.name,
            git_common_directory: target.repository.git_common_directory,
        },
        branch: WorkflowBranchTarget {
            id: target.branch.id,
            name: target.branch.name,
        },
        worktree: WorkflowWorktreeTarget {
            id: target.worktree.id,
            path: target.worktree.path,
        },
    })
}

fn send_to_node(
    context: &CliRuntimeContext,
    instance: &super::instance_domain::WorkflowInstance,
    node_id: &str,
    message: String,
) -> Result<SendAgentSessionMessageResult, String> {
    let mut existing = instance
        .sessions
        .iter()
        .filter(|session| session.node_id == node_id)
        .collect::<Vec<_>>();
    existing.sort_by(|left, right| right.associated_at.cmp(&left.associated_at));
    if let Some(session) = existing.first() {
        let session_id =
            AgentSessionId::new(session.session_id.clone()).map_err(|error| error.to_string())?;
        return context
            .sessions
            .send_message(SendAgentSessionMessageCommand {
                session_id: Some(session_id),
                submitted_text: message,
                title: None,
                working_directory: Some(instance.target.worktree.path.clone()),
                requested_options: None,
            })
            .map_err(|error| error.to_string());
    }
    let node = instance
        .recipe
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| format!("Workflow node {node_id} was not found."))?;
    if !node.is_starting_point {
        return Err("Only the starting node can receive the first human message.".to_string());
    }
    context.workflow.send_workflow_node_message(
        &instance.summary.id,
        node_id,
        message,
        Some(format!("{} · {}", instance.summary.name, node.name)),
    )
}

fn wait_until_settled(
    context: &CliRuntimeContext,
    instance_id: &str,
    source: &SendAgentSessionMessageResult,
    timeout: Duration,
) -> Result<Value, String> {
    let started = Instant::now();
    let mut idle_since = None;
    loop {
        if started.elapsed() >= timeout {
            return Err(format!(
                "Workflow instance did not settle within {} seconds.",
                timeout.as_secs()
            ));
        }
        let source_history = context
            .sessions
            .load_session(&source.session_id)
            .map_err(|error| error.to_string())?;
        let source_status = source_history
            .invocations
            .iter()
            .find(|entry| entry.invocation.id == source.invocation_id)
            .map(|entry| entry.invocation.status);
        let instance = context.workflow.load_workflow_instance(instance_id)?;
        let source_terminal = source_status.is_some_and(AgentInvocationStatus::is_terminal);
        if source_terminal && instance.summary.active_session_count == 0 {
            let since = idle_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(2) {
                return instance_report(context, instance_id);
            }
        } else {
            idle_since = None;
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn instance_report(context: &CliRuntimeContext, instance_id: &str) -> Result<Value, String> {
    let instance = context.workflow.load_workflow_instance(instance_id)?;
    let mut sessions = Vec::new();
    for association in &instance.sessions {
        let session_id = AgentSessionId::new(association.session_id.clone())
            .map_err(|error| error.to_string())?;
        let history = context
            .sessions
            .load_session(&session_id)
            .map_err(|error| error.to_string())?;
        sessions.push(json!({
            "nodeId": association.node_id,
            "sessionId": association.session_id,
            "title": association.title,
            "invocations": history.invocations.iter().map(|entry| json!({
                "id": entry.invocation.id.as_str(),
                "status": entry.invocation.status,
                "submittedText": entry.invocation.submitted_text,
                "runtimeError": entry.invocation.runtime_error.as_ref().map(|error| &error.message),
                "finalResponse": entry.events.iter().rev().find_map(|event| {
                    let normalized = event.normalized.as_ref()?;
                    (normalized.kind == NormalizedRuntimeEventKind::AgentMessage
                        && normalized.details.as_ref()
                            .and_then(|details| details.get("role"))
                            .and_then(Value::as_str) == Some("final"))
                        .then(|| normalized.text.clone())
                        .flatten()
                }),
            })).collect::<Vec<_>>(),
        }));
    }
    let failed_activations = failed_activation_details(&context.database_path, instance_id)?;
    Ok(json!({
        "instanceId": instance.summary.id,
        "instanceName": instance.summary.name,
        "workflowTypeId": instance.summary.workflow_type_id,
        "workflowTypeName": instance.summary.workflow_type_name,
        "recipeId": instance.summary.recipe_id,
        "target": instance.target,
        "sessions": sessions,
        "connectionActivations": instance.connection_activations,
        "failedConnectionActivations": failed_activations,
    }))
}

fn failed_activation_details(
    database_path: &Path,
    instance_id: &str,
) -> Result<Vec<Value>, String> {
    let connection = rusqlite::Connection::open(database_path)
        .map_err(|error| format!("Unable to open Workflow activation storage: {error}"))?;
    storage::configure_sqlite_connection(&connection)
        .map_err(|error| format!("Unable to configure Workflow activation storage: {error}"))?;
    let mut statement = connection
        .prepare(
            "SELECT id,connection_id,failure_stage,failure_reason,failed_at
             FROM workflow_connection_activations
             WHERE workflow_instance_id=?1 AND failed_at IS NOT NULL
             ORDER BY requested_at DESC,id DESC",
        )
        .map_err(|error| format!("Unable to prepare failed Workflow activation query: {error}"))?;
    let details = statement
        .query_map([instance_id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "connectionId": row.get::<_, String>(1)?,
                "failureStage": row.get::<_, Option<String>>(2)?,
                "failureReason": row.get::<_, Option<String>>(3)?,
                "failedAt": row.get::<_, String>(4)?,
            }))
        })
        .map_err(|error| format!("Unable to query failed Workflow activations: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Unable to read failed Workflow activations: {error}"))?;
    Ok(details)
}

fn print_json(value: Value) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(&value)
            .map_err(|error| format!("Unable to serialize Workflow CLI output: {error}"))?
    );
    Ok(())
}

fn usage() -> String {
    [
        "Workflow CLI usage:",
        "  workflow-cli import --app-data-dir <absolute-path> --file <absolute-json-path>",
        "  workflow-cli instantiate --app-data-dir <absolute-path> --workflow <id-or-name> --name <instance-name> --worktree <absolute-path>",
        "  workflow-cli send --app-data-dir <absolute-path> --instance <id-or-name> --node <id-or-name> --message <text> [--wait-seconds <seconds>]",
        "  workflow-cli show --app-data-dir <absolute-path> --instance <id-or-name>",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn parses_strict_flag_value_arguments() {
        let options = parse_options(&[
            "--file".to_string(),
            "C:/catalog.json".to_string(),
            "--app-data-dir".to_string(),
            "C:/data".to_string(),
        ])
        .unwrap();
        assert_eq!(options["file"], "C:/catalog.json");
        assert!(parse_options(&["file".to_string()]).is_err());
        assert!(parse_options(&["--file".to_string()]).is_err());
    }

    #[test]
    fn installs_and_activates_a_catalog_workflow_through_the_repository_boundary() {
        let repository =
            SqliteWorkflowRepository::new(Connection::open_in_memory().unwrap()).unwrap();
        let workflow = WorkflowBlueprint {
            name: "Example".to_string(),
            nodes: vec![serde_json::from_value(json!({
                "id": "example-start",
                "name": "Start",
                "harnessName": "Example harness",
                "roleName": null,
                "positionX": 100.0,
                "positionY": 100.0,
                "isStartingPoint": true,
                "harness": {
                    "kind": "standalone",
                    "config": {
                        "harnessName": "Example harness",
                        "instructions": "Finish the task.",
                        "runtime": {
                            "provider": "codex",
                            "model": "gpt-5.6-terra",
                            "reasoningEffort": "low"
                        }
                    }
                }
            }))
            .unwrap()],
            connections: vec![],
        };

        let installed = install_workflow(&repository, workflow).unwrap();

        assert!(installed.workflow_type.active_recipe_id.is_some());
        assert_eq!(installed.active_recipe.unwrap().nodes.len(), 1);
    }

    #[test]
    fn catalog_validation_rejects_element_ids_reused_across_workflows() {
        let node = |name: &str| WorkflowNodeConfig {
            id: "shared".to_string(),
            name: name.to_string(),
            harness_name: "Harness".to_string(),
            role_name: None,
            position_x: 0.0,
            position_y: 0.0,
            is_starting_point: true,
            harness: None,
        };
        let catalog = WorkflowCatalog {
            contract_version: CATALOG_CONTRACT.to_string(),
            workflows: vec![
                WorkflowBlueprint {
                    name: "One".to_string(),
                    nodes: vec![node("One")],
                    connections: vec![],
                },
                WorkflowBlueprint {
                    name: "Two".to_string(),
                    nodes: vec![node("Two")],
                    connections: vec![],
                },
            ],
        };
        assert!(validate_catalog(&catalog).is_err());
    }
}
