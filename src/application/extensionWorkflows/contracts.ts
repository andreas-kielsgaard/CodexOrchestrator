/** Retained only for legacy Voice activity records. */
export interface ExtensionRouteEvidence {
  sourceNodeId: string;
  connectionIds: string[];
  destinations: { nodeId: string; sessionId: string }[];
}
