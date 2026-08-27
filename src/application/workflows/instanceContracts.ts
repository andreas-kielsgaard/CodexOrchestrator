import type { ResolvedRepoBranchWorktreeTarget } from '../worktreeTargets';
import type { EffectiveWorkflowRecipe, WorkflowConnectionActivation } from './contracts';

export interface WorkflowInstanceSummary {
  readonly id: string;
  readonly workflowTypeId: string;
  readonly workflowTypeName: string;
  readonly recipeId: string;
  readonly name: string;
  readonly sessionCount: number;
  readonly activeSessionCount: number;
  readonly idleSessionCount: number;
  readonly createdAt: string;
}

export interface WorkflowInstanceSession {
  readonly nodeId: string;
  readonly sessionId: string;
  readonly title: string;
  readonly activity: 'active' | 'idle';
  readonly latestTurnSummary: string | null;
  readonly associatedAt: string;
}

export interface WorkflowInstance {
  readonly summary: WorkflowInstanceSummary;
  readonly target: ResolvedRepoBranchWorktreeTarget;
  readonly recipe: EffectiveWorkflowRecipe;
  readonly sessions: readonly WorkflowInstanceSession[];
  readonly connectionActivations: readonly WorkflowConnectionActivation[];
}

export interface CreateWorkflowInstanceInput {
  readonly workflowTypeId: string;
  readonly name: string;
  readonly target: ResolvedRepoBranchWorktreeTarget;
}
