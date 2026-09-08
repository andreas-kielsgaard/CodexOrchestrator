use crate::session_events::SessionLogicalAddress;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FileOperation {
    Create,
    Edit,
    Delete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReportedFileChange {
    pub(crate) path: String,
    pub(crate) operation: FileOperation,
}

#[derive(Clone, Debug)]
pub(crate) struct SessionFileChange {
    pub(crate) address: SessionLogicalAddress,
    pub(crate) session_id: String,
    pub(crate) invocation_id: String,
    pub(crate) working_directory: Option<String>,
    pub(crate) path: String,
    pub(crate) operation: FileOperation,
    pub(crate) recorded_at: String,
}
