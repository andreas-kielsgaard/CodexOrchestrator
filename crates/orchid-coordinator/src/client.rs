use crate::protocol::*;
use serde::{de::DeserializeOwned, Serialize};
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    url: String,
    token: String,
}
impl Client {
    pub fn new(url: &str, token: &str) -> Result<Self, String> {
        let parsed = reqwest::Url::parse(url).map_err(|_| "Enter a valid coordinator URL")?;
        if !(parsed.scheme() == "https"
            || (parsed.scheme() == "http"
                && matches!(
                    parsed.host_str(),
                    Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
                )))
        {
            return Err(
                "External coordinators require HTTPS; HTTP is supported on loopback only".into(),
            );
        }
        if !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err("Coordinator URL must not contain credentials, query, or fragment".into());
        }
        if token.trim().is_empty() {
            return Err("Enter a coordinator access token".into());
        }
        Ok(Self {
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(45))
                .build()
                .map_err(|_| "Cannot create coordinator client")?,
            url: url.trim_end_matches('/').into(),
            token: token.into(),
        })
    }
    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        input: Option<&impl Serialize>,
    ) -> Result<T, String> {
        let mut r = self
            .http
            .request(method, format!("{}/api/v1{path}", self.url))
            .bearer_auth(&self.token);
        if path == "/health" {
            r = r.timeout(std::time::Duration::from_secs(5));
        }
        if let Some(i) = input {
            r = r.json(i)
        }
        let response = r
            .send()
            .await
            .map_err(|_| "Coordinator is unreachable; no local takeover was started".to_string())?;
        if !response.status().is_success() {
            let e = response
                .json::<ApiError>()
                .await
                .map_err(|_| "Coordinator request failed")?;
            return Err(e.message);
        }
        response
            .json()
            .await
            .map_err(|_| "Coordinator returned an invalid reply".into())
    }
    pub async fn health(&self) -> Result<Health, String> {
        let h: Health = self
            .request(reqwest::Method::GET, "/health", None::<&()>)
            .await?;
        if h.contract_version != VERSION {
            return Err("Coordinator protocol is incompatible".into());
        }
        Ok(h)
    }
    pub async fn list(&self) -> Result<Vec<Device>, String> {
        self.request(reqwest::Method::GET, "/devices", None::<&()>)
            .await
    }
    pub async fn connect(&self, i: &Connect) -> Result<Discovery, String> {
        self.request(reqwest::Method::POST, "/hetzner/connect", Some(i))
            .await
    }
    pub async fn register(&self, i: &Register) -> Result<Device, String> {
        self.request(reqwest::Method::POST, "/devices", Some(i))
            .await
    }
    pub async fn refresh(&self, id: &str) -> Result<Device, String> {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid device identity")?;
        self.request(
            reqwest::Method::POST,
            &format!("/devices/{id}/refresh"),
            None::<&()>,
        )
        .await
    }
    pub async fn rename(&self, id: &str, name: &str) -> Result<Device, String> {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid device identity")?;
        self.request(
            reqwest::Method::PATCH,
            &format!("/devices/{id}"),
            Some(&Rename { name: name.into() }),
        )
        .await
    }
    pub async fn power(&self, id: &str, i: &PowerRequest) -> Result<Operation, String> {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid device identity")?;
        self.request(
            reqwest::Method::POST,
            &format!("/devices/{id}/power"),
            Some(i),
        )
        .await
    }
}
