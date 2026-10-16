use super::QuickModel;
use serde::{Deserialize, Serialize};

/// Last complete, non-secret model observation for one execution route.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredModelCatalogue {
    pub observed_at: String,
    pub models: Vec<QuickModel>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogueView {
    pub route: crate::execution_targets::domain::ExecutionRouteRef,
    pub observed_at: Option<String>,
    pub models: Vec<QuickModel>,
    pub observation_error: Option<String>,
}
