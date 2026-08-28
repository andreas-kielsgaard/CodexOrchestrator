mod addressing;
mod domain;
mod materialization;
mod ports;
mod repository;

use sha2::{Digest, Sha256};
use std::{error::Error, fmt, sync::Arc};

pub(crate) use domain::{
    DeliveryOutcome, DirectUserInvocationOptions, EventDeliveryRecord, EventGroupOutcome,
    EventGroupRecord, MissingTargetPolicy, PromptSource, PromptSourceDefinition, ReferenceIdentity,
    RunningFilter, SessionCreationConfiguration, SessionCreationFilter, SessionDirectoryEntry,
    SessionEventCommand, SessionEventDefinition, SessionEventDomainError, SessionEventResult,
    SessionEventSource, SessionEventTrigger, SessionEventTriggerBinding, SessionLogicalAddress,
    SessionTarget, TargetCardinality, TargetOrdering, TargetSelection,
};
pub(crate) use materialization::{
    SessionEventMaterializationError, SessionEventMaterializer, SessionEventOccurrence,
    SessionEventOccurrenceTrigger,
};
pub(crate) use ports::{
    SessionCreationSpec, SessionDirectory, SessionDirectoryError, SessionEventStore,
    SessionEventStoreError, SessionInvocationDispatcher, SessionInvocationError,
    SessionInvocationReceipt, SessionInvocationRequest,
};
pub(crate) use repository::{InMemorySessionEventStore, SqliteSessionEventStore};

pub(crate) struct SessionEventApplication {
    directory: Arc<dyn SessionDirectory>,
    dispatcher: Arc<dyn SessionInvocationDispatcher>,
    store: Arc<dyn SessionEventStore>,
}

impl SessionEventApplication {
    pub(crate) fn new(
        directory: Arc<dyn SessionDirectory>,
        dispatcher: Arc<dyn SessionInvocationDispatcher>,
        store: Arc<dyn SessionEventStore>,
    ) -> Self {
        Self {
            directory,
            dispatcher,
            store,
        }
    }

    pub(crate) fn dispatch(
        &self,
        command: SessionEventCommand,
    ) -> Result<SessionEventResult, SessionEventApplicationError> {
        command
            .validate()
            .map_err(SessionEventApplicationError::Domain)?;
        let mut targets = addressing::resolve_existing(self.directory.as_ref(), &command.target)
            .map_err(SessionEventApplicationError::Directory)?;
        let mut created_session = None;

        if targets.is_empty() {
            match command.target.missing {
                MissingTargetPolicy::Create => {
                    let SessionTarget::Logical { address } = &command.target.target else {
                        unreachable!("validated create-on-missing target must be logical")
                    };
                    let created = self
                        .directory
                        .create_session(SessionCreationSpec {
                            logical_address: address.clone(),
                            event_group_id: command.event_group_id.clone(),
                            created_by_event: command.event_group_id.clone(),
                            created_by_session: command.created_by_session.clone(),
                            configuration: command
                                .creation_configuration
                                .clone()
                                .expect("validated create-on-missing configuration"),
                        })
                        .map_err(SessionEventApplicationError::Directory)?;
                    validate_created_entry(&command, address, &created)?;
                    created_session = Some(created.session.clone());
                    targets.push(created);
                }
                MissingTargetPolicy::Fail => {
                    let result = build_empty_result(&command, EventGroupOutcome::NoTarget);
                    self.store_result(&result)?;
                    return Err(SessionEventApplicationError::NoTarget(
                        command.event_group_id,
                    ));
                }
                MissingTargetPolicy::Noop => {
                    let result = build_empty_result(&command, EventGroupOutcome::Noop);
                    self.store_result(&result)?;
                    return Ok(result);
                }
            }
        }

        let prompt = render_prompt(&command.prompt_sources);
        let created_prompt = render_optional_prompt(&command.created_session_prompt_sources);
        let mut deliveries = Vec::with_capacity(targets.len());
        let mut first_addressing_error = None;

        for (index, target) in targets.iter().enumerate() {
            let ordinal = u32::try_from(index + 1).unwrap_or(u32::MAX);
            let delivery_id = delivery_identity(&command.event_group_id, ordinal)?;
            let target_created = created_session.as_ref() == Some(&target.session);
            let included_created_session_contributions = if target_created {
                command.created_session_prompt_sources.clone()
            } else {
                Vec::new()
            };
            let request = SessionInvocationRequest {
                event_group_id: command.event_group_id.clone(),
                delivery_id: delivery_id.clone(),
                target_session: target.session.clone(),
                source: command.source.clone(),
                prompt: prompt.clone(),
                initial_prompt: target_created.then(|| created_prompt.clone()).flatten(),
                direct_user_options: command.direct_user_options.clone(),
            };

            let dispatch_outcome = self.dispatcher.dispatch(request);
            let (addressed_sequence, addressing_error) = match &target.logical_address {
                Some(_) => match self
                    .directory
                    .mark_addressed(&target.session, &command.event_group_id)
                {
                    Ok(sequence) => (Some(sequence), None),
                    Err(error) => {
                        let message = error.to_string();
                        first_addressing_error.get_or_insert_with(|| message.clone());
                        (None, Some(message))
                    }
                },
                None => (None, None),
            };
            let outcome = match dispatch_outcome {
                Ok(receipt) => DeliveryOutcome::Dispatched {
                    invocation: receipt.invocation,
                },
                Err(error) => DeliveryOutcome::Failed {
                    message: error.to_string(),
                },
            };
            deliveries.push(EventDeliveryRecord {
                delivery_id,
                event_group_id: command.event_group_id.clone(),
                ordinal,
                target_session: target.session.clone(),
                logical_address: target.logical_address.clone(),
                target_created,
                prompt_contributions: command.prompt_sources.clone(),
                included_created_session_contributions,
                addressed_sequence,
                addressing_error,
                outcome,
            });
        }

        let successful = deliveries
            .iter()
            .filter(|delivery| matches!(delivery.outcome, DeliveryOutcome::Dispatched { .. }))
            .count();
        let outcome = if successful == deliveries.len() {
            EventGroupOutcome::Delivered
        } else if successful == 0 {
            EventGroupOutcome::DeliveryFailed
        } else {
            EventGroupOutcome::PartiallyDelivered
        };
        let result = SessionEventResult {
            group: EventGroupRecord {
                event_group_id: command.event_group_id.clone(),
                definition_ref: command.definition_ref,
                trigger: command.trigger,
                source: command.source,
                prompt_sources: command.prompt_sources,
                created_session_prompt_sources: command.created_session_prompt_sources,
                target_selection: command.target,
                resolved_sessions: targets.into_iter().map(|entry| entry.session).collect(),
                created_session,
                outcome,
                delivery_count: u32::try_from(deliveries.len()).unwrap_or(u32::MAX),
            },
            deliveries,
        };
        self.store_result(&result)?;
        if let Some(message) = first_addressing_error {
            return Err(SessionEventApplicationError::AddressingAfterDispatch(
                message,
            ));
        }
        Ok(result)
    }

    pub(crate) fn dispatch_occurrence(
        &self,
        definition: &SessionEventDefinition,
        occurrence: SessionEventOccurrence,
    ) -> Result<SessionEventResult, SessionEventApplicationError> {
        let command = SessionEventMaterializer::materialize(definition, occurrence)
            .map_err(SessionEventApplicationError::Materialization)?;
        self.dispatch(command)
    }

    fn store_result(
        &self,
        result: &SessionEventResult,
    ) -> Result<(), SessionEventApplicationError> {
        self.store
            .record(result.group.clone(), result.deliveries.clone())
            .map_err(SessionEventApplicationError::Store)
    }
}

#[derive(Debug)]
pub(crate) enum SessionEventApplicationError {
    Domain(SessionEventDomainError),
    Materialization(SessionEventMaterializationError),
    Directory(SessionDirectoryError),
    Store(SessionEventStoreError),
    NoTarget(ReferenceIdentity),
    AddressingAfterDispatch(String),
}

impl fmt::Display for SessionEventApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Materialization(error) => error.fmt(formatter),
            Self::Directory(error) => error.fmt(formatter),
            Self::Store(error) => error.fmt(formatter),
            Self::NoTarget(group) => {
                write!(formatter, "Session-event group {group} found no target")
            }
            Self::AddressingAfterDispatch(message) => write!(
                formatter,
                "Session invocation was attempted but its address marker failed: {message}"
            ),
        }
    }
}

impl Error for SessionEventApplicationError {}

fn validate_created_entry(
    command: &SessionEventCommand,
    requested_address: &SessionLogicalAddress,
    created: &SessionDirectoryEntry,
) -> Result<(), SessionEventApplicationError> {
    if created.logical_address.as_ref() != Some(requested_address)
        || created.created_by_event.as_ref() != Some(&command.event_group_id)
        || created.created_by_session.as_ref() != command.created_by_session.as_ref()
    {
        return Err(SessionEventApplicationError::Directory(
            SessionDirectoryError::new(
                "Session-directory adapter returned creation provenance inconsistent with its request",
            ),
        ));
    }
    Ok(())
}

fn delivery_identity(
    group: &ReferenceIdentity,
    ordinal: u32,
) -> Result<ReferenceIdentity, SessionEventApplicationError> {
    let encoded = serde_json::to_vec(&(group, ordinal)).map_err(|error| {
        SessionEventApplicationError::Domain(SessionEventDomainError::InvalidReference(format!(
            "Unable to encode Session Event delivery identity: {error}"
        )))
    })?;
    let mut digest = Sha256::new();
    digest.update(b"session-events/delivery/v1\0");
    digest.update(encoded);
    ReferenceIdentity::new(
        "session_events",
        "delivery",
        format!("{:x}", digest.finalize()),
    )
    .map_err(SessionEventApplicationError::Domain)
}

fn render_prompt(sources: &[PromptSource]) -> String {
    sources
        .iter()
        .map(PromptSource::text)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_optional_prompt(sources: &[PromptSource]) -> Option<String> {
    (!sources.is_empty()).then(|| render_prompt(sources))
}

fn build_empty_result(
    command: &SessionEventCommand,
    outcome: EventGroupOutcome,
) -> SessionEventResult {
    SessionEventResult {
        group: EventGroupRecord {
            event_group_id: command.event_group_id.clone(),
            definition_ref: command.definition_ref.clone(),
            trigger: command.trigger.clone(),
            source: command.source.clone(),
            prompt_sources: command.prompt_sources.clone(),
            created_session_prompt_sources: command.created_session_prompt_sources.clone(),
            target_selection: command.target.clone(),
            resolved_sessions: Vec::new(),
            created_session: None,
            outcome,
            delivery_count: 0,
        },
        deliveries: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
