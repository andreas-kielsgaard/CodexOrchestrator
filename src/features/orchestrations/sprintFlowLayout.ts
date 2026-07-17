import type { ProductSprintRevisionViewV1 } from '../../application/orchestrations';

export interface SprintFlowPosition {
  readonly id: string;
  readonly x: number;
  readonly y: number;
}

export interface SprintFlowLayout {
  readonly width: number;
  readonly height: number;
  readonly positions: readonly SprintFlowPosition[];
  readonly sprintPlannerActivityGroupPositions: readonly {
    readonly id: string;
    readonly x: number;
    readonly y: number;
    readonly width: number;
    readonly height: number;
  }[];
}

export interface SprintFlowConnector {
  readonly from: string;
  readonly to: string;
  readonly kind: 'dependency' | 'gate';
}

export interface SprintFlowConnectorRoute extends SprintFlowConnector {
  readonly path: string;
  readonly scopeSprintPlannerActivityId?: string;
}

const CARD_WIDTH = 178;
const CARD_HEIGHT = 96;
const PLAN_Y = 72;
const CARD_TOP = 70;
const STAGE_GAP = 218;
const LANE_GAP = 126;

/** Visible Work Unit relationships derived only from declared dependency and gate edges. */
export function projectSprintFlowConnectors(
  view: ProductSprintRevisionViewV1,
): readonly SprintFlowConnector[] {
  const visibleUnits = new Set(view.workUnits.map(({ workUnitId }) => workUnitId));
  const connectors = new Map<string, SprintFlowConnector>();
  view.workUnits.forEach((unit) => {
    unit.dependencies.forEach((dependency) => {
      if (visibleUnits.has(dependency.workUnitId)) {
        connectors.set(`${dependency.workUnitId}->${unit.workUnitId}`, {
          from: dependency.workUnitId,
          to: unit.workUnitId,
          kind: 'dependency',
        });
      }
    });
  });
  return [...connectors.values()];
}

/** Replaceable Work Unit grouping layout; semantic contracts never retain coordinates. */
export function projectSprintFlowLayout(view: ProductSprintRevisionViewV1): SprintFlowLayout {
  const positions: SprintFlowPosition[] = [];
  const sprintPlannerActivityGroupPositions: SprintFlowLayout['sprintPlannerActivityGroupPositions'][number][] =
    [];
  let groupX = 24;
  let maxGroupBottom = 320;
  const connectors = projectSprintFlowConnectors(view);
  for (const group of view.plannerActivityGroups) {
    const owned = view.workUnits.filter((unit) =>
      group.workUnitScopeIds.includes(unit.workUnitScopeId),
    );
    const ownedIds = new Set(owned.map(({ workUnitId }) => workUnitId));
    const levelById = new Map<string, number>();
    const levelOf = (id: string, visiting = new Set<string>()): number => {
      const cached = levelById.get(id);
      if (cached !== undefined) return cached;
      if (visiting.has(id)) return 0;
      const nextVisiting = new Set(visiting).add(id);
      const dependencies = connectors
        .filter(({ to, from }) => to === id && ownedIds.has(from))
        .map(({ from }) => from);
      const level = dependencies.length
        ? Math.max(...dependencies.map((dependency) => levelOf(dependency, nextVisiting))) + 1
        : 0;
      levelById.set(id, level);
      return level;
    };
    owned.forEach(({ workUnitId }) => levelOf(workUnitId));
    const lanes = new Map<number, number>();
    owned.forEach((unit) => {
      const level = levelById.get(unit.workUnitId) ?? 0;
      const lane = lanes.get(level) ?? 0;
      lanes.set(level, lane + 1);
      positions.push({
        id: unit.workUnitId,
        x: groupX + 22 + level * STAGE_GAP,
        y: PLAN_Y + CARD_TOP + lane * LANE_GAP,
      });
    });
    const stages = Math.max(0, ...levelById.values()) + 1;
    const laneCount = Math.max(1, ...lanes.values());
    const width = Math.max(220, 44 + stages * STAGE_GAP);
    const height = Math.max(246, 100 + laneCount * 126);
    sprintPlannerActivityGroupPositions.push({
      id: group.sprintPlannerActivityId,
      x: groupX,
      y: PLAN_Y,
      width,
      height,
    });
    groupX += width + 34;
    maxGroupBottom = Math.max(maxGroupBottom, PLAN_Y + height);
  }
  return {
    width: Math.max(760, groupX + 12),
    height: maxGroupBottom + 72,
    positions,
    sprintPlannerActivityGroupPositions,
  };
}

/** Orthogonal routes stay inside a presentation group or use dedicated cross-group gutters. */
export function projectSprintConnectorRoutes(
  view: ProductSprintRevisionViewV1,
  layout: SprintFlowLayout,
): readonly SprintFlowConnectorRoute[] {
  const positions = new Map(layout.positions.map((position) => [position.id, position]));
  const groups = new Map(
    layout.sprintPlannerActivityGroupPositions.map((group) => [group.id, group]),
  );
  const ownerByUnit = new Map(
    view.plannerActivityGroups.flatMap((group) =>
      group.workUnitScopeIds
        .map((scopeId) => {
          const unit = view.workUnits.find(({ workUnitScopeId }) => workUnitScopeId === scopeId);
          return unit ? ([unit.workUnitId, group.sprintPlannerActivityId] as const) : undefined;
        })
        .filter((entry) => entry !== undefined),
    ),
  );
  const groupOrder = new Map(
    view.plannerActivityGroups.map((group, index) => [group.sprintPlannerActivityId, index]),
  );
  const bottomGutterY =
    Math.max(...layout.sprintPlannerActivityGroupPositions.map(({ y, height }) => y + height)) + 24;

  return projectSprintFlowConnectors(view).flatMap((connector) => {
    const from = positions.get(connector.from);
    const to = positions.get(connector.to);
    const fromGroupId = ownerByUnit.get(connector.from);
    const toGroupId = ownerByUnit.get(connector.to);
    if (!from || !to || !fromGroupId || !toGroupId) return [];
    const startX = from.x + CARD_WIDTH;
    const startY = from.y + CARD_HEIGHT / 2;
    const endX = to.x;
    const endY = to.y + CARD_HEIGHT / 2;

    if (fromGroupId === toGroupId) {
      const group = groups.get(fromGroupId)!;
      const localStartX = startX - group.x;
      const localStartY = startY - group.y;
      const localEndX = endX - group.x;
      const localEndY = endY - group.y;
      const path =
        localStartY === localEndY
          ? `M ${localStartX} ${localStartY} H ${localEndX}`
          : `M ${localStartX} ${localStartY} H ${(localStartX + localEndX) / 2} V ${localEndY} H ${localEndX}`;
      return [{ ...connector, path, scopeSprintPlannerActivityId: fromGroupId }];
    }

    const fromGroup = groups.get(fromGroupId)!;
    const toGroup = groups.get(toGroupId)!;
    const adjacent =
      Math.abs((groupOrder.get(fromGroupId) ?? 0) - (groupOrder.get(toGroupId) ?? 0)) === 1;
    const sourceAtGroupExit = Math.abs(startX - (fromGroup.x + fromGroup.width - 22)) < 1;
    const targetAtGroupEntry = Math.abs(endX - (toGroup.x + 22)) < 1;
    if (adjacent && sourceAtGroupExit && targetAtGroupEntry && startY === endY)
      return [{ ...connector, path: `M ${startX} ${startY} H ${endX}` }];

    const sourceGutterX = startX + 10;
    const targetGutterX = endX - 10;
    return [
      {
        ...connector,
        path: `M ${startX} ${startY} H ${sourceGutterX} V ${bottomGutterY} H ${targetGutterX} V ${endY} H ${endX}`,
      },
    ];
  });
}
