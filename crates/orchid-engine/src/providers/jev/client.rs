use super::contracts::{JevEvaluation, JevResult};
use serde_json::{json, Value};
use std::time::Duration;

pub(super) const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

pub(super) struct JevClient {
    http: reqwest::Client,
    endpoint: String,
    key: String,
}

impl JevClient {
    pub(super) fn new(endpoint: String, key: String, timeout: Duration) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Unable to initialize Jev HTTP client")?;
        Ok(Self {
            http,
            endpoint,
            key,
        })
    }

    pub(super) async fn evaluate(
        &self,
        model: &str,
        input: &JevEvaluation,
    ) -> Result<(JevResult, Value), String> {
        let mut response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.key)
            .json(&json!({ "model": model, "state": input.state, "questions": input.questions }))
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    "Jev request timed out"
                } else {
                    "Jev request failed"
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            // Native error bodies may echo input. Persist only the bounded HTTP classification.
            return Err(format!("Jev API returned HTTP {}", status.as_u16()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Unable to read Jev response")?
        {
            if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err("Jev response exceeds 1 MB".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let raw: Value = serde_json::from_slice(&bytes).map_err(|_| "Jev returned invalid JSON")?;
        let result: JevResult = serde_json::from_value(raw.clone())
            .map_err(|_| "Jev returned an invalid result shape")?;
        result.validate_for(input)?;
        Ok((result, raw))
    }
}
