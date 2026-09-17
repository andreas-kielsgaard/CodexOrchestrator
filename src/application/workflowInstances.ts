import type {
  WorkflowRecipeDraftDto,
  OtpCapabilityRefDto,
  OtpOutputRefDto,
} from './workflowAuthoring';
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
  readonly context: {
    readonly instanceId: string;
    readonly occurrenceId: string;
    readonly capability: OtpCapabilityRefDto;
    readonly source: {
      readonly nodeId: string;
      readonly nodeName: string;
      readonly sessionId: string;
      readonly invocationId: string;
    } | null;
    readonly connectionId: string | null;
    readonly outputNodeId: string | null;
  };
  readonly output: OtpOutputRefDto | null;
  readonly payload: unknown;
  readonly sessionRequests: readonly {
    readonly nodeId: string;
    readonly target:
      { readonly kind: 'new' } | { readonly kind: 'exact'; readonly sessionId: string };
    readonly prompt: readonly { readonly reference: string; readonly text: string }[];
  }[];
  readonly stopOutcomes?: readonly SessionStopOutcome[];
  readonly message?: string;
  readonly createdAt: string;
  readonly eventGroups: readonly ReferenceIdentityDto[];
  readonly error: string | null;
}
export interface WorkflowInstanceDetails {
  readonly instance: WorkflowRecipeInstance;
  readonly sessions: readonly WorkflowInstanceSession[];
  readonly attempts: readonly WorkflowEventAttempt[];
}
export interface SessionStopOutcome {
  readonly nodeId: string;
  readonly sessionId: string;
  readonly invocationId: string | null;
  readonly status: 'pending' | 'requested' | 'no_op' | 'failed';
  readonly error: string | null;
}
export interface WorkflowActionResult {
  readonly attemptId: string;
  readonly eventGroups: readonly SessionEventResultDto[];
  readonly stopOutcomes: readonly SessionStopOutcome[];
  readonly message: string;
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
    readonly data?: Readonly<Record<string, unknown>>;
  }): Promise<WorkflowActionResult>;
}
