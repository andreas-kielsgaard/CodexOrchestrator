//! Manual live exercise: real Codex subprocesses, MCP transport, routing and file events.
//! The proxy listener runs in-process; this does not automate the native desktop window.
use super::*;
use crate::agent_sessions::application::SystemAgentSessionProviders;
use crate::workflows::{compiled_plan::FileAssociation, file_inputs::resolve_node_files};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct LiveRun {
    root: PathBuf,
    repository: Arc<SqliteAgentSessionRepository>,
    sessions: Arc<AgentSessionApplication>,
    engine: Arc<HarnessEngineService>,
    execution: Arc<WorkflowExecutionService>,
    instance: RecipeInstance,
}

impl LiveRun {
    fn open(root: PathBuf) -> Self {
        assert!(
            !root.join("live.sqlite").exists(),
            "Use a fresh exercise directory"
        );
        let workspace = root.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        assert!(std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&workspace)
            .status()
            .unwrap()
            .success());
        let database = root.join("live.sqlite");
        let repository = Arc::new(SqliteAgentSessionRepository::open(&database).unwrap());
        let registry = Arc::new(ManagedMcpUpstreamRegistry::default());
        let engine = HarnessEngineService::new(
            Arc::new(SqliteHarnessBindingRepository::open(&database).unwrap()),
            Arc::new(LocalProxy::new()),
            registry.clone(),
        )
        .unwrap();
        let providers = Arc::new(SystemAgentSessionProviders);
        let codex_program =
            std::env::var("WORKFLOW_LIVE_CODEX_PROGRAM").unwrap_or_else(|_| "codex".into());
        let runtime = Arc::new(crate::runtime::codex::CodexCliRuntime::system(
            codex_program,
            None,
        ));
        let notifier = Arc::new(WorkflowNotifier {
            recording: RecordingNotifier::new(repository.clone()),
            execution: Mutex::new(None),
            errors: Mutex::new(Vec::new()),
        });
        let sessions = Arc::new(
            AgentSessionApplication::new(
                repository.clone(),
                runtime,
                notifier.clone(),
                providers.clone(),
                providers,
                None,
            )
            .with_session_harness_launch_authority(Arc::new(
                SessionProfileHarnessAuthority {
                    engine: engine.clone(),
                    sessions: repository.clone(),
                },
            )),
        );
        let capabilities = CapabilitySet {
            sandbox_modes: [ExecutionSandboxMode::WorkspaceWrite].into_iter().collect(),
            mcp_tools: [(
                "workflow".into(),
                ["trigger_workflow_continuation".into()]
                    .into_iter()
                    .collect(),
            )]
            .into_iter()
            .collect(),
            ..CapabilitySet::default()
        };
        let source = Arc::new(FixedSelectedRuntimeProfileSource(RuntimeProfileSnapshot {
            contract_version: 1,
            profile_ref: "live:installed-codex-defaults".into(),
            exposure: capabilities.clone(),
            locked: RuntimeSelections {
                sandbox_mode: Some(ExecutionSandboxMode::WorkspaceWrite),
                ..RuntimeSelections::default()
            },
        }));
        let profiles = Arc::new(CapabilityProfileService::new(
            Arc::new(SqliteCapabilityProfileRepository::open(&database).unwrap()),
            source.clone(),
        ));
        profiles
            .create(
                "live".into(),
                "Live continuation".into(),
                capabilities.clone(),
            )
            .unwrap();
        let adapter = Arc::new(
            AgentSessionEventAdapter::open(
                &database,
                sessions.clone(),
                repository.clone(),
                source,
                IdentityService::open(&database).unwrap(),
            )
            .unwrap()
            .with_capability_profiles(profiles.clone()),
        );
        let events = Arc::new(SessionEventApplication::new(
            adapter.clone(),
            adapter.clone(),
            Arc::new(SqliteSessionEventStore::open(&database).unwrap()),
        ));
        let authoring = Arc::new(WorkflowAuthoringService::new(
            Arc::new(SqliteWorkflowAuthoringRepository::open(&database).unwrap()),
            profiles,
            crate::otp_host::OtpRegistry::import(&["workflow"]).unwrap(),
        ));
        let execution = Arc::new(WorkflowExecutionService::new(
            authoring.clone(),
            events,
            Arc::new(WorkflowInstanceStore::open(&database).unwrap()),
            adapter,
            repository.clone(),
        ));
        *notifier.execution.lock().unwrap() = Some(Arc::downgrade(&execution));
        let (mut descriptors, owner) = crate::otp_host::mcp::start_server(
            execution.registry.clone(),
            Arc::downgrade(&execution),
        )
        .unwrap();
        let registration = registry.register(descriptors.remove(0)).unwrap();
        assert!(registry.retain_owner(&registration, owner).is_ok());
        let mut draft = authoring
            .create("Live continuation exercise".into())
            .unwrap()
            .draft;
        draft.starting_node_id = Some("discussion".into());
        let base="This is a small live Workflow exercise in a disposable workspace. Only work on the named test documents. Use apply_patch for every document creation or edit so the Harness reports file changes. Do not spawn other agents. Keep final replies brief.";
        draft.nodes=[
            ("discussion","Discussion","On the first request, discuss briefly and reply READY_FOR_APPROVAL. Do not write files or call a workflow tool until the user explicitly approves. After approval, follow the user's requested file creation and continuation."),
            ("overview","Overview","Read spec.md and the delivered JSON metadata. Verify that spec.md was reported as created by node discussion. Append OVERVIEW_REVIEWED to spec.md using apply_patch and create overview.md containing: Implement the approved greeting. Do not trigger continuation. Reply OVERVIEW_DONE."),
            ("clarification","Clarification","On initial activation create decisions.md containing: greeting=hello. Do not trigger continuation yet. When a later user explicitly requests revision, update the decisions and call trigger_workflow_continuation with empty arguments exactly once."),
            ("revision","Revision","Read the supplied JSON metadata and spec.md, overview.md, decisions.md. Check that these files are associated with Discussion, Overview and Clarification. Append REVISION_REVIEWED to spec.md. Create revision.md summarizing the current greeting decision and the supplied creator/editor metadata. Then call trigger_workflow_continuation with empty arguments once; you have no outgoing connections, so zero deliveries is valid. Reply REVISION_DONE.")
        ].into_iter().enumerate().map(|(i,(id,name,prompt))|WorkflowAuthoringNode {
            node_id:id.into(),name:name.into(),position_x:i as f64*200.0,position_y:50.0,capability_profile_id:"live".into(),
            node_profile:crate::execution_configuration::NodeProfile {contract_version:1,allowed_capabilities:capabilities.clone(),pinned_defaults:RuntimeSelections::default()},
            initial_prompt:Some(format!("{base}\n{prompt}")),agent_identity_id:None
        }).collect();
        let trigger = otp_output("trigger_workflow_continuation", "continuation");
        let files = |id: &str| WorkflowConnectionPromptInput::NodeFiles {
            node_id: id.into(),
            association: FileAssociation::Either,
        };
        draft.connections = [
            (
                "to-overview",
                "discussion",
                "overview",
                vec![files("discussion")],
            ),
            (
                "to-clarification",
                "discussion",
                "clarification",
                vec![files("discussion")],
            ),
            (
                "to-revision",
                "clarification",
                "revision",
                vec![
                    files("discussion"),
                    files("overview"),
                    files("clarification"),
                ],
            ),
        ]
        .into_iter()
        .map(|(id, from, to, mut inputs)| {
            inputs.insert(
                0,
                WorkflowConnectionPromptInput::OutputField {
                    field: "sourceNode".into(),
                },
            );
            inputs.insert(
                1,
                WorkflowConnectionPromptInput::OutputField {
                    field: "outputFiles".into(),
                },
            );
            WorkflowAuthoringConnection {
                connection_id: id.into(),
                name: id.into(),
                source_node_id: from.into(),
                destination_node_id: to.into(),
                trigger: trigger.clone(),
                prompt_inputs: inputs,
                prompt_text: "Use the supplied file metadata and follow your node instructions."
                    .into(),
                action: crate::otp_api::CapabilityRef {
                    package: "workflow".into(),
                    tool: "prompt_agent".into(),
                },
                configuration: json!({}),
            }
        })
        .collect();
        let draft = authoring.save_draft(draft).unwrap().draft;
        authoring
            .activate_revision(&draft.recipe_id, draft.revision)
            .unwrap();
        let instance = execution
            .create_instance(
                &draft.recipe_id,
                draft.revision,
                "Live exercise".into(),
                ResolvedRepoBranchWorktreeTarget {
                    repository: WorkflowRepositoryTarget {
                        id: "exercise".into(),
                        name: "Exercise".into(),
                        git_common_directory: workspace.join(".git").to_string_lossy().into_owned(),
                    },
                    branch: WorkflowBranchTarget {
                        id: "exercise".into(),
                        name: "exercise".into(),
                    },
                    worktree: WorkflowWorktreeTarget {
                        id: "exercise".into(),
                        path: workspace.to_string_lossy().into_owned(),
                    },
                },
            )
            .unwrap();
        std::fs::write(
            root.join("recipe.json"),
            serde_json::to_string_pretty(&instance).unwrap(),
        )
        .unwrap();
        Self {
            root,
            repository,
            sessions,
            engine,
            execution,
            instance,
        }
    }

    fn send(&self, node: &str, text: &str) {
        let result = self
            .execution
            .dispatch_node_user_request(
                &self.instance.recipe.recipe_id,
                &self.instance.id,
                Some(node),
                text.into(),
            )
            .unwrap();
        assert!(
            result.deliveries.iter().all(|d| matches!(
                d.outcome,
                crate::session_events::DeliveryOutcome::Dispatched { .. }
            )),
            "{result:?}"
        );
    }

    fn histories(&self) -> Vec<crate::agent_sessions::ports::AgentSessionHistory> {
        self.repository
            .list_sessions(ListAgentSessionsQuery::default())
            .unwrap()
            .iter()
            .map(|s| {
                self.repository
                    .load_session_history(&s.id)
                    .unwrap()
                    .unwrap()
            })
            .collect()
    }

    fn wait(&self, count: usize) {
        let deadline = Instant::now() + Duration::from_secs(300);
        let mut last = String::new();
        loop {
            let histories = self.histories();
            let summary = histories
                .iter()
                .map(|h| {
                    format!(
                        "{}:{:?}",
                        h.session.title,
                        h.invocations.last().map(|i| i.invocation.status)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            if summary != last {
                println!("LIVE {summary}");
                last = summary.clone();
                self.capture("progress");
            }
            for history in &histories {
                for invocation in &history.invocations {
                    if invocation.invocation.status.is_terminal() {
                        assert_eq!(
                            invocation.invocation.status,
                            AgentInvocationStatus::Completed,
                            "Runtime failed: {:?}",
                            invocation.invocation
                        );
                    }
                }
            }
            if histories.len() == count
                && histories.iter().all(|h| {
                    h.invocations
                        .last()
                        .is_some_and(|i| i.invocation.status == AgentInvocationStatus::Completed)
                })
            {
                self.capture("terminal");
                return;
            }
            if Instant::now() > deadline {
                self.capture("timeout");
                panic!("Live exercise timed out: {summary}");
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    fn capture(&self, phase: &str) {
        std::fs::write(
            self.root.join("catalogue.json"),
            serde_json::to_string_pretty(&self.execution.registry.catalogue()).unwrap(),
        )
        .unwrap();
        std::fs::write(
            self.root.join("node-sessions.json"),
            serde_json::to_string_pretty(
                &self.execution.instance_sessions(&self.instance).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let histories = self.histories();
        let sessions:Vec<_>=histories.iter().map(|history|json!({
            "sessionId":history.session.id.as_str(),"title":history.session.title,
            "invocations":history.invocations.iter().map(|invocation|json!({
                "invocationId":invocation.invocation.id.as_str(),"status":format!("{:?}",invocation.invocation.status),
                "submittedText":invocation.invocation.submitted_text,
                "events":invocation.events.iter().map(|event|json!({"sequence":event.sequence,"raw":event.raw_payload,"normalized":event.normalized})).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        })).collect();
        std::fs::write(
            self.root.join("sessions.json"),
            serde_json::to_string_pretty(&json!({"phase":phase,"sessions":sessions})).unwrap(),
        )
        .unwrap();
        let history = self
            .repository
            .file_history_at_scope(
                &ReferenceIdentity::new("workflow", "instance", &self.instance.id).unwrap(),
            )
            .unwrap();
        std::fs::write(
            self.root.join("history.json"),
            serde_json::to_string_pretty(&history.iter().map(|change|json!({
                "address":change.address,"sessionId":change.session_id,"invocationId":change.invocation_id,
                "path":change.path,"operation":change.operation,"recordedAt":change.recorded_at
            })).collect::<Vec<_>>()).unwrap(),
        )
        .unwrap();
        std::fs::write(
            self.root.join("attempts.json"),
            serde_json::to_string_pretty(
                &self
                    .execution
                    .instances
                    .attempts(&self.instance.id)
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
impl Drop for LiveRun {
    fn drop(&mut self) {
        let _ = self.sessions.shutdown_runtime();
        let _ = self.engine.shutdown();
    }
}

#[test]
#[ignore = "launches real Codex agents; requires WORKFLOW_LIVE_EXERCISE_ROOT"]
fn real_codex_continuation_exercise() {
    let root = PathBuf::from(
        std::env::var("WORKFLOW_LIVE_EXERCISE_ROOT")
            .expect("Explicit live-exercise directory required"),
    );
    let live = LiveRun::open(root);
    println!("LIVE evidence {}", live.root.display());
    live.send(
        "discussion",
        "We want a one-line greeting. Discuss the task; wait for approval.",
    );
    live.wait(1);
    assert!(!live.root.join("workspace/spec.md").exists());
    println!("LIVE approval instruction respected: no file or downstream session before approval");
    live.send("discussion","Approved. Use apply_patch to create spec.md with the exact text: The program prints hello. Then call trigger_workflow_continuation once with outputFiles [\"spec.md\", \"claimed-only.md\"]. Do not create claimed-only.md. Reply DISCUSSION_DONE.");
    live.wait(3);
    assert!(live.root.join("workspace/overview.md").is_file());
    assert!(live.root.join("workspace/decisions.md").is_file());
    assert!(!live.root.join("workspace/claimed-only.md").exists());
    live.send("clarification","The user now requests revision: change greeting=hello to greeting=welcome in decisions.md using apply_patch, then call trigger_workflow_continuation with empty arguments.");
    live.wait(4);
    assert!(live.root.join("workspace/revision.md").is_file());
    let spec = std::fs::read_to_string(live.root.join("workspace/spec.md")).unwrap();
    assert!(
        spec.contains("OVERVIEW_REVIEWED") && spec.contains("REVISION_REVIEWED"),
        "{spec}"
    );
    let reopened = SqliteAgentSessionRepository::open(live.root.join("live.sqlite")).unwrap();
    let selected: serde_json::Value = serde_json::from_str(
        &resolve_node_files(
            &live.instance,
            "overview",
            FileAssociation::Edited,
            &reopened,
        )
        .unwrap(),
    )
    .unwrap();
    let spec = selected["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"] == "spec.md")
        .unwrap();
    assert_eq!(spec["created"]["nodeId"], "discussion");
    assert_eq!(spec["lastEdited"]["nodeId"], "revision");
    let authored = resolve_node_files(
        &live.instance,
        "discussion",
        FileAssociation::Created,
        &reopened,
    )
    .unwrap();
    assert!(!authored.contains("claimed-only.md"));
    std::fs::write(
        live.root.join("historical-editor-selection.json"),
        serde_json::to_string_pretty(&selected).unwrap(),
    )
    .unwrap();
    live.capture("passed");
    // Exercise the package's deliberate fresh-session mode against an existing population.
    use crate::otp_api::*;
    let mut context = InvocationContext {
        instance_id: live.instance.id.clone(),
        occurrence_id: "live-fresh-session".into(),
        capability: live.instance.recipe.entry_action.clone(),
        source: None,
        connection_id: None,
        output_node_id: Some("discussion".into()),
    };
    let run = |context: &InvocationContext, configuration| {
        live.execution.dispatch_otp_action(&live.instance,context,None,json!({}),configuration,Ok(vec![ResolvedInput{reference:context.occurrence_id.clone(),value:json!("Discuss this additional session briefly. Reply READY_FOR_APPROVAL. Do not create files or call a workflow tool.")}])).unwrap()
    };
    let fresh = run(&context, json!({"mode":"new"}));
    let created = fresh[0].group.created_session.clone().unwrap();
    live.wait(5);
    context.occurrence_id = "live-exact-session".into();
    let selected = run(&context, json!({}));
    assert_eq!(selected[0].deliveries[0].target_session, created);
    assert!(selected[0].group.created_session.is_none());
    assert!(selected[0].deliveries[0]
        .included_created_session_contributions
        .is_empty());
    live.wait(5);
    let history = live
        .sessions
        .load_session(&AgentSessionId::new(created.id()).unwrap())
        .unwrap();
    assert_eq!(history.invocations.len(), 2);
    let store = SqliteSessionEventStore::open(&live.root.join("live.sqlite")).unwrap();
    let mut deliveries = Vec::new();
    for attempt in live
        .execution
        .instances
        .attempts(&live.instance.id)
        .unwrap()
    {
        assert!(attempt.error.is_none(), "{attempt:?}");
        for group in attempt.event_groups {
            deliveries.extend(store.deliveries_for_group(&group).unwrap());
        }
    }
    std::fs::write(
        live.root.join("deliveries.json"),
        serde_json::to_string_pretty(&deliveries).unwrap(),
    )
    .unwrap();
    live.capture("passed-with-fresh-and-exact");
    println!("LIVE PASS: approval, MCP fan-out, later revision, historical editor retention, reopen and output-path separation");
    println!("LIVE PASS: new Session with existing candidates, exact Session follow-up and initialization once");
}
