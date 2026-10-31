use crate::{credentials::Credentials, hetzner::Hetzner, protocol::*, repository::Repository};
use std::{path::Path, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;
pub struct Service {
    pub repository: Repository,
    credentials: Credentials,
    provider: Hetzner,
    operations: Mutex<()>,
}
impl Service {
    pub fn open(root: &Path, cloud_url: &str) -> Result<Arc<Self>, String> {
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            repository: Repository::open(&root.join("coordinator.sqlite"))?,
            credentials: Credentials::new(&root.join("credentials"))?,
            provider: Hetzner::new(cloud_url)?,
            operations: Mutex::new(()),
        }))
    }
    pub fn health(&self) -> Result<Health, String> {
        Ok(Health {
            contract_version: VERSION,
            coordinator_id: self.repository.identity()?,
        })
    }
    pub async fn connect(&self, input: Connect) -> Result<Discovery, String> {
        if input.token.trim().is_empty() {
            return Err("Enter a Hetzner Cloud API token".into());
        }
        let servers = self.provider.servers(input.token.trim()).await?;
        let credential_id = self.credentials.save(input.token.trim())?;
        Ok(Discovery {
            credential_id,
            servers,
        })
    }
    pub async fn register(&self, input: Register) -> Result<Device, String> {
        let _gate = self.operations.lock().await;
        if input.display_name.trim().is_empty() {
            return Err("Enter a device name".into());
        }
        let token = self.credentials.read(&input.credential_id)?;
        let server = self.provider.get(&token, input.server_id).await?;
        if self
            .repository
            .list()?
            .iter()
            .any(|d| d.server.id == server.id)
        {
            return Err("This server is already registered on this coordinator".into());
        }
        let device = Device {
            id: Uuid::new_v4().to_string(),
            display_name: input.display_name.trim().into(),
            server,
            observed_at: chrono::Utc::now().to_rfc3339(),
            credential_configured: true,
            operation: None,
            observation_error: None,
        };
        self.repository.save_device(&device, &input.credential_id)?;
        Ok(device)
    }
    pub async fn refresh(&self, id: &str) -> Result<Device, String> {
        let _gate = self.operations.lock().await;
        let (mut d, cred) = self.repository.device(id)?;
        let token = self.credentials.read(&cred)?;
        if let Some(mut o) = d.operation.clone() {
            if o.active() {
                if let Some(action) = o.provider_action_id {
                    match self.provider.action(&token, action).await {
                        Ok(status) => {
                            o.status = match status.as_str() {
                                "success" => "succeeded",
                                "error" => "failed",
                                _ => "running",
                            }
                            .into();
                            o.detail = None;
                        }
                        Err(e) => o.detail = Some(e),
                    }
                } else {
                    o.status = "unconfirmed".into();
                    o.detail = Some(
                        "No provider receipt retained; the power request was not replayed".into(),
                    );
                }
                d.operation = Some(o);
            }
        }
        match self.provider.get(&token, d.server.id).await {
            Ok(server) => {
                d.server = server;
                d.observed_at = chrono::Utc::now().to_rfc3339();
                d.observation_error = None
            }
            Err(e) => d.observation_error = Some(e),
        }
        if let Some(mut o) = d.operation.clone() {
            if o.status == "unconfirmed"
                && d.observation_error.is_none()
                && ((o.command == PowerCommand::Start && d.server.status == "running")
                    || (o.command == PowerCommand::Stop && d.server.status == "off"))
            {
                o.status = "observed".into();
                o.detail = Some(
                    "Requested power state observed; submission receipt remains unconfirmed".into(),
                );
                d.operation = Some(o)
            }
        }
        self.repository.save_state(&d, &cred)?;
        Ok(d)
    }
    pub async fn rename(&self, id: &str, name: &str) -> Result<Device, String> {
        let _gate = self.operations.lock().await;
        if name.trim().is_empty() {
            return Err("Enter a device name".into());
        }
        let (mut d, c) = self.repository.device(id)?;
        d.display_name = name.trim().into();
        self.repository.save_device(&d, &c)?;
        Ok(d)
    }
    pub async fn power(&self, id: &str, request: PowerRequest) -> Result<Operation, String> {
        Uuid::parse_str(&request.operation_id).map_err(|_| "Invalid operation identity")?;
        let _gate = self.operations.lock().await;
        if let Some(o) = self.repository.operation(&request.operation_id)? {
            if o.device_id != id || o.command != request.command {
                return Err("Operation identity belongs to another request".into());
            }
            return Ok(o);
        }
        let (mut d, c) = self.repository.device(id)?;
        if d.operation
            .as_ref()
            .is_some_and(|o| o.active() || o.status == "unconfirmed")
        {
            return Err("Observe the previous power operation before starting another".into());
        }
        let token = self.credentials.read(&c)?;
        d.server = self.provider.get(&token, d.server.id).await?;
        let required_state = match request.command {
            PowerCommand::Start => "off",
            PowerCommand::Stop => "running",
        };
        if d.server.status != required_state {
            return Err(format!(
                "Current power state is {}; Refresh before requesting {:?}",
                d.server.status, request.command
            ));
        }
        d.observed_at = chrono::Utc::now().to_rfc3339();
        d.observation_error = None;
        let mut o = Operation {
            id: request.operation_id,
            device_id: id.into(),
            command: request.command,
            status: "accepted".into(),
            provider_action_id: None,
            requested_at: chrono::Utc::now().to_rfc3339(),
            detail: None,
        };
        d.operation = Some(o.clone());
        self.repository.save_state(&d, &c)?;
        match self.provider.power(&token, d.server.id, &o.command).await {
            Ok(action) => {
                o.provider_action_id = Some(action);
                o.status = "running".into()
            }
            Err(e) => {
                o.status = if e.starts_with("Hetzner request failed (HTTP 4") {
                    "failed"
                } else {
                    "unconfirmed"
                }
                .into();
                o.detail = Some(e)
            }
        }
        d.operation = Some(o.clone());
        self.repository.save_state(&d, &c)?;
        Ok(o)
    }
}
