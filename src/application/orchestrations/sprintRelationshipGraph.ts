import type {
  SprintControlSurfaceProjection,
  SprintPlannerOutputV1,
} from './sprintControlSurfaceCompatibility';
import type { DependencyKind, SprintRelationshipGraph } from './sprintReadModels';

export function projectSprintRelationshipGraph(
  planner: SprintPlannerOutputV1,
  revisionId: string,
  workUnits: SprintControlSurfaceProjection['workUnits'],
  sprintPlannerActivities: SprintPlannerOutputV1['sprintPlannerActivities'],
  gates: SprintPlannerOutputV1['gates'],
): SprintRelationshipGraph {
  const nodes: SprintRelationshipGraph['nodes'] = [
    node('sprint_plan', planner.sprintPlan.id),
    node('plan_revision', revisionId),
    ...sprintPlannerActivities.map(({ id }) => node('sprint_planner_activity', id)),
    ...workUnits.map(({ id, parallelGroupId }) => node('work_unit', id, parallelGroupId)),
    ...gates.map(({ id }) => node('gate', id)),
  ];
  const included = new Set(nodes.map(({ id }) => id));
  const edges: SprintRelationshipGraph['edges'] = [
    edge(`sprint_plan:${planner.sprintPlan.id}`, `plan_revision:${revisionId}`, 'revision'),
    ...sprintPlannerActivities.map(({ id, planRevisionId }) =>
      edge(`sprint_planner_activity:${id}`, `plan_revision:${planRevisionId}`, 'assessment'),
    ),
    ...workUnits.map(({ id }) => edge(`plan_revision:${revisionId}`, `work_unit:${id}`, 'plan')),
    ...gates.flatMap((gate) =>
      gate.specRevisions.flatMap((spec) => [
        ...spec.requiresWorkUnitIds.map((workUnitId) =>
          edge(`work_unit:${workUnitId}`, `gate:${gate.id}`, 'gate', undefined, gate.id),
        ),
        ...spec.requiresGateIds.map((gateId) =>
          edge(`gate:${gateId}`, `gate:${gate.id}`, 'gate', undefined, gate.id),
        ),
      ]),
    ),
    ...workUnits.flatMap((unit) =>
      unit.dependencies.flatMap((dependency) => [
        ...(dependency.workUnitId
          ? [
              edge(
                `work_unit:${dependency.workUnitId}`,
                `work_unit:${unit.id}`,
                'dependency',
                dependency.kind,
              ),
            ]
          : []),
        ...(dependency.gateId
          ? [
              edge(
                `gate:${dependency.gateId}`,
                `work_unit:${unit.id}`,
                'gate',
                undefined,
                dependency.gateId,
              ),
            ]
          : []),
      ]),
    ),
  ].filter(({ from, to }) => included.has(from) && included.has(to));
  return { nodes, edges };
}

function node(
  type: SprintRelationshipGraph['nodes'][number]['type'],
  semanticId: string,
  parallelGroupId?: string,
) {
  return {
    id: `${type}:${semanticId}`,
    type,
    semanticId,
    ...(parallelGroupId ? { parallelGroupId } : {}),
  } as const;
}
function edge(
  from: string,
  to: string,
  kind: SprintRelationshipGraph['edges'][number]['kind'],
  dependencyKind?: DependencyKind,
  gateId?: string,
) {
  return {
    id: `${kind}:${from}->${to}${gateId ? `:${gateId}` : ''}`,
    from,
    to,
    kind,
    ...(dependencyKind ? { dependencyKind } : {}),
    ...(gateId ? { gateId } : {}),
  } as const;
}
