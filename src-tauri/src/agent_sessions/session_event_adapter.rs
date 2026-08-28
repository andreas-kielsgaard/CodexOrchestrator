use super::{
    application::{
        AgentSessionApplication, AgentSessionOwnership, CreateAgentSessionCommand,
        SendAgentSessionMessageCommand, SendIdempotentApplicationAgentSessionMessageCommand,
    },
    domain::{AgentInvocationId, AgentRuntimeOptions, AgentSessionId, RuntimeSandboxMode},
    ports::{InitialPromptPrefix, RuntimeLaunchExtension},
    repository::SqliteAgentSessionRepository,
};
use crate::{
    execution_configuration::{
        DirectUserInvocationRequest, RuntimeSelections, SandboxMode, SelectedRuntimeProfileSource,
        SessionCreationRequest, SessionProfileResolver,
    },
    session_events::{
        ReferenceIdentity, SessionCreationSpec, SessionDirectory, SessionDirectoryEntry,
        SessionDirectoryError, SessionEventSource, SessionInvocationDispatcher,
        SessionInvocationError, SessionInvocationReceipt, SessionInvocationRequest,
        SessionLogicalAddress,
    },
};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

pub(crate) const SESSION_ADDRESS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_address_clock (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT
);

CREATE TABLE IF NOT EXISTS agent_session_addresses (
    session_id TEXT PRIMARY KEY,
    scope_namespace TEXT NOT NULL,
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    subject_namespace TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    created_by_event_json TEXT CHECK (created_by_event_json IS NULL OR json_valid(created_by_event_json)),
    created_by_session_json TEXT CHECK (created_by_session_json IS NULL OR json_valid(created_by_session_json)),
    created_sequence INTEGER NOT NULL CHECK (created_sequence > 0),
    last_addressed_sequence INTEGER CHECK (last_addressed_sequence IS NULL OR last_addressed_sequence > 0),
    FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS agent_session_addresses_by_logical_address
ON agent_session_addresses(
    scope_namespace, scope_kind, scope_id,
    subject_namespace, subject_kind, subject_id,
    created_sequence DESC
);
"#;

const SESSION_REF_NAMESPACE: &str = "orchestrator.agent_sessions";
const SESSION_REF_KIND: &str = "session";
const CREATION_REQUEST_NAMESPACE: &str = "orchestrator.execution_configuration";
const CREATION_REQUEST_KIND: &str = "session_creation_request";
const CREATION_REQUEST_VERSION: &str = "v1";

/// The concrete application boundary between generic Session addressing and managed Agent
/// Sessions. Workflow identities remain opaque references on this side of the port.
pub(crate) struct AgentSessionEventAdapter {
    application: Arc<AgentSessionApplication>,
    repository: Arc<SqliteAgentSessionRepository>,
    profile_source: Arc<dyn SelectedRuntimeProfileSource>,
    connection: Mutex<Connection>,
}

impl AgentSessionEventAdapter {
    pub(crate) fn open(
        path: impl AsRef<std::path::Path>,
        application: Arc<AgentSessionApplication>,
        repository: Arc<SqliteAgentSessionRepository>,
        profile_source: Arc<dyn SelectedRuntimeProfileSource>,
    ) -> Result<Self, String> {
        let connection = Connection::open(path)
            .map_err(|error| format!("Unable to open Agent Session address storage: {error}"))?;
        crate::storage::configure_sqlite_connection(&connection).map_err(|error| {
            format!("Unable to configure Agent Session address storage: {error}")
        })?;
        connection
            .execute_batch(SESSION_ADDRESS_SCHEMA)
            .map_err(|error| {
                format!("Unable to initialize Agent Session address storage: {error}")
            })?;
        Ok(Self {
            application,
            repository,
            profile_source,
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, SessionDirectoryError> {
        self.connection.lock().map_err(|_| {
            SessionDirectoryError::new("Agent Session address storage lock is poisoned")
        })
    }

    fn load_address(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<StoredAddressDomain>, SessionDirectoryError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT scope_namespace,scope_kind,scope_id,subject_namespace,subject_kind,subject_id,created_by_event_json,created_by_session_json,created_sequence,last_addressed_sequence FROM agent_session_addresses WHERE session_id=?1",
                [session_id.as_str()],
                stored_address_row,
            )
            .optional()
            .map_err(|error| SessionDirectoryError::new(format!("Unable to load Agent Session address: {error}")))?
            .map(StoredAddress::into_domain)
            .transpose()
    }
}

impl SessionDirectory for AgentSessionEventAdapter {
    fn find_exact(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<Option<SessionDirectoryEntry>, SessionDirectoryError> {
        let session_id = parse_session_reference(session)?;
        let history = match self.application.load_session(&session_id) {
            Ok(history) => history,
            Err(error) if error.to_string() == "Agent Session not found" => return Ok(None),
            Err(error) => return Err(SessionDirectoryError::new(error.to_string())),
        };
        let running = history
            .invocations
            .iter()
            .any(|history| history.invocation.status.is_active());
        let stored = self.load_address(&session_id)?;
        Ok(Some(match stored {
            Some(stored) => stored.entry(session.clone(), running),
            None => SessionDirectoryEntry {
                session: session.clone(),
                logical_address: None,
                running,
                created_sequence: 0,
                last_addressed_sequence: None,
                created_by_event: None,
                created_by_session: None,
            },
        }))
    }

    fn list_at_address(
        &self,
        address: &SessionLogicalAddress,
    ) -> Result<Vec<SessionDirectoryEntry>, SessionDirectoryError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT address.session_id,address.scope_namespace,address.scope_kind,address.scope_id,address.subject_namespace,address.subject_kind,address.subject_id,address.created_by_event_json,address.created_by_session_json,address.created_sequence,address.last_addressed_sequence,EXISTS(SELECT 1 FROM agent_session_invocations invocation WHERE invocation.session_id=address.session_id AND invocation.status IN ('pending','running')) FROM agent_session_addresses address JOIN agent_sessions session ON session.id=address.session_id WHERE session.availability='available' AND address.scope_namespace=?1 AND address.scope_kind=?2 AND address.scope_id=?3 AND address.subject_namespace=?4 AND address.subject_kind=?5 AND address.subject_id=?6",
            )
            .map_err(|error| SessionDirectoryError::new(format!("Unable to prepare logical Session lookup: {error}")))?;
        let rows = statement
            .query_map(
                params![
                    address.scope.namespace(),
                    address.scope.kind(),
                    address.scope.id(),
                    address.subject.namespace(),
                    address.subject.kind(),
                    address.subject.id(),
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        StoredAddress {
                            scope_namespace: row.get(1)?,
                            scope_kind: row.get(2)?,
                            scope_id: row.get(3)?,
                            subject_namespace: row.get(4)?,
                            subject_kind: row.get(5)?,
                            subject_id: row.get(6)?,
                            created_by_event_json: row.get(7)?,
                            created_by_session_json: row.get(8)?,
                            created_sequence: row.get(9)?,
                            last_addressed_sequence: row.get(10)?,
                        },
                        row.get::<_, bool>(11)?,
                    ))
                },
            )
            .map_err(|error| {
                SessionDirectoryError::new(format!("Unable to query logical Sessions: {error}"))
            })?;
        rows.map(|row| {
            let (session_id, stored, running) = row.map_err(|error| {
                SessionDirectoryError::new(format!("Unable to read logical Session: {error}"))
            })?;
            let session = session_reference(&session_id)?;
            Ok(stored.into_domain()?.entry(session, running))
        })
        .collect()
    }

    fn create_session(
        &self,
        request: SessionCreationSpec,
    ) -> Result<SessionDirectoryEntry, SessionDirectoryError> {
        validate_creation_contract(&request.configuration.contract)?;
        let creation_request: SessionCreationRequest =
            serde_json::from_value(request.configuration.payload).map_err(|error| {
                SessionDirectoryError::new(format!(
                    "Pinned Session creation request is invalid: {error}"
                ))
            })?;
        let resolution = SessionProfileResolver::resolve_creation(
            self.profile_source.as_ref(),
            creation_request,
        )
        .map_err(|error| SessionDirectoryError::new(error.to_string()))?;
        let requested_options = runtime_options(resolution.session_profile().pinned_defaults());
        let session_id = session_id_for_creation(&request.event_group_id)?;
        let session = self.application.prepare_session_with_id(
            CreateAgentSessionCommand {
                title: Some(request.logical_address.subject.id().to_string()),
                working_directory: None,
                requested_options,
            },
            session_id.clone(),
            AgentSessionOwnership {
                harness_version: None,
                assigned_identity: None,
                session_profile: Some(resolution.clone()),
            },
        );
        let session_ref = session_reference(session.id.as_str())?;
        let (_session, created_sequence) = match self.repository.create_addressed_session(
            session,
            &request.logical_address,
            &request.created_by_event,
            request.created_by_session.as_ref(),
        ) {
            Ok(created) => created,
            Err(error)
                if error.kind == crate::agent_sessions::ports::RepositoryErrorKind::Conflict =>
            {
                let history = self
                    .application
                    .load_session(&session_id)
                    .map_err(|load_error| SessionDirectoryError::new(load_error.to_string()))?;
                if history.session.session_profile.as_ref() != Some(&resolution) {
                    return Err(SessionDirectoryError::new(
                        "Session Event creation identity was already used for a different pinned profile",
                    ));
                }
                let existing = self.find_exact(&session_ref)?.ok_or_else(|| {
                    SessionDirectoryError::new(
                        "Session Event creation conflicted without an addressed Session",
                    )
                })?;
                if existing.logical_address.as_ref() != Some(&request.logical_address)
                    || existing.created_by_event.as_ref() != Some(&request.created_by_event)
                    || existing.created_by_session.as_ref() != request.created_by_session.as_ref()
                {
                    return Err(SessionDirectoryError::new(
                        "Session Event creation identity was already used for different addressing semantics",
                    ));
                }
                return Ok(existing);
            }
            Err(error) => return Err(SessionDirectoryError::new(error.to_string())),
        };
        Ok(SessionDirectoryEntry {
            session: session_ref,
            logical_address: Some(request.logical_address),
            running: false,
            created_sequence,
            last_addressed_sequence: None,
            created_by_event: Some(request.created_by_event),
            created_by_session: request.created_by_session,
        })
    }

    fn mark_addressed(
        &self,
        session: &ReferenceIdentity,
        _event_group_id: &ReferenceIdentity,
    ) -> Result<u64, SessionDirectoryError> {
        let session_id = parse_session_reference(session)?;
        let connection = self.lock()?;
        let transaction = connection.unchecked_transaction().map_err(|error| {
            SessionDirectoryError::new(format!("Unable to begin Session address update: {error}"))
        })?;
        transaction
            .execute("INSERT INTO agent_session_address_clock DEFAULT VALUES", [])
            .map_err(|error| {
                SessionDirectoryError::new(format!(
                    "Unable to allocate Session address sequence: {error}"
                ))
            })?;
        let sequence = u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| SessionDirectoryError::new("Invalid Session address sequence"))?;
        let changed = transaction
            .execute(
                "UPDATE agent_session_addresses SET last_addressed_sequence=?1 WHERE session_id=?2",
                params![sequence, session_id.as_str()],
            )
            .map_err(|error| {
                SessionDirectoryError::new(format!("Unable to mark Session addressed: {error}"))
            })?;
        if changed != 1 {
            return Err(SessionDirectoryError::new(
                "Addressed Session has no logical address record",
            ));
        }
        transaction.commit().map_err(|error| {
            SessionDirectoryError::new(format!("Unable to commit Session address update: {error}"))
        })?;
        Ok(sequence)
    }
}

impl SessionInvocationDispatcher for AgentSessionEventAdapter {
    fn dispatch(
        &self,
        request: SessionInvocationRequest,
    ) -> Result<SessionInvocationReceipt, SessionInvocationError> {
        let SessionInvocationRequest {
            event_group_id,
            delivery_id,
            target_session,
            source,
            prompt,
            initial_prompt,
            direct_user_options,
        } = request;
        let session_id = parse_session_reference(&target_session)
            .map_err(|error| SessionInvocationError::new(error.to_string()))?;
        let history = self
            .application
            .load_session(&session_id)
            .map_err(|error| SessionInvocationError::new(error.to_string()))?;
        let creation = history.session.session_profile.as_ref().ok_or_else(|| {
            SessionInvocationError::new(
                "Session Event delivery requires an immutable pinned Session Profile",
            )
        })?;
        let selections = match direct_user_options {
            Some(options) => {
                SessionProfileResolver::validate_direct_user_invocation(
                    self.profile_source.as_ref(),
                    creation,
                    DirectUserInvocationRequest {
                        contract_version: 1,
                        model: options.model,
                        reasoning_mode: options.reasoning_mode,
                    },
                )
                .map_err(|error| SessionInvocationError::new(error.to_string()))?
                .selections
            }
            None => {
                SessionProfileResolver::validate_pinned_session(
                    self.profile_source.as_ref(),
                    creation,
                )
                .map_err(|error| SessionInvocationError::new(error.to_string()))?;
                creation.session_profile().pinned_defaults().clone()
            }
        };
        let requested_options = runtime_options(&selections);
        let mut extension = RuntimeLaunchExtension::default();
        if let Some(reasoning) = &selections.reasoning_mode {
            extension.additional_args = vec![
                "-c".to_string(),
                format!("model_reasoning_effort=\"{reasoning}\""),
            ];
        }
        if let Some(initial_prompt) = initial_prompt {
            extension.initial_prompt_prefix = Some(InitialPromptPrefix {
                source: format!("session_event:{event_group_id}"),
                version: 1,
                content: initial_prompt,
            });
        }
        let invocation_id = invocation_id_for_delivery(&delivery_id)
            .map_err(|error| SessionInvocationError::new(error.to_string()))?;
        let direct_user = matches!(source, SessionEventSource::UserRequest { .. });
        let command = SendIdempotentApplicationAgentSessionMessageCommand {
            invocation_id,
            message: SendAgentSessionMessageCommand {
                session_id: Some(session_id),
                submitted_text: prompt,
                title: None,
                working_directory: history.session.working_directory,
                requested_options: Some(requested_options),
            },
        };
        let extension = (extension != RuntimeLaunchExtension::default()).then_some(extension);
        let launched = if direct_user {
            self.application
                .send_idempotent_user_message_with_launch_observation(command, extension)
        } else {
            self.application
                .send_idempotent_application_message_with_launch_observation(command, extension)
        }
        .map_err(|error| SessionInvocationError::new(error.to_string()))?;
        if !launched.launch_accepted {
            return Err(SessionInvocationError::new(
                "Agent runtime did not accept the Session Event delivery",
            ));
        }
        let invocation =
            session_invocation_reference(launched.acknowledgement.invocation_id.as_str())
                .map_err(|error| SessionInvocationError::new(error.to_string()))?;
        Ok(SessionInvocationReceipt { invocation })
    }
}

#[derive(Debug)]
struct StoredAddress {
    scope_namespace: String,
    scope_kind: String,
    scope_id: String,
    subject_namespace: String,
    subject_kind: String,
    subject_id: String,
    created_by_event_json: Option<String>,
    created_by_session_json: Option<String>,
    created_sequence: u64,
    last_addressed_sequence: Option<u64>,
}

impl StoredAddress {
    fn into_domain(self) -> Result<StoredAddressDomain, SessionDirectoryError> {
        Ok(StoredAddressDomain {
            logical_address: SessionLogicalAddress::new(
                ReferenceIdentity::new(self.scope_namespace, self.scope_kind, self.scope_id)
                    .map_err(|error| SessionDirectoryError::new(error.to_string()))?,
                ReferenceIdentity::new(self.subject_namespace, self.subject_kind, self.subject_id)
                    .map_err(|error| SessionDirectoryError::new(error.to_string()))?,
            ),
            created_by_event: decode_optional_reference(self.created_by_event_json)?,
            created_by_session: decode_optional_reference(self.created_by_session_json)?,
            created_sequence: self.created_sequence,
            last_addressed_sequence: self.last_addressed_sequence,
        })
    }
}

struct StoredAddressDomain {
    logical_address: SessionLogicalAddress,
    created_by_event: Option<ReferenceIdentity>,
    created_by_session: Option<ReferenceIdentity>,
    created_sequence: u64,
    last_addressed_sequence: Option<u64>,
}

impl StoredAddressDomain {
    fn entry(self, session: ReferenceIdentity, running: bool) -> SessionDirectoryEntry {
        SessionDirectoryEntry {
            session,
            logical_address: Some(self.logical_address),
            running,
            created_sequence: self.created_sequence,
            last_addressed_sequence: self.last_addressed_sequence,
            created_by_event: self.created_by_event,
            created_by_session: self.created_by_session,
        }
    }
}

fn stored_address_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredAddress> {
    Ok(StoredAddress {
        scope_namespace: row.get(0)?,
        scope_kind: row.get(1)?,
        scope_id: row.get(2)?,
        subject_namespace: row.get(3)?,
        subject_kind: row.get(4)?,
        subject_id: row.get(5)?,
        created_by_event_json: row.get(6)?,
        created_by_session_json: row.get(7)?,
        created_sequence: row.get(8)?,
        last_addressed_sequence: row.get(9)?,
    })
}

fn decode_optional_reference(
    value: Option<String>,
) -> Result<Option<ReferenceIdentity>, SessionDirectoryError> {
    value
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| SessionDirectoryError::new(error.to_string()))
}

fn validate_creation_contract(contract: &ReferenceIdentity) -> Result<(), SessionDirectoryError> {
    if contract.namespace() == CREATION_REQUEST_NAMESPACE
        && contract.kind() == CREATION_REQUEST_KIND
        && contract.id() == CREATION_REQUEST_VERSION
    {
        Ok(())
    } else {
        Err(SessionDirectoryError::new(format!(
            "Unsupported Session creation configuration contract `{contract}`"
        )))
    }
}

fn parse_session_reference(
    reference: &ReferenceIdentity,
) -> Result<AgentSessionId, SessionDirectoryError> {
    if reference.namespace() != SESSION_REF_NAMESPACE || reference.kind() != SESSION_REF_KIND {
        return Err(SessionDirectoryError::new(format!(
            "Reference `{reference}` is not an Agent Session"
        )));
    }
    AgentSessionId::new(reference.id().to_string())
        .map_err(|error| SessionDirectoryError::new(error.to_string()))
}

fn session_reference(id: &str) -> Result<ReferenceIdentity, SessionDirectoryError> {
    ReferenceIdentity::new(SESSION_REF_NAMESPACE, SESSION_REF_KIND, id)
        .map_err(|error| SessionDirectoryError::new(error.to_string()))
}

fn session_id_for_creation(
    event_group: &ReferenceIdentity,
) -> Result<AgentSessionId, SessionDirectoryError> {
    let encoded = serde_json::to_vec(event_group)
        .map_err(|error| SessionDirectoryError::new(error.to_string()))?;
    let mut digest = Sha256::new();
    digest.update(b"agent-sessions/session-event-creation/v1\0");
    digest.update(encoded);
    AgentSessionId::new(format!("session-event-{:x}", digest.finalize()))
        .map_err(|error| SessionDirectoryError::new(error.to_string()))
}

fn session_invocation_reference(
    id: &str,
) -> Result<ReferenceIdentity, crate::session_events::SessionEventDomainError> {
    ReferenceIdentity::new("orchestrator.agent_sessions", "invocation", id)
}

fn invocation_id_for_delivery(
    delivery: &ReferenceIdentity,
) -> Result<AgentInvocationId, crate::agent_sessions::domain::ContractViolation> {
    AgentInvocationId::new(format!(
        "session-event:{}:{}:{}:{}:{}:{}",
        delivery.namespace().len(),
        delivery.namespace(),
        delivery.kind().len(),
        delivery.kind(),
        delivery.id().len(),
        delivery.id(),
    ))
}

fn runtime_options(selections: &RuntimeSelections) -> AgentRuntimeOptions {
    AgentRuntimeOptions {
        model: selections.model.clone(),
        sandbox: selections.sandbox_mode.map(|sandbox| match sandbox {
            SandboxMode::ReadOnly => RuntimeSandboxMode::ReadOnly,
            SandboxMode::WorkspaceWrite => RuntimeSandboxMode::WorkspaceWrite,
            SandboxMode::DangerFullAccess => RuntimeSandboxMode::DangerFullAccess,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invocation_identity_preserves_the_complete_delivery_reference() {
        let first = ReferenceIdentity::new("a-b", "delivery", "shared").unwrap();
        let second = ReferenceIdentity::new("a", "b-delivery", "shared").unwrap();

        assert_ne!(
            invocation_id_for_delivery(&first).unwrap(),
            invocation_id_for_delivery(&second).unwrap()
        );
    }
}
