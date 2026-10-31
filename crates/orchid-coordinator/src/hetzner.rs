use crate::protocol::{PowerCommand, Server};
use serde_json::Value;
#[derive(Clone)]
pub struct Hetzner {
    client: reqwest::Client,
    base: String,
}
impl Hetzner {
    pub fn new(base: &str) -> Result<Self, String> {
        Ok(Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .map_err(|_| "Cannot create cloud client")?,
            base: base.trim_end_matches('/').into(),
        })
    }
    async fn decode(response: Result<reqwest::Response, reqwest::Error>) -> Result<Value, String> {
        let response = response.map_err(|_| {
            "Hetzner did not acknowledge the request; outcome may be unconfirmed".to_string()
        })?;
        if !response.status().is_success() {
            return Err(format!(
                "Hetzner request failed (HTTP {})",
                response.status().as_u16()
            ));
        }
        response
            .json()
            .await
            .map_err(|_| "Hetzner returned an invalid response".into())
    }
    fn server(v: &Value) -> Result<Server, String> {
        Ok(Server {
            id: v["id"].as_u64().ok_or("Server identity missing")?,
            name: v["name"].as_str().unwrap_or("Server").into(),
            status: v["status"].as_str().unwrap_or("unknown").into(),
            address: v["public_net"]["ipv4"]["ip"].as_str().map(str::to_owned),
            location: v["location"]["name"].as_str().map(str::to_owned),
        })
    }
    pub async fn servers(&self, token: &str) -> Result<Vec<Server>, String> {
        let mut result = Vec::new();
        let mut page = 1;
        loop {
            let v = Self::decode(
                self.client
                    .get(format!("{}/servers?page={page}&per_page=50", self.base))
                    .bearer_auth(token)
                    .send()
                    .await,
            )
            .await?;
            for server in v["servers"].as_array().ok_or("Invalid server discovery")? {
                result.push(Self::server(server)?);
            }
            match v["meta"]["pagination"]["next_page"].as_u64() {
                Some(next) if next > page => page = next,
                _ => break,
            }
        }
        Ok(result)
    }
    pub async fn get(&self, token: &str, id: u64) -> Result<Server, String> {
        let v = Self::decode(
            self.client
                .get(format!("{}/servers/{id}", self.base))
                .bearer_auth(token)
                .send()
                .await,
        )
        .await?;
        Self::server(&v["server"])
    }
    pub async fn power(&self, token: &str, id: u64, command: &PowerCommand) -> Result<u64, String> {
        let action = match command {
            PowerCommand::Start => "poweron",
            PowerCommand::Stop => "shutdown",
        };
        let v = Self::decode(
            self.client
                .post(format!("{}/servers/{id}/actions/{action}", self.base))
                .bearer_auth(token)
                .json(&serde_json::json!({}))
                .send()
                .await,
        )
        .await?;
        v["action"]["id"]
            .as_u64()
            .ok_or("Hetzner action receipt is missing; outcome unconfirmed".into())
    }
    pub async fn action(&self, token: &str, id: u64) -> Result<String, String> {
        let v = Self::decode(
            self.client
                .get(format!("{}/actions/{id}", self.base))
                .bearer_auth(token)
                .send()
                .await,
        )
        .await?;
        v["action"]["status"]
            .as_str()
            .map(str::to_owned)
            .ok_or("Invalid action status".into())
    }
}
