import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { WorkflowInstanceClient } from '../../application/workflowInstances';

export function createTauriWorkflowInstanceClient(
  call: typeof invoke = invoke,
): WorkflowInstanceClient {
  return {
    subscribeChanged: (listener) =>
      listen<string>('workflow-instance-updated', (event) => listener(event.payload)),
    list: () => call('list_workflow_recipe_instances'),
    create: (input) => call('create_workflow_recipe_instance', { input }),
    load: (instanceId) => call('load_workflow_recipe_instance', { input: { instanceId } }),
    messageNode: (input) => call('dispatch_workflow_user_request', { input }),
  };
}
export const tauriWorkflowInstanceClient = createTauriWorkflowInstanceClient();
