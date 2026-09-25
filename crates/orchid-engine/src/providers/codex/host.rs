//! Codex configuration registered on an Orchid host. The host owns dispatch and transport; only
//! Codex interprets the executable and native home.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexConfiguration {
    pub id: String,
    #[serde(default = "codex_provider")]
    pub provider: String,
    pub executable: String,
    pub home: PathBuf,
}

fn codex_provider() -> String {
    super::options::PROVIDER.into()
}
