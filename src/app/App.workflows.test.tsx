import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import type { WorkflowApplicationClient, WorkflowDefinition } from '../application/workflows';
import { createRecordedDevelopmentApplicationComposition } from '../dev/orchestrationSection/recordedOrchestrationClient';
import { App } from './App';

describe('App Workflow navigation', () => {
  it('opens a Workflow type through typed product navigation and Back restores the landing page', async () => {
    const definition = workflowDefinition();
    const workflowClient = mutableWorkflowClient(definition);
    render(
      <App
        {...createRecordedDevelopmentApplicationComposition()}
        workflowClient={workflowClient}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Workflow' }));
    expect(await screen.findByRole('main', { name: 'Workflows' })).toBeVisible();
    fireEvent.click(screen.getByRole('tab', { name: 'Workflow types' }));
    fireEvent.click(await screen.findByRole('button', { name: /Review loop/ }));

    expect(await screen.findByRole('main', { name: 'Edit Review loop' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));

    expect(await screen.findByRole('main', { name: 'Workflows' })).toBeVisible();
    expect(screen.getByRole('tab', { name: 'Launched workflows' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });

  it('flushes an edited node draft through product navigation and reloads it on reopen', async () => {
    const workflowClient = mutableWorkflowClient(workflowDefinition());
    render(
      <App
        {...createRecordedDevelopmentApplicationComposition()}
        workflowClient={workflowClient}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Workflow' }));
    fireEvent.click(await screen.findByRole('tab', { name: 'Workflow types' }));
    fireEvent.click(await screen.findByRole('button', { name: /Review loop/ }));
    const canvas = await screen.findByLabelText('Workflow canvas');
    fireEvent.click(screen.getByRole('button', { name: 'Node' }));
    fireEvent.keyDown(canvas, { key: 'Enter' });
    const harnessName = await screen.findByLabelText('Harness name');
    await waitFor(() => expect(harnessName).toBeEnabled());
    fireEvent.change(harnessName, { target: { value: 'Architecture reviewer' } });
    const releaseDelayedSave = workflowClient.holdNextSave();
    fireEvent.change(screen.getByLabelText('Node name'), {
      target: { value: 'Review architecture' },
    });

    fireEvent.click(screen.getByRole('button', { name: 'Orchestration' }));
    fireEvent.click(screen.getByRole('button', { name: 'Workflow' }));
    fireEvent.click(await screen.findByRole('tab', { name: 'Workflow types' }));
    expect(workflowClient.listWorkflowTypes).toHaveBeenCalledOnce();
    expect(screen.getByText('Loading workflow types…')).toBeVisible();

    releaseDelayedSave();
    fireEvent.click(await screen.findByRole('button', { name: /Review loop/ }));

    expect(
      await screen.findByRole('button', { name: 'Configure Review architecture' }),
    ).toBeVisible();
    expect(vi.mocked(workflowClient.saveNodeDraft).mock.calls.at(-1)?.[1]).toMatchObject({
      name: 'Review architecture',
      harnessName: 'Architecture reviewer',
      roleName: null,
    });
  });
});

type ControlledWorkflowClient = WorkflowApplicationClient & {
  holdNextSave(): () => void;
};

function mutableWorkflowClient(initial: WorkflowDefinition): ControlledWorkflowClient {
  let definition = initial;
  let nextSaveGate: Promise<void> | null = null;
  let releaseNextSave: (() => void) | null = null;
  return {
    listWorkflowTypes: vi.fn(async () => [definition.workflowType]),
    createWorkflowType: vi.fn(async () => definition),
    loadWorkflowType: vi.fn(async () => definition),
    saveNodeDraft: vi.fn(async (_workflowTypeId, node) => {
      const gate = nextSaveGate;
      nextSaveGate = null;
      if (gate) await gate;
      definition = {
        ...definition,
        workflowType: { ...definition.workflowType, editedElementCount: 1 },
        nodes: [{ id: node.id, draft: node, live: null, hasUnpublishedChanges: true }],
      };
      return definition;
    }),
    deleteNodeDraft: vi.fn(async () => definition),
    saveConnectionDraft: vi.fn(async () => definition),
    deleteConnectionDraft: vi.fn(async () => definition),
    activateChanges: vi.fn(async () => definition),
    holdNextSave() {
      nextSaveGate = new Promise<void>((resolve) => {
        releaseNextSave = resolve;
      });
      return () => releaseNextSave?.();
    },
  };
}

function workflowDefinition(): WorkflowDefinition {
  return {
    workflowType: {
      id: 'workflow-review',
      name: 'Review loop',
      activeRecipeId: null,
      editedElementCount: 0,
      createdAt: '2026-08-08T20:00:00.000Z',
      updatedAt: '2026-08-08T20:00:00.000Z',
    },
    nodes: [],
    connections: [],
    activeRecipe: null,
  };
}
