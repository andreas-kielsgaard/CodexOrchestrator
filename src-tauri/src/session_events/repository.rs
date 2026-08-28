use super::{
    EventDeliveryRecord, EventGroupRecord, ReferenceIdentity, SessionEventStore,
    SessionEventStoreError,
};
use rusqlite::{params, Connection};
use std::{path::Path, sync::Mutex};

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
    connection: Mutex<Connection>,
}

impl SqliteSessionEventStore {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, SessionEventStoreError> {
        let connection = Connection::open(path).map_err(|error| {
            SessionEventStoreError::new(format!("Unable to open Session-event storage: {error}"))
        })?;
        crate::storage::configure_sqlite_connection(&connection).map_err(|error| {
            SessionEventStoreError::new(format!(
                "Unable to configure Session-event storage: {error}"
            ))
        })?;
        connection
            .execute_batch(SESSION_EVENT_SCHEMA)
            .map_err(|error| {
                SessionEventStoreError::new(format!(
                    "Unable to initialize Session-event storage: {error}"
                ))
            })?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
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
        let connection = self
            .connection
            .lock()
            .map_err(|_| SessionEventStoreError::new("Session-event storage lock was poisoned"))?;
        let transaction = connection.unchecked_transaction().map_err(|error| {
            SessionEventStoreError::new(format!(
                "Unable to begin Session-event persistence: {error}"
            ))
        })?;
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
        transaction.commit().map_err(|error| {
            SessionEventStoreError::new(format!(
                "Unable to commit Session-event persistence: {error}"
            ))
        })
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
