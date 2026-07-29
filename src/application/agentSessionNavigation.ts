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
      readonly kind: 'sprint_planner_activity';
      readonly epicId: string;
      readonly sprintId: string;
      readonly revisionId: string;
      readonly sprintPlannerActivityId: string;
      readonly label: string;
    }
  | {
      readonly kind: 'work_unit';
      readonly epicId: string;
      readonly sprintId: string;
      readonly revisionId: string;
      readonly sprintPlannerActivityId: string;
      readonly workUnitId: string;
      readonly label: string;
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

export interface AgentSessionNavigationModel {
  readonly roots: readonly AgentSessionNavigationFolder[];
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

/** Cross-capability presentation projection. It records no ownership and performs no effects. */
export function buildAgentSessionNavigation(
  input: AgentSessionNavigationInput,
): AgentSessionNavigationModel {
  const roots: MutableFolder[] = [];
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
    else roots.push(created);
    return created;
  };

  const folderPath = (path: readonly { readonly id: string; readonly label: string }[]) => {
    let current: MutableFolder | null = null;
    for (const item of path) current = folder(current, item.id, item.label);
    return current!;
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
      resolved.map(({ reference }) =>
        roleLabel(reference.semanticRole, reference.otherSemanticRole),
      ),
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

    const placementPaths = unique(resolved.map(({ folderPath: path }) => JSON.stringify(path)));
    const ambiguous =
      locations.length > 1 || placementPaths.length > 1 || Boolean(draft && resolved.length);
    if (resolved.length > 0 && !draft && !ambiguous) {
      folderPath(resolved[0].folderPath).children.push(item);
      continue;
    }
    if (resolved.length === 0 && draft) {
      folderPath([
        { id: 'independent', label: 'Independent Sessions' },
        { id: 'independent:planning-drafts', label: 'Epic planning drafts' },
      ]).children.push(item);
      continue;
    }
    if (ambiguous) {
      const epicIds = unique(
        locations.flatMap((location) => ('epicId' in location ? [location.epicId] : [])),
      );
      if (epicIds.length === 1) {
        const epic = input.orchestrations?.epics.find(({ epicId }) => epicId === epicIds[0]);
        folderPath([
          { id: `epic:${epicIds[0]}`, label: epic?.title ?? epicIds[0] },
          { id: `epic:${epicIds[0]}:related`, label: 'Multiple related views' },
        ]).children.push(item);
      } else {
        folderPath([
          { id: 'independent', label: 'Independent Sessions' },
          { id: 'independent:multiple', label: 'Multiple related views' },
        ]).children.push(item);
      }
      continue;
    }
    folderPath([
      { id: 'independent', label: 'Independent Sessions' },
      { id: 'independent:unassigned', label: 'Unassigned' },
    ]).children.push(item);
  }

  return { roots: sortFolders(roots), sessions };
}

function collectReferences(orchestrations?: ProductReadModelsV1) {
  const result = new Map<string, ResolvedReference[]>();
  for (const epic of orchestrations?.epics ?? []) {
    for (const reference of epic.agentSessionReferences) {
      const resolved = resolveReference(epic, reference);
      const existing = result.get(reference.agentSessionId) ?? [];
      if (
        !existing.some(
          ({ reference: item }) => item.agentSessionRefId === reference.agentSessionRefId,
        )
      )
        existing.push(resolved);
      result.set(reference.agentSessionId, existing);
    }
  }
  for (const reference of orchestrations?.unassociatedAgentSessionReferences ?? []) {
    const existing = result.get(reference.agentSessionId) ?? [];
    if (
      !existing.some(
        ({ reference: item }) => item.agentSessionRefId === reference.agentSessionRefId,
      )
    )
      existing.push({
        reference,
        locations: [],
        folderPath: [
          { id: 'independent', label: 'Independent Sessions' },
          { id: 'independent:other', label: 'Other recorded relationships' },
        ],
      });
    result.set(reference.agentSessionId, existing);
  }
  return result;
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
  if (!sprint)
    return {
      reference,
      locations: [],
      folderPath: [
        { id: 'independent', label: 'Independent Sessions' },
        { id: 'independent:other', label: 'Other recorded relationships' },
      ],
    };
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
  if (reference.targetKind === 'sprint_planner_activity') {
    const matches = sprint.revisionViews.flatMap((view) =>
      view.plannerActivityGroups
        .filter(({ sprintPlannerActivityId }) => sprintPlannerActivityId === reference.targetId)
        .map((activity) => ({ view, activity })),
    );
    const match = matches[0];
    if (!match) return { reference, locations: [], folderPath: sprintPath };
    return {
      reference,
      locations: matches.map(({ view, activity }) => ({
        kind: 'sprint_planner_activity' as const,
        epicId: epic.epicId,
        sprintId: sprint.sprintId,
        revisionId: view.sprintPlanRevisionId,
        sprintPlannerActivityId: activity.sprintPlannerActivityId,
        label: activity.title,
      })),
      folderPath: [
        ...sprintPath,
        { id: `${sprintPath[1].id}:planning`, label: 'Planning' },
        {
          id: `${sprintPath[1].id}:planning:${match.activity.sprintPlannerActivityId}`,
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
          const activity = view.plannerActivityGroups.find(({ workUnitScopeIds }) =>
            workUnitScopeIds.includes(unit.workUnitScopeId),
          );
          return activity ? [{ view, unit, activity }] : [];
        }),
    );
    const match = matches[0];
    if (!match) return { reference, locations: [], folderPath: sprintPath };
    return {
      reference,
      locations: matches.map(({ view, activity, unit }) => ({
        kind: 'work_unit' as const,
        epicId: epic.epicId,
        sprintId: sprint.sprintId,
        revisionId: view.sprintPlanRevisionId,
        sprintPlannerActivityId: activity.sprintPlannerActivityId,
        workUnitId: unit.workUnitId,
        label: unit.title,
      })),
      folderPath: [
        ...sprintPath,
        { id: `${sprintPath[1].id}:execution`, label: 'Execution' },
        {
          id: `${sprintPath[1].id}:execution:${match.unit.workUnitId}`,
          label: match.unit.title,
        },
      ],
    };
  }
  return {
    reference,
    locations: [],
    folderPath: [
      { id: 'independent', label: 'Independent Sessions' },
      { id: 'independent:other', label: 'Other recorded relationships' },
    ],
  };
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

function roleLabel(role: AgentSessionSemanticRole, other?: string): string {
  if (role === 'other') return other ?? 'Other role';
  return {
    epic_runner: 'Epic Runner',
    epic_plan_builder: 'Epic Plan Builder',
    sprint_runner: 'Sprint Runner',
    sprint_planner: 'Sprint Planner',
    work_unit_planner: 'Work Unit planner',
    work_unit_handler: 'Work Unit handler',
    work_unit_worker: 'Work Unit worker',
    reviewer: 'Reviewer',
  }[role];
}

function sortFolders(folders: readonly MutableFolder[]): readonly AgentSessionNavigationFolder[] {
  return folders.map((item) => ({
    ...item,
    children: item.children.map((child) =>
      child.kind === 'folder' ? (sortFolders([child])[0] as AgentSessionNavigationFolder) : child,
    ),
  }));
}
