import { composeProductOrchestrationReadModels } from './orchestrations';
import type { ProductReadModelsV1 } from './orchestrations';
import { recordedProductReadCompositionInput } from '../dev/orchestrationSection/recordedProductReadCompositionInput';
import {
  buildAgentSessionNavigation,
  type AgentSessionNavigationFolder,
  type AgentSessionNavigationNode,
} from './agentSessionNavigation';
import type { AgentSessionSummaryDto } from './agentSessions';

describe('Agent Session product navigation projection', () => {
  it('uses recorded containment and semantic references for Epic, Sprint, planning, and execution folders', () => {
    const read = composeProductOrchestrationReadModels(recordedProductReadCompositionInput);
    const navigation = buildAgentSessionNavigation({
      orchestrations: read,
      summaries: [
        summary('recorded-epic-runner-manual-continuation-ready'),
        summary('recorded-sprint-control-surface-discovery'),
        summary('recorded-session-planner-r4-integration'),
        summary('recorded-session-WU-ECS2E'),
        summary('recorded-session-reviewer-WU-ECS2E'),
        summary('independent-session'),
      ],
    });

    const epic = folder(navigation.roots, 'epic:epic-codex-runner-workspace');
    expect(epic.children).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: 'session',
          summary: expect.objectContaining({
            id: 'recorded-epic-runner-manual-continuation-ready',
          }),
          relationshipRoles: ['Epic Runner'],
        }),
      ]),
    );
    const sprint = folder(
      epic.children,
      'epic:epic-codex-runner-workspace:sprint:sprint-control-surface',
    );
    expect(folder(sprint.children, `${sprint.id}:planning`).children).toContainEqual(
      expect.objectContaining({ kind: 'folder', label: 'Integrated detail surfaces' }),
    );
    const workUnit = folder(
      folder(sprint.children, `${sprint.id}:execution`).children,
      `${sprint.id}:execution:WU-ECS2E`,
    );
    expect(sessionIds(workUnit)).toEqual(
      expect.arrayContaining(['recorded-session-WU-ECS2E', 'recorded-session-reviewer-WU-ECS2E']),
    );
    expect(navigation.sessions.get('recorded-session-WU-ECS2E')?.productLocations).toEqual([
      expect.objectContaining({
        kind: 'work_unit',
        workUnitId: 'WU-ECS2E',
        sprintPlannerActivityId: 'planner-r4-integration',
      }),
    ]);
    expect(
      sessionIds(
        folder(folder(navigation.roots, 'independent').children, 'independent:unassigned'),
      ),
    ).toEqual(['independent-session']);
  });

  it('places a Session with multiple legitimate targets once and preserves every explicit view', () => {
    const read = structuredClone(
      composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
    ) as Mutable<ProductReadModelsV1>;
    const sessionId = 'recorded-epic-runner-manual-continuation-ready';
    const extra = {
      agentSessionRefId: 'session-ref-cross-sprint',
      agentSessionId: sessionId,
      title: 'Orientation discovery handler',
      source: read.epics[0].source,
      targetKind: 'sprint' as const,
      targetId: 'sprint-control-surface',
      semanticRole: 'sprint_runner' as const,
    };
    read.epics[0].agentSessionReferences.push(extra);

    const navigation = buildAgentSessionNavigation({
      orchestrations: read,
      summaries: [summary(sessionId)],
    });
    const related = folder(
      folder(navigation.roots, 'epic:epic-codex-runner-workspace').children,
      'epic:epic-codex-runner-workspace:related',
    );

    expect(sessionIds(related)).toEqual([sessionId]);
    expect(navigation.sessions.get(sessionId)?.productLocations).toEqual([
      expect.objectContaining({ kind: 'epic' }),
      expect.objectContaining({ kind: 'sprint', sprintId: 'sprint-control-surface' }),
    ]);
    expect(navigation.sessions.get(sessionId)?.relationshipRoles).toEqual([
      'Epic Runner',
      'Sprint Runner',
    ]);
  });

  it('keeps an uninitiated durable Plan Builder Session in an independent draft folder', () => {
    const navigation = buildAgentSessionNavigation({
      summaries: [summary('draft-session')],
      planningDrafts: [
        {
          epicPlanningDraftId: 'draft-1',
          agentSessionId: 'draft-session',
          title: 'Navigation redesign',
          status: 'active',
          createdAt: '2026-07-29T09:00:00.000Z',
          updatedAt: '2026-07-29T09:00:00.000Z',
        },
      ],
    });
    const drafts = folder(
      folder(navigation.roots, 'independent').children,
      'independent:planning-drafts',
    );
    expect(sessionIds(drafts)).toEqual(['draft-session']);
    expect(navigation.sessions.get('draft-session')?.productLocations).toEqual([
      {
        kind: 'epic_planning_draft',
        epicPlanningDraftId: 'draft-1',
        label: 'Navigation redesign',
      },
    ]);
  });
});

type Mutable<T> = {
  -readonly [K in keyof T]: T[K] extends readonly (infer U)[]
    ? Mutable<U>[]
    : T[K] extends object
      ? Mutable<T[K]>
      : T[K];
};

function summary(id: string): AgentSessionSummaryDto {
  return {
    id,
    title: id,
    availability: 'available',
    hasActiveInvocation: false,
    latestInvocationStatus: 'completed',
    createdAt: '2026-07-29T09:00:00.000Z',
    updatedAt: '2026-07-29T09:00:00.000Z',
  };
}

function folder(nodes: readonly AgentSessionNavigationNode[], id: string) {
  const found = nodes.find(
    (node): node is AgentSessionNavigationFolder => node.kind === 'folder' && node.id === id,
  );
  if (!found) throw new Error(`Missing folder ${id}`);
  return found;
}

function sessionIds(folderNode: AgentSessionNavigationFolder) {
  return folderNode.children.flatMap((node) => (node.kind === 'session' ? [node.summary.id] : []));
}
