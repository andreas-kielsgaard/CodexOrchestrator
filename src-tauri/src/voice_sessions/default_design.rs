use super::VoiceSessionApplication;
use crate::workflows::authoring::*;
use serde_json::json;

impl VoiceSessionApplication {
    pub(super) fn ensure_default_design(&self) -> Result<Option<String>, String> {
        let state = self.snapshot()?;
        if let Some(id) = state.default_design_id {
            return Ok(Some(id));
        }
        let recipes = self.workflows.authoring.list()?;
        if let Some(recipe) = recipes.iter().find(|r| r.name == "Voice") {
            self.update(|s| {
                s.default_design_id = Some(recipe.recipe_id.clone());
                Ok(())
            })?;
            return Ok(Some(recipe.recipe_id.clone()));
        }
        let settings = self.snapshot()?.settings;
        let base = if settings.profile_id.is_empty() {
            self.profiles.default_profile()
        } else {
            self.profiles.read(&settings.profile_id)
        };
        let Ok(base) = base else {
            return Ok(None);
        };
        let mut allowed = base.allowed_capabilities.clone();
        allowed.mcp_tools.remove("voice_session");
        allowed.mcp_tools.extend(crate::orchid_control::tool_map());
        allowed.mcp_tools.insert(
            "voice".into(),
            [
                "read_context",
                "report_progress",
                "request_output",
                "route_input",
                "delegate_work",
                "report_outcome",
                "observe_sessions",
                "acknowledge",
                "publish",
            ]
            .map(str::to_owned)
            .into(),
        );
        let mut routes = base.route_policies.clone();
        for route in &mut routes {
            route.mcp_groups.remove("otp:voice_session:mcps");
            route.mcp_groups.insert("otp:voice:mcps".into());
        }
        let profile = match self.profiles.read("voice-workflow") {
            Ok(profile) => profile,
            Err(_) => self
                .profiles
                .create_with_routes(
                    "voice-workflow".into(),
                    "Voice workflow".into(),
                    allowed,
                    base.defaults.clone(),
                    base.execution.clone(),
                    routes,
                    base.default_route_id.clone(),
                )
                .map_err(|e| e.to_string())?,
        };
        let mut draft = self.workflows.authoring.create()?.draft;
        draft.name = "Voice".into();
        draft.entry_node_ids = vec!["router".into()];
        let prompts = settings.prompts;
        for (i,(id,prompt,tools)) in [
            ("summary",prompts.summary.as_str(),vec!["read_context","acknowledge"]),
            ("router",prompts.router.as_str(),vec!["read_context","delegate_work","observe_sessions","report_progress"]),
            ("digest",prompts.digest.as_str(),vec!["read_context","request_output"]),
            ("output",prompts.output.as_str(),vec!["read_context","publish","route_input"]),
            ("worker","Perform the assignment. Preserve its task and revision identity. Report completion using voice.report_outcome and the supplied assignmentId and revision.",vec!["read_context","delegate_work","report_progress","report_outcome"]),
        ].into_iter().enumerate() {
            let mut capabilities = profile.allowed_capabilities.clone();
            capabilities.mcp_tools.insert("voice".into(),tools.into_iter().map(str::to_owned).collect());
            let mut defaults = profile.defaults.clone();
            let model = if id == "router" || id == "worker" {&settings.models.routing} else {&settings.models.response};
            if capabilities.models.contains(&model.model) { defaults.model = Some(model.model.clone()); }
            if capabilities.reasoning_modes.contains(&model.reasoning_mode) { defaults.reasoning_mode = Some(model.reasoning_mode.clone()); }
            draft.nodes.push(WorkflowAuthoringNode {node_id:id.into(),name:id.into(),position_x:600.0+(i%3) as f64*300.0,position_y:(i/3) as f64*250.0,
                binding:WorkflowNodeBinding::Agent(WorkflowAgentNode {capability_profile_id:profile.capability_profile_id.clone(),node_profile:crate::execution_configuration::NodeProfile {contract_version:1,allowed_capabilities:capabilities,pinned_defaults:defaults}, initial_prompt:Some(format!("{}\n{}",if id=="summary" {prompt.replace("voice_session","voice").replace("voice.publish","voice.acknowledge")} else {prompt.replace("voice_session","voice")},settings.instructions)),agent_identity_id:None,notifications:crate::agent_sessions::notifications::NodeNotifications {turn_completion_sound:settings.workflow.notification_sounds.get(id).copied().unwrap_or(false)}})});
        }
        for (i, (id, configuration)) in [
            (
                "transcript",
                serde_json::to_value(&self.snapshot()?.input_policy).map_err(|e| e.to_string())?,
            ),
            ("evaluation", json!({"prompts":prompts})),
            (
                "buffer",
                json!({"directReactions":settings.workflow.direct_reactions}),
            ),
            ("speech", json!({})),
            ("external_work", json!({})),
        ]
        .into_iter()
        .enumerate()
        {
            draft.nodes.push(WorkflowAuthoringNode {
                node_id: id.into(),
                name: id.replace('_', " "),
                position_x: 0.0,
                position_y: i as f64 * 180.0,
                binding: WorkflowNodeBinding::Capability(WorkflowCapabilityNode {
                    capability: orchid_core::otp_packages::voice::reference(id),
                    configuration,
                }),
            });
        }
        let mut connect = |source: &str,
                           destination: &str,
                           package: &str,
                           tool: &str,
                           output: &str,
                           action_package: &str,
                           action: &str,
                           field: Option<&str>| {
            draft.connections.push(WorkflowAuthoringConnection {connection_id:format!("{source}-{tool}-{destination}"),name:format!("{source} to {destination}"),source_node_id:source.into(),destination_node_id:destination.into(),trigger:crate::otp_api::OutputRef {capability:crate::otp_api::CapabilityRef {package:package.into(),tool:tool.into()},output:output.into()},action:crate::otp_api::CapabilityRef{package:action_package.into(),tool:action.into()},configuration:if action=="prompt_agent" {json!({"deliveryMode":"steer_active_otherwise_queue"})} else {json!({})},prompt_inputs:field.into_iter().map(|field|crate::workflows::compiled_plan::WorkflowConnectionPromptInput::OutputField {field:field.into()}).collect(),prompt_text:String::new()});
        };
        connect(
            "transcript",
            "evaluation",
            "voice",
            "transcript",
            "window",
            "voice",
            "evaluate",
            None,
        );
        connect(
            "evaluation",
            "buffer",
            "voice",
            "evaluate",
            "evaluated",
            "voice",
            "accept_evaluation",
            None,
        );
        for destination in ["summary", "router", "digest"] {
            connect(
                "buffer",
                destination,
                "voice",
                "accept_evaluation",
                "work_ready",
                "workflow",
                "prompt_agent",
                Some("data"),
            );
        }
        connect(
            "buffer",
            "output",
            "voice",
            "accept_evaluation",
            "reaction_ready",
            "workflow",
            "prompt_agent",
            Some("data"),
        );
        connect("summary","router","workflow","on_invocation_completed","completed","workflow","prompt_agent",Some("output"));
        for source in ["summary", "router", "worker"] {
            connect(
                source,
                "digest",
                "workflow",
                "on_invocation_completed",
                "completed",
                "workflow",
                "prompt_agent",
                Some("output"),
            );
        }
        for source in ["router", "worker"] {
            connect(
                source,
                "worker",
                "voice",
                "delegate_work",
                "requested",
                "voice",
                "assign_work",
                Some("data"),
            );
            connect(
                source,
                "digest",
                "voice",
                "report_progress",
                "progress",
                "workflow",
                "prompt_agent",
                Some("data"),
            );
        }
        connect(
            "worker",
            "router",
            "voice",
            "report_outcome",
            "outcome",
            "voice",
            "return_to_requester",
            Some("data"),
        );
        connect(
            "worker",
            "worker",
            "voice",
            "report_outcome",
            "outcome",
            "voice",
            "return_to_requester",
            Some("data"),
        );
        connect(
            "worker",
            "digest",
            "voice",
            "report_outcome",
            "outcome",
            "workflow",
            "prompt_agent",
            Some("data"),
        );
        connect(
            "digest",
            "output",
            "voice",
            "request_output",
            "requested",
            "workflow",
            "prompt_agent",
            Some("data"),
        );
        connect(
            "output",
            "router",
            "voice",
            "route_input",
            "rerouted",
            "workflow",
            "prompt_agent",
            Some("data"),
        );
        connect(
            "output",
            "speech",
            "voice",
            "publish",
            "requested",
            "voice",
            "speak",
            None,
        );
        connect(
            "summary",
            "speech",
            "voice",
            "acknowledge",
            "requested",
            "voice",
            "speak",
            None,
        );
        connect(
            "speech",
            "digest",
            "voice",
            "speak",
            "published",
            "workflow",
            "prompt_agent",
            Some("data"),
        );
        connect(
            "external_work",
            "digest",
            "voice",
            "observe",
            "observed",
            "workflow",
            "prompt_agent",
            Some("data"),
        );
        let saved = self.workflows.authoring.save_draft(draft)?;
        self.update(|s| {
            s.default_design_id = Some(saved.draft.recipe_id.clone());
            Ok(())
        })?;
        Ok(Some(saved.draft.recipe_id))
    }
}
