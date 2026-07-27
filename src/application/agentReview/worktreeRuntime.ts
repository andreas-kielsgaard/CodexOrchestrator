import type { AgentReviewInstanceRef, AgentReviewTestSourceRef } from './contracts';

export interface AgentReviewInstanceRequest {
  readonly source: AgentReviewTestSourceRef;
  readonly purpose: string;
}

export type AgentReviewInstancePhase =
  'prepared' | 'starting' | 'running' | 'stopping' | 'stopped' | 'recovering' | 'recovered';

export type AgentReviewInstanceHealth = 'not-observed' | 'healthy' | 'unhealthy' | 'closed';

export interface AgentReviewInstanceStatus {
  readonly phase: AgentReviewInstancePhase;
  readonly health: AgentReviewInstanceHealth;
  readonly stale: boolean;
}

export interface AgentReviewRequestedInstance {
  readonly handle: AgentReviewInstanceRef;
  readonly status: AgentReviewInstanceStatus;
}

export interface AgentReviewActionResult {
  readonly outcome: 'passed' | 'failed';
  readonly failedStep: string | null;
  readonly status: AgentReviewInstanceStatus;
}

/** Application-facing lifecycle port. Driver protocols belong to review adapters. */
export interface AgentReviewWorktreeRuntime {
  request(request: AgentReviewInstanceRequest): Promise<AgentReviewRequestedInstance>;
  build(handle: AgentReviewInstanceRef): Promise<AgentReviewActionResult>;
  test(handle: AgentReviewInstanceRef): Promise<AgentReviewActionResult>;
  start(handle: AgentReviewInstanceRef): Promise<AgentReviewInstanceStatus>;
  status(handle: AgentReviewInstanceRef): Promise<AgentReviewInstanceStatus>;
  stop(handle: AgentReviewInstanceRef): Promise<AgentReviewInstanceStatus>;
  recover(handle: AgentReviewInstanceRef): Promise<AgentReviewInstanceStatus>;
}

export interface AgentReviewInstanceReadiness {
  readonly ready: boolean;
  readonly reasons: readonly string[];
}

export function evaluateAgentReviewInstanceStatus(
  status: AgentReviewInstanceStatus,
): AgentReviewInstanceReadiness {
  const reasons: string[] = [];
  if (status.phase !== 'running') reasons.push('instance is not running');
  if (status.health !== 'healthy') reasons.push('instance is not healthy');
  if (status.stale) reasons.push('instance status is stale');

  return { ready: reasons.length === 0, reasons };
}
