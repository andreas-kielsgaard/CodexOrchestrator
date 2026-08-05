import type { AgentSessionSummaryDto } from './agentSessions';
import type {
  AgentSessionSemanticRole,
  EpicPlanningDraftSummary,
  ProductAgentSessionReferenceReadModelV1,
  ProductReadModelsV1,
} from './orchestrations';

export type AgentSessionProductLocation =
  | {
      readonly kind: 'epic';
      readonly epicId: string;
      readonly label: string;
    }
  | {
      readonly kind: 'sprint';
      readonly epicId: string;
      readonly sprintId: string;
      readonly label: string;
    }
  | {
      readonly kind: 'work_slice_planning_point';
      readonly epicId: string;
      readonly sprintId: string;
      readonly revisionId: string;
      readonly workSlicePlanningPointId: string;
      readonly label: string;
    }
  | {
      readonly kind: 'work_unit';
      readonly epicId: string;
      readonly sprintId: string;
      readonly revisionId: string;
      readonly workSlicePlanningPointId: string;
      readonly workUnitId: string;
      readonly label: string;
      readonly inspectionState?: Readonly<{
        readonly tab: 'activity' | 'evidence';
        readonly activityId: string;
        readonly sessionId: string;
        readonly invocationId: string;
      }>;
    }
  | {
      readonly kind: 'epic_planning_draft';
      readonly epicPlanningDraftId: string;
      readonly label: string;
    };

export interface AgentSessionNavigationIdentity {
  readonly sessionId: string;
  readonly agentName: string;
  readonly harnessRole: string;
  readonly visualIdentity?: Readonly<{
    readonly token: string;
    readonly accentColor: string;
  }>;
}

export interface AgentSessionNavigationSession {
  readonly kind: 'session';
  readonly id: string;
  readonly summary: AgentSessionSummaryDto;
  readonly relationshipRoles: readonly string[];
  readonly productLocations: readonly AgentSessionProductLocation[];
  readonly identity?: AgentSessionNavigationIdentity;
}

export interface AgentSessionNavigationFolder {
  readonly kind: 'folder';
  readonly id: string;
  readonly label: string;
  readonly children: readonly AgentSessionNavigationNode[];
}

export type AgentSessionNavigationNode =
  AgentSessionNavigationFolder | AgentSessionNavigationSession;

export interface AgentSessionNavigationSection {
  readonly kind: 'section';
  readonly id: 'epics' | 'independent';
  readonly label: string;
  readonly children: readonly AgentSessionNavigationNode[];
}

export interface AgentSessionNavigationModel {
  readonly sections: readonly AgentSessionNavigationSection[];
  readonly sessions: ReadonlyMap<string, AgentSessionNavigationSession>;
}

export interface AgentSessionNavigationInput {
  readonly summaries: readonly AgentSessionSummaryDto[];
  readonly orchestrations?: ProductReadModelsV1;
  readonly planningDrafts?: readonly EpicPlanningDraftSummary[];
  readonly identities?: readonly AgentSessionNavigationIdentity[];
}

interface MutableFolder {
  readonly kind: 'folder';
  readonly id: string;
  readonly label: string;
  readonly children: Array<MutableFolder | AgentSessionNavigationSession>;
}

interface ResolvedReference {
  readonly reference: ProductAgentSessionReferenceReadModelV1;
  readonly locations: readonly AgentSessionProductLocation[];
  readonly folderPath: readonly { readonly id: string; readonly label: string }[];
}

/** Read-only projection. Placement comes only from typed product references. */
export function buildAgentSessionNavigation(
  input: AgentSessionNavigationInput,
): AgentSessionNavigationModel {
  const epicRoots: MutableFolder[] = [];
  const independentSessions: AgentSessionNavigationSession[] = [];
  const folders = new Map<string, MutableFolder>();
  const referencesBySession = collectReferences(input.orchestrations);
  const draftsBySession = new Map(
    (input.planningDrafts ?? []).map((draft) => [draft.agentSessionId, draft]),
  );
  const identities = new Map(
    (input.identities ?? []).map((identity) => [identity.sessionId, identity]),
  );
  const sessions = new Map<string, AgentSessionNavigationSession>();

  const folder = (parent: MutableFolder | null, id: string, label: string): MutableFolder => {
    const existing = folders.get(id);
    if (existing) return existing;
    const created: MutableFolder = { kind: 'folder', id, label, children: [] };
    folders.set(id, created);
    if (parent) parent.children.push(created);
    else epicRoots.push(created);
    return created;
  };

  const folderPath = (path: readonly { readonly id: string; readonly label: string }[]) => {
    let current: MutableFolder | null = null;
    for (const item of path) current = folder(current, item.id, item.label);
    return current;
  };

  for (const summary of input.summaries) {
    const resolved = referencesBySession.get(summary.id) ?? [];
    const draft = draftsBySession.get(summary.id);
    const locations = uniqueLocations([
      ...resolved.flatMap(({ locations: items }) => items),
      ...(draft
        ? [
            {
              kind: 'epic_planning_draft' as const,
              epicPlanningDraftId: draft.epicPlanningDraftId,
              label: draft.title ?? 'Untitled Epic planning draft',
            },
          ]
        : []),
    ]);
    const relationshipRoles = unique(
      resolved.map(({ reference }) => roleLabel(reference.semanticRole)),
    );
    const item: AgentSessionNavigationSession = {
      kind: 'session',
      id: `session:${summary.id}`,
      summary,
      relationshipRoles,
      productLocations: locations,
      ...(identities.has(summary.id) ? { identity: identities.get(summary.id)! } : {}),
    };
    sessions.set(summary.id, item);

    const placementPaths = uniquePaths(
      resolved.map(({ folderPath: path }) => path).filter((path) => path.length > 0),
    );
    if (!draft && placementPaths.length === 1) {
      folderPath(placementPaths[0])?.children.push(item);
      continue;
    }

    if (!draft && placementPaths.length > 1) {
      const epicRootIds = unique(placementPaths.map((path) => path[0].id));
      if (epicRootIds.length === 1) {
        folderPath([placementPaths[0][0]])?.children.push(item);
        continue;
      }
    }

    independentSessions.push(item);
  }

  return {
    sections: [
      {
        kind: 'section',
        id: 'epics',
        label: 'Epics',
        children: sortNodes(epicRoots),
      },
      {
        kind: 'section',
        id: 'independent',
        label: 'Independent Sessions',
        children: sortNodes(independentSessions),
      },
    ],
    sessions,
  };
}

function collectReferences(orchestrations?: ProductReadModelsV1) {
  const result = new Map<string, ResolvedReference[]>();
  for (const epic of orchestrations?.epics ?? []) {
    for (const reference of epic.agentSessionReferences) {
      addResolvedReference(result, reference.agentSessionId, resolveReference(epic, reference));
    }
  }
  for (const reference of orchestrations?.unassociatedAgentSessionReferences ?? []) {
    addResolvedReference(result, reference.agentSessionId, {
      reference,
      locations: [],
      folderPath: [],
    });
  }
  return result;
}

function addResolvedReference(
  result: Map<string, ResolvedReference[]>,
  sessionId: string,
  resolved: ResolvedReference,
) {
  const existing = result.get(sessionId) ?? [];
  if (
    !existing.some(
      ({ reference }) => reference.agentSessionRefId === resolved.reference.agentSessionRefId,
    )
  )
    existing.push(resolved);
  result.set(sessionId, existing);
}

function resolveReference(
  epic: ProductReadModelsV1['epics'][number],
  reference: ProductAgentSessionReferenceReadModelV1,
): ResolvedReference {
  const epicPath = [{ id: `epic:${epic.epicId}`, label: epic.title }];
  if (reference.targetKind === 'epic')
    return {
      reference,
      locations: [{ kind: 'epic', epicId: epic.epicId, label: epic.title }],
      folderPath: epicPath,
    };

  const sprint = epic.sprints.find(
    (item) =>
      (reference.targetKind === 'sprint' && item.sprintId === reference.targetId) ||
      item.agentSessionReferences.some(
        ({ agentSessionRefId }) => agentSessionRefId === reference.agentSessionRefId,
      ),
  );
  if (!sprint) return { reference, locations: [], folderPath: [] };

  const sprintPath = [
    ...epicPath,
    { id: `epic:${epic.epicId}:sprint:${sprint.sprintId}`, label: sprint.title },
  ];
  if (reference.targetKind === 'sprint')
    return {
      reference,
      locations: [
        {
          kind: 'sprint',
          epicId: epic.epicId,
          sprintId: sprint.sprintId,
          label: sprint.title,
        },
      ],
      folderPath: sprintPath,
    };

  if (reference.targetKind === 'work_slice_planning_point') {
    const matches = sprint.revisionViews.flatMap((view) =>
      view.workSlicePlanningPointGroups
        .filter(({ workSlicePlanningPointId }) => workSlicePlanningPointId === reference.targetId)
        .map((activity) => ({ view, activity })),
    );
    const match = matches[0];
    if (!match) return { reference, locations: [], folderPath: [] };
    return {
      reference,
      locations: matches.map(({ view, activity }) => ({
        kind: 'work_slice_planning_point' as const,
        epicId: epic.epicId,
        sprintId: sprint.sprintId,
        revisionId: view.sprintPlanRevisionId,
        workSlicePlanningPointId: activity.workSlicePlanningPointId,
        label: activity.title,
      })),
      folderPath: [
        ...sprintPath,
        {
          id: `${sprintPath[1].id}:activity:${match.activity.workSlicePlanningPointId}`,
          label: match.activity.title,
        },
      ],
    };
  }

  if (reference.targetKind === 'work_unit_execution') {
    const matches = sprint.revisionViews.flatMap((view) =>
      view.workUnits
        .filter((unit) =>
          unit.attempts.some(
            ({ workUnitExecutionId }) => workUnitExecutionId === reference.targetId,
          ),
        )
        .flatMap((unit) => {
          const activity = view.workSlicePlanningPointGroups.find(({ workUnitScopeIds }) =>
            workUnitScopeIds.includes(unit.workUnitScopeId),
          );
          return activity ? [{ view, unit, activity }] : [];
        }),
    );
    const match = matches[0];
    if (!match) return { reference, locations: [], folderPath: [] };
    return {
      reference,
      locations: matches.map(({ view, activity, unit }) => ({
        kind: 'work_unit' as const,
        epicId: epic.epicId,
        sprintId: sprint.sprintId,
        revisionId: view.sprintPlanRevisionId,
        workSlicePlanningPointId: activity.workSlicePlanningPointId,
        workUnitId: unit.workUnitId,
        label: unit.title,
      })),
      folderPath: [
        ...sprintPath,
        {
          id: `${sprintPath[1].id}:activity:${match.activity.workSlicePlanningPointId}`,
          label: match.activity.title,
        },
        {
          id: `${sprintPath[1].id}:activity:${match.activity.workSlicePlanningPointId}:work-unit:${match.unit.workUnitId}`,
          label: match.unit.title,
        },
      ],
    };
  }

  return { reference, locations: [], folderPath: [] };
}

function uniqueLocations(locations: readonly AgentSessionProductLocation[]) {
  const seen = new Set<string>();
  return locations.filter((location) => {
    const key = JSON.stringify(location);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function unique(values: readonly string[]) {
  return [...new Set(values)];
}

function uniquePaths(
  paths: readonly (readonly { readonly id: string; readonly label: string }[])[],
) {
  const seen = new Set<string>();
  return paths.filter((path) => {
    const key = JSON.stringify(path);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function roleLabel(role: AgentSessionSemanticRole): string {
  return {
    epic: 'Epic Runner',
    sprint: 'Sprint Runner',
    work_slice_planner: 'Work Slice Planner',
    work_unit_handler: 'Work Unit Handler',
    work_unit_implementer: 'Work Unit Implementer',
  }[role];
}

function sortNodes(
  nodes: readonly (MutableFolder | AgentSessionNavigationSession)[],
): readonly AgentSessionNavigationNode[] {
  return nodes
    .map((node) =>
      node.kind === 'folder'
        ? {
            ...node,
            children: sortNodes(node.children),
          }
        : node,
    )
    .sort((left, right) => {
      if (left.kind !== right.kind) return left.kind === 'session' ? -1 : 1;
      const leftLabel = left.kind === 'session' ? left.summary.title : left.label;
      const rightLabel = right.kind === 'session' ? right.summary.title : right.label;
      return leftLabel.localeCompare(rightLabel);
    });
}
