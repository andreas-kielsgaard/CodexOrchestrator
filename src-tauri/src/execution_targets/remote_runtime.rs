use super::{domain::ExecutionBinding, ssh_connection::SshConnection};
use crate::agent_sessions::{domain::*, ports::*};
use orchid_engine::protocol::HostCommand;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::{Arc, Mutex};

trait HostConnection: Send + Sync {
    fn request(&self, command: HostCommand) -> Result<Value, RuntimePortError>;
    fn is_closed(&self) -> Result<bool, RuntimePortError>;
    fn has_active_invocations(&self) -> bool;
    fn register(&self, id: AgentInvocationId, sink: Arc<dyn AgentRuntimeUpdateSink>);
    fn unregister(&self, id: &AgentInvocationId);
    fn shutdown(&self) -> Result<(), RuntimePortError>;
}

impl HostConnection for SshConnection {
    fn request(&self, command: HostCommand) -> Result<Value, RuntimePortError> {
        self.request(command)
    }
    fn is_closed(&self) -> Result<bool, RuntimePortError> {
        self.is_closed()
    }
    fn has_active_invocations(&self) -> bool {
        self.has_active_invocations()
    }
    fn register(&self, id: AgentInvocationId, sink: Arc<dyn AgentRuntimeUpdateSink>) {
        self.register(id, sink);
    }
    fn unregister(&self, id: &AgentInvocationId) {
        self.unregister(id);
    }
    fn shutdown(&self) -> Result<(), RuntimePortError> {
        self.shutdown()
    }
}

type ConnectHost = dyn Fn() -> Result<Arc<dyn HostConnection>, RuntimePortError> + Send + Sync;

pub(crate) struct RemoteRuntime {
    connection: Mutex<Arc<dyn HostConnection>>,
    connect: Arc<ConnectHost>,
    binding: ExecutionBinding,
}

impl AgentRuntime for RemoteRuntime {
    fn prepare_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external_context_id: Option<ExternalRuntimeContextId>,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<RuntimeInvocationReady, RuntimePortError> {
        let id = request.invocation_id.clone();
        let connection = self.connection_for_invocation()?;
        connection.register(id.clone(), sink);
        let result = request_host(
            &*connection,
            HostCommand::PrepareInvocation {
                configuration_ref: self.binding.configuration_ref.clone(),
                request,
                external_context_id,
            },
        );
        if result.is_err() {
            connection.unregister(&id);
        }
        result
    }
    fn deliver_prepared_invocation(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<(), RuntimePortError> {
        request_host(
            &*self.current_connection(),
            HostCommand::DeliverPreparedInvocation {
                invocation_id: invocation_id.clone(),
            },
        )
    }
    fn preflight_invocation(
        &self,
        mode: RuntimeInvocationMode,
        options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        request_host(
            &*self.connection_for_invocation()?,
            HostCommand::Preflight {
                configuration_ref: self.binding.configuration_ref.clone(),
                mode,
                options: options.clone(),
            },
        )
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
        request_host(
            &*self.current_connection(),
            HostCommand::Respond {
                invocation_id: invocation_id.clone(),
                request_id: request_id.into(),
                response,
            },
        )
    }
    fn cancel_invocation(&self, invocation_id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        request_host(
            &*self.current_connection(),
            HostCommand::Cancel {
                invocation_id: invocation_id.clone(),
            },
        )
    }
    fn active_turn(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<RuntimeTurnTarget, RuntimePortError> {
        request_host(
            &*self.current_connection(),
            HostCommand::ActiveTurn {
                invocation_id: invocation_id.clone(),
            },
        )
    }
    fn steer(
        &self,
        invocation_id: &AgentInvocationId,
        target: &RuntimeTurnTarget,
        input_id: &str,
        text: &str,
    ) -> Result<(), RuntimePortError> {
        request_host(
            &*self.current_connection(),
            HostCommand::Steer {
                invocation_id: invocation_id.clone(),
                target: target.clone(),
                input_id: input_id.into(),
                text: text.into(),
            },
        )
    }
    fn shutdown(&self) -> Result<(), RuntimePortError> {
        self.current_connection().shutdown()
    }
}
impl RemoteRuntime {
    pub(crate) fn connect(binding: ExecutionBinding) -> Result<Self, RuntimePortError> {
        let super::domain::ExecutionConnection::Ssh {
            target,
            host_executable,
        } = binding.connection.clone()
        else {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::UnsupportedOptions,
                "A remote runtime requires an SSH connection",
            ));
        };
        let connect: Arc<ConnectHost> =
            Arc::new(move || Ok(Arc::new(SshConnection::connect(&target, &host_executable)?)));
        Ok(Self {
            connection: Mutex::new(connect()?),
            connect,
            binding,
        })
    }

    fn current_connection(&self) -> Arc<dyn HostConnection> {
        self.connection.lock().unwrap().clone()
    }

    fn connection_for_invocation(&self) -> Result<Arc<dyn HostConnection>, RuntimePortError> {
        let mut connection = self.connection.lock().unwrap();
        if connection.is_closed()? && !connection.has_active_invocations() {
            *connection = (self.connect)()?;
        }
        Ok(connection.clone())
    }

    fn invoke(
        &self,
        request: RuntimeInvocationRequest,
        external_context_id: Option<ExternalRuntimeContextId>,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        let id = request.invocation_id.clone();
        let connection = self.connection_for_invocation()?;
        connection.register(id.clone(), sink);
        let result = request_host(
            &*connection,
            HostCommand::Invoke {
                configuration_ref: self.binding.configuration_ref.clone(),
                request,
                external_context_id,
            },
        );
        if result.is_err() {
            connection.unregister(&id);
        }
        result
    }
}

fn request_host<T: DeserializeOwned>(
    connection: &dyn HostConnection,
    command: HostCommand,
) -> Result<T, RuntimePortError> {
    serde_json::from_value(connection.request(command)?).map_err(|error| {
        RuntimePortError::new(
            RuntimePortErrorKind::Unavailable,
            format!("Invalid remote reply: {error}"),
        )
    })
}

#[cfg(test)]
#[path = "remote_runtime_tests.rs"]
mod tests;
