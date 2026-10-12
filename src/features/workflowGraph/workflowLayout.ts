import ELK, { type ElkNode, type ELK as ElkEngine } from 'elkjs/lib/elk-api.js';
import ElkWorker from 'elkjs/lib/elk-worker.min.js?worker';
import type { WorkflowGraphConnection, WorkflowGraphNode } from './workflowGraphModel';
import { WORKFLOW_GRAPH_NODE_WIDTH, WORKFLOW_GRAPH_NODE_HEIGHT } from './workflowGraphModel';

export interface GraphLayout {
  positions: Record<string, { x: number; y: number }>;
  ports: Record<string, { x: number; y: number }>;
  routes: Record<string, { points: { x: number; y: number }[]; label?: { x: number; y: number } }>;
}
let engine: Promise<ElkEngine> | undefined;
function layoutEngine() {
  engine ??=
    typeof Worker === 'undefined'
      ? import('elkjs/lib/elk.bundled.js').then((module) => new module.default())
      : Promise.resolve(new ELK({ workerFactory: () => new ElkWorker() }));
  return engine;
}
export const connectionPort = (id: string, direction: 'in' | 'out') => `${direction}:${id}`;

/** Presentation coordinates only. The caller decides whether to persist them. */
export async function organizeWorkflow(
  nodes: readonly WorkflowGraphNode[],
  connections: readonly WorkflowGraphConnection[],
): Promise<GraphLayout> {
  const valid = connections.filter(
    (edge) =>
      nodes.some((n) => n.id === edge.source) && nodes.some((n) => n.id === edge.destination),
  );
  const result: ElkNode = await (
    await layoutEngine()
  ).layout({
    id: 'workflow',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'RIGHT',
      'elk.edgeRouting': 'ORTHOGONAL',
      'elk.layered.feedbackEdges': 'true',
      'elk.spacing.nodeNode': '60',
      'elk.layered.spacing.nodeNodeBetweenLayers': '100',
      'elk.spacing.edgeNode': '30',
      'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES',
      'elk.padding': '[top=40,left=40,bottom=40,right=40]',
    },
    children: nodes.map((node) => ({
      id: node.id,
      width: WORKFLOW_GRAPH_NODE_WIDTH,
      height: WORKFLOW_GRAPH_NODE_HEIGHT,
      layoutOptions: { 'elk.portConstraints': 'FIXED_ORDER' },
      ports: valid.flatMap((edge) => [
        ...(edge.source === node.id
          ? [
              {
                id: connectionPort(edge.id, 'out'),
                width: 1,
                height: 1,
                layoutOptions: { 'elk.port.side': edge.role === 'feedback' ? 'NORTH' : 'EAST' },
              },
            ]
          : []),
        ...(edge.destination === node.id
          ? [
              {
                id: connectionPort(edge.id, 'in'),
                width: 1,
                height: 1,
                layoutOptions: { 'elk.port.side': edge.role === 'feedback' ? 'NORTH' : 'WEST' },
              },
            ]
          : []),
      ]),
    })),
    edges: valid.map((edge) => ({
      id: edge.id,
      sources: [connectionPort(edge.id, 'out')],
      targets: [connectionPort(edge.id, 'in')],
      labels: [{ text: edge.name, width: Math.min(180, edge.name.length * 7), height: 20 }],
    })),
  });
  return {
    positions: Object.fromEntries(
      (result.children ?? []).map((n) => [n.id, { x: n.x ?? 0, y: n.y ?? 0 }]),
    ),
    ports: Object.fromEntries(
      (result.children ?? []).flatMap((n) =>
        (n.ports ?? []).map((p) => [p.id, { x: p.x ?? 0, y: p.y ?? 0 }]),
      ),
    ),
    routes: Object.fromEntries(
      (result.edges ?? []).map((edge) => {
        const section = edge.sections?.[0];
        const label = edge.labels?.[0];
        return [
          edge.id,
          {
            points: section
              ? [section.startPoint, ...(section.bendPoints ?? []), section.endPoint]
              : [],
            label:
              label && label.x !== undefined && label.y !== undefined
                ? { x: label.x + (label.width ?? 0) / 2, y: label.y + 10 }
                : undefined,
          },
        ];
      }),
    ),
  };
}

export interface WorkflowVisualGroup {
  id: string;
  name: string;
  nodeIds: readonly string[];
}
export function projectWorkflowGroups(
  nodes: readonly WorkflowGraphNode[],
  connections: readonly WorkflowGraphConnection[],
  groups: readonly WorkflowVisualGroup[],
  collapsed: ReadonlySet<string>,
) {
  const representative = (id: string) =>
    groups.find((g) => collapsed.has(g.id) && g.nodeIds.includes(id))?.id ?? id;
  return {
    nodes: [
      ...nodes.filter((n) => representative(n.id) === n.id),
      ...groups
        .filter((g) => collapsed.has(g.id))
        .map((g, index) => ({ id: g.id, name: g.name, x: 40 + index * 320, y: 40, group: g.id })),
    ],
    connections: connections
      .map((e) => ({
        ...e,
        source: representative(e.source),
        destination: representative(e.destination),
      }))
      .filter((e) => e.source !== e.destination),
  };
}
