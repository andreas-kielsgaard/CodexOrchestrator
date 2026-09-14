//! Preview and persist an independent Codex conversation, without submitting input.
use super::{AgentSessionApplication, AgentSessionOwnership, CreateAgentSessionCommand};
use crate::agent_sessions::{domain::*, imports::*, ports::*};
use std::sync::{Arc, Mutex};

pub(crate) struct AgentSessionImportService {
    pub application: Arc<AgentSessionApplication>,
    pub homes: Arc<dyn ImportHomeSource>,
    pub history: Arc<dyn CodexHistorySource>,
    pub store: Arc<dyn AgentSessionImportStore>,
    pub lane: Mutex<()>,
}
impl AgentSessionImportService {
    pub(crate) fn preview(&self, link: &str) -> Result<ImportPreview, String> {
        let id = thread_id_from_link(link)?;
        let home = self.homes.selected_import_home()?;
        let source = self.history.read(&home, &id)?;
        let last = source
            .turns
            .last()
            .ok_or("This conversation has no settled turns to import")?;
        let cwd = existing_directory(source.cwd.as_deref());
        self.application
            .resolve_default_creation(cwd.as_deref())
            .map_err(|e| e.to_string())?;
        let capability = self
            .application
            .capability_profiles
            .as_ref()
            .ok_or("Choose a default Capability Profile before importing")?
            .default_profile()
            .map_err(|e| e.to_string())?
            .name;
        Ok(ImportPreview {
            thread_id: id,
            title: source.title,
            source_directory: source.cwd,
            allocate_workspace: cwd.is_none(),
            turn_count: source.turns.len(),
            last_turn_id: last.id.clone(),
            native_home: home.path,
            profile_id: home.profile_id,
            capability_profile: capability,
            excerpt: last
                .items
                .iter()
                .rev()
                .find(|i| i.kind == "assistant")
                .map(|i| i.text.chars().take(400).collect())
                .unwrap_or_default(),
        })
    }

    pub(crate) fn import(&self, command: ImportCommand) -> Result<AgentSessionId, String> {
        let _lane = self.lane.lock().map_err(|_| "Import lock unavailable")?;
        uuid::Uuid::parse_str(&command.request_id)
            .map_err(|_| "Invalid import request identity")?;
        let source_id = thread_id_from_link(&command.link)?;
        let home = self.homes.selected_import_home()?;
        if home.profile_id != command.profile_id {
            return Err("The selected Codex home changed. Preview the conversation again.".into());
        }
        let mut receipt = if let Some(receipt) = self.store.receipt(&command.request_id)? {
            if receipt.source_thread_id != source_id
                || receipt.last_turn_id != command.last_turn_id
                || receipt.home.profile_id != home.profile_id
                || receipt.home.filesystem_identity != home.filesystem_identity
            {
                return Err(
                    "This import request belongs to a different conversation or native home".into(),
                );
            }
            if receipt.completed {
                return Ok(receipt.session.id);
            }
            if receipt.fork.is_none() {
                return Err("The previous fork outcome is uncertain. Orchid will not repeat it automatically.".into());
            }
            receipt
        } else {
            let source = self.history.read(&home, &source_id)?;
            if !source.turns.iter().any(|t| t.id == command.last_turn_id) {
                return Err(
                    "The previewed history boundary is no longer available. Preview again.".into(),
                );
            }
            let id =
                AgentSessionId::new(uuid::Uuid::new_v4().to_string()).map_err(|e| e.to_string())?;
            let session = self
                .application
                .prepare_default_session(
                    id,
                    CreateAgentSessionCommand {
                        title: Some(source.title.clone()),
                        working_directory: existing_directory(source.cwd.as_deref()),
                        requested_options: Default::default(),
                    },
                    AgentSessionOwnership::default(),
                )
                .map_err(|e| e.to_string())?;
            let receipt = ImportReceipt {
                request_id: command.request_id,
                source_thread_id: source_id,
                last_turn_id: command.last_turn_id,
                home: home.clone(),
                session,
                fork: None,
                completed: false,
            };
            self.store.claim(&receipt)?;
            receipt
        };
        if receipt.fork.is_none() {
            let cwd = receipt
                .session
                .working_directory
                .as_deref()
                .ok_or("Import workspace was not prepared")?;
            let fork =
                self.history
                    .fork(&home, &receipt.source_thread_id, &receipt.last_turn_id, cwd)?;
            if fork.id == receipt.source_thread_id
                || fork.turns.last().map(|t| t.id.as_str()) != Some(&receipt.last_turn_id)
            {
                return Err(
                    "Codex did not return the requested independent history boundary".into(),
                );
            }
            receipt.session.runtime_binding.external_context_id =
                Some(ExternalRuntimeContextId::new(fork.id.clone()).map_err(|e| e.to_string())?);
            receipt.session.runtime_binding.runtime_version = fork.version.clone();
            receipt.fork = Some(fork);
            self.store.record_fork(&receipt)?;
        }
        let current = self.homes.selected_import_home()?;
        if current.profile_id != home.profile_id
            || current.filesystem_identity != home.filesystem_identity
        {
            return Err(
                "Native Codex home changed during import; retry with the original home selected"
                    .into(),
            );
        }
        self.store.materialize(&receipt)?;
        Ok(receipt.session.id)
    }
}
