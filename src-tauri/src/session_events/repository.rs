use super::{
    EventDeliveryRecord, EventGroupRecord, ReferenceIdentity, SessionEventStore,
    SessionEventStoreError,
};
use rusqlite::{params, Connection};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use crate::persistence::{ActiveDatabase, ManagedOperationError};

pub(crate) const SESSION_EVENT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS session_event_groups (
    id TEXT PRIMARY KEY,
    record_json TEXT NOT NULL CHECK (json_valid(record_json)),
    recorded_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_event_deliveries (
    id TEXT PRIMARY KEY,
    event_group_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal > 0),
    record_json TEXT NOT NULL CHECK (json_valid(record_json)),
    FOREIGN KEY (event_group_id) REFERENCES session_event_groups(id) ON DELETE CASCADE,
    UNIQUE (event_group_id, ordinal)
);

CREATE INDEX IF NOT EXISTS session_event_deliveries_by_group
ON session_event_deliveries(event_group_id, ordinal);
"#;

pub(crate) struct SqliteSessionEventStore {
    database: Arc<ActiveDatabase>,
}

impl SqliteSessionEventStore {
    pub(crate) fn from_database(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }

    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, SessionEventStoreError> {
        ActiveDatabase::open(path, initialize_session_event_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(|error| SessionEventStoreError::new(error.to_string()))
    }

    fn read<T>(
        &self,
        operation: &'static str,
        read: impl FnOnce(&Connection) -> Result<T, SessionEventStoreError>,
    ) -> Result<T, SessionEventStoreError> {
        self.database.read(operation, read).map_err(managed_error)
    }

    fn write<T>(
        &self,
        operation: &'static str,
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, SessionEventStoreError>,
    ) -> Result<T, SessionEventStoreError> {
        self.database.write(operation, write).map_err(managed_error)
    }
}

impl SessionEventStore for SqliteSessionEventStore {
    fn record(
        &self,
        group: EventGroupRecord,
        deliveries: Vec<EventDeliveryRecord>,
    ) -> Result<(), SessionEventStoreError> {
        let group_json = serde_json::to_string(&group)
            .map_err(|error| SessionEventStoreError::new(error.to_string()))?;
        let group_key = persistence_key(&group.event_group_id)?;
        let delivery_json = deliveries
            .iter()
            .map(|delivery| {
                serde_json::to_string(delivery)
                    .map(|json| (delivery, json))
                    .map_err(|error| SessionEventStoreError::new(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.write("record Session Event", |transaction| {
        transaction
            .execute(
                "INSERT INTO session_event_groups(id,record_json,recorded_at) VALUES(?1,?2,?3)",
                params![
                    group_key.as_str(),
                    group_json,
                    chrono::Utc::now().to_rfc3339(),
                ],
            )
            .map_err(|error| {
                SessionEventStoreError::new(format!(
                    "Unable to persist Session-event group {}: {error}",
                    reference_label(&group.event_group_id)
                ))
            })?;
        for (delivery, json) in delivery_json {
            transaction
                .execute(
                    "INSERT INTO session_event_deliveries(id,event_group_id,ordinal,record_json) VALUES(?1,?2,?3,?4)",
                    params![
                        persistence_key(&delivery.delivery_id)?,
                        group_key.as_str(),
                        delivery.ordinal,
                        json,
                    ],
                )
                .map_err(|error| {
                    SessionEventStoreError::new(format!(
                        "Unable to persist Session-event delivery {}: {error}",
                        reference_label(&delivery.delivery_id)
                    ))
                })?;
        }
        Ok(())
        })
    }

    fn event_group(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Option<EventGroupRecord>, SessionEventStoreError> {
        self.read("load Session Event group", |connection| {
            let mut statement = connection
                .prepare("SELECT record_json FROM session_event_groups WHERE id = ?1")
                .map_err(|error| {
                    SessionEventStoreError::new(format!(
                        "Unable to prepare Session-event group query: {error}"
                    ))
                })?;
            let mut rows = statement
                .query(params![persistence_key(event_group_id)?])
                .map_err(|error| {
                    SessionEventStoreError::new(format!(
                        "Unable to query Session-event group {}: {error}",
                        reference_label(event_group_id)
                    ))
                })?;
            let Some(row) = rows.next().map_err(|error| {
                SessionEventStoreError::new(format!(
                    "Unable to read Session-event group {}: {error}",
                    reference_label(event_group_id)
                ))
            })?
            else {
                return Ok(None);
            };
            let json = row.get::<_, String>(0).map_err(|error| {
                SessionEventStoreError::new(format!(
                    "Unable to decode stored Session-event group {}: {error}",
                    reference_label(event_group_id)
                ))
            })?;
            deserialize_group(&json).map(Some)
        })
    }

    fn deliveries_for_group(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Vec<EventDeliveryRecord>, SessionEventStoreError> {
        self.read("list Session Event deliveries for group", |connection| {
            let mut statement = connection
                .prepare(
                    "SELECT record_json FROM session_event_deliveries \
                 WHERE event_group_id = ?1 ORDER BY ordinal",
                )
                .map_err(|error| {
                    SessionEventStoreError::new(format!(
                        "Unable to prepare Session-event delivery query: {error}"
                    ))
                })?;
            let rows = statement
                .query_map(params![persistence_key(event_group_id)?], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|error| {
                    SessionEventStoreError::new(format!(
                        "Unable to query deliveries for Session-event group {}: {error}",
                        reference_label(event_group_id)
                    ))
                })?;
            rows.map(|row| {
                let json = row.map_err(|error| {
                    SessionEventStoreError::new(format!(
                        "Unable to read stored Session-event delivery: {error}"
                    ))
                })?;
                deserialize_delivery(&json)
            })
            .collect()
        })
    }

    fn deliveries_for_session(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<Vec<EventDeliveryRecord>, SessionEventStoreError> {
        self.read("list Session Event deliveries for Session", |connection| {
            let mut statement = connection
                .prepare(
                    "SELECT record_json FROM session_event_deliveries \
                 ORDER BY rowid",
                )
                .map_err(|error| {
                    SessionEventStoreError::new(format!(
                        "Unable to prepare Session-linked delivery query: {error}"
                    ))
                })?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|error| {
                    SessionEventStoreError::new(format!(
                        "Unable to query deliveries for Session {}: {error}",
                        reference_label(session)
                    ))
                })?;
            let deliveries = rows
                .map(|row| {
                    let json = row.map_err(|error| {
                        SessionEventStoreError::new(format!(
                            "Unable to read stored Session-event delivery: {error}"
                        ))
                    })?;
                    deserialize_delivery(&json)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(deliveries
                .into_iter()
                .filter(|delivery| &delivery.target_session == session)
                .collect())
        })
    }
}

pub(crate) fn initialize_session_event_storage(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(SESSION_EVENT_SCHEMA)
        .map_err(|error| format!("Unable to initialize Session-event storage: {error}"))
}

fn managed_error(error: ManagedOperationError<SessionEventStoreError>) -> SessionEventStoreError {
    match error {
        ManagedOperationError::Infrastructure(error) => {
            SessionEventStoreError::new(error.to_string())
        }
        ManagedOperationError::Domain(error) => error,
    }
}

#[derive(Default)]
pub(crate) struct InMemorySessionEventStore {
    records: Mutex<Vec<(EventGroupRecord, Vec<EventDeliveryRecord>)>>,
}

impl InMemorySessionEventStore {
    pub(crate) fn records(
        &self,
    ) -> Result<Vec<(EventGroupRecord, Vec<EventDeliveryRecord>)>, SessionEventStoreError> {
        self.records
            .lock()
            .map(|records| records.clone())
            .map_err(|_| SessionEventStoreError::new("Session-event store lock was poisoned"))
    }
}

impl SessionEventStore for InMemorySessionEventStore {
    fn record(
        &self,
        group: EventGroupRecord,
        deliveries: Vec<EventDeliveryRecord>,
    ) -> Result<(), SessionEventStoreError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| SessionEventStoreError::new("Session-event store lock was poisoned"))?;
        if records
            .iter()
            .any(|(existing, _)| existing.event_group_id == group.event_group_id)
        {
            return Err(SessionEventStoreError::new(format!(
                "Session-event group {} was already recorded",
                reference_label(&group.event_group_id)
            )));
        }
        records.push((group, deliveries));
        Ok(())
    }

    fn event_group(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Option<EventGroupRecord>, SessionEventStoreError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| SessionEventStoreError::new("Session-event store lock was poisoned"))?
            .iter()
            .find(|(group, _)| &group.event_group_id == event_group_id)
            .map(|(group, _)| group.clone()))
    }

    fn deliveries_for_group(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Vec<EventDeliveryRecord>, SessionEventStoreError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| SessionEventStoreError::new("Session-event store lock was poisoned"))?
            .iter()
            .find(|(group, _)| &group.event_group_id == event_group_id)
            .map(|(_, deliveries)| deliveries.clone())
            .unwrap_or_default())
    }

    fn deliveries_for_session(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<Vec<EventDeliveryRecord>, SessionEventStoreError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| SessionEventStoreError::new("Session-event store lock was poisoned"))?
            .iter()
            .flat_map(|(_, deliveries)| deliveries)
            .filter(|delivery| &delivery.target_session == session)
            .cloned()
            .collect())
    }
}

fn deserialize_group(json: &str) -> Result<EventGroupRecord, SessionEventStoreError> {
    serde_json::from_str(json).map_err(|error| {
        SessionEventStoreError::new(format!(
            "Unable to deserialize stored Session-event group: {error}"
        ))
    })
}

fn deserialize_delivery(json: &str) -> Result<EventDeliveryRecord, SessionEventStoreError> {
    serde_json::from_str(json).map_err(|error| {
        SessionEventStoreError::new(format!(
            "Unable to deserialize stored Session-event delivery: {error}"
        ))
    })
}

fn reference_label(reference: &ReferenceIdentity) -> String {
    format!(
        "{}:{}/{}",
        reference.namespace(),
        reference.kind(),
        reference.id()
    )
}

fn persistence_key(reference: &ReferenceIdentity) -> Result<String, SessionEventStoreError> {
    serde_json::to_string(reference).map_err(|error| {
        SessionEventStoreError::new(format!("Unable to encode reference identity: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_keys_include_the_complete_reference_identity() {
        let first = ReferenceIdentity::new("workflow", "event_group", "shared").unwrap();
        let other_namespace =
            ReferenceIdentity::new("agent_sessions", "event_group", "shared").unwrap();
        let other_kind = ReferenceIdentity::new("workflow", "delivery", "shared").unwrap();

        assert_ne!(
            persistence_key(&first).unwrap(),
            persistence_key(&other_namespace).unwrap()
        );
        assert_ne!(
            persistence_key(&first).unwrap(),
            persistence_key(&other_kind).unwrap()
        );
    }
}
