import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { SessionNavigationClient } from '../../application/agentSessions/organization';
export const tauriSessionNavigationClient: SessionNavigationClient = {
  load: () => invoke('load_agent_session_navigation'),
  move: (sessionId, placement, orderedIds) =>
    invoke('move_agent_session', { sessionId, placement, orderedIds }),
  pin: (sessionId, pinned) => invoke('pin_agent_session', { sessionId, pinned }),
  reorder: (scope, orderedIds) => invoke('reorder_session_navigation', { scope, orderedIds }),
  subscribeChanged: async (listener) => {
    const stops = await Promise.all([
      listen('workflow-instance-updated', listener),
      listen('agent-session-navigation-changed', listener),
    ]);
    return () => stops.forEach((stop) => stop());
  },
};
