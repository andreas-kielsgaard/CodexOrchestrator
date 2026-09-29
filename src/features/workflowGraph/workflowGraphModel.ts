export interface WorkflowGraphNode {
  readonly id: string;
  readonly name: string;
  readonly x: number;
  readonly y: number;
  readonly entry?: boolean;
}

export interface WorkflowGraphConnection {
  readonly id: string;
  readonly name: string;
  readonly source: string;
  readonly destination: string;
}

export interface WorkflowGraphConnectionGroup {
  readonly id: string;
  readonly firstNode: string;
  readonly secondNode: string;
  readonly connections: readonly WorkflowGraphConnection[];
  readonly firstToSecond: boolean;
  readonly secondToFirst: boolean;
}

export const WORKFLOW_GRAPH_GRID_SIZE = 20;
export const WORKFLOW_GRAPH_NODE_WIDTH = 220;
export const WORKFLOW_GRAPH_NODE_HEIGHT = 100;
export const WORKFLOW_GRAPH_CONNECTION_Y = WORKFLOW_GRAPH_NODE_HEIGHT / 2;

export function groupWorkflowGraphConnections(
  connections: readonly WorkflowGraphConnection[],
): readonly WorkflowGraphConnectionGroup[] {
  const groups = new Map<string, WorkflowGraphConnection[]>();
  for (const connection of connections) {
    const [first, second] = [connection.source, connection.destination].sort();
    const id = `${first}\u0000${second}`;
    groups.set(id, [...(groups.get(id) ?? []), connection]);
  }
  return [...groups].map(([id, members]) => {
    const [firstNode, secondNode] = id.split('\u0000');
    return {
      id,
      firstNode,
      secondNode,
      connections: members,
      firstToSecond: members.some(
        (connection) => connection.source === firstNode && connection.destination === secondNode,
      ),
      secondToFirst: members.some(
        (connection) => connection.source === secondNode && connection.destination === firstNode,
      ),
    };
  });
}

export function workflowGraphBounds(nodes: readonly WorkflowGraphNode[]) {
  return {
    width: Math.max(1000, ...nodes.map((node) => node.x + WORKFLOW_GRAPH_NODE_WIDTH + 100)),
    height: Math.max(620, ...nodes.map((node) => node.y + WORKFLOW_GRAPH_NODE_HEIGHT + 100)),
  };
}

export function workflowGraphNodeById(
  nodes: readonly WorkflowGraphNode[],
  id: string,
): WorkflowGraphNode | undefined {
  return nodes.find((node) => node.id === id);
}
