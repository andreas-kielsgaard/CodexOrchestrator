use super::{domain::ExecutionBinding, ssh_connection::SshConnection};
use crate::agent_sessions::{domain::*, ports::*};
use orchid_engine::protocol::HostCommand;
use std::sync::Arc;

pub(crate) struct RemoteRuntime {
    pub(crate) connection: Arc<SshConnection>,
    pub(crate) binding: ExecutionBinding,
}

impl AgentRuntime for RemoteRuntime {
    fn preflight_invocation(
        &self,
        mode: RuntimeInvocationMode,
        options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        self.connection.request(HostCommand::Preflight {
            configuration_ref: self.binding.configuration_ref.clone(),
            mode,
            options: options.clone(),
        })
    }
    fn start_invocation(
        &self,
        request: RuntimeInvocationRequest,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.invoke(request, None, sink)
    }
    fn resume_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external_context_id: ExternalRuntimeContextId,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.invoke(request, Some(external_context_id), sink)
    }
    fn respond(
        &self,
        invocation_id: &AgentInvocationId,
        request_id: &str,
        response: serde_json::Value,
    ) -> Result<(), RuntimePortError> {
        self.connection.request(HostCommand::Respond {
            invocation_id: invocation_id.clone(),
            request_id: request_id.into(),
            response,
        })
    }
    fn cancel_invocation(&self, invocation_id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        self.connection.request(HostCommand::Cancel {
            invocation_id: invocation_id.clone(),
        })
    }
    fn active_turn(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<RuntimeTurnTarget, RuntimePortError> {
        self.connection.request(HostCommand::ActiveTurn {
            invocation_id: invocation_id.clone(),
        })
    }
    fn steer(
        &self,
        invocation_id: &AgentInvocationId,
        target: &RuntimeTurnTarget,
        input_id: &str,
        text: &str,
    ) -> Result<(), RuntimePortError> {
        self.connection.request(HostCommand::Steer {
            invocation_id: invocation_id.clone(),
            target: target.clone(),
            input_id: input_id.into(),
            text: text.into(),
        })
    }
    fn shutdown(&self) -> Result<(), RuntimePortError> {
        self.connection.shutdown()
    }
}
impl RemoteRuntime {
    fn invoke(
        &self,
        request: RuntimeInvocationRequest,
        external_context_id: Option<ExternalRuntimeContextId>,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        let id = request.invocation_id.clone();
        self.connection.register(id.clone(), sink);
        let result = self.connection.request(HostCommand::Invoke {
            configuration_ref: self.binding.configuration_ref.clone(),
            request,
            external_context_id,
        });
        if result.is_err() {
            self.connection.unregister(&id);
        }
        result
    }
}
