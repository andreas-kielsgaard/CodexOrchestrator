import { vi } from 'vitest';
import type { WorkflowRecipeDraftDto } from '../../application/workflowAuthoring';
import { createTauriWorkflowAuthoringClient } from './tauriWorkflowAuthoringClient';

describe('Tauri Workflow Authoring client', () => {
  it('maps recipe, node-copy, compilation, and user-entry operations without legacy fields', async () => {
    const invoke = vi.fn().mockResolvedValue(null);
    const client = createTauriWorkflowAuthoringClient(invoke);
    const draft: WorkflowRecipeDraftDto = {
      contractVersion: 1,
      recipeId: 'review',
      name: 'Review',
      revision: 2,
      startingNodeId: 'reviewer',
      nodes: [
        {
          nodeId: 'reviewer',
          name: 'Reviewer',
          positionX: 100,
          positionY: 200,
          capabilityProfileId: 'review-capabilities',
          nodeProfile: {
            contractVersion: 1,
            allowedCapabilities: {
              models: ['codex-a'],
              reasoningModes: ['high'],
              sandboxModes: ['workspace_write'],
              mcpTools: {},
              skills: [],
            },
            pinnedDefaults: {
              model: 'codex-a',
              reasoningMode: 'high',
              sandboxMode: 'workspace_write',
            },
          },
          initialPrompt: 'Review carefully.',
          agentIdentityId: 'avery',
        },
      ],
      connections: [],
    };

    await client.listRecipes();
    await client.loadRecipe('review');
    await client.createRecipe('Review');
    await client.saveDraft(draft);
    await client.copyNodeConfiguration({
      recipeId: 'review',
      expectedRevision: 2,
      sourceNodeId: 'reviewer',
      destinationNodeId: 'implementer',
    });
    await client.activateRecipe('review', 2);
    await client.compileRecipeInstance('review', 'run-1');
    await client.dispatchUserRequest({
      recipeId: 'review',
      instanceId: 'run-1',
      text: 'Review this plan.',
    });

    expect(invoke.mock.calls).toEqual([
      ['list_workflow_recipes'],
      ['load_workflow_recipe', { input: { recipeId: 'review' } }],
      ['create_workflow_recipe', { input: { name: 'Review' } }],
      ['save_workflow_recipe_draft', { input: { draft } }],
      [
        'copy_workflow_node_configuration',
        {
          input: {
            recipeId: 'review',
            expectedRevision: 2,
            sourceNodeId: 'reviewer',
            destinationNodeId: 'implementer',
          },
        },
      ],
      ['activate_workflow_recipe', { input: { recipeId: 'review', expectedRevision: 2 } }],
      ['compile_workflow_recipe_instance', { input: { recipeId: 'review', instanceId: 'run-1' } }],
      [
        'dispatch_workflow_user_request',
        {
          input: {
            recipeId: 'review',
            instanceId: 'run-1',
            text: 'Review this plan.',
          },
        },
      ],
    ]);
  });
});
