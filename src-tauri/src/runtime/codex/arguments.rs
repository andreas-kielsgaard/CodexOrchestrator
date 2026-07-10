use super::capabilities::CodexCliCapabilities;
use crate::agent_sessions::{
    domain::{AgentRuntimeOptions, ExternalRuntimeContextId, RuntimeSandboxMode},
    ports::{RuntimePortError, RuntimePortErrorKind},
};

pub(super) enum InvocationCommand<'a> {
    Start,
    Resume(&'a ExternalRuntimeContextId),
}

pub(super) fn build_args(
    command: InvocationCommand<'_>,
    prompt: &str,
    options: &AgentRuntimeOptions,
    capabilities: Option<&CodexCliCapabilities>,
) -> Result<Vec<String>, RuntimePortError> {
    let resume = matches!(command, InvocationCommand::Resume(_));
    require_json(capabilities, resume)?;

    let mut args = vec!["exec".to_string()];
    if let InvocationCommand::Resume(context_id) = command {
        args.push("resume".to_string());
        push_supported_flag(&mut args, "--json", None);
        map_model(&mut args, options, capabilities, true)?;
        map_sandbox(&mut args, options, capabilities, true)?;
        args.push(context_id.as_str().to_string());
    } else {
        push_supported_flag(&mut args, "--json", None);
        map_model(&mut args, options, capabilities, false)?;
        map_sandbox(&mut args, options, capabilities, false)?;
    }
    args.push(prompt.to_string());
    Ok(args)
}

fn require_json(
    capabilities: Option<&CodexCliCapabilities>,
    resume: bool,
) -> Result<(), RuntimePortError> {
    let supported = capabilities.and_then(|caps| {
        if resume {
            caps.resume_json
        } else {
            caps.exec_json
        }
    });
    if supported == Some(false) {
        return Err(unsupported(
            "installed Codex CLI does not support JSON output for this command",
        ));
    }
    Ok(())
}

fn map_model(
    args: &mut Vec<String>,
    options: &AgentRuntimeOptions,
    capabilities: Option<&CodexCliCapabilities>,
    resume: bool,
) -> Result<(), RuntimePortError> {
    let Some(model) = options.model.as_deref() else {
        return Ok(());
    };
    if model.trim().is_empty() {
        return Err(unsupported("the requested Codex model must not be empty"));
    }
    let support = capabilities.and_then(|caps| {
        if resume {
            caps.resume_model
        } else {
            caps.exec_model
        }
    });
    match support {
        Some(true) => push_supported_flag(args, "--model", Some(model)),
        Some(false) => return Err(unsupported(
            "the installed Codex CLI does not support the requested model option for this command",
        )),
        None => {}
    }
    Ok(())
}

fn map_sandbox(
    args: &mut Vec<String>,
    options: &AgentRuntimeOptions,
    capabilities: Option<&CodexCliCapabilities>,
    resume: bool,
) -> Result<(), RuntimePortError> {
    let Some(sandbox) = options.sandbox else {
        return Ok(());
    };
    let support = capabilities.and_then(|caps| {
        if resume {
            caps.resume_sandbox
        } else {
            caps.exec_sandbox
        }
    });
    match support {
        Some(true) => push_supported_flag(args, "--sandbox", Some(sandbox_value(sandbox))),
        Some(false) => return Err(unsupported("the installed Codex CLI does not support the requested sandbox option for this command")),
        None => {}
    }
    Ok(())
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
