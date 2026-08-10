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
      harness: {
        kind: 'role' as const,
        roleId: 'role-1',
        overrides: { promptPrefixContent: 'Review this exact proposal.' },
      },
    };
    const connection = {
      id: 'connection-1',
      name: 'Review handoff',
      senderNodeId: 'node-1',
      receiverNodeId: 'node-2',
      mechanism: null,
    };
    const harness = {
      identity: {
        name: 'Architecture reviewer',
        machineKey: 'architecture_reviewer',
        permittedAgentNames: null,
        visualIdentity: null,
      },
      promptPrefix: {
        content: 'Review the proposal.',
        initialDelivery: 'prepend' as const,
        contextCompressionDelivery: 'deferred' as const,
      },
      skills: { availableDiscoveryPolicy: 'whitelist' as const, items: [] },
      tools: {
        availableDiscoveryPolicy: 'whitelist' as const,
        items: [],
        schemaBoundary: 'Runtime-owned schemas.',
        mcpServers: [],
      },
      runtime: {
        modelPolicyMode: 'revision_owned' as const,
        models: [],
        defaultModel: null,
        defaultReasoning: null,
        sandbox: 'workspace_write' as const,
        sandboxOptions: ['workspace_write'] as const,
        approvalPolicy: 'never' as const,
        approvalPolicyOptions: ['never'] as const,
        authoritySummary: 'Architecture reviewer',
      },
      hooks: [],
      updatePolicy: { status: 'not_configured' as const, reason: 'Not configured.' },
    };

    await client.listWorkflowTypes();
    await client.listWorkflowInstances();
    await client.listWorkflowMcpComponents();
    await client.launchWorkflowInstance({
      workflowTypeId: 'workflow-1',
      name: null,
      startingPrompt: 'Start the review.',
    });
    await client.loadWorkflowInstance('instance-1');
    await client.listRoles();
    await client.createRole({ name: 'Reviewer', harness });
    await client.updateRole({ roleId: 'role-1', name: 'Senior reviewer', harness });
    await client.createWorkflowType({ name: 'Review loop' });
    await client.loadWorkflowType('workflow-1');
    await client.saveNodeDraft('workflow-1', node);
    await client.deleteNodeDraft('workflow-1', node.id);
    await client.detachNodeRole('workflow-1', node.id);
    await client.saveNodeAsRole('workflow-1', node.id, 'Saved reviewer');
    await client.saveConnectionDraft('workflow-1', connection);
    await client.deleteConnectionDraft('workflow-1', connection.id);
    await client.activateChanges('workflow-1', [{ kind: 'node', id: node.id }]);

    expect(invoke.mock.calls).toEqual([
      ['list_workflow_types'],
      ['list_workflow_instances'],
      ['list_workflow_mcp_components'],
      [
        'launch_workflow_instance',
        {
          input: {
            workflowTypeId: 'workflow-1',
            name: null,
            startingPrompt: 'Start the review.',
          },
        },
      ],
      ['load_workflow_instance', { query: { workflowInstanceId: 'instance-1' } }],
      ['list_workflow_roles'],
      ['create_workflow_role', { input: { name: 'Reviewer', harness } }],
      ['update_workflow_role', { input: { roleId: 'role-1', name: 'Senior reviewer', harness } }],
      ['create_workflow_type', { input: { name: 'Review loop' } }],
      ['load_workflow_type', { query: { workflowTypeId: 'workflow-1' } }],
      ['save_workflow_node_draft', { input: { workflowTypeId: 'workflow-1', node } }],
      ['delete_workflow_node_draft', { input: { workflowTypeId: 'workflow-1', nodeId: 'node-1' } }],
      ['detach_workflow_node_role', { input: { workflowTypeId: 'workflow-1', nodeId: 'node-1' } }],
      [
        'save_workflow_node_as_role',
        { input: { workflowTypeId: 'workflow-1', nodeId: 'node-1', roleName: 'Saved reviewer' } },
      ],
      ['save_workflow_connection_draft', { input: { workflowTypeId: 'workflow-1', connection } }],
      [
        'delete_workflow_connection_draft',
        { input: { workflowTypeId: 'workflow-1', connectionId: 'connection-1' } },
      ],
      [
        'activate_workflow_changes',
        { input: { workflowTypeId: 'workflow-1', elements: [{ kind: 'node', id: 'node-1' }] } },
      ],
    ]);
  });
});
