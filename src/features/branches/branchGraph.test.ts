import { orderReviewTargets, targetKey } from '../../application/branches';
import { branchLineage } from './branchLineage';
import { branchGraphLayout, grid, targetControl, rangeControl } from './branchGraphLayout';
import { overviewFixture } from '../worktreeReview/WorktreeReviewScreen.fixtures';
import type { BranchGraphData } from '../../application/branches';

const graph: BranchGraphData = {
  snapshotId: 'snapshot',
  referenceTarget: overviewFixture.branches[0].target,
  targets: overviewFixture.branches.map((branch, index) => ({
    ...branch,
    tip: { ...branch.tip, objectId: index === 0 ? 'selected' : 'sibling' },
  })),
  anchors: [
    { objectId: 'merge', parentIds: ['selected', 'sibling'], merge: true, boundary: false },
    { objectId: 'selected', parentIds: ['base'], merge: false, boundary: false },
    { objectId: 'sibling', parentIds: ['base'], merge: false, boundary: false },
    { objectId: 'base', parentIds: [], merge: false, boundary: false },
  ],
  connections: [
    ['base', 'selected'],
    ['base', 'sibling'],
    ['selected', 'merge'],
    ['sibling', 'merge'],
  ].map(([from, to]) => ({
    id: `${from}-${to}`,
    from,
    to,
    commitCount: 1,
    collapsed: false,
    incomplete: false,
    eligibleSources: overviewFixture.branches.map((b) => b.target),
    scope: { kind: 'graph_range', snapshotId: 'snapshot', rangeId: `${from}-${to}` },
  })),
  hasMore: false,
  loadedCommitCount: 4,
};

it('highlights ancestors and downstream integration without highlighting sibling-only paths', () => {
  const lineage = branchLineage(graph, 'selected');
  expect([...lineage.nodes].sort()).toEqual(['base', 'merge', 'selected']);
  expect([...lineage.edges].sort()).toEqual(['base-selected', 'selected-merge']);
});

it('puts available worktrees first and uses approximate edit activity without disturbing graph geometry', () => {
  const oldWithWorktree = {
    ...graph.targets[0],
    activity: {
      changedAt: '2026-01-01T00:00:00Z',
      observedAt: '2026-09-09T00:00:00Z',
      basis: 'changed_file_estimate' as const,
      stagedFiles: 0,
      unstagedFiles: 1,
      untrackedFiles: 0,
    },
  };
  const noWorktree = {
    ...graph.targets[1],
    availableWorktreeCount: 0,
    tip: { ...graph.targets[1].tip, committedAt: '2026-09-09T00:00:00Z' },
  };
  expect(orderReviewTargets([noWorktree, oldWithWorktree])[0]).toBe(oldWithWorktree);
  const recentWithWorktree = {
    ...noWorktree,
    availableWorktreeCount: 1,
    activity: { ...oldWithWorktree.activity, changedAt: '2026-09-08T00:00:00Z' },
  };
  expect(orderReviewTargets([oldWithWorktree, recentWithWorktree])[0]).toBe(recentWithWorktree);
  expect(branchGraphLayout({ ...graph, targets: [noWorktree, oldWithWorktree] })).toEqual(
    branchGraphLayout({ ...graph, targets: [recentWithWorktree, oldWithWorktree] }),
  );
});

function assertClear(data: BranchGraphData) {
  const layout = branchGraphLayout(data);
  const controls = [...layout.ranges.values(), ...layout.labels.values()];
  const overlaps = (
    a: { x: number; y: number; width: number; height: number },
    b: { x: number; y: number; width: number; height: number },
  ) => a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
  controls.forEach((a, i) =>
    controls.slice(i + 1).forEach((b) => expect(overlaps(a, b)).toBe(false)),
  );
  for (const rect of layout.ranges.values()) {
    expect(rect.height).toBeGreaterThanOrEqual(32);
    for (const p of layout.positions.values())
      expect(overlaps(rect, { x: p.x - 8, y: p.y - 8, width: 16, height: 16 })).toBe(false);
  }
  for (const [id, route] of layout.routes)
    for (let i = 1; i < route.length; i++) {
      const a = route[i - 1],
        b = route[i];
      expect(a.x === b.x || a.y === b.y).toBe(true);
      for (const [other, rect] of layout.ranges) {
        if (other === id) continue; // Each route deliberately meets its own count cell.
        if (a.x === b.x)
          expect(
            a.x > rect.x &&
              a.x < rect.x + rect.width &&
              Math.min(a.y, b.y) < rect.y + rect.height &&
              Math.max(a.y, b.y) > rect.y,
          ).toBe(false);
        else
          expect(
            a.y > rect.y &&
              a.y < rect.y + rect.height &&
              Math.min(a.x, b.x) < rect.x + rect.width &&
              Math.max(a.x, b.x) > rect.x,
          ).toBe(false);
      }
    }
  expect(new Set([...layout.labels.values()].map((r) => r.x)).size).toBe(1);
  return layout;
}
it('keeps counts, compact labels, and junctions clear of one another', () => {
  assertClear({
    ...graph,
    targets: [
      graph.targets[0],
      {
        ...graph.targets[1],
        displayName: 'main',
        tip: { ...graph.targets[1].tip, objectId: 'merge' },
      },
    ],
  });
});
it('keeps shared-head labels separate and compact without duplicating graph points', () => {
  const shared = {
    ...graph,
    targets: graph.targets.map((t) => ({ ...t, tip: { ...t.tip, objectId: 'selected' } })),
  };
  const layout = assertClear(shared);
  expect(new Set([...layout.labels.values()].map((r) => r.y)).size).toBe(2);
  expect([...layout.labels.values()].every((r) => r.height === 32)).toBe(true);
  expect(layout.positions.size).toBe(shared.anchors.length); // Aliases are labels, never extra junctions.
});
it('reuses vertical lanes for disjoint connections instead of growing horizontally', () => {
  const anchors = Array.from({ length: 20 }, (_, i) => ({
    objectId: `n${i}`,
    parentIds: i ? [`n${i - 1}`] : [],
    merge: false,
    boundary: false,
  })).reverse();
  const chain = {
    ...graph,
    anchors,
    targets: [{ ...graph.targets[0], tip: { ...graph.targets[0].tip, objectId: 'n19' } }],
    connections: anchors
      .filter((n) => n.parentIds.length)
      .map((n) => ({
        ...graph.connections[0],
        id: n.objectId,
        from: n.parentIds[0],
        to: n.objectId,
      })),
  };
  const layout = assertClear(chain);
  expect(layout.graphWidth).toBeLessThan(grid.lane * 4);
  expect(layout.positions.get('n19')!.y).toBeLessThan(layout.positions.get('n0')!.y);
  expect([...layout.positions.values()].every((p) => (p.x - grid.left) % grid.lane === 0)).toBe(
    true,
  );
});
it('provides stable directional navigation between cards and their history', () => {
  const layout = branchGraphLayout(graph),
    selected = targetControl(targetKey(graph.targets[0].target));
  expect(layout.neighbors.get(selected)?.ArrowLeft).toBe(rangeControl('base-selected'));
  const range = rangeControl('base-selected');
  expect(layout.neighbors.get(range)?.ArrowRight).toBeDefined();
  expect(branchGraphLayout({ ...graph, targets: [...graph.targets].reverse() })).toEqual(layout);
});
