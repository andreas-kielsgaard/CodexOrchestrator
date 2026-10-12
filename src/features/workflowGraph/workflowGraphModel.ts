export interface WorkflowGraphNode {
  readonly id: string;
  readonly name: string;
  readonly x: number;
  readonly y: number;
  readonly entry?: boolean;
  readonly group?: string;
}

export interface WorkflowGraphConnection {
  readonly id: string;
  readonly name: string;
  readonly source: string;
  readonly destination: string;
  readonly role?: 'flow' | 'context' | 'feedback';
  readonly enabled?: boolean;
}

export const WORKFLOW_GRAPH_GRID_SIZE = 20;
export const WORKFLOW_GRAPH_NODE_WIDTH = 220;
export const WORKFLOW_GRAPH_NODE_HEIGHT = 100;
