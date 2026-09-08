use super::{
    domain::{IdentityId, IdentityShape},
    repository::IdentityCatalogEntry,
    service::IdentityService,
};
use serde::Deserialize;
use tauri::State;

pub(crate) struct IdentityTauriState {
    service: IdentityService,
}

impl IdentityTauriState {
    pub(crate) fn new(service: IdentityService) -> Self {
        Self { service }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateIdentityInput {
    display_name: String,
    color: String,
    shape: IdentityShape,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateIdentityInput {
    identity_id: IdentityId,
    display_name: String,
    color: String,
    shape: IdentityShape,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeleteIdentityInput {
    identity_id: IdentityId,
}

#[tauri::command]
pub(crate) fn list_identities(
    state: State<'_, IdentityTauriState>,
) -> Result<Vec<IdentityCatalogEntry>, String> {
    state.service.list()
}

#[tauri::command]
pub(crate) fn create_identity(
    state: State<'_, IdentityTauriState>,
    input: CreateIdentityInput,
) -> Result<IdentityCatalogEntry, String> {
    state
        .service
        .create(input.display_name, input.color, input.shape)
}

#[tauri::command]
pub(crate) fn update_identity(
    state: State<'_, IdentityTauriState>,
    input: UpdateIdentityInput,
) -> Result<IdentityCatalogEntry, String> {
    state.service.update(
        input.identity_id,
        input.display_name,
        input.color,
        input.shape,
    )
}

#[tauri::command]
pub(crate) fn delete_identity(
    state: State<'_, IdentityTauriState>,
    input: DeleteIdentityInput,
) -> Result<(), String> {
    state.service.delete(&input.identity_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_are_strict_and_use_domain_validation() {
        let created: CreateIdentityInput = serde_json::from_value(serde_json::json!({
            "displayName": "Avery",
            "color": "#39745a",
            "shape": "circle"
        }))
        .unwrap();
        assert_eq!(created.display_name, "Avery");

        assert!(
            serde_json::from_value::<CreateIdentityInput>(serde_json::json!({
                "displayName": "Avery",
                "color": "#39745a",
                "shape": "circle",
                "providerId": "not-an-identity-property"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UpdateIdentityInput>(serde_json::json!({
                "identityId": " ",
                "displayName": "Avery",
                "color": "#39745a",
                "shape": "square"
            }))
            .is_err()
        );
    }

    #[test]
    fn catalog_entry_serializes_as_the_flat_frontend_contract() {
        let now = chrono::Utc::now();
        let entry = IdentityCatalogEntry {
            definition: super::super::domain::IdentityDefinition::new(
                IdentityId::new("identity-avery").unwrap(),
                "Avery",
                "#39745a",
                IdentityShape::Hexagon,
            )
            .unwrap(),
            created_at: now,
            updated_at: now,
        };
        let value = serde_json::to_value(entry).unwrap();
        assert_eq!(value["id"], "identity-avery");
        assert_eq!(value["displayName"], "Avery");
        assert_eq!(value["shape"], "hexagon");
        assert!(value.get("definition").is_none());
        assert!(value.get("providerId").is_none());
    }
}
