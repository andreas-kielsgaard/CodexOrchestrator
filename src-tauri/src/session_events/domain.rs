use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReferenceIdentity {
    namespace: String,
    kind: String,
    id: String,
}

impl ReferenceIdentity {
    pub(crate) fn new(
        namespace: impl Into<String>,
        kind: impl Into<String>,
        id: impl Into<String>,
    ) -> Result<Self, SessionEventDomainError> {
        let identity = Self {
            namespace: namespace.into(),
            kind: kind.into(),
            id: id.into(),
        };
        validate_reference_part("namespace", &identity.namespace)?;
        validate_reference_part("kind", &identity.kind)?;
        validate_reference_part("id", &identity.id)?;
        Ok(identity)
    }

    pub(crate) fn namespace(&self) -> &str {
        &self.namespace
    }

    pub(crate) fn kind(&self) -> &str {
        &self.kind
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Display for ReferenceIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}/{}", self.namespace, self.kind, self.id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionLogicalAddress {
    pub(crate) scope: ReferenceIdentity,
    pub(crate) subject: ReferenceIdentity,
}

impl SessionLogicalAddress {
    pub(crate) fn new(scope: ReferenceIdentity, subject: ReferenceIdentity) -> Self {
        Self { scope, subject }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SessionEventTriggerBinding {
    UserRequest,
    InvocationCompleted {
        source_address: Option<SessionLogicalAddress>,
    },
    McpCall {
        server: ReferenceIdentity,
        tool: ReferenceIdentity,
    },
    ApplicationEvent {
        event_kind: ReferenceIdentity,
    },
    EventGroupCompleted {
        source_definition: ReferenceIdentity,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SessionEventTrigger {
    UserRequest {
        request: ReferenceIdentity,
    },
    InvocationCompleted {
        session: ReferenceIdentity,
        invocation: ReferenceIdentity,
    },
    McpCall {
        call: ReferenceIdentity,
        server: ReferenceIdentity,
        tool: ReferenceIdentity,
    },
    ApplicationEvent {
        event: ReferenceIdentity,
    },
    EventGroupCompleted {
        event_group: ReferenceIdentity,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SessionEventSource {
    UserRequest {
        request: ReferenceIdentity,
    },
    Application {
        component: ReferenceIdentity,
    },
    SessionInvocation {
        session: ReferenceIdentity,
        invocation: ReferenceIdentity,
    },
    McpCall {
        call: ReferenceIdentity,
        server: ReferenceIdentity,
        tool: ReferenceIdentity,
    },
    ApplicationEvent {
        event: ReferenceIdentity,
    },
    EventGroup {
        event_group: ReferenceIdentity,
    },
    ExternalReference {
        reference: ReferenceIdentity,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum PromptSourceDefinition {
    Literal { text: String },
    UserRequestText,
    InvocationOutput,
    McpArgument { name: String },
    ApplicationEventField { field: String },
    ReferencedContent { reference: ReferenceIdentity },
}

impl PromptSourceDefinition {
    fn validate(&self, label: &str) -> Result<(), SessionEventDomainError> {
        match self {
            Self::Literal { text } => validate_prompt_text(label, text),
            Self::McpArgument { name } => validate_symbol(label, name),
            Self::ApplicationEventField { field } => validate_symbol(label, field),
            Self::UserRequestText | Self::InvocationOutput | Self::ReferencedContent { .. } => {
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum PromptSource {
    Literal {
        text: String,
    },
    UserRequestText {
        request: ReferenceIdentity,
        text: String,
    },
    InvocationOutput {
        invocation: ReferenceIdentity,
        text: String,
    },
    McpArgument {
        call: ReferenceIdentity,
        name: String,
        text: String,
    },
    ApplicationEventField {
        event: ReferenceIdentity,
        field: String,
        text: String,
    },
    ReferencedContent {
        reference: ReferenceIdentity,
        text: String,
    },
}

impl PromptSource {
    pub(crate) fn literal(text: impl Into<String>) -> Self {
        Self::Literal { text: text.into() }
    }

    pub(crate) fn text(&self) -> &str {
        match self {
            Self::Literal { text }
            | Self::UserRequestText { text, .. }
            | Self::InvocationOutput { text, .. }
            | Self::McpArgument { text, .. }
            | Self::ApplicationEventField { text, .. }
            | Self::ReferencedContent { text, .. } => text,
        }
    }

    fn validate(&self, label: &str) -> Result<(), SessionEventDomainError> {
        validate_prompt_text(label, self.text())?;
        match self {
            Self::McpArgument { name, .. } => validate_symbol(label, name),
            Self::ApplicationEventField { field, .. } => validate_symbol(label, field),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SessionTarget {
    Exact { session: ReferenceIdentity },
    Logical { address: SessionLogicalAddress },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetCardinality {
    First,
    All,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetOrdering {
    Newest,
    LastAddressed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunningFilter {
    Any,
    RunningOnly,
    NotRunning,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MissingTargetPolicy {
    Create,
    Fail,
    Noop,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionCreationFilter {
    pub(crate) event: Option<ReferenceIdentity>,
    pub(crate) session: Option<ReferenceIdentity>,
}

impl SessionCreationFilter {
    fn validate(&self) -> Result<(), SessionEventDomainError> {
        if self.event.is_none() && self.session.is_none() {
            return Err(SessionEventDomainError::InvalidTarget(
                "Session creation filter must select an event or Session".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TargetSelection {
    pub(crate) target: SessionTarget,
    pub(crate) cardinality: TargetCardinality,
    pub(crate) ordering: TargetOrdering,
    pub(crate) running: RunningFilter,
    pub(crate) created_by: Option<SessionCreationFilter>,
    pub(crate) missing: MissingTargetPolicy,
}

impl TargetSelection {
    fn validate(&self) -> Result<(), SessionEventDomainError> {
        if let Some(filter) = &self.created_by {
            filter.validate()?;
        }
        if self.missing == MissingTargetPolicy::Create
            && !matches!(self.target, SessionTarget::Logical { .. })
        {
            return Err(SessionEventDomainError::InvalidTarget(
                "Create-on-missing requires a logical Session address".into(),
            ));
        }
        Ok(())
    }
}

/// Opaque configuration pinned by the producer and interpreted by the Session-directory adapter.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionCreationConfiguration {
    pub(crate) contract: ReferenceIdentity,
    pub(crate) payload: Value,
    /// Optional presentation identity assigned by the concrete Session directory.
    /// It does not participate in capability resolution.
    pub(crate) assigned_identity: Option<ReferenceIdentity>,
}

impl SessionCreationConfiguration {
    fn validate(&self) -> Result<(), SessionEventDomainError> {
        if self.payload.is_null() {
            return Err(SessionEventDomainError::InvalidCreationConfiguration(
                "Pinned Session creation configuration must not be null".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionEventDefinition {
    pub(crate) definition_ref: ReferenceIdentity,
    pub(crate) trigger: SessionEventTriggerBinding,
    pub(crate) target: TargetSelection,
    pub(crate) prompt_sources: Vec<PromptSourceDefinition>,
    pub(crate) created_session_prompt_sources: Vec<PromptSourceDefinition>,
    pub(crate) creation_configuration: Option<SessionCreationConfiguration>,
}

impl SessionEventDefinition {
    pub(crate) fn validate(&self) -> Result<(), SessionEventDomainError> {
        self.target.validate()?;
        validate_prompt_definitions("Session event prompt source", &self.prompt_sources, true)?;
        validate_prompt_definitions(
            "Created Session prompt source",
            &self.created_session_prompt_sources,
            false,
        )?;
        if let Some(configuration) = &self.creation_configuration {
            configuration.validate()?;
        }
        if self.target.missing == MissingTargetPolicy::Create
            && self.creation_configuration.is_none()
        {
            return Err(SessionEventDomainError::InvalidCreationConfiguration(
                "Create-on-missing requires a pinned Session creation configuration".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionEventCommand {
    pub(crate) event_group_id: ReferenceIdentity,
    pub(crate) definition_ref: ReferenceIdentity,
    pub(crate) trigger: SessionEventTrigger,
    pub(crate) source: SessionEventSource,
    pub(crate) prompt_sources: Vec<PromptSource>,
    pub(crate) created_session_prompt_sources: Vec<PromptSource>,
    pub(crate) creation_configuration: Option<SessionCreationConfiguration>,
    pub(crate) target: TargetSelection,
    pub(crate) created_by_session: Option<ReferenceIdentity>,
    pub(crate) direct_user_options: Option<DirectUserInvocationOptions>,
}

impl SessionEventCommand {
    pub(crate) fn validate(&self) -> Result<(), SessionEventDomainError> {
        self.target.validate()?;
        validate_prompt_sources("Session event prompt source", &self.prompt_sources, true)?;
        validate_prompt_sources(
            "Created Session prompt source",
            &self.created_session_prompt_sources,
            false,
        )?;
        if let Some(configuration) = &self.creation_configuration {
            configuration.validate()?;
        }
        if self.target.missing == MissingTargetPolicy::Create
            && self.creation_configuration.is_none()
        {
            return Err(SessionEventDomainError::InvalidCreationConfiguration(
                "Create-on-missing requires a pinned Session creation configuration".into(),
            ));
        }
        if let Some(options) = &self.direct_user_options {
            options.validate()?;
            if !matches!(self.trigger, SessionEventTrigger::UserRequest { .. })
                || !matches!(self.source, SessionEventSource::UserRequest { .. })
                || !matches!(self.target.target, SessionTarget::Exact { .. })
            {
                return Err(SessionEventDomainError::InvalidInvocationOptions(
                    "Direct-user invocation options require a UserRequest occurrence with an exact Session target"
                        .into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DirectUserInvocationOptions {
    pub(crate) model: Option<String>,
    pub(crate) reasoning_mode: Option<String>,
}

impl DirectUserInvocationOptions {
    fn validate(&self) -> Result<(), SessionEventDomainError> {
        if self.model.is_none() && self.reasoning_mode.is_none() {
            return Err(SessionEventDomainError::InvalidInvocationOptions(
                "Direct-user invocation options must request a model or reasoning mode".into(),
            ));
        }
        if let Some(model) = &self.model {
            validate_symbol("Direct-user model", model)?;
        }
        if let Some(reasoning) = &self.reasoning_mode {
            validate_symbol("Direct-user reasoning mode", reasoning)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionDirectoryEntry {
    pub(crate) session: ReferenceIdentity,
    pub(crate) logical_address: Option<SessionLogicalAddress>,
    pub(crate) running: bool,
    pub(crate) created_sequence: u64,
    pub(crate) last_addressed_sequence: Option<u64>,
    pub(crate) created_by_event: Option<ReferenceIdentity>,
    pub(crate) created_by_session: Option<ReferenceIdentity>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventGroupOutcome {
    Delivered,
    PartiallyDelivered,
    DeliveryFailed,
    NoTarget,
    Noop,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EventGroupRecord {
    pub(crate) event_group_id: ReferenceIdentity,
    pub(crate) definition_ref: ReferenceIdentity,
    pub(crate) trigger: SessionEventTrigger,
    pub(crate) source: SessionEventSource,
    pub(crate) prompt_sources: Vec<PromptSource>,
    pub(crate) created_session_prompt_sources: Vec<PromptSource>,
    pub(crate) target_selection: TargetSelection,
    pub(crate) resolved_sessions: Vec<ReferenceIdentity>,
    pub(crate) created_session: Option<ReferenceIdentity>,
    pub(crate) outcome: EventGroupOutcome,
    pub(crate) delivery_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum DeliveryOutcome {
    Dispatched { invocation: ReferenceIdentity },
    Failed { message: String },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EventDeliveryRecord {
    pub(crate) delivery_id: ReferenceIdentity,
    pub(crate) event_group_id: ReferenceIdentity,
    pub(crate) ordinal: u32,
    pub(crate) target_session: ReferenceIdentity,
    pub(crate) logical_address: Option<SessionLogicalAddress>,
    pub(crate) target_created: bool,
    pub(crate) prompt_contributions: Vec<PromptSource>,
    pub(crate) included_created_session_contributions: Vec<PromptSource>,
    pub(crate) addressed_sequence: Option<u64>,
    pub(crate) addressing_error: Option<String>,
    pub(crate) outcome: DeliveryOutcome,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionEventResult {
    pub(crate) group: EventGroupRecord,
    pub(crate) deliveries: Vec<EventDeliveryRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionEventDomainError {
    InvalidReference(String),
    InvalidPrompt(String),
    InvalidTarget(String),
    InvalidCreationConfiguration(String),
    InvalidInvocationOptions(String),
}

impl fmt::Display for SessionEventDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReference(message)
            | Self::InvalidPrompt(message)
            | Self::InvalidTarget(message)
            | Self::InvalidCreationConfiguration(message)
            | Self::InvalidInvocationOptions(message) => formatter.write_str(message),
        }
    }
}

impl Error for SessionEventDomainError {}

fn validate_reference_part(label: &str, value: &str) -> Result<(), SessionEventDomainError> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.len() > 256
        || value.chars().any(char::is_control)
    {
        return Err(SessionEventDomainError::InvalidReference(format!(
            "Reference identity {label} must be a non-empty trimmed opaque value"
        )));
    }
    Ok(())
}

fn validate_prompt_text(label: &str, text: &str) -> Result<(), SessionEventDomainError> {
    if text.trim().is_empty() {
        return Err(SessionEventDomainError::InvalidPrompt(format!(
            "{label} must not be blank"
        )));
    }
    if text.contains('\0') || text.len() > 262_144 {
        return Err(SessionEventDomainError::InvalidPrompt(format!(
            "{label} is invalid for delivery"
        )));
    }
    Ok(())
}

fn validate_symbol(label: &str, value: &str) -> Result<(), SessionEventDomainError> {
    if value.trim().is_empty() || value != value.trim() || value.chars().any(char::is_control) {
        return Err(SessionEventDomainError::InvalidPrompt(format!(
            "{label} selector must be a non-empty trimmed value"
        )));
    }
    Ok(())
}

fn validate_prompt_definitions(
    label: &str,
    values: &[PromptSourceDefinition],
    required: bool,
) -> Result<(), SessionEventDomainError> {
    if required && values.is_empty() {
        return Err(SessionEventDomainError::InvalidPrompt(format!(
            "{label} list must not be empty"
        )));
    }
    for value in values {
        value.validate(label)?;
    }
    Ok(())
}

fn validate_prompt_sources(
    label: &str,
    values: &[PromptSource],
    required: bool,
) -> Result<(), SessionEventDomainError> {
    if required && values.is_empty() {
        return Err(SessionEventDomainError::InvalidPrompt(format!(
            "{label} list must not be empty"
        )));
    }
    for value in values {
        value.validate(label)?;
    }
    Ok(())
}
