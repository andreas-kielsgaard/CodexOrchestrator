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
  it('uses titled sections and typed Epic, Sprint, planning-step, and Work Unit containment', () => {
    const read = composeProductOrchestrationReadModels(recordedProductReadCompositionInput);
    const navigation = buildAgentSessionNavigation({
      orchestrations: read,
      summaries: [
        summary('recorded-epic-runner-manual-continuation-ready'),
        summary('recorded-sprint-control-surface-discovery'),
        summary('recorded-session-planner-r4-integration'),
        summary('recorded-session-WU-ECS2E'),
        summary('recorded-implementer-WU-ECS2E'),
        summary('independent-session'),
      ],
    });

    expect(navigation.sections.map(({ id, label }) => ({ id, label }))).toEqual([
      { id: 'epics', label: 'Epics' },
      { id: 'independent', label: 'Independent Sessions' },
    ]);
    const epic = folder(section(navigation, 'epics').children, 'epic:epic-codex-runner-workspace');
    expect(epic.children[0]).toEqual(
      expect.objectContaining({
        kind: 'session',
        summary: expect.objectContaining({
          id: 'recorded-epic-runner-manual-continuation-ready',
        }),
        relationshipRoles: ['Epic Runner'],
      }),
    );
    const sprint = folder(
      epic.children,
      'epic:epic-codex-runner-workspace:sprint:sprint-control-surface',
    );
    expect(sprint.children[0]).toEqual(
      expect.objectContaining({
        kind: 'session',
        summary: expect.objectContaining({ id: 'recorded-sprint-control-surface-discovery' }),
      }),
    );
    const planningStep = folder(sprint.children, `${sprint.id}:activity:planner-r4-integration`);
    expect(planningStep.label).toBe('Integrated detail surfaces');
    expect(planningStep.children[0]).toEqual(
      expect.objectContaining({
        kind: 'session',
        summary: expect.objectContaining({ id: 'recorded-session-planner-r4-integration' }),
      }),
    );
    const workUnit = folder(
      planningStep.children,
      `${sprint.id}:activity:planner-r4-integration:work-unit:WU-ECS2E`,
    );
    expect(sessionIds(workUnit)).toEqual(
      expect.arrayContaining(['recorded-session-WU-ECS2E', 'recorded-implementer-WU-ECS2E']),
    );
    expect(sessionIds(workUnit)).toHaveLength(2);
    expect(navigation.sessions.get('recorded-session-WU-ECS2E')).toEqual(
      expect.objectContaining({
        relationshipRoles: ['Work Unit Handler'],
        productLocations: [
          expect.objectContaining({
            kind: 'work_unit',
            workUnitId: 'WU-ECS2E',
            workSlicePlanningPointId: 'planner-r4-integration',
          }),
        ],
      }),
    );
    expect(navigation.sessions.get('recorded-implementer-WU-ECS2E')).toEqual(
      expect.objectContaining({ relationshipRoles: ['Work Unit Implementer'] }),
    );
    expect(navigation.sessions.has('recorded-session-reviewer-WU-ECS2E')).toBe(false);
    expect(sessionIds(section(navigation, 'independent'))).toEqual(['independent-session']);
  });

  it('places a Session with multiple same-Epic targets once at the truthful shared Epic', () => {
    const read = structuredClone(
      composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
    ) as Mutable<ProductReadModelsV1>;
    const sessionId = 'recorded-epic-runner-manual-continuation-ready';
    read.epics[0].agentSessionReferences.push({
      agentSessionRefId: 'session-ref-cross-sprint',
      agentSessionId: sessionId,
      title: 'Orientation discovery handler',
      source: read.epics[0].source,
      targetKind: 'sprint',
      targetId: 'sprint-control-surface',
      semanticRole: 'sprint',
    });

    const navigation = buildAgentSessionNavigation({
      orchestrations: read,
      summaries: [summary(sessionId)],
    });
    const epic = folder(section(navigation, 'epics').children, 'epic:epic-codex-runner-workspace');

    expect(sessionIds(epic)).toEqual([sessionId]);
    expect(navigation.sessions.get(sessionId)?.productLocations).toEqual([
      expect.objectContaining({ kind: 'epic' }),
      expect.objectContaining({ kind: 'sprint', sprintId: 'sprint-control-surface' }),
    ]);
    expect(navigation.sessions.get(sessionId)?.relationshipRoles).toEqual([
      'Epic Runner',
      'Sprint Runner',
    ]);
  });

  it('keeps durable drafts and provider-neutral Sessions directly in Independent Sessions', () => {
    const navigation = buildAgentSessionNavigation({
      summaries: [summary('draft-session'), summary('provider-neutral')],
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
    const independent = section(navigation, 'independent');

    expect(independent.children.every(({ kind }) => kind === 'session')).toBe(true);
    expect(sessionIds(independent)).toEqual(['draft-session', 'provider-neutral']);
    expect(navigation.sessions.get('draft-session')?.productLocations).toEqual([
      {
        kind: 'epic_planning_draft',
        epicPlanningDraftId: 'draft-1',
        label: 'Navigation redesign',
      },
    ]);
    expect(navigation.sessions.get('provider-neutral')?.productLocations).toEqual([]);
  });

  it('projects linked Work Unit attention separately from a completed Session process', () => {
    const read = structuredClone(
      composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
    ) as Mutable<ProductReadModelsV1>;
    const units = read.epics
      .flatMap((epic) => epic.sprints)
      .flatMap((sprint) => sprint.revisionViews)
      .flatMap((view) => view.workUnits)
      .filter(({ workUnitId }) => workUnitId === 'WU-ECS2E');
    if (units.length === 0) throw new Error('Missing recorded WU-ECS2E Work Unit');
    units.forEach((unit) => {
      unit.executionState = { state: 'attention', recordedAt: '2026-08-08T19:30:00.000Z' };
    });

    const navigation = buildAgentSessionNavigation({
      orchestrations: read,
      summaries: [summary('recorded-session-WU-ECS2E')],
    });

    expect(navigation.sessions.get('recorded-session-WU-ECS2E')).toEqual(
      expect.objectContaining({
        summary: expect.objectContaining({ latestInvocationStatus: 'completed' }),
        semanticPresentation: {
          kind: 'attention',
          label: 'Work Unit needs attention',
          technicalDetails: expect.arrayContaining([
            'Target: work_unit_execution',
            'Work Unit state: attention',
          ]),
        },
      }),
    );
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

function section(
  navigation: ReturnType<typeof buildAgentSessionNavigation>,
  id: 'epics' | 'independent',
) {
  const found = navigation.sections.find((item) => item.id === id);
  if (!found) throw new Error(`Missing section ${id}`);
  return found;
}

function folder(nodes: readonly AgentSessionNavigationNode[], id: string) {
  const found = nodes.find(
    (node): node is AgentSessionNavigationFolder => node.kind === 'folder' && node.id === id,
  );
  if (!found) throw new Error(`Missing folder ${id}`);
  return found;
}

function sessionIds(node: { readonly children: readonly AgentSessionNavigationNode[] }) {
  return node.children.flatMap((child) => (child.kind === 'session' ? [child.summary.id] : []));
}
