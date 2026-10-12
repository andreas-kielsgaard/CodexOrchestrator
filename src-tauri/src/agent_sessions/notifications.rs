use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct NodeNotifications {
    pub(crate) turn_completion_sound: bool,
}
impl Default for NodeNotifications {
    fn default() -> Self {
        Self {
            turn_completion_sound: true,
        }
    }
}
