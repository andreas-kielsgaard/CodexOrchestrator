import { invoke } from '@tauri-apps/api/core';
import type {
  WorkflowAuthoringClient,
  WorkflowCompiledPlanDto,
  OtpPackageDto,
  WorkflowRecipeStateDto,
  WorkflowRecipeSummaryDto,
} from '../../application/workflowAuthoring';
import type { SessionEventResultDto } from '../../application/sessionEvents';

export type WorkflowAuthoringInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export function createTauriWorkflowAuthoringClient(
  invokeCommand: WorkflowAuthoringInvoke = invoke,
): WorkflowAuthoringClient {
  return {
    listCapabilities: () => invokeCommand<OtpPackageDto[]>('list_workflow_capabilities'),
    listRecipes: () => invokeCommand<WorkflowRecipeSummaryDto[]>('list_workflow_recipes'),
    loadRecipe: (recipeId) =>
      invokeCommand<WorkflowRecipeStateDto>('load_workflow_recipe', {
        input: { recipeId },
      }),
    createRecipe: (name) =>
      invokeCommand<WorkflowRecipeStateDto>('create_workflow_recipe', {
        input: { name },
      }),
    saveDraft: (draft) =>
      invokeCommand<WorkflowRecipeStateDto>('save_workflow_recipe_draft', {
        input: { draft },
      }),
    copyNodeConfiguration: (input) =>
      invokeCommand<WorkflowRecipeStateDto>('copy_workflow_node_configuration', { input }),
    activateRecipe: (recipeId, expectedRevision) =>
      invokeCommand<WorkflowRecipeStateDto>('activate_workflow_recipe', {
        input: { recipeId, expectedRevision },
      }),
    compileRecipeInstance: (recipeId, instanceId) =>
      invokeCommand<WorkflowCompiledPlanDto>('compile_workflow_recipe_instance', {
        input: { recipeId, instanceId },
      }),
    dispatchUserRequest: (input) =>
      invokeCommand<SessionEventResultDto>('dispatch_workflow_user_request', { input }),
  };
}

export const tauriWorkflowAuthoringClient = createTauriWorkflowAuthoringClient();
