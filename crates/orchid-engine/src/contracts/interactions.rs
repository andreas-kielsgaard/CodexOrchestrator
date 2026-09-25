use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Provider-neutral answer to a pending runtime request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeInteractionResponse {
    Choose { choice_id: String },
    Answer { answers: BTreeMap<String, Vec<String>> },
}
