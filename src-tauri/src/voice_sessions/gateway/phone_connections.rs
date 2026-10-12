use std::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) struct PhoneConnection {
    pub(super) id: Uuid,
    pub(super) cancellation: CancellationToken,
}

/// The newest authenticated socket owns phone state; replaced sockets cannot clear it.
#[derive(Default)]
pub(super) struct PhoneConnections(Mutex<Option<PhoneConnection>>);

impl PhoneConnections {
    pub(super) fn open(
        &self,
        cancellation: CancellationToken,
        connected: impl FnOnce() -> Result<(), String>,
    ) -> Result<Uuid, String> {
        let mut current = self.0.lock().map_err(|_| "Phone connection unavailable")?;
        connected()?;
        if let Some(previous) = current.take() {
            previous.cancellation.cancel();
        }
        let id = Uuid::new_v4();
        *current = Some(PhoneConnection { id, cancellation });
        Ok(id)
    }

    pub(super) fn close(
        &self,
        id: Uuid,
        disconnected: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let mut current = self.0.lock().map_err(|_| "Phone connection unavailable")?;
        if current
            .as_ref()
            .is_some_and(|connection| connection.id == id)
        {
            current.take();
            disconnected()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_cancels_old_socket_without_clearing_new_phone_state() {
        let phones = PhoneConnections::default();
        let old = CancellationToken::new();
        let old_id = phones.open(old.clone(), || Ok(())).unwrap();
        let new = CancellationToken::new();
        let new_id = phones.open(new.clone(), || Ok(())).unwrap();
        assert!(old.is_cancelled());
        assert!(!new.is_cancelled());
        phones
            .close(old_id, || panic!("Old socket cleared the new phone state"))
            .unwrap();
        let mut cleared = false;
        phones
            .close(new_id, || {
                cleared = true;
                Ok(())
            })
            .unwrap();
        assert!(cleared);
    }
}
