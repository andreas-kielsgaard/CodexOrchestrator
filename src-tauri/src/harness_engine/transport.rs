use super::{
    catalog::{HarnessRecord, HarnessVersion, ResolvedHarnessVersion},
    catalog_service::HarnessCatalogService,
    configuration::HarnessConfiguration,
    domain::{HarnessId, HarnessVersionNumber, HarnessVersionRef},
};
use serde::{Deserialize, Serialize};
use tauri::State;

pub(crate) struct HarnessCatalogTauriState {
    service: HarnessCatalogService,
}

impl HarnessCatalogTauriState {
    pub(crate) fn new(service: HarnessCatalogService) -> Self {
        Self { service }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessDraftView {
    based_on_version: Option<HarnessVersionRef>,
    configuration: HarnessConfiguration,
    saved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessManagementView {
    harness: HarnessRecord,
    draft: Option<HarnessDraftView>,
    versions: Vec<HarnessVersion>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessQuery {
    harness_id: HarnessId,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateHarnessInput {
    name: String,
    initial_configuration: HarnessConfiguration,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RenameHarnessInput {
    harness_id: HarnessId,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveHarnessDraftInput {
    harness_id: HarnessId,
    based_on_version: Option<HarnessVersionRef>,
    configuration: HarnessConfiguration,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PublishSessionHarnessOverrideInput {
    harness_id: HarnessId,
    session_id: String,
    configuration: HarnessConfiguration,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessReplacementInput {
    source: HarnessVersionRef,
    target: HarnessVersionRef,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolveHarnessVersionInput {
    reference: HarnessVersionRef,
}

#[tauri::command]
pub(crate) fn list_harnesses(
    state: State<'_, HarnessCatalogTauriState>,
) -> Result<Vec<HarnessRecord>, String> {
    state.service.list()
}

#[tauri::command]
pub(crate) fn load_harness(
    state: State<'_, HarnessCatalogTauriState>,
    input: HarnessQuery,
) -> Result<HarnessManagementView, String> {
    management_view(&state.service, &input.harness_id)
}

#[tauri::command]
pub(crate) fn create_harness(
    state: State<'_, HarnessCatalogTauriState>,
    input: CreateHarnessInput,
) -> Result<HarnessManagementView, String> {
    let harness = state
        .service
        .create_harness(input.name, input.initial_configuration)?;
    management_view(&state.service, &harness.id)
}

#[tauri::command]
pub(crate) fn rename_harness(
    state: State<'_, HarnessCatalogTauriState>,
    input: RenameHarnessInput,
) -> Result<HarnessManagementView, String> {
    state.service.rename(&input.harness_id, input.name)?;
    management_view(&state.service, &input.harness_id)
}

#[tauri::command]
pub(crate) fn save_harness_draft(
    state: State<'_, HarnessCatalogTauriState>,
    input: SaveHarnessDraftInput,
) -> Result<HarnessManagementView, String> {
    let (_, current_draft, _) = state.service.load(&input.harness_id)?;
    let expected_current_revision = current_draft
        .as_ref()
        .map(|draft| draft.draft_revision)
        .unwrap_or(0);
    state.service.save_draft(
        &input.harness_id,
        input.based_on_version.as_ref(),
        input.configuration,
        expected_current_revision,
    )?;
    management_view(&state.service, &input.harness_id)
}

#[tauri::command]
pub(crate) fn publish_harness_draft(
    state: State<'_, HarnessCatalogTauriState>,
    input: HarnessQuery,
) -> Result<HarnessManagementView, String> {
    state.service.publish_draft(&input.harness_id)?;
    management_view(&state.service, &input.harness_id)
}

#[tauri::command]
pub(crate) fn publish_session_harness_override(
    state: State<'_, HarnessCatalogTauriState>,
    input: PublishSessionHarnessOverrideInput,
) -> Result<HarnessVersion, String> {
    state.service.publish_session_override(
        &input.harness_id,
        input.session_id,
        input.configuration,
    )
}

#[tauri::command]
pub(crate) fn order_harness_version_replacement(
    state: State<'_, HarnessCatalogTauriState>,
    input: HarnessReplacementInput,
) -> Result<HarnessManagementView, String> {
    let harness_id = input.source.harness_id().clone();
    state
        .service
        .order_replacement(input.source, input.target)?;
    management_view(&state.service, &harness_id)
}

#[tauri::command]
pub(crate) fn resolve_harness_version(
    state: State<'_, HarnessCatalogTauriState>,
    input: ResolveHarnessVersionInput,
) -> Result<ResolvedHarnessVersion, String> {
    state.service.resolve(&input.reference)
}

fn management_view(
    service: &HarnessCatalogService,
    harness_id: &HarnessId,
) -> Result<HarnessManagementView, String> {
    let (harness, draft, versions) = service.load(harness_id)?;
    let draft = draft.map(|draft| HarnessDraftView {
        based_on_version: draft.based_on_version.map(|version: HarnessVersionNumber| {
            HarnessVersionRef::new(harness_id.clone(), version)
        }),
        configuration: draft.configuration,
        saved_at: draft.saved_at.to_rfc3339(),
    });
    Ok(HarnessManagementView {
        harness,
        draft,
        versions,
    })
}

