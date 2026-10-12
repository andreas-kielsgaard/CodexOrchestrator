import type { NodeNotifications } from '../completionNotifications';
/** An extension supplies its graph and owns execution. Ordinary recipes keep their own compiler. */
export interface ExtensionWorkflowNode {
  id: string;
  name: string;
  kind: 'function' | 'capability' | 'agent' | 'external_sessions';
  description: string;
  x: number;
  y: number;
  promptKeys: string[];
  sessionIds: string[];
  notifications?: NodeNotifications;
}
export interface ExtensionWorkflowConnection {
  id: string;
  name: string;
  source: string;
  destination: string;
  event: string;
  condition: string;
  payload: string;
  enabled: boolean;
}
export interface ExtensionWorkflowDefinition {
  id: string;
  name: string;
  owner: string;
  nodes: ExtensionWorkflowNode[];
  connections: ExtensionWorkflowConnection[];
  runtime: { id: string; name: string; services: string[] };
}
export interface ExtensionRouteEvidence {
  sourceNodeId: string;
  connectionIds: string[];
  destinations: { nodeId: string; sessionId: string }[];
}
