import type {
  WorkflowEventAttempt,
  WorkflowInstanceDetails,
  WorkflowInstanceSession,
} from '../../application/workflowInstances';
import type {
  WorkflowAuthoringConnectionDto,
  WorkflowAuthoringNodeDto,
} from '../../application/workflowAuthoring';
import type { WorkflowGraphConnection, WorkflowGraphNode } from '../workflowGraph';

export type WorkflowInstanceSelection =
  | { readonly kind: 'node'; readonly id: string }
  | { readonly kind: 'connection'; readonly id: string }
  | { readonly kind: 'session'; readonly id: string; readonly nodeId: string | null }
  | null;

export function instanceGraphNodes(details: WorkflowInstanceDetails): readonly WorkflowGraphNode[] {
  return details.instance.recipe.nodes.map((node) => ({
    id: node.nodeId,
    name: node.name,
    x: node.positionX,
    y: node.positionY,
    starting: details.instance.recipe.startingNodeId === node.nodeId,
  }));
}

export function instanceGraphConnections(
  details: WorkflowInstanceDetails,
): readonly WorkflowGraphConnection[] {
  return details.instance.recipe.connections.map((connection) => ({
    id: connection.connectionId,
    name: connection.name,
    source: connection.sourceNodeId,
    destination: connection.destinationNodeId,
  }));
}

export function sessionsForNode(
  sessions: readonly WorkflowInstanceSession[],
  nodeId: string,
): readonly WorkflowInstanceSession[] {
  return sessions.filter(
    (entry) =>
      entry.logicalAddress?.subject.kind === 'node' && entry.logicalAddress.subject.id === nodeId,
  );
}

export function attemptsForConnection(
  attempts: readonly WorkflowEventAttempt[],
  connectionId: string,
): readonly WorkflowEventAttempt[] {
  return attempts.filter(
    (attempt) =>
      attempt.workflowElementRef?.kind === 'connection' &&
      attempt.workflowElementRef.id === connectionId,
  );
}

export function attemptsForNode(
  attempts: readonly WorkflowEventAttempt[],
  nodeId: string,
): readonly WorkflowEventAttempt[] {
  return attempts.filter(
    (attempt) =>
      attempt.workflowElementRef?.kind === 'node' && attempt.workflowElementRef.id === nodeId,
  );
}

export function attemptsWithoutElement(
  attempts: readonly WorkflowEventAttempt[],
): readonly WorkflowEventAttempt[] {
  return attempts.filter((attempt) => !attempt.workflowElementRef);
}

export function nodeById(
  details: WorkflowInstanceDetails,
  nodeId: string,
): WorkflowAuthoringNodeDto | undefined {
  return details.instance.recipe.nodes.find((node) => node.nodeId === nodeId);
}

export function connectionById(
  details: WorkflowInstanceDetails,
  connectionId: string,
): WorkflowAuthoringConnectionDto | undefined {
  return details.instance.recipe.connections.find(
    (connection) => connection.connectionId === connectionId,
  );
}

export function attemptStatus(attempt: WorkflowEventAttempt): string {
  if (attempt.error) return 'Failed';
  if (attempt.eventGroup) return 'Delivered';
  return 'Preparing';
}
