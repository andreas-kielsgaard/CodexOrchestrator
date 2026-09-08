import { describe, expect, it, vi } from 'vitest';
import type {
  WorkflowConnectionConfig,
  WorkflowElementRef,
  WorkflowNodeConfig,
} from '../../../application/workflows';
import { WorkflowEditorController, type WorkflowEditableConfig } from './workflowEditorController';

describe('WorkflowEditorController', () => {
  it('consolidates focused field changes into one undo entry', async () => {
    const harness = fixture();
    const controller = harness.controller;
    const target = { kind: 'node', id: 'node-1' } as const;

    controller.beginFieldEdit(target);
    controller.changeElement(target, { ...node, name: 'R' });
    controller.changeElement(target, { ...node, name: 'Review' });
    controller.commitFieldEdit(target);

    await controller.undo();
    expect(harness.read(target)).toEqual(node);
    await controller.redo();
    expect(harness.read(target)).toMatchObject({ name: 'Review' });
  });

  it('deletes local and persisted targets through the same entry point', async () => {
    const localHarness = fixture({ persisted: [] });
    await localHarness.controller.deleteElement({ kind: 'node', id: node.id });
    expect(localHarness.removeLocalElement).toHaveBeenCalledWith({ kind: 'node', id: node.id });
    expect(localHarness.deletePersistedElement).not.toHaveBeenCalled();

    const persistedHarness = fixture({ persisted: ['node:node-1'] });
    await persistedHarness.controller.deleteElement({ kind: 'node', id: node.id });
    expect(persistedHarness.deletePersistedElement).toHaveBeenCalledWith({
      kind: 'node',
      id: node.id,
    });
    expect(persistedHarness.removeLocalElement).not.toHaveBeenCalled();
  });

  it('deletes dependent connections with a node and restores them on undo', async () => {
    const harness = fixture({ connections: [connection] });

    await harness.controller.deleteElement({ kind: 'node', id: node.id });
    expect(harness.read({ kind: 'connection', id: connection.id })).toBeNull();
    expect(harness.read({ kind: 'node', id: node.id })).toBeNull();

    await harness.controller.undo();
    expect(harness.read({ kind: 'node', id: node.id })).toEqual(node);
    expect(harness.read({ kind: 'connection', id: connection.id })).toEqual(connection);
  });
});

const node: WorkflowNodeConfig = {
  id: 'node-1',
  name: 'Review',
  harnessName: 'Reviewer',
  roleName: null,
  positionX: 10,
  positionY: 20,
  isStartingPoint: true,
  harness: { kind: 'standalone', config: emptyHarness() },
};

const connection: WorkflowConnectionConfig = {
  id: 'connection-1',
  name: 'Review to finish',
  senderNodeId: node.id,
  receiverNodeId: 'node-2',
  mechanism: null,
};

function fixture({
  persisted = [],
  connections = [],
}: {
  readonly persisted?: readonly string[];
  readonly connections?: readonly WorkflowConnectionConfig[];
} = {}) {
  const values = new Map<string, WorkflowEditableConfig>([
    [`node:${node.id}`, structuredClone(node)],
  ]);
  for (const candidate of connections)
    values.set(`connection:${candidate.id}`, structuredClone(candidate));
  const persistedTargets = new Set(persisted);
  const key = (target: WorkflowElementRef) => `${target.kind}:${target.id}`;
  const read = (target: WorkflowElementRef) => values.get(key(target)) ?? null;
  const removeLocalElement = vi.fn((target: WorkflowElementRef) => values.delete(key(target)));
  const deletePersistedElement = vi.fn(async (target: WorkflowElementRef) => {
    values.delete(key(target));
  });
  const controller = new WorkflowEditorController();
  controller.configure({
    readElement: read,
    isPersisted: (target) => persistedTargets.has(key(target)),
    dependentConnections: (nodeId) =>
      [...values.values()].filter(
        (value): value is WorkflowConnectionConfig =>
          'senderNodeId' in value &&
          (value.senderNodeId === nodeId || value.receiverNodeId === nodeId),
      ),
    saveElement: (target, value) => values.set(key(target), structuredClone(value)),
    removeLocalElement,
    deletePersistedElement,
  });
  return { controller, read, removeLocalElement, deletePersistedElement };
}

function emptyHarness(): WorkflowNodeConfig['harness'] extends { config: infer T } ? T : never {
  return {
    identity: { name: '', description: '' },
    model: { model: null, reasoningEffort: null },
    execution: { sandboxMode: null, webSearchEnabled: null },
    instructions: { developerInstructions: '', userInstructions: '' },
    skills: [],
    mcp: { serverNames: [] },
    agent: { maxTurns: null },
  } as never;
}
