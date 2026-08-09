import { invoke } from '@tauri-apps/api/core';
import type {
  WorkflowApplicationClient,
  WorkflowConnectionConfig,
  WorkflowDefinition,
  WorkflowElementRef,
  WorkflowNodeConfig,
  WorkflowRole,
  WorkflowTypeSummary,
} from '../../application/workflows';

export type WorkflowInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriWorkflowClient(
  invokeCommand: WorkflowInvoke = invoke,
): WorkflowApplicationClient {
  return {
    listWorkflowTypes: () => invokeCommand<WorkflowTypeSummary[]>('list_workflow_types'),
    listRoles: () => invokeCommand<WorkflowRole[]>('list_workflow_roles'),
    createRole: (input) => invokeCommand<WorkflowRole>('create_workflow_role', { input }),
    updateRole: (input) => invokeCommand<WorkflowRole>('update_workflow_role', { input }),
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
    deleteNodeDraft: (workflowTypeId, nodeId) =>
      invokeCommand<WorkflowDefinition>('delete_workflow_node_draft', {
        input: { workflowTypeId, nodeId },
      }),
    detachNodeRole: (workflowTypeId, nodeId) =>
      invokeCommand<WorkflowDefinition>('detach_workflow_node_role', {
        input: { workflowTypeId, nodeId },
      }),
    saveNodeAsRole: (workflowTypeId, nodeId, roleName) =>
      invokeCommand<WorkflowDefinition>('save_workflow_node_as_role', {
        input: { workflowTypeId, nodeId, roleName },
      }),
    saveConnectionDraft: (workflowTypeId, connection: WorkflowConnectionConfig) =>
      invokeCommand<WorkflowDefinition>('save_workflow_connection_draft', {
        input: { workflowTypeId, connection },
      }),
    deleteConnectionDraft: (workflowTypeId, connectionId) =>
      invokeCommand<WorkflowDefinition>('delete_workflow_connection_draft', {
        input: { workflowTypeId, connectionId },
      }),
    activateChanges: (workflowTypeId, elements: readonly WorkflowElementRef[]) =>
      invokeCommand<WorkflowDefinition>('activate_workflow_changes', {
        input: { workflowTypeId, elements },
      }),
  };
}

export const tauriWorkflowClient = createTauriWorkflowClient();
