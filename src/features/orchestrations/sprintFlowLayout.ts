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
  readonly workSlicePlanningPointGroupPositions: readonly {
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
  readonly scopeWorkSlicePlanningPointId?: string;
}

const CARD_WIDTH = 158;
const CARD_HEIGHT = 76;
const PLAN_Y = 58;
const CARD_TOP = 48;
const STAGE_GAP = 168;
const LANE_GAP = 86;
const GROUP_INSET = 14;

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
  const workSlicePlanningPointGroupPositions: SprintFlowLayout['workSlicePlanningPointGroupPositions'][number][] =
    [];
  let groupX = 14;
  let maxGroupBottom = 260;
  const connectors = projectSprintFlowConnectors(view);
  for (const group of view.workSlicePlanningPointGroups) {
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
        x: groupX + GROUP_INSET + level * STAGE_GAP,
        y: PLAN_Y + CARD_TOP + lane * LANE_GAP,
      });
    });
    const stages = Math.max(0, ...levelById.values()) + 1;
    const laneCount = Math.max(1, ...lanes.values());
    const width = Math.max(182, 28 + stages * STAGE_GAP);
    const height = Math.max(170, 72 + laneCount * LANE_GAP);
    workSlicePlanningPointGroupPositions.push({
      id: group.workSlicePlanningPointId,
      x: groupX,
      y: PLAN_Y,
      width,
      height,
    });
    groupX += width + 10;
    maxGroupBottom = Math.max(maxGroupBottom, PLAN_Y + height);
  }
  return {
    width: Math.max(680, groupX + 8),
    height: maxGroupBottom + 48,
    positions,
    workSlicePlanningPointGroupPositions,
  };
}

/** Orthogonal routes stay inside a presentation group or use dedicated cross-group gutters. */
export function projectSprintConnectorRoutes(
  view: ProductSprintRevisionViewV1,
  layout: SprintFlowLayout,
): readonly SprintFlowConnectorRoute[] {
  const positions = new Map(layout.positions.map((position) => [position.id, position]));
  const groups = new Map(
    layout.workSlicePlanningPointGroupPositions.map((group) => [group.id, group]),
  );
  const ownerByUnit = new Map(
    view.workSlicePlanningPointGroups.flatMap((group) =>
      group.workUnitScopeIds
        .map((scopeId) => {
          const unit = view.workUnits.find(({ workUnitScopeId }) => workUnitScopeId === scopeId);
          return unit ? ([unit.workUnitId, group.workSlicePlanningPointId] as const) : undefined;
        })
        .filter((entry) => entry !== undefined),
    ),
  );
  const groupOrder = new Map(
    view.workSlicePlanningPointGroups.map((group, index) => [
      group.workSlicePlanningPointId,
      index,
    ]),
  );
  const bottomGutterY =
    Math.max(...layout.workSlicePlanningPointGroupPositions.map(({ y, height }) => y + height)) +
    24;

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
      return [{ ...connector, path, scopeWorkSlicePlanningPointId: fromGroupId }];
    }

    const fromGroup = groups.get(fromGroupId)!;
    const toGroup = groups.get(toGroupId)!;
    const adjacent =
      Math.abs((groupOrder.get(fromGroupId) ?? 0) - (groupOrder.get(toGroupId) ?? 0)) === 1;
    const sourceAtGroupExit = Math.abs(startX - (fromGroup.x + fromGroup.width - GROUP_INSET)) < 1;
    const targetAtGroupEntry = Math.abs(endX - (toGroup.x + GROUP_INSET)) < 1;
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
