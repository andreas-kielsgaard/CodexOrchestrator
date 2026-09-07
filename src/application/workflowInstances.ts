import type { WorkflowRecipeDraftDto } from './workflowAuthoring';
import type {
  ReferenceIdentityDto,
  SessionLogicalAddressDto,
  SessionEventResultDto,
} from './sessionEvents';
import type { ResolvedRepoBranchWorktreeTarget } from './worktreeTargets';

export interface WorkflowRecipeInstance {
  readonly id: string;
  readonly name: string;
  readonly recipe: WorkflowRecipeDraftDto;
  readonly target: ResolvedRepoBranchWorktreeTarget;
  readonly createdAt: string;
}
export interface WorkflowInstanceSession {
  readonly session: ReferenceIdentityDto;
  readonly logicalAddress: SessionLogicalAddressDto | null;
  readonly running: boolean;
}
export interface WorkflowEventAttempt {
  readonly id: string;
  readonly instanceId: string;
  readonly definitionRef: ReferenceIdentityDto;
  readonly sourceSessionId: string | null;
  readonly createdAt: string;
  readonly eventGroup: ReferenceIdentityDto | null;
  readonly error: string | null;
}
export interface WorkflowInstanceDetails {
  readonly instance: WorkflowRecipeInstance;
  readonly sessions: readonly WorkflowInstanceSession[];
  readonly attempts: readonly WorkflowEventAttempt[];
}
/** Runtime instances are separate from editable recipes. Creation does not launch a Session. */
export interface WorkflowInstanceClient {
  subscribeChanged?(listener: (instanceId: string) => void): Promise<() => void>;
  list(): Promise<readonly WorkflowRecipeInstance[]>;
  create(input: {
    readonly recipeId: string;
    readonly expectedRevision: number;
    readonly name: string;
    readonly target: ResolvedRepoBranchWorktreeTarget;
  }): Promise<WorkflowRecipeInstance>;
  load(instanceId: string): Promise<WorkflowInstanceDetails>;
  messageNode(input: {
    readonly recipeId: string;
    readonly instanceId: string;
    readonly nodeId: string | null;
    readonly text: string;
  }): Promise<SessionEventResultDto>;
}
