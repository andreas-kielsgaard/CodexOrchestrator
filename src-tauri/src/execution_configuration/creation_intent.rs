use super::NodeProfile;
use serde::{Deserialize, Serialize};

/// Inputs resolved only when a Session is created. The event layer carries these opaquely.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionCreationIntent {
    pub(crate) capability_profile_id: String,
    pub(crate) node_profile: NodeProfile,
    pub(crate) working_directory: String,
    pub(crate) title: String,
}
