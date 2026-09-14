import { invoke } from '@tauri-apps/api/core';
import type { AgentSessionImportClient } from '../../application/agentSessions/importContracts';
export const tauriAgentSessionImportClient: AgentSessionImportClient = {
  preview: (link) => invoke('preview_codex_import', { link }),
  importConversation: (input) => invoke('import_codex_conversation', { input }),
};
