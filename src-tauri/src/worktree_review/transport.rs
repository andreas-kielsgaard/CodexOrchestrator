use super::service::{HumanReviewLauncherService, ReviewInstanceView, ReviewSourceView};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub(crate) struct HumanReviewLauncherTauriState(Arc<HumanReviewLauncherService>);

impl HumanReviewLauncherTauriState {
    pub(crate) fn new(service: HumanReviewLauncherService) -> Self {
        Self(Arc::new(service))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareReviewInput {
    source_ref: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewInstanceInput {
    instance_ref: String,
}

#[tauri::command]
pub(crate) fn list_human_review_worktrees(
    state: State<'_, HumanReviewLauncherTauriState>,
) -> Vec<ReviewSourceView> {
    state.0.sources()
}

#[tauri::command]
pub(crate) fn list_human_review_instances(
    state: State<'_, HumanReviewLauncherTauriState>,
) -> Vec<ReviewInstanceView> {
    state.0.instances()
}

#[tauri::command]
pub(crate) async fn prepare_human_review_instance(
    state: State<'_, HumanReviewLauncherTauriState>,
    input: PrepareReviewInput,
) -> Result<ReviewInstanceView, String> {
    run(state.0.clone(), move |service| {
        service.prepare(input.source_ref, input.name)
    })
    .await
}

macro_rules! lifecycle_command {
    ($name:ident, $method:ident) => {
        #[tauri::command]
        pub(crate) async fn $name(
            state: State<'_, HumanReviewLauncherTauriState>,
            input: ReviewInstanceInput,
        ) -> Result<ReviewInstanceView, String> {
            run(state.0.clone(), move |service| {
                service.$method(input.instance_ref)
            })
            .await
        }
    };
}

lifecycle_command!(build_human_review_instance, build);
lifecycle_command!(start_human_review_instance, start);
lifecycle_command!(status_human_review_instance, status);
lifecycle_command!(focus_human_review_instance, focus);
lifecycle_command!(stop_human_review_instance, stop);
lifecycle_command!(recover_human_review_instance, recover);

async fn run(
    service: Arc<HumanReviewLauncherService>,
    operation: impl FnOnce(&HumanReviewLauncherService) -> Result<ReviewInstanceView, String>
        + Send
        + 'static,
) -> Result<ReviewInstanceView, String> {
    tauri::async_runtime::spawn_blocking(move || operation(&service))
        .await
        .map_err(|error| format!("Review runtime task failed: {error}"))?
}
