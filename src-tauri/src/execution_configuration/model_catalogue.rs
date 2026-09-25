use super::QuickModel;
use serde::{Deserialize, Serialize};

/// Last complete, non-secret model observation for one execution route.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredModelCatalogue {
    pub(crate) observed_at: String,
    pub(crate) models: Vec<QuickModel>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelCatalogueView {
    pub(crate) route: crate::execution_targets::domain::ExecutionRouteRef,
    pub(crate) observed_at: Option<String>,
    pub(crate) models: Vec<QuickModel>,
    pub(crate) observation_error: Option<String>,
}
