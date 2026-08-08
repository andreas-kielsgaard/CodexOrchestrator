import { invoke } from '@tauri-apps/api/core';
import type {
  WorkflowApplicationClient,
  WorkflowDefinition,
  WorkflowElementRef,
  WorkflowNodeConfig,
  WorkflowTypeSummary,
} from '../../application/workflows';

export type WorkflowInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriWorkflowClient(
  invokeCommand: WorkflowInvoke = invoke,
): WorkflowApplicationClient {
  return {
    listWorkflowTypes: () => invokeCommand<WorkflowTypeSummary[]>('list_workflow_types'),
    createWorkflowType: (input) =>
      invokeCommand<WorkflowDefinition>('create_workflow_type', { input }),
    loadWorkflowType: (workflowTypeId) =>
      invokeCommand<WorkflowDefinition>('load_workflow_type', {
        query: { workflowTypeId },
      }),
    saveNodeDraft: (workflowTypeId, node: WorkflowNodeConfig) =>
      invokeCommand<WorkflowDefinition>('save_workflow_node_draft', {
        input: { workflowTypeId, node },
      }),
    activateChanges: (workflowTypeId, elements: readonly WorkflowElementRef[]) =>
      invokeCommand<WorkflowDefinition>('activate_workflow_changes', {
        input: { workflowTypeId, elements },
      }),
  };
}

export const tauriWorkflowClient = createTauriWorkflowClient();
