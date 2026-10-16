use crate::{
    agent_sessions::{
        application::{
            AgentSessionApplication, CancelAgentInvocationCommand, RespondToRuntimeRequestCommand,
            SteerAgentSessionCommand,
        },
        domain::{AgentInvocationId, AgentInvocationInputProvenance, AgentSessionId},
        ports::{InvocationContent, ListAgentSessionsQuery},
    },
    execution_configuration::CapabilityProfileService,
    repository_catalog::RepositoryCatalog,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub struct OrchidControl {
    pub agents: Arc<AgentSessionApplication>,
    pub profiles: Arc<CapabilityProfileService>,
    pub repositories: Arc<RepositoryCatalog>,
}
pub fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{key} is required"))
}
pub fn optional(args: &Value, key: &str) -> Option<String> {
    args[key].as_str().map(str::to_owned)
}
pub fn tool_map() -> BTreeMap<String, BTreeSet<String>> {
    [(
        "orchid_control",
        [
            "list_sessions",
            "read_session",
            "list_workspaces",
            "list_devices",
            "list_targets",
            "list_profiles",
            "start_session",
            "send_work",
            "steer_session",
            "cancel_invocation",
            "respond_to_request",
        ]
        .as_slice(),
    )]
    .into_iter()
    .map(|(server, tools)| (server.into(), tools.iter().map(|s| s.to_string()).collect()))
    .collect()
}
impl OrchidControl {
    pub fn invoke(&self, tool: &str, args: Value) -> Result<Value, String> {
        if tool == "list_devices" {
            return Ok(json!(self
                .agents
                .execution_targets()?
                .attached_devices()?));
        }
        if tool == "list_targets" {
            let scope = match optional(&args, "deviceId") {
                Some(device_id) => {
                    crate::execution_targets::domain::WorktreeChoiceScope::Device { device_id }
                }
                None => crate::execution_targets::domain::WorktreeChoiceScope::Local,
            };
            return Ok(json!(self
                .agents
                .execution_targets()?
                .attached_worktree_choices(&scope)?));
        }
        if tool == "list_workspaces" {
            if let Some(device) = optional(&args, "deviceId").filter(|d| d != "local") {
                return self.agents.execution_targets()?.endpoints.host_command(
                    &device,
                    crate::agent_sessions::client::HostCommand {
                        method: "device.repositories".into(),
                        input: json!({}),
                    },
                );
            }
        }
        if tool == "start_session" && args.get("executionSelection").is_some() {
            let receipt = self
                .agents
                .accept_prepared_product(
                    serde_json::from_value(json!({
                        "submissionId": required(&args,"submissionId")?,
                        "submittedText": required(&args,"task")?,
                        "title": optional(&args,"title"),
                        "executionSelection": args["executionSelection"],
                    }))
                    .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            return Ok(
                json!({"sessionId":receipt.session_id,"invocationId":receipt.invocation_id}),
            );
        }
        let session = || {
            AgentSessionId::new(required(&args, "sessionId")?.to_string())
                .map_err(|e| e.to_string())
        };
        match tool {
            "list_sessions"=>Ok(json!(self.agents.list_sessions(ListAgentSessionsQuery{limit:Some(args["limit"].as_u64().unwrap_or(50).min(200) as u32),availability:None}).map_err(|e|e.to_string())?.iter().map(|s|json!({"session":s.session,"latestStatus":s.latest_invocation_status,"pendingRequestCount":s.pending_request_count})).collect::<Vec<_>>())),
            "read_session"=>{let history=self.agents.load_session(&session()?).map_err(|e|e.to_string())?;
                let invocations=history.invocations.iter().rev().take(args["limit"].as_u64().unwrap_or(5).min(20) as usize).map(|h|json!({"invocation":h.invocation,"finalReply":h.final_reply(),"events":h.events})).collect::<Vec<_>>();
                Ok(json!({"session":history.session,"invocations":invocations,"interactions":self.agents.session_interactions(&session()?).map_err(|e|e.to_string())?}))},
            "list_workspaces"=>Ok(json!(self.repositories.list_registered()?.iter().map(|r|json!({"repositoryId":r.repository_id,"name":r.label,"path":r.anchor_root})).collect::<Vec<_>>())),
            "list_profiles"=>Ok(json!(self.profiles.list().map_err(|e|e.to_string())?)),
            "start_session"=>{let session=self.agents.create_product_session(optional(&args,"title").unwrap_or_else(||"Agent work".into()),optional(&args,"workingDirectory"),required(&args,"profileId")?, [("orchid_control".into(),tool_map().remove("orchid_control").unwrap())].into_iter().collect())?;
                let receipt=self.agents.deliver_product_content(session.id.clone(),AgentInvocationInputProvenance::Application,InvocationContent::query(required(&args,"task")?)).map_err(|e|e.to_string())?;
                Ok(json!({"sessionId":session.id,"invocationId":receipt.acknowledgement.invocation_id,"launchAccepted":receipt.launch_accepted}))},
            "send_work"=>{let receipt=self.agents.deliver_product_content(session()?,AgentInvocationInputProvenance::Application,InvocationContent::query(required(&args,"text")?)).map_err(|e|e.to_string())?;Ok(json!({"sessionId":receipt.acknowledgement.session_id,"invocationId":receipt.acknowledgement.invocation_id,"launchAccepted":receipt.launch_accepted}))},
            "steer_session"=>Ok(json!(self.agents.steer_session(serde_json::from_value::<SteerAgentSessionCommand>(args).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?)),
            "cancel_invocation"=>{self.agents.cancel_invocation(CancelAgentInvocationCommand{invocation_id:AgentInvocationId::new(required(&args,"invocationId")?.to_string()).map_err(|e|e.to_string())?}).map_err(|e|e.to_string())?;Ok(json!({"cancellationRequested":true}))},
            "respond_to_request"=>Ok(json!(self.agents.respond_to_runtime_request(serde_json::from_value::<RespondToRuntimeRequestCommand>(args).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?)),
            _=>Err("Unknown Orchid tool".into()),
        }
    }
}
