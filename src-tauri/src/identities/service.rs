use super::{
    domain::{AssignedAgentIdentity, IdentityDefinition, IdentityId, IdentityShape},
    repository::{IdentityCatalogEntry, IdentityRepository, SqliteIdentityRepository},
};
use chrono::Utc;
use std::{path::Path, sync::Arc};
use uuid::Uuid;

use crate::persistence::ActiveDatabase;

#[derive(Clone)]
pub(crate) struct IdentityService {
    repository: Arc<dyn IdentityRepository>,
}

impl IdentityService {
    pub(crate) fn from_database(database: Arc<ActiveDatabase>) -> Self {
        Self::new(Arc::new(SqliteIdentityRepository::from_database(database)))
    }

    pub(crate) fn open(database_path: &Path) -> Result<Self, String> {
        Ok(Self::new(Arc::new(SqliteIdentityRepository::open(
            database_path,
        )?)))
    }

    pub(crate) fn new(repository: Arc<dyn IdentityRepository>) -> Self {
        Self { repository }
    }

    pub(crate) fn list(&self) -> Result<Vec<IdentityCatalogEntry>, String> {
        self.repository.list()
    }

    /// Resolves a reusable definition into the immutable value assigned to a Session.
    pub(crate) fn assignment(
        &self,
        identity_id: &IdentityId,
    ) -> Result<AssignedAgentIdentity, String> {
        self.repository
            .find(identity_id)?
            .map(|entry| entry.definition.assign())
            .ok_or_else(|| "Identity definition does not exist.".to_string())
    }

    pub(crate) fn create(
        &self,
        display_name: String,
        color: String,
        shape: IdentityShape,
    ) -> Result<IdentityCatalogEntry, String> {
        let now = Utc::now();
        let entry = IdentityCatalogEntry {
            definition: normalized_definition(
                IdentityId::new(format!("identity-{}", Uuid::new_v4()))
                    .map_err(|error| error.to_string())?,
                display_name,
                color,
                shape,
            )?,
            created_at: now,
            updated_at: now,
        };
        self.repository.create(&entry)?;
        Ok(entry)
    }

    pub(crate) fn update(
        &self,
        identity_id: IdentityId,
        display_name: String,
        color: String,
        shape: IdentityShape,
    ) -> Result<IdentityCatalogEntry, String> {
        let current = self
            .repository
            .find(&identity_id)?
            .ok_or_else(|| "Identity definition does not exist.".to_string())?;
        let entry = IdentityCatalogEntry {
            definition: normalized_definition(identity_id, display_name, color, shape)?,
            created_at: current.created_at,
            updated_at: Utc::now(),
        };
        self.repository.update(&entry)?;
        Ok(entry)
    }

    pub(crate) fn delete(&self, identity_id: &IdentityId) -> Result<(), String> {
        self.repository.delete(identity_id)
    }
}

fn normalized_definition(
    identity_id: IdentityId,
    display_name: String,
    color: String,
    shape: IdentityShape,
) -> Result<IdentityDefinition, String> {
    IdentityDefinition::new(
        identity_id,
        display_name.trim().to_string(),
        color.to_ascii_lowercase(),
        shape,
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identities::repository::SqliteIdentityRepository;

    fn service() -> IdentityService {
        IdentityService::new(Arc::new(SqliteIdentityRepository::in_memory()))
    }

    #[test]
    fn owns_the_complete_catalog_lifecycle_without_assignment_policy() {
        let service = service();
        let created = service
            .create("  Avery  ".into(), "#AABBCC".into(), IdentityShape::Circle)
            .unwrap();

        assert!(created.definition.id.as_str().starts_with("identity-"));
        assert_eq!(created.definition.display_name, "Avery");
        assert_eq!(created.definition.color, "#aabbcc");
        assert_eq!(service.list().unwrap(), vec![created.clone()]);

        let updated = service
            .update(
                created.definition.id.clone(),
                "Avery Stone".into(),
                "#112233".into(),
                IdentityShape::Hexagon,
            )
            .unwrap();
        assert_eq!(updated.created_at, created.created_at);
        assert!(updated.updated_at >= created.updated_at);
        assert_eq!(updated.definition.display_name, "Avery Stone");

        service.delete(&created.definition.id).unwrap();
        assert!(service.list().unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_values_and_missing_mutation_targets() {
        let service = service();
        assert!(service
            .create(" ".into(), "#abcdef".into(), IdentityShape::Circle)
            .is_err());
        assert!(service
            .create("Avery".into(), "green".into(), IdentityShape::Circle)
            .is_err());

        let missing = IdentityId::new("identity-missing").unwrap();
        assert!(service
            .update(
                missing.clone(),
                "Avery".into(),
                "#abcdef".into(),
                IdentityShape::Square,
            )
            .is_err());
        assert!(service.delete(&missing).is_err());
    }
}
