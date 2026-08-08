import { vi } from 'vitest';
import { createTauriWorkflowClient } from './tauriWorkflowClient';

describe('Tauri Workflow client', () => {
  it('uses the product Workflow command boundary and preserves typed inputs', async () => {
    const invoke = vi.fn().mockResolvedValue({});
    const client = createTauriWorkflowClient(invoke);
    const node = {
      id: 'node-1',
      name: 'Architecture review',
      harnessName: 'Architecture reviewer',
      roleName: null,
      positionX: 180,
      positionY: 120,
      isStartingPoint: true,
    };

    await client.listWorkflowTypes();
    await client.createWorkflowType({ name: 'Review loop' });
    await client.loadWorkflowType('workflow-1');
    await client.saveNodeDraft('workflow-1', node);
    await client.activateChanges('workflow-1', [{ kind: 'node', id: node.id }]);

    expect(invoke.mock.calls).toEqual([
      ['list_workflow_types'],
      ['create_workflow_type', { input: { name: 'Review loop' } }],
      ['load_workflow_type', { query: { workflowTypeId: 'workflow-1' } }],
      ['save_workflow_node_draft', { input: { workflowTypeId: 'workflow-1', node } }],
      [
        'activate_workflow_changes',
        { input: { workflowTypeId: 'workflow-1', elements: [{ kind: 'node', id: 'node-1' }] } },
      ],
    ]);
  });
});
