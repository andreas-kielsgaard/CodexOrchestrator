import {
  emptySessionNavigation,
  type SessionNavigationClient,
  type SessionNavigationData,
} from '../../application/agentSessions/organization';
import { sessionSummary } from './testFixtures';
export function navigationData(): SessionNavigationData {
  return {
    ...emptySessionNavigation(),
    repositories: [
      { id: 'repo-a', name: 'Alpha' },
      { id: 'repo-b', name: 'Empty repo' },
    ],
    instances: [
      {
        id: 'flow-a',
        name: 'Feature build',
        repositoryId: 'repo-a',
        nodes: [{ id: 'worker', name: 'Worker' }],
      },
    ],
    summaries: Array.from({ length: 8 }, (_, i) => ({
      ...sessionSummary(),
      id: `session-${i + 1}`,
      title: `Session ${i + 1}`,
    })),
    organization: Array.from({ length: 6 }, (_, i) => ({
      sessionId: `session-${i + 1}`,
      placement: { kind: 'repository' as const, repositoryId: 'repo-a' },
      pinnedAt: null,
    })),
    owners: [
      { sessionId: 'session-7', instanceId: 'flow-a', nodeId: 'worker', nodeName: 'Worker' },
    ],
  };
}
export function recordedNavigation(
  initial: SessionNavigationData = navigationData(),
): SessionNavigationClient {
  let data = structuredClone(initial);
  return {
    load: async () => structuredClone(data),
    move: async (sessionId, placement) => {
      const existing = data.organization.find((o) => o.sessionId === sessionId);
      data = {
        ...data,
        organization: [
          ...data.organization.filter((o) => o.sessionId !== sessionId),
          { sessionId, placement, pinnedAt: existing?.pinnedAt ?? null },
        ],
      };
    },
    pin: async (sessionId, pinned) => {
      const existing = data.organization.find((o) => o.sessionId === sessionId);
      data = {
        ...data,
        organization: [
          ...data.organization.filter((o) => o.sessionId !== sessionId),
          {
            sessionId,
            placement: existing?.placement ?? { kind: 'default' },
            pinnedAt: pinned ? '2026-09-14T12:00:00Z' : null,
          },
        ],
      };
    },
  };
}
