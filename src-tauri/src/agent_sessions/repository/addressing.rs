//! Logical Session addresses and atomic addressed-session creation.

use super::{mapping::*, SqliteAgentSessionRepository};
use crate::agent_sessions::{
    domain::{validate_session, AgentSession, AgentSessionId},
    ports::{RepositoryError, RepositoryErrorKind},
    references::{parse_session_reference, session_reference},
};
use crate::session_events::{
    ReferenceIdentity, SessionDirectoryEntry, SessionDirectoryError, SessionLogicalAddress,
};
use rusqlite::{params, OptionalExtension};

impl crate::agent_sessions::ports::AgentSessionAddressStore for SqliteAgentSessionRepository {
    fn create_addressed_session(
        &self,
        session: AgentSession,
        address: &SessionLogicalAddress,
        event: &ReferenceIdentity,
        creator: Option<&ReferenceIdentity>,
    ) -> Result<(AgentSession, u64), RepositoryError> {
        SqliteAgentSessionRepository::create_addressed_session(
            self, session, address, event, creator,
        )
    }
    fn find_session_entry(
        &self,
        id: &AgentSessionId,
    ) -> Result<Option<SessionDirectoryEntry>, SessionDirectoryError> {
        use crate::agent_sessions::ports::AgentSessionRepository;
        let history = self
            .load_session_history(id)
            .map_err(|e| SessionDirectoryError::new(e.to_string()))?;
        let Some(history) = history else {
            return Ok(None);
        };
        let running = history
            .invocations
            .iter()
            .any(|i| i.invocation.status.is_active());
        let session = session_reference(id.as_str())?;
        Ok(self
            .load_address(id)?
            .map(|address| address.entry(session, running)))
    }
}

pub(super) const SCHEMA: &str = r#"
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

impl SqliteAgentSessionRepository {
    /// Persists a newly prepared Agent Session and its generic logical address in one transaction.
    /// This is the concrete atomic boundary used by the Session Event adapter.
    pub(crate) fn create_addressed_session(
        &self,
        session: AgentSession,
        logical_address: &SessionLogicalAddress,
        created_by_event: &ReferenceIdentity,
        created_by_session: Option<&ReferenceIdentity>,
    ) -> Result<(AgentSession, u64), RepositoryError> {
        validate_session(&session).map_err(contract_error)?;
        let created_by_event_json = to_json(created_by_event)?;
        let created_by_session_json = created_by_session.map(to_json).transpose()?;
        self.write("create addressed Agent Session", |transaction| {
            insert_session(transaction, &session)?;
            transaction
                .execute("INSERT INTO agent_session_address_clock DEFAULT VALUES", [])
                .map_err(sql_unavailable("allocate Session address sequence"))?;
            let created_sequence =
                u64::try_from(transaction.last_insert_rowid()).map_err(|_| {
                    RepositoryError::new(
                        RepositoryErrorKind::InvalidState,
                        "Invalid Session address sequence",
                    )
                })?;
            transaction
                .execute(
                    "INSERT INTO agent_session_addresses(session_id,scope_namespace,scope_kind,scope_id,subject_namespace,subject_kind,subject_id,created_by_event_json,created_by_session_json,created_sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                    params![
                        session.id.as_str(),
                        logical_address.scope.namespace(),
                        logical_address.scope.kind(),
                        logical_address.scope.id(),
                        logical_address.subject.namespace(),
                        logical_address.subject.kind(),
                        logical_address.subject.id(),
                        created_by_event_json,
                        created_by_session_json,
                        created_sequence,
                    ],
                )
                .map_err(sql_write("store Agent Session address"))?;
            Ok((session, created_sequence))
        })
    }

    pub(crate) fn list_logical_addresses(
        &self,
    ) -> Result<Vec<(AgentSessionId, SessionLogicalAddress)>, SessionDirectoryError> {
        self.database.read("list Session logical addresses", |connection| {
            let mut query = connection.prepare("SELECT session_id,scope_namespace,scope_kind,scope_id,subject_namespace,subject_kind,subject_id FROM agent_session_addresses")
                .map_err(|e| SessionDirectoryError::new(e.to_string()))?;
            let rows = query.query_map([], |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?, row.get::<_,String>(4)?, row.get::<_,String>(5)?, row.get::<_,String>(6)?)))
                .map_err(|e| SessionDirectoryError::new(e.to_string()))?;
            rows.map(|row| {
                let (id,sn,sk,si,tn,tk,ti) = row.map_err(|e| SessionDirectoryError::new(e.to_string()))?;
                let id = AgentSessionId::new(id).map_err(|e| SessionDirectoryError::new(e.to_string()))?;
                let scope = ReferenceIdentity::new(sn,sk,si).map_err(|e| SessionDirectoryError::new(e.to_string()))?;
                let subject = ReferenceIdentity::new(tn,tk,ti).map_err(|e| SessionDirectoryError::new(e.to_string()))?;
                Ok((id,SessionLogicalAddress::new(scope,subject)))
            }).collect()
        }).map_err(directory_error)
    }

    pub(crate) fn load_address(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<StoredAddressDomain>, SessionDirectoryError> {
        self.database.read("read Session addresses", |connection| {
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
        }).map_err(directory_error)
    }

    pub(crate) fn list_session_directory(
        &self,
        address: &SessionLogicalAddress,
    ) -> Result<Vec<SessionDirectoryEntry>, SessionDirectoryError> {
        self.database.read("read Session addresses", |connection| {
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
        }).map_err(directory_error)
    }

    pub(crate) fn mark_session_addressed(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<u64, SessionDirectoryError> {
        let session_id = parse_session_reference(session)?;
        self.database
            .write("mark Session addressed", |transaction| {
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
                Ok(sequence)
            })
            .map_err(directory_error)
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

pub(crate) struct StoredAddressDomain {
    logical_address: SessionLogicalAddress,
    created_by_event: Option<ReferenceIdentity>,
    created_by_session: Option<ReferenceIdentity>,
    created_sequence: u64,
    last_addressed_sequence: Option<u64>,
}

impl StoredAddressDomain {
    pub(crate) fn entry(self, session: ReferenceIdentity, running: bool) -> SessionDirectoryEntry {
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

fn directory_error(
    error: crate::persistence::ManagedOperationError<SessionDirectoryError>,
) -> SessionDirectoryError {
    match error {
        crate::persistence::ManagedOperationError::Domain(error) => error,
        crate::persistence::ManagedOperationError::Infrastructure(error) => {
            SessionDirectoryError::new(error.to_string())
        }
    }
}
