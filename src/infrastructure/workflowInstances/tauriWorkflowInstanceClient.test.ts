import type { invoke } from '@tauri-apps/api/core';
import { createTauriWorkflowInstanceClient } from './tauriWorkflowInstanceClient';

it('uses stored-instance commands and carries the exact recipe revision, target and node', async () => {
  const call = vi.fn(async () => undefined) as unknown as typeof invoke;
  const client = createTauriWorkflowInstanceClient(call);
  const target = {
    repository: { id: 'repo', name: 'Repo', gitCommonDirectory: 'C:/repo/.git' },
    branch: { id: 'main', name: 'main' },
    worktree: { id: 'worktree', path: 'C:/repo' },
  };
  const input = { recipeId: 'recipe', expectedRevision: 4, name: 'Review', target };
  await client.create(input);
  await client.list();
  await client.load('instance');
  const message = {
    recipeId: 'recipe',
    instanceId: 'instance',
    nodeId: 'reviewer',
    text: 'Review the plan',
  };
  await client.messageNode(message);
  expect(call).toHaveBeenNthCalledWith(1, 'create_workflow_recipe_instance', { input });
  expect(call).toHaveBeenNthCalledWith(2, 'list_workflow_recipe_instances');
  expect(call).toHaveBeenNthCalledWith(3, 'load_workflow_recipe_instance', {
    input: { instanceId: 'instance' },
  });
  expect(call).toHaveBeenNthCalledWith(4, 'dispatch_workflow_user_request', { input: message });
});
