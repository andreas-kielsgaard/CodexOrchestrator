import type { BranchGraphData } from '../../application/branches';

export function branchLineage(graph: BranchGraphData, head: string | undefined) {
  const parents = new Map(graph.anchors.map((node) => [node.objectId, node.parentIds]));
  const children = new Map<string, string[]>();
  graph.connections.forEach((edge) =>
    children.set(edge.from, [...(children.get(edge.from) ?? []), edge.to]),
  );
  const walk = (adjacency: ReadonlyMap<string, readonly string[]>) => {
    const visited = new Set<string>();
    const queue = head ? [head] : [];
    while (queue.length) {
      const id = queue.pop()!;
      if (visited.has(id)) continue;
      visited.add(id);
      queue.push(...(adjacency.get(id) ?? []));
    }
    return visited;
  };
  const ancestors = walk(parents);
  const descendants = walk(children);
  return {
    ancestors,
    nodes: new Set([...ancestors, ...descendants]),
    edges: new Set(
      graph.connections
        .filter(
          (edge) =>
            (ancestors.has(edge.from) && ancestors.has(edge.to)) ||
            (descendants.has(edge.from) && descendants.has(edge.to)),
        )
        .map((edge) => edge.id),
    ),
  };
}
