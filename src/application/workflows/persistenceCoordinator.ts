import type { WorkflowApplicationClient, WorkflowDefinition } from './contracts';

interface WorkflowBarrier {
  tail: Promise<void>;
  failure: unknown | null;
}

const coordinatedClients = new WeakMap<WorkflowApplicationClient, WorkflowApplicationClient>();

/** Keeps Workflow writes ordered across editor unmount/reopen boundaries. */
export function workflowPersistenceCoordinator(
  client: WorkflowApplicationClient,
): WorkflowApplicationClient {
  const existing = coordinatedClients.get(client);
  if (existing) return existing;

  const barriers = new Map<string, WorkflowBarrier>();
  const barrierFor = (workflowTypeId: string) => {
    let barrier = barriers.get(workflowTypeId);
    if (!barrier) {
      barrier = { tail: Promise.resolve(), failure: null };
      barriers.set(workflowTypeId, barrier);
    }
    return barrier;
  };
  const enqueue = <T>(workflowTypeId: string, operation: () => Promise<T>): Promise<T> => {
    const barrier = barrierFor(workflowTypeId);
    const result = barrier.tail.then(operation);
    barrier.tail = result.then(
      () => {
        barrier.failure = null;
      },
      (error: unknown) => {
        barrier.failure = error;
      },
    );
    return result;
  };
  const waitForWorkflow = async (workflowTypeId: string) => {
    const barrier = barrierFor(workflowTypeId);
    let observed: Promise<void>;
    do {
      observed = barrier.tail;
      await observed;
    } while (observed !== barrier.tail);
    if (barrier.failure !== null) {
      const failure = barrier.failure;
      barrier.failure = null;
      throw failure;
    }
  };
  const waitForAll = async () => {
    for (const workflowTypeId of barriers.keys()) await waitForWorkflow(workflowTypeId);
  };

  const coordinated: WorkflowApplicationClient = {
    listWorkflowInstances: async () => {
      await waitForAll();
      return client.listWorkflowInstances();
    },
    listWorkflowMcpComponents: () => client.listWorkflowMcpComponents(),
    launchWorkflowInstance: async (input) => {
      await waitForWorkflow(input.workflowTypeId);
      return client.launchWorkflowInstance(input);
    },
    loadWorkflowInstance: (workflowInstanceId) => client.loadWorkflowInstance(workflowInstanceId),
    listWorkflowTypes: async () => {
      await waitForAll();
      return client.listWorkflowTypes();
    },
    listRoles: async () => {
      await waitForAll();
      return client.listRoles();
    },
    createRole: async (input) => {
      await waitForAll();
      return client.createRole(input);
    },
    updateRole: async (input) => {
      await waitForAll();
      return client.updateRole(input);
    },
    createWorkflowType: (input) => client.createWorkflowType(input),
    loadWorkflowType: async (workflowTypeId) => {
      await waitForWorkflow(workflowTypeId);
      return client.loadWorkflowType(workflowTypeId);
    },
    saveNodeDraft: (workflowTypeId, node) =>
      enqueue<WorkflowDefinition>(workflowTypeId, () => client.saveNodeDraft(workflowTypeId, node)),
    deleteNodeDraft: (workflowTypeId, nodeId) =>
      enqueue<WorkflowDefinition>(workflowTypeId, () =>
        client.deleteNodeDraft(workflowTypeId, nodeId),
      ),
    detachNodeRole: (workflowTypeId, nodeId) =>
      enqueue<WorkflowDefinition>(workflowTypeId, () =>
        client.detachNodeRole(workflowTypeId, nodeId),
      ),
    saveNodeAsRole: (workflowTypeId, nodeId, roleName) =>
      enqueue<WorkflowDefinition>(workflowTypeId, () =>
        client.saveNodeAsRole(workflowTypeId, nodeId, roleName),
      ),
    saveConnectionDraft: (workflowTypeId, connection) =>
      enqueue<WorkflowDefinition>(workflowTypeId, () =>
        client.saveConnectionDraft(workflowTypeId, connection),
      ),
    deleteConnectionDraft: (workflowTypeId, connectionId) =>
      enqueue<WorkflowDefinition>(workflowTypeId, () =>
        client.deleteConnectionDraft(workflowTypeId, connectionId),
      ),
    activateChanges: (workflowTypeId, elements) =>
      enqueue<WorkflowDefinition>(workflowTypeId, () =>
        client.activateChanges(workflowTypeId, elements),
      ),
  };
  coordinatedClients.set(client, coordinated);
  coordinatedClients.set(coordinated, coordinated);
  return coordinated;
}
