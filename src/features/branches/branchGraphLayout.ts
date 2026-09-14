import { targetKey, type BranchGraphData } from '../../application/branches';

export interface Point {
  readonly x: number;
  readonly y: number;
}
export interface Rect extends Point {
  readonly width: number;
  readonly height: number;
}
export type Direction = 'ArrowLeft' | 'ArrowRight' | 'ArrowUp' | 'ArrowDown';
export const grid = {
  lane: 24,
  row: 38,
  top: 44,
  left: 24,
  count: 108,
  controlHeight: 32,
  label: 360,
};
export const targetControl = (key: string) => `target:${key}`;
export const rangeControl = (key: string) => `range:${key}`;
export const laneColors = [
  '#287e62',
  '#6859b3',
  '#bd6728',
  '#337ca9',
  '#ae5074',
  '#79802b',
  '#277e88',
];

/** Topological rows and reusable lanes own both geometry and keyboard navigation. */
export function branchGraphLayout(graph: BranchGraphData) {
  const reference = graph.referenceTarget && targetKey(graph.referenceTarget);
  const targets = [...graph.targets].sort((a, b) => {
    const ak = targetKey(a.target),
      bk = targetKey(b.target);
    return (
      Number(bk === reference) - Number(ak === reference) ||
      a.displayName.localeCompare(b.displayName) ||
      ak.localeCompare(bk)
    );
  });
  const rows = new Map<string, number>();
  const targetRows = new Map<string, number>();
  const rangeRows = new Map<string, number>();
  const edges = [...graph.connections].sort((a, b) => a.id.localeCompare(b.id));
  let row = 0;
  for (const node of graph.anchors) {
    rows.set(node.objectId, row);
    const aliases = targets.filter((t) => t.tip.objectId === node.objectId);
    const incoming = edges.filter((e) => e.to === node.objectId);
    aliases.forEach((t, i) => targetRows.set(targetKey(t.target), row + i));
    incoming.forEach((e, i) => rangeRows.set(e.id, row + i));
    row += Math.max(1, aliases.length, incoming.length);
  }
  const lanes = new Map<string, number>();
  const pending: (string | undefined)[] = [];
  for (const node of graph.anchors) {
    let lane = pending.indexOf(node.objectId);
    if (lane < 0) {
      lane = pending.findIndex((id) => !id);
      if (lane < 0) lane = pending.length;
    }
    lanes.set(node.objectId, lane);
    pending[lane] = undefined;
    for (const parent of node.parentIds) {
      if (pending.includes(parent)) continue;
      let slot = !pending[lane] ? lane : pending.findIndex((id) => !id);
      if (slot < 0) slot = pending.length;
      pending[slot] = parent;
    }
  }
  const occupied: { start: number; end: number }[][] = [];
  const reserve = (lane: number, start: number, end: number) =>
    (occupied[lane] ??= []).push({ start, end });
  for (const [id, lane] of lanes) reserve(lane, rows.get(id)! - 0.2, rows.get(id)! + 0.2);
  const edgeLanes = new Map<string, number>();
  // Longer spans claim a clear track first; no unrelated point sits on their vertical stroke.
  const bySpan = [...edges].sort(
    (a, b) =>
      rows.get(b.from)! - rows.get(b.to)! - (rows.get(a.from)! - rows.get(a.to)!) ||
      a.id.localeCompare(b.id),
  );
  for (const edge of bySpan) {
    const start = rows.get(edge.to)! + 0.3,
      end = rows.get(edge.from)! - 0.3;
    const preferred = lanes.get(edge.to)!;
    const candidates = Array.from({ length: occupied.length }, (_, i) => i).sort(
      (a, b) => Math.abs(a - preferred) - Math.abs(b - preferred) || a - b,
    );
    const lane =
      candidates.find((i) => !(occupied[i] ?? []).some((r) => start < r.end && end > r.start)) ??
      occupied.length;
    reserve(lane, start, end);
    edgeLanes.set(edge.id, lane);
  }
  const graphWidth = grid.left * 2 + Math.max(1, occupied.length) * grid.lane;
  const positions = new Map(
    graph.anchors.map((node) => [
      node.objectId,
      {
        x: grid.left + lanes.get(node.objectId)! * grid.lane,
        y: grid.top + rows.get(node.objectId)! * grid.row + grid.row / 2,
        lane: lanes.get(node.objectId)!,
      },
    ]),
  );
  const labelX = graphWidth + grid.count + 20;
  const labels = new Map(
    targets.map((t) => [
      targetKey(t.target),
      {
        x: labelX,
        y: grid.top + targetRows.get(targetKey(t.target))! * grid.row + 3,
        width: Math.min(grid.label, Math.max(92, t.displayName.length * 7.5 + 40)),
        height: grid.controlHeight,
      },
    ]),
  );
  const ranges = new Map<string, Rect>();
  const routes = new Map<string, readonly Point[]>();
  const colors = new Map<string, string>();
  for (const edge of edges) {
    const newer = positions.get(edge.to)!,
      older = positions.get(edge.from)!;
    const x = grid.left + edgeLanes.get(edge.id)! * grid.lane;
    const edgeRow = rangeRows.get(edge.id)!;
    routes.set(edge.id, [
      newer,
      { x: newer.x, y: newer.y + 12 },
      { x, y: newer.y + 12 },
      { x, y: older.y - 12 },
      { x: older.x, y: older.y - 12 },
      older,
    ]);
    ranges.set(edge.id, {
      x: graphWidth,
      y: grid.top + edgeRow * grid.row + 3,
      width: grid.count,
      height: grid.controlHeight,
    });
    colors.set(edge.id, laneColors[edgeLanes.get(edge.id)! % laneColors.length]);
  }
  const controls = new Map<string, Rect>([
    ...[...ranges].map(([id, rect]) => [rangeControl(id), rect] as const),
    ...[...labels].map(([id, rect]) => [targetControl(id), rect] as const),
  ]);
  const neighbors = new Map<string, Partial<Record<Direction, string>>>();
  for (const [id, rect] of controls) {
    const origin = { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
    const result: Partial<Record<Direction, string>> = {};
    for (const direction of ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'] as const) {
      const horizontal = direction === 'ArrowLeft' || direction === 'ArrowRight';
      const sign = direction === 'ArrowLeft' || direction === 'ArrowUp' ? -1 : 1;
      const candidates = [...controls]
        .filter(([key]) => key !== id)
        .map(([key, r]) => {
          const dx = r.x + r.width / 2 - origin.x,
            dy = r.y + r.height / 2 - origin.y;
          return {
            key,
            forward: (horizontal ? dx : dy) * sign,
            cross: Math.abs(horizontal ? dy : r.x - rect.x),
          };
        })
        .filter((c) => c.forward > 1)
        .sort(
          (a, b) =>
            Number(b.cross < 1) - Number(a.cross < 1) ||
            a.cross * 4 + a.forward - (b.cross * 4 + b.forward) ||
            a.key.localeCompare(b.key),
        );
      if (candidates[0]) result[direction] = candidates[0].key;
    }
    neighbors.set(id, result);
  }
  return {
    positions,
    labels,
    ranges,
    routes,
    colors,
    neighbors,
    graphWidth,
    labelX,
    width: labelX + 200,
    height: Math.max(220, grid.top + row * grid.row + 24),
    rowCount: row,
  };
}
