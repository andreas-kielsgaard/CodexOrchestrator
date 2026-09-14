import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  SessionNavigationAgentAccess,
  SessionNavigationCommandRequest,
} from '../../application/agentSessions/agentAccess';
export const tauriSessionNavigationAgent: SessionNavigationAgentAccess = {
  subscribe: (listener) =>
    listen<SessionNavigationCommandRequest>('session-navigation-command', ({ payload }) =>
      listener(payload),
    ),
  complete: (requestId, state, error) =>
    invoke('complete_session_navigation_command', { requestId, state, error }),
};
