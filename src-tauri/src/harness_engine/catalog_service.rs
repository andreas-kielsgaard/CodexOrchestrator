use super::{
    catalog::{
        configuration_digest, next_version, HarnessDraft, HarnessRecord, HarnessVersion,
        ResolvedHarnessVersion,
    },
    catalog_repository::{HarnessCatalogRepository, SqliteHarnessCatalogRepository},
    configuration::{HarnessConfiguration, HarnessMetadata},
    domain::{HarnessId, HarnessVersionRef, HarnessVersionReplacement, HarnessVersionScope},
    resolution::resolve_harness_version,
};
use crate::agent_sessions::{application::SessionHarnessVersionResolver, domain::AgentSessionId};
use chrono::Utc;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct HarnessCatalogService {
    repository: Arc<dyn HarnessCatalogRepository>,
    workflow_materialization_lane: Arc<Mutex<()>>,
}

impl HarnessCatalogService {
    pub(crate) fn open(database_path: &Path) -> Result<Self, String> {
        Ok(Self::new(Arc::new(SqliteHarnessCatalogRepository::open(
            database_path,
        )?)))
    }

    pub(crate) fn new(repository: Arc<dyn HarnessCatalogRepository>) -> Self {
        Self {
            repository,
            workflow_materialization_lane: Arc::new(Mutex::new(())),
        }
    }

    #[cfg(test)]
    pub(crate) fn in_memory() -> Self {
        Self::new(Arc::new(SqliteHarnessCatalogRepository::in_memory()))
    }

    pub(crate) fn create_harness(
        &self,
        name: String,
        initial_configuration: HarnessConfiguration,
    ) -> Result<HarnessRecord, String> {
        if name.trim().is_empty() {
            return Err("Harness name must not be empty.".into());
        }
        initial_configuration.validate()?;
        let now = Utc::now();
        let harness = HarnessRecord {
            id: HarnessId::new(format!("harness-{}", Uuid::new_v4()))
                .map_err(|error| error.to_string())?,
            metadata: HarnessMetadata { name },
            created_at: now,
            updated_at: now,
        };
        self.repository.create_harness(&harness)?;
        self.repository.save_draft(
            &HarnessDraft {
                harness_id: harness.id.clone(),
                based_on_version: None,
                configuration: initial_configuration,
                draft_revision: 1,
                saved_at: now,
            },
            0,
        )?;
        Ok(harness)
    }

    /// Materializes one activated Workflow node as an ordinary reusable Harness. Repeated
    /// launches with the same effective configuration reuse the latest immutable version.
    pub(crate) fn materialize_workflow_harness(
        &self,
        harness_id: HarnessId,
        name: String,
        configuration: HarnessConfiguration,
    ) -> Result<HarnessVersionRef, String> {
        let _guard = self
            .workflow_materialization_lane
            .lock()
            .map_err(|_| "Workflow Harness materialization is unavailable.".to_string())?;
        if name.trim().is_empty() {
            return Err("Workflow Harness name must not be empty.".into());
        }
        configuration.validate()?;
        let now = Utc::now();
        match self.repository.harness(&harness_id)? {
            Some(existing) if existing.metadata.name != name => {
                self.repository.update_metadata(
                    &harness_id,
                    &HarnessMetadata { name: name.clone() },
                    now,
                )?;
            }
            Some(_) => {}
            None => self.repository.create_harness(&HarnessRecord {
                id: harness_id.clone(),
                metadata: HarnessMetadata { name },
                created_at: now,
                updated_at: now,
            })?,
        }

        let versions = self.repository.versions(&harness_id)?;
        let digest = configuration_digest(&configuration)?;
        if let Some(latest) = versions
            .iter()
            .rev()
            .find(|version| version.scope == HarnessVersionScope::Reusable)
        {
            if latest.configuration_digest == digest {
                return Ok(latest.reference.clone());
            }
        }
        let version = HarnessVersion::build(
            HarnessVersionRef::new(harness_id, next_version(&versions)?),
            HarnessVersionScope::Reusable,
            configuration,
            now,
        )?;
        self.repository.publish(&version, None)?;
        Ok(version.reference)
    }

    pub(crate) fn list(&self) -> Result<Vec<HarnessRecord>, String> {
        self.repository.harnesses()
    }

    pub(crate) fn replacements(
        &self,
        harness_id: &HarnessId,
    ) -> Result<Vec<HarnessVersionReplacement>, String> {
        self.repository.replacements(harness_id)
    }

    pub(crate) fn load(
        &self,
        harness_id: &HarnessId,
    ) -> Result<(HarnessRecord, Option<HarnessDraft>, Vec<HarnessVersion>), String> {
        let harness = self
            .repository
            .harness(harness_id)?
            .ok_or_else(|| "Harness does not exist.".to_string())?;
        let draft = self.repository.draft(harness_id)?;
        let versions = self.repository.versions(harness_id)?;
        Ok((harness, draft, versions))
    }

    pub(crate) fn rename(
        &self,
        harness_id: &HarnessId,
        name: String,
    ) -> Result<HarnessRecord, String> {
        let metadata = HarnessMetadata { name };
        self.repository
            .update_metadata(harness_id, &metadata, Utc::now())?;
        self.repository
            .harness(harness_id)?
            .ok_or_else(|| "Harness disappeared after it was renamed.".to_string())
    }

    pub(crate) fn save_draft(
        &self,
        harness_id: &HarnessId,
        based_on: Option<&HarnessVersionRef>,
        configuration: HarnessConfiguration,
        expected_current_revision: u64,
    ) -> Result<HarnessDraft, String> {
        if self.repository.harness(harness_id)?.is_none() {
            return Err("Harness does not exist.".into());
        }
        if based_on.is_some_and(|reference| reference.harness_id() != harness_id) {
            return Err("Harness draft base belongs to a different Harness.".into());
        }
        if let Some(reference) = based_on {
            if self.repository.version(reference)?.is_none() {
                return Err("Harness draft base version does not exist.".into());
            }
        }
        let draft = HarnessDraft {
            harness_id: harness_id.clone(),
            based_on_version: based_on.map(HarnessVersionRef::version),
            configuration,
            draft_revision: expected_current_revision
                .checked_add(1)
                .ok_or_else(|| "Harness draft revision overflowed.".to_string())?,
            saved_at: Utc::now(),
        };
        self.repository
            .save_draft(&draft, expected_current_revision)?;
        Ok(draft)
    }

    pub(crate) fn publish_draft(&self, harness_id: &HarnessId) -> Result<HarnessVersion, String> {
        let draft = self
            .repository
            .draft(harness_id)?
            .ok_or_else(|| "Harness has no draft to publish.".to_string())?;
        let versions = self.repository.versions(harness_id)?;
        let version = HarnessVersion::build(
            HarnessVersionRef::new(harness_id.clone(), next_version(&versions)?),
            HarnessVersionScope::Reusable,
            draft.configuration,
            Utc::now(),
        )?;
        self.repository
            .publish(&version, Some(draft.draft_revision))?;
        Ok(version)
    }

    /// Session edits remain volatile until the user explicitly publishes this immutable,
    /// Session-scoped version. There is no persistent Session draft table.
    pub(crate) fn publish_session_override(
        &self,
        harness_id: &HarnessId,
        session_id: String,
        base_harness_ref: &HarnessVersionRef,
        configuration: HarnessConfiguration,
    ) -> Result<HarnessVersion, String> {
        if self.repository.harness(harness_id)?.is_none() {
            return Err("Harness does not exist.".into());
        }
        if base_harness_ref.harness_id() != harness_id {
            return Err("Session Harness override base belongs to a different Harness.".into());
        }
        if self.repository.version(base_harness_ref)?.is_none() {
            return Err("Session Harness override base version does not exist.".into());
        }
        let version = HarnessVersion::build(
            HarnessVersionRef::new(
                harness_id.clone(),
                next_version(&self.repository.versions(harness_id)?)?,
            ),
            HarnessVersionScope::session_specific(session_id)
                .map_err(|error| error.to_string())?,
            configuration,
            Utc::now(),
        )?;
        self.repository.publish(&version, None)?;
        Ok(version)
    }

    pub(crate) fn order_replacement(
        &self,
        source: HarnessVersionRef,
        target: HarnessVersionRef,
    ) -> Result<HarnessVersionReplacement, String> {
        let source_version = self
            .repository
            .version(&source)?
            .ok_or_else(|| "Replacement source Harness version does not exist.".to_string())?;
        let target_version = self
            .repository
            .version(&target)?
            .ok_or_else(|| "Replacement target Harness version does not exist.".to_string())?;
        if source_version.scope != HarnessVersionScope::Reusable
            || target_version.scope != HarnessVersionScope::Reusable
        {
            return Err("Global Harness replacements require reusable versions.".into());
        }
        let replacement = HarnessVersionReplacement::new(source, target)
            .map_err(|error| error.to_string())?;
        self.repository
            .order_replacement(&replacement, Utc::now())?;
        Ok(replacement)
    }

    /// The single read path used by Sessions and runtimes. It resolves ordered replacement
    /// chains before returning configuration so consumers never read stale versions directly.
    pub(crate) fn resolve(
        &self,
        requested: &HarnessVersionRef,
    ) -> Result<ResolvedHarnessVersion, String> {
        let versions = self.repository.versions(requested.harness_id())?;
        let references = versions
            .iter()
            .map(|version| version.reference.clone())
            .collect::<Vec<_>>();
        let replacements = self.repository.replacements(requested.harness_id())?;
        let resolution = resolve_harness_version(requested, &references, &replacements)
            .map_err(|error| error.to_string())?;
        let version = versions
            .into_iter()
            .find(|version| &version.reference == resolution.resolved())
            .ok_or_else(|| "Resolved Harness version disappeared.".to_string())?;
        let replacement_path = match resolution.outcome() {
            super::domain::HarnessMigrationOutcome::Current => vec![requested.clone()],
            super::domain::HarnessMigrationOutcome::ReplacementApplied { path } => path.clone(),
        };
        Ok(ResolvedHarnessVersion {
            requested: requested.clone(),
            version,
            replacement_path,
        })
    }
}

impl SessionHarnessVersionResolver for HarnessCatalogService {
    fn resolve_session_harness_version(
        &self,
        session_id: &AgentSessionId,
        requested: &HarnessVersionRef,
    ) -> Result<HarnessVersionRef, String> {
        let resolved = self.resolve(requested)?;
        if let HarnessVersionScope::SessionSpecific {
            session_id: owning_session_id,
        } = &resolved.version.scope
        {
            if owning_session_id != session_id.as_str() {
                return Err(
                    "Session-specific Harness version belongs to a different Agent Session.".into(),
                );
            }
        }
        Ok(resolved.version.reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness_engine::{
        configuration::{
            HarnessApprovalPolicy, HarnessContextCompressionDelivery, HarnessDiscoveryPolicy,
            HarnessIdentityAssignmentPolicy, HarnessInitialDelivery, HarnessPromptPrefixConfiguration,
            HarnessRuntimeConfiguration, HarnessSandbox, HarnessSkillsConfiguration,
            HarnessToolsConfiguration, HarnessUpdatePolicy,
        },
    };

    fn configuration(prompt: &str) -> HarnessConfiguration {
        HarnessConfiguration {
            identity_assignment: HarnessIdentityAssignmentPolicy::Unrestricted,
            prompt_prefix: HarnessPromptPrefixConfiguration {
                content: prompt.into(),
                initial_delivery: HarnessInitialDelivery::Prepend,
                context_compression_delivery: HarnessContextCompressionDelivery::Deferred,
            },
            skills: HarnessSkillsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: Vec::new(),
            },
            tools: HarnessToolsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: Vec::new(),
                schema_boundary: "Application tools".into(),
                mcp_servers: Vec::new(),
            },
            runtime: HarnessRuntimeConfiguration {
                preferred_model: None,
                sandbox: HarnessSandbox::WorkspaceWrite,
                approval_policy: HarnessApprovalPolicy::Never,
                authority_summary: "User authority".into(),
            },
            hooks: Vec::new(),
            update_policy: HarnessUpdatePolicy::NotConfigured {
                reason: "Not connected".into(),
            },
        }
    }

    fn service() -> HarnessCatalogService {
        HarnessCatalogService::in_memory()
    }

    #[test]
    fn workflow_materialization_reuses_unchanged_configuration_and_versions_real_changes() {
        let service = service();
        let harness_id = HarnessId::new("harness-workflow-stable").unwrap();

        let first = service
            .materialize_workflow_harness(
                harness_id.clone(),
                "Workflow reviewer".into(),
                configuration("Version one"),
            )
            .unwrap();
        let session_override = service
            .publish_session_override(
                &harness_id,
                "session-1".into(),
                &first,
                configuration("Session override"),
            )
            .unwrap();
        let repeated = service
            .materialize_workflow_harness(
                harness_id.clone(),
                "Renamed Workflow reviewer".into(),
                configuration("Version one"),
            )
            .unwrap();
        let second = service
            .materialize_workflow_harness(
                harness_id.clone(),
                "Renamed Workflow reviewer".into(),
                configuration("Version two"),
            )
            .unwrap();

        assert_eq!(first, repeated);
        assert_eq!(first.version().get(), 1);
        assert_eq!(session_override.reference.version().get(), 2);
        assert_eq!(second.version().get(), 3);
        let (record, draft, versions) = service.load(&harness_id).unwrap();
        assert_eq!(record.metadata.name, "Renamed Workflow reviewer");
        assert!(draft.is_none());
        assert_eq!(versions.len(), 3);
    }

    #[test]
    fn reusable_publication_has_one_draft_and_exact_numbered_versions() {
        let service = service();
        let harness = service
            .create_harness("Plan builder".into(), configuration("Version one"))
            .unwrap();

        let first = service.publish_draft(&harness.id).unwrap();
        assert_eq!(first.reference.version().get(), 1);
        let saved = service
            .save_draft(
                &harness.id,
                Some(&first.reference),
                configuration("Version two"),
                0,
            )
            .unwrap();
        assert_eq!(saved.draft_revision, 1);
        let second = service.publish_draft(&harness.id).unwrap();

        assert_eq!(second.reference.version().get(), 2);
        assert!(service.load(&harness.id).unwrap().1.is_none());
    }

    #[test]
    fn session_publication_is_immutable_without_creating_a_persistent_draft() {
        let service = service();
        let harness = service
            .create_harness("Plan builder".into(), configuration("Reusable"))
            .unwrap();
        let reusable = service.publish_draft(&harness.id).unwrap();

        let session_version = service
            .publish_session_override(
                &harness.id,
                "session-1".into(),
                &reusable.reference,
                configuration("Session-specific"),
            )
            .unwrap();

        assert!(matches!(
            session_version.scope,
            HarnessVersionScope::SessionSpecific { .. }
        ));
        assert!(service.load(&harness.id).unwrap().1.is_none());
    }

    #[test]
    fn resolving_an_old_reference_applies_the_engine_owned_replacement_chain() {
        let service = service();
        let harness = service
            .create_harness("Plan builder".into(), configuration("One"))
            .unwrap();
        let first = service.publish_draft(&harness.id).unwrap();
        service
            .save_draft(
                &harness.id,
                Some(&first.reference),
                configuration("Two"),
                0,
            )
            .unwrap();
        let second = service.publish_draft(&harness.id).unwrap();
        service
            .order_replacement(first.reference.clone(), second.reference.clone())
            .unwrap();

        let resolved = service.resolve(&first.reference).unwrap();

        assert!(resolved.was_replaced());
        assert_eq!(resolved.version.reference, second.reference);
        assert_eq!(resolved.replacement_path.len(), 2);
    }
}
