use super::*;

impl ExecutionDeviceService {
    pub(crate) fn acquire_activity(
        &self,
        device_id: &str,
        owner_kind: &str,
        owner_id: &str,
    ) -> Result<(), String> {
        self.database.write("acquire device activity", |transaction| {
            transaction.execute(
                "INSERT OR IGNORE INTO execution_device_activity_leases(device_id,owner_kind,owner_id,acquired_at) VALUES(?1,?2,?3,?4)",
                params![device_id,owner_kind,owner_id,Utc::now().to_rfc3339()],
            ).map_err(|error| error.to_string())?;
            transaction.execute(
                "UPDATE execution_devices SET last_orchid_activity_at=?2,idle_shutdown_claimed_at=NULL WHERE device_id=?1",
                params![device_id,Utc::now().to_rfc3339()],
            ).map_err(|error| error.to_string())?;
            Ok(())
        }).map_err(|error| error.into_string())
    }

    pub(crate) fn release_activity(
        &self,
        device_id: &str,
        owner_kind: &str,
        owner_id: &str,
    ) -> Result<(), String> {
        self.database.write("release device activity", |transaction| {
            transaction.execute(
                "DELETE FROM execution_device_activity_leases WHERE device_id=?1 AND owner_kind=?2 AND owner_id=?3",
                params![device_id,owner_kind,owner_id],
            ).map_err(|error| error.to_string())?;
            transaction.execute(
                "UPDATE execution_devices SET last_orchid_activity_at=?2 WHERE device_id=?1",
                params![device_id,Utc::now().to_rfc3339()],
            ).map_err(|error| error.to_string())?;
            Ok(())
        }).map_err(|error| error.into_string())
    }

    pub(crate) fn reconcile_activity_leases(&self) -> Result<(), String> {
        self.database
            .write("reconcile execution device activity", |transaction| {
                let now = Utc::now().to_rfc3339();
                transaction
                    .execute(
                        "UPDATE execution_devices SET last_orchid_activity_at=?1 WHERE device_id IN (
                           SELECT device_id FROM execution_device_activity_leases l
                           WHERE (l.owner_kind='agent_invocation' AND NOT EXISTS(
                             SELECT 1 FROM agent_session_invocations i WHERE i.id=l.owner_id AND i.status='running'
                           )) OR l.owner_kind='target_transition'
                         )",
                        [now],
                    )
                    .map_err(|error| error.to_string())?;
                transaction
                    .execute(
                        "DELETE FROM execution_device_activity_leases AS l
                         WHERE (l.owner_kind='agent_invocation' AND NOT EXISTS(
                           SELECT 1 FROM agent_session_invocations i WHERE i.id=l.owner_id AND i.status='running'
                         )) OR l.owner_kind='target_transition'",
                        [],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .map_err(|error| error.into_string())
    }
}
