mod docker;
mod domain;
mod local_process;
pub(crate) mod repository;
#[cfg(test)]
mod tests;
pub(crate) mod transport;
use domain::*;
use orchid_coordinator::{client::Client, credentials::Credentials, protocol::*};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
pub(crate) struct RuntimeCoordinator {
    root: PathBuf,
    local: local_process::LocalCoordinator,
    settings: Mutex<Settings>,
    bindings: repository::Bindings,
    devices: Arc<crate::execution_devices::ExecutionDeviceService>,
}
impl RuntimeCoordinator {
    pub fn new(
        root: PathBuf,
        database: Arc<crate::persistence::ActiveDatabase>,
        devices: Arc<crate::execution_devices::ExecutionDeviceService>,
    ) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let path = root.join("connection.json");
        let settings = if path.exists() {
            serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|_| "Coordinator connection settings are invalid")?
        } else {
            Settings::default()
        };
        Ok(Self {
            local: local_process::LocalCoordinator::new(root.join("local")),
            root,
            settings: Mutex::new(settings),
            bindings: repository::Bindings(database),
            devices,
        })
    }
    async fn client(&self) -> Result<Client, String> {
        let s = self.settings.lock().await.clone();
        match s.external_url {
            Some(url) => {
                let token = Credentials::new(&self.root.join("client-credentials"))?.read(
                    &s.credential_ref
                        .ok_or("External coordinator credential is missing")?,
                )?;
                Client::new(&url, &token)
            }
            None => self.local.client().await,
        }
    }
    pub async fn overview(&self) -> Result<Overview, String> {
        let s = self.settings.lock().await.clone();
        let mut o = Overview {
            mode: if s.external_url.is_some() {
                "external"
            } else {
                "local"
            }
            .into(),
            endpoint: s.external_url,
            coordinator_id: None,
            error: None,
            devices: Vec::new(),
            bindings: self.bindings.all()?,
            legacy_devices: self.devices.list()?,
        };
        match self.client().await {
            Ok(c) => match c.health().await {
                Ok(h) => {
                    o.coordinator_id = Some(h.coordinator_id);
                    match c.list().await {
                        Ok(d) => o.devices = d,
                        Err(e) => o.error = Some(e),
                    }
                }
                Err(e) => o.error = Some(e),
            },
            Err(e) => o.error = Some(e),
        }
        Ok(o)
    }
    pub async fn settings(&self, input: SettingsInput) -> Result<Overview, String> {
        let settings = if let Some(url) = input.external_url {
            let current = self.settings.lock().await.clone();
            let credentials = Credentials::new(&self.root.join("client-credentials"))?;
            let token = match input.token.filter(|s| !s.trim().is_empty()) {
                Some(t) => t,
                None if current.external_url.as_ref() == Some(&url) => credentials.read(
                    &current
                        .credential_ref
                        .ok_or("Enter a coordinator access token")?,
                )?,
                None => return Err("Enter a coordinator access token".into()),
            };
            let c = Client::new(&url, &token)?;
            c.health().await?;
            Settings {
                external_url: Some(url),
                credential_ref: Some(credentials.save(&token)?),
            }
        } else {
            self.local.client().await?.health().await?;
            Settings::default()
        };
        let temporary = self
            .root
            .join(format!("connection-{}.tmp", uuid::Uuid::new_v4()));
        fs::write(
            &temporary,
            serde_json::to_vec(&settings).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(&temporary, self.root.join("connection.json")).map_err(|e| e.to_string())?;
        *self.settings.lock().await = settings;
        self.overview().await
    }
    pub async fn connect(&self, token: String) -> Result<Discovery, String> {
        self.client().await?.connect(&Connect { token }).await
    }
    pub async fn docker_settings(&self) -> Result<Overview, String> {
        let (url, token) = docker::connection().await?;
        self.settings(SettingsInput {
            external_url: Some(url),
            token: Some(token),
        })
        .await
    }
    pub async fn register(&self, i: RegisterInput) -> Result<Device, String> {
        let c = self.client().await?;
        let health = c.health().await?;
        if let Some(id) = &i.execution_device_id {
            if repository::managed(&self.bindings.0, id)? {
                return Err("This execution device already has coordinator power controls".into());
            }
            if id == "local" || !self.devices.list()?.iter().any(|d| &d.device_id == id) {
                return Err("Select an existing remote execution device".into());
            }
        }
        let d = c
            .register(&Register {
                credential_id: i.credential_id,
                server_id: i.server_id,
                display_name: i.display_name,
            })
            .await?;
        if let Some(id) = i.execution_device_id {
            self.bindings.associate(&repository::Binding {
                coordinator_id: health.coordinator_id,
                device_id: d.id.clone(),
                execution_device_id: id,
            })?;
        }
        Ok(d)
    }
    pub async fn refresh(&self, id: String) -> Result<Device, String> {
        self.client().await?.refresh(&id).await
    }
    pub async fn rename(&self, id: String, name: String) -> Result<Device, String> {
        self.client().await?.rename(&id, &name).await
    }
    pub async fn power(&self, id: String, request: PowerRequest) -> Result<Operation, String> {
        let c = self.client().await?;
        let h = c.health().await?;
        if request.command == PowerCommand::Stop
            && self.bindings.active(&h.coordinator_id, &id)? > 0
        {
            return Err("Device cannot be stopped while Orchid-managed work is active".into());
        }
        c.power(&id, &request).await
    }
}
