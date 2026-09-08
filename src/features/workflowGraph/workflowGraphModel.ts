export interface WorkflowGraphNode {
  readonly id: string;
  readonly name: string;
  readonly x: number;
  readonly y: number;
  readonly starting?: boolean;
}

export interface WorkflowGraphConnection {
  readonly id: string;
  readonly name: string;
  readonly source: string;
  readonly destination: string;
}

export const WORKFLOW_GRAPH_NODE_WIDTH = 210;
export const WORKFLOW_GRAPH_CONNECTION_Y = 46;

export function workflowGraphBounds(nodes: readonly WorkflowGraphNode[]) {
  return {
    width: Math.max(1000, ...nodes.map((node) => node.x + 320)),
    height: Math.max(620, ...nodes.map((node) => node.y + 220)),
  };
}

export function workflowGraphNodeById(
  nodes: readonly WorkflowGraphNode[],
  id: string,
): WorkflowGraphNode | undefined {
  return nodes.find((node) => node.id === id);
}
