import { expect, it } from 'vitest';
import { organizeWorkflow, projectWorkflowGroups } from './workflowLayout';
const nodes = ['input', 'router', 'worker', 'output'].map((id) => ({ id, name: id, x: 0, y: 0 }));
const edges = [
  { id: 'in', name: 'Work', source: 'input', destination: 'router' },
  { id: 'delegate', name: 'Delegate', source: 'router', destination: 'worker' },
  {
    id: 'return',
    name: 'Return',
    source: 'worker',
    destination: 'router',
    role: 'feedback' as const,
  },
  { id: 'reply', name: 'Respond', source: 'router', destination: 'output' },
];
it('lays out a cyclic workflow without overlapping cards and preserves individual routed edges', async () => {
  const layout = await organizeWorkflow(nodes, edges);
  expect(Object.keys(layout.routes).sort()).toEqual(edges.map((e) => e.id).sort());
  for (const [a, first] of Object.entries(layout.positions))
    for (const [b, second] of Object.entries(layout.positions)) {
      if (a !== b)
        expect(Math.abs(first.x - second.x) >= 220 || Math.abs(first.y - second.y) >= 100).toBe(
          true,
        );
    }
  for (const edge of edges) expect(layout.routes[edge.id].points.length).toBeGreaterThanOrEqual(2);
  expect(layout.routes.delegate.points).not.toEqual(layout.routes.return.points);
});
it('collapses a visual group without losing external connection identity', () => {
  const result = projectWorkflowGroups(
    nodes,
    edges,
    [{ id: 'work', name: 'Work', nodeIds: ['router', 'worker'] }],
    new Set(['work']),
  );
  expect(result.nodes.map((n) => n.id)).toEqual(['input', 'output', 'work']);
  expect(result.connections.map((e) => [e.id, e.source, e.destination])).toEqual([
    ['in', 'input', 'work'],
    ['reply', 'work', 'output'],
  ]);
});
