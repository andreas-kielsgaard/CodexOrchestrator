use crate::agent_sessions::{
    domain::{AgentRuntimeOptions, ExternalRuntimeContextId, RuntimeSandboxMode},
    ports::{CapabilitySupport, InvocationCapabilities, RuntimePortError, RuntimePortErrorKind},
};

pub(super) enum InvocationCommand<'a> {
    Start,
    Resume(&'a ExternalRuntimeContextId),
}

pub(super) fn build_args_from_effective_options(
    command: InvocationCommand<'_>,
    prompt: &str,
    effective_options: &AgentRuntimeOptions,
    launch_extension: Option<&crate::agent_sessions::ports::RuntimeLaunchExtension>,
) -> Vec<String> {
    let mut args = vec!["exec".to_string()];
    let resume_context = match command {
        InvocationCommand::Resume(context_id) => {
            args.push("resume".to_string());
            push_supported_flag(&mut args, "--json", None);
            push_effective_options(&mut args, effective_options, true);
            Some(context_id)
        }
        InvocationCommand::Start => {
            push_supported_flag(&mut args, "--json", None);
            push_effective_options(&mut args, effective_options, false);
            None
        }
    };
    if let Some(extension) = launch_extension {
        args.extend(extension.additional_args.iter().cloned());
    }
    if let Some(context_id) = resume_context {
        args.push(context_id.as_str().to_string());
    }
    args.push(prompt.to_string());
    args
}

pub(super) fn prepare_options(
    options: &AgentRuntimeOptions,
    capabilities: &InvocationCapabilities,
) -> Result<AgentRuntimeOptions, RuntimePortError> {
    require_json(capabilities.structured_events)?;
    let model = prepare_model(options.model.as_deref(), capabilities.model_selection)?;
    let sandbox = prepare_sandbox(options.sandbox, capabilities.sandbox_selection)?;
    Ok(AgentRuntimeOptions { model, sandbox })
}

fn require_json(support: CapabilitySupport) -> Result<(), RuntimePortError> {
    if support == CapabilitySupport::Unsupported {
        return Err(unsupported(
            "installed Codex CLI does not support JSON output for this command",
        ));
    }
    Ok(())
}

fn prepare_model(
    model: Option<&str>,
    support: CapabilitySupport,
) -> Result<Option<String>, RuntimePortError> {
    let Some(model) = model else {
        return Ok(None);
    };
    if model.trim().is_empty() {
        return Err(unsupported("the requested Codex model must not be empty"));
    }
    match support {
        CapabilitySupport::Supported => Ok(Some(model.to_string())),
        CapabilitySupport::Unsupported => return Err(unsupported(
            "the installed Codex CLI does not support the requested model option for this command",
        )),
        CapabilitySupport::Unknown => Ok(None),
    }
}

fn prepare_sandbox(
    sandbox: Option<RuntimeSandboxMode>,
    support: CapabilitySupport,
) -> Result<Option<RuntimeSandboxMode>, RuntimePortError> {
    let Some(sandbox) = sandbox else {
        return Ok(None);
    };
    match support {
        CapabilitySupport::Supported => Ok(Some(sandbox)),
        CapabilitySupport::Unsupported => return Err(unsupported("the installed Codex CLI does not support the requested sandbox option for this command")),
        CapabilitySupport::Unknown => Err(unsupported(
            "the installed Codex CLI sandbox support is unknown; refusing to launch without the requested sandbox",
        )),
    }
}

fn push_effective_options(args: &mut Vec<String>, options: &AgentRuntimeOptions, resume: bool) {
    if let Some(model) = options.model.as_deref() {
        push_supported_flag(args, "--model", Some(model));
    }
    if let Some(sandbox) = options.sandbox {
        if resume {
            push_supported_flag(
                args,
                "-c",
                Some(&format!("sandbox_mode=\"{}\"", sandbox_value(sandbox))),
            );
        } else {
            push_supported_flag(args, "--sandbox", Some(sandbox_value(sandbox)));
        }
    }
}

fn push_supported_flag(args: &mut Vec<String>, flag: &str, value: Option<&str>) {
    args.push(flag.to_string());
    if let Some(value) = value {
        args.push(value.to_string());
    }
}

fn sandbox_value(mode: RuntimeSandboxMode) -> &'static str {
    match mode {
        RuntimeSandboxMode::ReadOnly => "read-only",
        RuntimeSandboxMode::WorkspaceWrite => "workspace-write",
        RuntimeSandboxMode::DangerFullAccess => "danger-full-access",
    }
}

fn unsupported(message: &str) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::UnsupportedOptions, message)
}
