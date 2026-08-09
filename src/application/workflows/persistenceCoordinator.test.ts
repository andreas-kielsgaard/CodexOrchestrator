import { vi } from 'vitest';
import type { WorkflowApplicationClient, WorkflowDefinition } from './contracts';
import { workflowPersistenceCoordinator } from './persistenceCoordinator';

describe('Workflow persistence coordinator', () => {
  it('holds reopen reads behind an already registered write', async () => {
    let resolveWrite!: (definition: WorkflowDefinition) => void;
    const write = new Promise<WorkflowDefinition>((resolve) => {
      resolveWrite = resolve;
    });
    const definition = workflowDefinition();
    const raw = client({ saveNodeDraft: vi.fn(() => write) });
    const coordinated = workflowPersistenceCoordinator(raw);

    const pendingWrite = coordinated.saveNodeDraft('workflow-1', definition.nodes[0]!.draft!);
    const pendingLoad = coordinated.loadWorkflowType('workflow-1');
    await Promise.resolve();
    expect(raw.loadWorkflowType).not.toHaveBeenCalled();

    resolveWrite(definition);
    await expect(pendingWrite).resolves.toEqual(definition);
    await expect(pendingLoad).resolves.toEqual(definition);
    expect(raw.loadWorkflowType).toHaveBeenCalledOnce();
  });

  it('surfaces a failed write once, then permits durable recovery and later writes', async () => {
    const failure = new Error('Unable to persist Workflow draft.');
    const definition = workflowDefinition();
    const saveNodeDraft = vi
      .fn<WorkflowApplicationClient['saveNodeDraft']>()
      .mockRejectedValueOnce(failure)
      .mockResolvedValue(definition);
    const raw = client({ saveNodeDraft });
    const coordinated = workflowPersistenceCoordinator(raw);

    await expect(coordinated.saveNodeDraft('workflow-1', definition.nodes[0]!.draft!)).rejects.toBe(
      failure,
    );
    await expect(coordinated.loadWorkflowType('workflow-1')).rejects.toBe(failure);
    expect(raw.loadWorkflowType).not.toHaveBeenCalled();

    await expect(coordinated.loadWorkflowType('workflow-1')).resolves.toEqual(definition);
    expect(raw.loadWorkflowType).toHaveBeenCalledOnce();
    await expect(
      coordinated.saveNodeDraft('workflow-1', definition.nodes[0]!.draft!),
    ).resolves.toEqual(definition);
    expect(saveNodeDraft).toHaveBeenCalledTimes(2);
  });

  it('orders connection drafts before reopening the workflow definition', async () => {
    let resolveWrite!: (definition: WorkflowDefinition) => void;
    const definition = workflowDefinition();
    const connection = {
      id: 'connection-1',
      name: 'Handoff',
      senderNodeId: 'node-1',
      receiverNodeId: 'node-1',
      mechanism: null,
    };
    const raw = client({
      saveConnectionDraft: vi.fn(
        () =>
          new Promise<WorkflowDefinition>((resolve) => {
            resolveWrite = resolve;
          }),
      ),
    });
    const coordinated = workflowPersistenceCoordinator(raw);

    const pendingWrite = coordinated.saveConnectionDraft('workflow-1', connection);
    const pendingLoad = coordinated.loadWorkflowType('workflow-1');
    await Promise.resolve();
    expect(raw.loadWorkflowType).not.toHaveBeenCalled();

    resolveWrite(definition);
    await expect(pendingWrite).resolves.toEqual(definition);
    await expect(pendingLoad).resolves.toEqual(definition);
  });
});

function client(overrides: Partial<WorkflowApplicationClient> = {}): WorkflowApplicationClient {
  const definition = workflowDefinition();
  return {
    listWorkflowTypes: vi.fn(async () => [definition.workflowType]),
    listWorkflowInstances: vi.fn(async () => []),
    launchWorkflowInstance: vi.fn(async () => {
      throw new Error('not configured');
    }),
    loadWorkflowInstance: vi.fn(async () => {
      throw new Error('not configured');
    }),
    listRoles: vi.fn(async () => []),
    createRole: vi.fn(async ({ name, harness }) => ({
      id: 'role-1',
      name,
      harness,
      createdAt: '',
      updatedAt: '',
    })),
    updateRole: vi.fn(async ({ roleId, name, harness }) => ({
      id: roleId,
      name,
      harness,
      createdAt: '',
      updatedAt: '',
    })),
    createWorkflowType: vi.fn(async () => definition),
    loadWorkflowType: vi.fn(async () => definition),
    saveNodeDraft: vi.fn(async () => definition),
    deleteNodeDraft: vi.fn(async () => definition),
    detachNodeRole: vi.fn(async () => definition),
    saveNodeAsRole: vi.fn(async () => definition),
    saveConnectionDraft: vi.fn(async () => definition),
    deleteConnectionDraft: vi.fn(async () => definition),
    activateChanges: vi.fn(async () => definition),
    ...overrides,
  };
}

function workflowDefinition(): WorkflowDefinition {
  const node = {
    id: 'node-1',
    name: 'Review architecture',
    harnessName: 'Architecture reviewer',
    roleName: null,
    positionX: 100,
    positionY: 100,
    isStartingPoint: true,
  };
  return {
    workflowType: {
      id: 'workflow-1',
      name: 'Review loop',
      activeRecipeId: null,
      editedElementCount: 1,
      createdAt: '2026-08-08T20:00:00.000Z',
      updatedAt: '2026-08-08T20:00:00.000Z',
    },
    nodes: [{ id: node.id, draft: node, live: null, hasUnpublishedChanges: true }],
    connections: [],
    activeRecipe: null,
  };
}
