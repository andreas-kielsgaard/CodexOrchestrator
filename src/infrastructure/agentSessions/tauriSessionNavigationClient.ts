import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { SessionNavigationClient } from '../../application/agentSessions/organization';
export const tauriSessionNavigationClient: SessionNavigationClient = {
  createWorktree: (repositoryId, branchRef) =>
    invoke('create_worktree_review_worktree', { input: { repositoryId, branchRef } }),
  markAttention: (sessionId, unread) =>
    invoke('mark_agent_session_attention', { sessionId, unread }),
  previewGitAction: (input) => invoke('preview_session_git_action', { input }),
  executeGitAction: (input, expected) => invoke('execute_session_git_action', { input, expected }),
  inspectWorktree: (repositoryId, branchRef, worktree) =>
    invoke('inspect_session_worktree', {
      repositoryId,
      branchRef,
      deviceId: worktree.deviceId,
      path: worktree.path,
    }),
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
