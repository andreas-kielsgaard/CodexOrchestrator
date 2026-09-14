export interface SessionWorkflowTarget {
  readonly instanceId: string;
  readonly session?: { readonly nodeId: string; readonly sessionId: string };
}
