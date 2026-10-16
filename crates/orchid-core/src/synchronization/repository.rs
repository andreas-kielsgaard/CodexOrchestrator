use super::*;
use crate::persistence::ActiveDatabase;
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;

#[derive(Clone)]
pub struct ReplicaStore {
    database: Arc<ActiveDatabase>,
}
impl ReplicaStore {
    pub fn new(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }
    pub fn device_id(&self) -> Result<String, String> {
        self.database
            .read("read instance identity", |db| {
                db.query_row(
                    "SELECT device_id FROM orchid_instance WHERE singleton=1",
                    [],
                    |r| r.get(0),
                )
            })
            .map_err(|e| e.to_string())
    }
    pub fn pending_sessions(&self, after: u64) -> Result<Vec<(String, u64)>, String> {
        self.database.read("read committed sync journal", |db| {
            db.prepare("SELECT session_id,MAX(sequence) FROM orchid_sync_changes WHERE sequence>?1 GROUP BY session_id ORDER BY MAX(sequence) LIMIT 100")?
                .query_map([after],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<Result<Vec<_>,rusqlite::Error>>()
        }).map_err(|e|e.to_string())
    }
    pub fn cursor(&self, peer: &str) -> Result<(u64, u64), String> {
        self.database
            .read("read synchronization cursor", |db| {
                db.query_row(
                    "SELECT sent,received FROM orchid_sync_cursors WHERE peer=?1",
                    [peer],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
            })
            .map(|v| v.unwrap_or_default())
            .map_err(|e| e.to_string())
    }
    pub fn acknowledge(&self, peer: &str, sent: u64, received: u64) -> Result<(), String> {
        self.database.write("save synchronization cursor",|db|db.execute("INSERT INTO orchid_sync_cursors VALUES(?1,?2,?3) ON CONFLICT(peer) DO UPDATE SET sent=MAX(sent,excluded.sent),received=MAX(received,excluded.received)",params![peer,sent,received])).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn accept(&self, source: &str, change: &SessionChange) -> Result<bool, String> {
        if source != change.device_id {
            return Err("Session changes must come from their execution owner".into());
        }
        let id = change.session.history.session.id.as_str();
        if change.session.history.invocations.iter().any(|turn| {
            turn.invocation.session_id.as_str() != id
                || turn
                    .events
                    .iter()
                    .any(|event| event.invocation_id != turn.invocation.id)
        }) {
            return Err(
                "Replicated history contains foreign invocation or event identities".into(),
            );
        }
        let json = serde_json::to_string(change).map_err(|e| e.to_string())?;
        self.database.write("import session observation",|db|->Result<bool,String>{
            let local:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM agent_sessions WHERE id=?1) AND NOT EXISTS(SELECT 1 FROM orchid_session_authority WHERE session_id=?1 AND owner!=(SELECT device_id FROM orchid_instance WHERE singleton=1))",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
            if local {return Err("Cannot replace locally owned execution with a replica".into());}
            let owner:Option<String>=db.query_row("SELECT device_id FROM orchid_session_replica_summaries WHERE session_id=?1 UNION SELECT device_id FROM orchid_session_replicas WHERE session_id=?1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
            let previous:Option<String>=db.query_row("SELECT transfers_json FROM orchid_session_authority WHERE session_id=?1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
            let previous:Vec<OwnershipTransfer>=previous.map(|s|serde_json::from_str(&s).map_err(|e|e.to_string())).transpose()?.unwrap_or_default();
            if change.transfers.len()<previous.len(){return Ok(false);}
            if !change.transfers.starts_with(&previous) || change.transfers.windows(2).any(|p|p[0].destination!=p[1].source) {return Err("Conflicting execution ownership history".into());}
            if let Some(last)=change.transfers.last(){if last.destination!=source{return Err("Snapshot does not come from the current execution owner".into());}}
            let changed_owner=owner.as_deref().is_some_and(|owner|owner!=source);
            if changed_owner && (change.transfers.len()==previous.len() || change.transfers.get(previous.len()).is_none_or(|t|Some(t.source.as_str())!=owner.as_deref())) {return Err("Session ownership changed without an explicit handoff".into());}
            let known:Option<u64>=db.query_row("SELECT source_sequence FROM orchid_session_replica_summaries WHERE session_id=?1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
            if !changed_owner && known.is_some_and(|seq|seq>change.sequence){return Ok(false);}
            let updated=db.execute("INSERT INTO orchid_session_replicas VALUES(?1,?2,?3,?4) ON CONFLICT(session_id) DO UPDATE SET device_id=excluded.device_id,source_sequence=excluded.source_sequence,payload_json=excluded.payload_json WHERE excluded.device_id!=device_id OR excluded.source_sequence>source_sequence",params![id,source,change.sequence,json]).map_err(|e|e.to_string())?;
            if updated==0{return Ok(false);}
            db.execute("INSERT INTO orchid_session_replica_summaries VALUES(?1,?2,?3,?4) ON CONFLICT(session_id) DO UPDATE SET device_id=excluded.device_id,source_sequence=excluded.source_sequence,payload_json=excluded.payload_json",params![id,source,change.sequence,serde_json::to_string(&summarize(&change.session.history)).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
            db.execute("INSERT INTO orchid_session_authority VALUES(?1,?2,?3,0) ON CONFLICT(session_id) DO UPDATE SET owner=excluded.owner,transfers_json=excluded.transfers_json,delivery_pending=0",params![id,source,serde_json::to_string(&change.transfers).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
            if updated>0 {db.execute("INSERT INTO orchid_replica_changes(session_id) VALUES(?1)",[id]).map_err(|e|e.to_string())?;}
            Ok(updated>0)
        }).map_err(|e|e.to_string())
    }
    pub fn accept_for_retention(&self, change: &SessionChange) -> Result<bool, String> {
        let cache = self.retention("sessions")? == RetentionMode::Cache
            && self
                .get(change.session.history.session.id.as_str())?
                .is_none();
        let changed = self.accept(&change.device_id, change)?;
        if cache {
            self.evict(change.session.history.session.id.as_str())?;
        }
        Ok(changed)
    }
    pub fn evict(&self, id: &str) -> Result<(), String> {
        if self.retention("sessions")? != RetentionMode::Cache {
            return Err("Retained histories cannot be evicted".into());
        }
        self.database
            .write("evict cached replica", |db| {
                db.execute(
                    "DELETE FROM orchid_session_replicas WHERE session_id=?1",
                    [id],
                )
            })
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn summaries(
        &self,
    ) -> Result<Vec<crate::agent_sessions::ports::AgentSessionSummary>, String> {
        let rows = self
            .database
            .read("read replica summaries", |db| {
                db.prepare(
                    "SELECT payload_json FROM orchid_session_replica_summaries ORDER BY session_id",
                )?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, rusqlite::Error>>()
            })
            .map_err(|e| e.to_string())?;
        rows.into_iter()
            .map(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
            .collect()
    }
    pub fn owner(&self, id: &str) -> Result<Option<String>, String> {
        self.database
            .read("read replica owner", |db| {
                db.query_row(
                    "SELECT device_id FROM orchid_session_replica_summaries WHERE session_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
            })
            .map_err(|e| e.to_string())
    }
    pub fn get(&self, id: &str) -> Result<Option<SessionChange>, String> {
        self.database
            .read("read session replica", |db| {
                db.query_row(
                    "SELECT payload_json FROM orchid_session_replicas WHERE session_id=?1",
                    [id],
                    |r| r.get::<_, String>(0),
                )
                .optional()
            })
            .map_err(|e| e.to_string())?
            .map(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
            .transpose()
    }
    pub fn list(&self) -> Result<Vec<SessionChange>, String> {
        let rows = self
            .database
            .read("list session replicas", |db| {
                db.prepare("SELECT payload_json FROM orchid_session_replicas ORDER BY session_id")?
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, rusqlite::Error>>()
            })
            .map_err(|e| e.to_string())?;
        rows.into_iter()
            .map(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
            .collect()
    }
    pub fn changes(&self, after: u64) -> Result<Vec<(u64, SessionChange)>, String> {
        let rows=self.database.read("read replica subscription",|db|db.prepare("SELECT MAX(c.sequence),r.payload_json FROM orchid_replica_changes c JOIN orchid_session_replicas r USING(session_id) WHERE c.sequence>?1 GROUP BY c.session_id ORDER BY MAX(c.sequence) LIMIT 100")?.query_map([after],|r|Ok((r.get::<_,u64>(0)?,r.get::<_,String>(1)?)))?.collect::<Result<Vec<_>,rusqlite::Error>>()).map_err(|e|e.to_string())?;
        rows.into_iter()
            .map(|(seq, json)| {
                serde_json::from_str(&json)
                    .map(|change| (seq, change))
                    .map_err(|e| e.to_string())
            })
            .collect()
    }
    pub fn retention(&self, category: &str) -> Result<RetentionMode, String> {
        let mode: String = self
            .database
            .read("read retention policy", |db| {
                db.query_row(
                    "SELECT mode FROM orchid_retention WHERE category=?1",
                    [category],
                    |r| r.get(0),
                )
            })
            .map_err(|e| e.to_string())?;
        Ok(if mode == "cache" {
            RetentionMode::Cache
        } else {
            RetentionMode::Retained
        })
    }
    pub fn set_retention(&self, category: &str, mode: RetentionMode) -> Result<(), String> {
        if !["sessions", "definitions", "skills"].contains(&category) {
            return Err("Unknown retention category".into());
        }
        if category != "sessions" && mode == RetentionMode::Cache {
            return Err("Definitions and skill packages currently require retained storage".into());
        }
        self.database
            .write("set retention policy", |db| -> Result<(), String> {
                db.execute(
                    "UPDATE orchid_retention SET mode=?2 WHERE category=?1",
                    params![
                        category,
                        if mode == RetentionMode::Cache {
                            "cache"
                        } else {
                            "retained"
                        }
                    ],
                )
                .map_err(|e| e.to_string())?;
                if category == "sessions" && mode == RetentionMode::Retained {
                    db.execute("UPDATE orchid_sync_cursors SET received=0", [])
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            })
            .map_err(|e| e.to_string())
    }
    pub fn completeness(&self) -> Result<(u64, u64), String> {
        self.database.read("read replica completeness",|db|db.query_row("SELECT (SELECT COUNT(*) FROM orchid_session_replica_summaries),(SELECT COUNT(*) FROM orchid_session_replicas)",[],|r|Ok((r.get(0)?,r.get(1)?)))).map_err(|e|e.to_string())
    }
    pub fn evict_cached_sessions(&self) -> Result<(), String> {
        if self.retention("sessions")? != RetentionMode::Cache {
            return Err("Change retention to cache before evicting copied histories".into());
        }
        self.database
            .write("evict copied histories", |db| {
                db.execute("DELETE FROM orchid_session_replicas", [])
            })
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
