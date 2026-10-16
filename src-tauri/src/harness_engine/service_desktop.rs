use super::*;
use std::sync::Arc;
pub(crate) struct HarnessEngineTauriState {
    service: Arc<HarnessEngineService>,
}

impl HarnessEngineTauriState {
    pub(crate) fn new(service: Arc<HarnessEngineService>) -> Self {
        Self { service }
    }

    pub(crate) fn service(&self) -> &Arc<HarnessEngineService> {
        &self.service
    }
}
