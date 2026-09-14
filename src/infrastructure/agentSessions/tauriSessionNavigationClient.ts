import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { SessionNavigationClient } from '../../application/agentSessions/organization';
export const tauriSessionNavigationClient: SessionNavigationClient = {
  load: () => invoke('load_agent_session_navigation'),
  move: (sessionId, placement) => invoke('move_agent_session', { sessionId, placement }),
  pin: (sessionId, pinned) => invoke('pin_agent_session', { sessionId, pinned }),
  subscribeChanged: async (listener) => listen('workflow-instance-updated', listener),
};
