import { targetId } from '../../application/agentSessions/navigation';
import {
  emptySessionNavigation,
  type SessionNavigationClient,
  type SessionNavigationData,
} from '../../application/agentSessions/organization';
import { orderScopeKey } from '../../application/agentSessions/navigationOrder';
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
export function populatedNavigationData(): SessionNavigationData {
  const base = navigationData();
  return {
    ...base,
    repositories: [...base.repositories, { id: 'repo-c', name: 'Story Game' }],
    instances: [
      ...base.instances,
      { ...base.instances[0], id: 'flow-b', name: 'Release preparation' },
    ],
    summaries: Array.from({ length: 18 }, (_, i) => ({
      ...sessionSummary(),
      id: `demo-${i + 1}`,
      title: `Discussion ${i + 1}`,
      updatedAt: `2026-09-${String(28 - i).padStart(2, '0')}T12:00:00Z`,
    })),
    organization: Array.from({ length: 18 }, (_, i) => ({
      sessionId: `demo-${i + 1}`,
      placement:
        i < 8
          ? { kind: 'repository' as const, repositoryId: 'repo-a' }
          : i < 11
            ? { kind: 'workflow_instance' as const, instanceId: 'flow-a' }
            : { kind: 'default' as const },
      pinnedAt: i < 6 ? `2026-09-14T12:00:0${i}Z` : null,
    })),
    owners: Array.from({ length: 5 }, (_, i) => ({
      sessionId: `demo-${i + 12}`,
      instanceId: 'flow-a',
      nodeId: 'worker',
      nodeName: 'Worker',
    })),
  };
}
export function recordedNavigation(
  initial: SessionNavigationData = navigationData(),
): SessionNavigationClient {
  let data = structuredClone(initial);
  return {
    load: async () => structuredClone(data),
    move: async (sessionId, placement, orderedIds) => {
      const existing = data.organization.find((o) => o.sessionId === sessionId);
      data = {
        ...data,
        orders: orderedIds
          ? [
              ...data.orders.filter(
                (o) => !(o.scope.kind === 'sessions' && o.scope.folderId === targetId(placement)),
              ),
              { scope: { kind: 'sessions', folderId: targetId(placement) }, orderedIds },
            ]
          : data.orders,
        organization: [
          ...data.organization.filter((o) => o.sessionId !== sessionId),
          { sessionId, placement, pinnedAt: existing?.pinnedAt ?? null },
        ],
      };
    },
    reorder: async (scope, orderedIds) => {
      data = {
        ...data,
        orders: [
          ...data.orders.filter((o) => orderScopeKey(o.scope) !== orderScopeKey(scope)),
          { scope, orderedIds },
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
