use crate::session_events::SessionLogicalAddress;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    Create,
    Edit,
    Delete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportedFileChange {
    pub path: String,
    pub operation: FileOperation,
}

#[derive(Clone, Debug)]
pub struct SessionFileChange {
    pub address: SessionLogicalAddress,
    pub session_id: String,
    pub invocation_id: String,
    pub working_directory: Option<String>,
    pub path: String,
    pub operation: FileOperation,
    pub recorded_at: String,
}
