import type { WorkflowActionResult } from '../workflowInstances';
import type { NodeProfileDto } from '../executionConfiguration';
import type { ReferenceIdentityDto } from '../sessionEvents';

import type { OtpCapabilityRefDto, OtpOutputRefDto, OtpPackageDto } from '../otp';
export type * from '../otp';

export type WorkflowConnectionPromptInputDto =
  | { readonly kind: 'output_field'; readonly field: string }
  | {
      readonly kind: 'node_files';
      readonly nodeId: string;
      readonly association: 'created' | 'edited' | 'either';
    }
  | { readonly kind: 'file_content'; readonly path: string };

export interface WorkflowCompiledPlanDto {
  readonly instance: ReferenceIdentityDto;
  readonly recipe: ReferenceIdentityDto;
  readonly startingNode: ReferenceIdentityDto;
  readonly entryAction: OtpCapabilityRefDto;
  readonly entryConfiguration?: Readonly<Record<string, unknown>>;
  readonly nodes: readonly {
    readonly reference: ReferenceIdentityDto;
    readonly initialPrompt: string | null;
    readonly assignedIdentity: ReferenceIdentityDto | null;
    readonly sessionCreation: unknown;
  }[];
  readonly connections: readonly {
    readonly reference: ReferenceIdentityDto;
    readonly sourceNode: ReferenceIdentityDto;
    readonly destinationNode: ReferenceIdentityDto;
    readonly trigger: OtpOutputRefDto;
    readonly action: OtpCapabilityRefDto;
    readonly configuration: Readonly<Record<string, unknown>>;
    readonly promptInputs: readonly WorkflowConnectionPromptInputDto[];
    readonly promptText: string;
  }[];
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
  readonly trigger: OtpOutputRefDto;
  readonly promptInputs: readonly WorkflowConnectionPromptInputDto[];
  readonly promptText: string;
  readonly action: OtpCapabilityRefDto;
  readonly configuration: Readonly<Record<string, unknown>>;
}

export interface WorkflowRecipeDraftDto {
  readonly contractVersion: 2;
  readonly recipeId: string;
  readonly name: string;
  readonly revision: number;
  readonly startingNodeId: string | null;
  readonly entryAction: OtpCapabilityRefDto;
  readonly entryConfiguration?: Readonly<Record<string, unknown>>;
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
  listCapabilities(): Promise<readonly OtpPackageDto[]>;
  listRecipes(): Promise<readonly WorkflowRecipeSummaryDto[]>;
  loadRecipe(recipeId: string): Promise<WorkflowRecipeStateDto>;
  createRecipe(name: string): Promise<WorkflowRecipeStateDto>;
  saveDraft(draft: WorkflowRecipeDraftDto): Promise<WorkflowRecipeStateDto>;
  copyNodeConfiguration(input: CopyWorkflowNodeConfigurationInput): Promise<WorkflowRecipeStateDto>;
  activateRecipe(recipeId: string, expectedRevision: number): Promise<WorkflowRecipeStateDto>;
  compileRecipeInstance(recipeId: string, instanceId: string): Promise<WorkflowCompiledPlanDto>;
  dispatchUserRequest(input: DispatchWorkflowUserRequestInput): Promise<WorkflowActionResult>;
}
