import type { NodeProfileDto } from '../executionConfiguration';
import type {
  ReferenceIdentityDto,
  RunningFilterDto,
  SessionCreationFilterDto,
  SessionEventDefinitionDto,
  SessionEventResultDto,
  TargetCardinalityDto,
  TargetOrderingDto,
  MissingTargetPolicyDto,
} from '../sessionEvents';

export type WorkflowConnectionTriggerDto =
  | { readonly kind: 'invocation_completed' }
  | {
      readonly kind: 'mcp_call';
      readonly server: ReferenceIdentityDto;
      readonly tool: ReferenceIdentityDto;
    }
  | { readonly kind: 'application_event'; readonly eventKind: ReferenceIdentityDto }
  | {
      readonly kind: 'event_group_completed';
      readonly sourceDefinition: ReferenceIdentityDto;
    };

export type WorkflowConnectionPromptInputDto =
  | { readonly kind: 'invocation_output' }
  | { readonly kind: 'mcp_argument'; readonly name: string }
  | { readonly kind: 'application_event_field'; readonly field: string }
  | { readonly kind: 'referenced_content'; readonly reference: ReferenceIdentityDto };

export interface WorkflowConnectionTargetPlanDto {
  readonly cardinality: TargetCardinalityDto;
  readonly ordering: TargetOrderingDto;
  readonly running: RunningFilterDto;
  readonly createdBy: SessionCreationFilterDto | null;
  readonly missing: MissingTargetPolicyDto;
}

export interface WorkflowAuthoringNodeDto {
  readonly nodeId: string;
  readonly name: string;
  readonly positionX: number;
  readonly positionY: number;
  readonly capabilityProfileId: string;
  readonly nodeProfile: NodeProfileDto;
  readonly initialPrompt: string | null;
  readonly agentIdentityId: string | null;
}

export interface WorkflowAuthoringConnectionDto {
  readonly connectionId: string;
  readonly name: string;
  readonly sourceNodeId: string;
  readonly destinationNodeId: string;
  readonly trigger: WorkflowConnectionTriggerDto;
  readonly promptInputs: readonly WorkflowConnectionPromptInputDto[];
  readonly promptText: string;
  readonly target: WorkflowConnectionTargetPlanDto;
}

export interface WorkflowRecipeDraftDto {
  readonly contractVersion: 1;
  readonly recipeId: string;
  readonly name: string;
  readonly revision: number;
  readonly startingNodeId: string | null;
  readonly nodes: readonly WorkflowAuthoringNodeDto[];
  readonly connections: readonly WorkflowAuthoringConnectionDto[];
}

export interface WorkflowRecipeStateDto {
  readonly draft: WorkflowRecipeDraftDto;
  readonly active: WorkflowRecipeDraftDto | null;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface WorkflowRecipeSummaryDto {
  readonly recipeId: string;
  readonly name: string;
  readonly draftRevision: number;
  readonly activeRevision: number | null;
  readonly updatedAt: string;
}

export interface CopyWorkflowNodeConfigurationInput {
  readonly recipeId: string;
  readonly expectedRevision: number;
  readonly sourceNodeId: string;
  readonly destinationNodeId: string;
}

export interface DispatchWorkflowUserRequestInput {
  readonly recipeId: string;
  readonly instanceId: string;
  readonly text: string;
}

export interface WorkflowAuthoringClient {
  listRecipes(): Promise<readonly WorkflowRecipeSummaryDto[]>;
  loadRecipe(recipeId: string): Promise<WorkflowRecipeStateDto>;
  createRecipe(name: string): Promise<WorkflowRecipeStateDto>;
  saveDraft(draft: WorkflowRecipeDraftDto): Promise<WorkflowRecipeStateDto>;
  copyNodeConfiguration(input: CopyWorkflowNodeConfigurationInput): Promise<WorkflowRecipeStateDto>;
  activateRecipe(recipeId: string): Promise<WorkflowRecipeStateDto>;
  compileRecipeInstance(
    recipeId: string,
    instanceId: string,
  ): Promise<readonly SessionEventDefinitionDto[]>;
  dispatchUserRequest(input: DispatchWorkflowUserRequestInput): Promise<SessionEventResultDto>;
}
