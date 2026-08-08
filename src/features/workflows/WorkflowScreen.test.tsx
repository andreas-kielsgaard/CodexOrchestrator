import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { vi } from 'vitest';
import type {
  WorkflowApplicationClient,
  WorkflowDefinition,
  WorkflowNodeConfig,
} from '../../application/workflows';
import { WorkflowScreen } from './WorkflowScreen';

describe('WorkflowScreen', () => {
  it('shows both landing lists and creates a Workflow type through its client', async () => {
    const client = workflowClient(emptyDefinition());
    const onOpen = vi.fn();
    render(<WorkflowScreen client={client} workflowTypeId={null} onOpenWorkflowType={onOpen} />);

    expect(await screen.findByText('No launched workflows')).toBeVisible();
    fireEvent.click(screen.getByRole('tab', { name: 'Workflow types' }));
    await waitFor(() => expect(client.listWorkflowTypes).toHaveBeenCalledOnce());

    fireEvent.change(screen.getByLabelText('New workflow type'), {
      target: { value: 'Architecture review loop' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Create workflow' }));

    await waitFor(() =>
      expect(client.createWorkflowType).toHaveBeenCalledWith({
        name: 'Architecture review loop',
      }),
    );
    expect(onOpen).toHaveBeenCalledWith('workflow-1');
  });

  it('keeps the node brush active, persists a closed draft, and activates it', async () => {
    const client = workflowClient(emptyDefinition());
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const canvas = await screen.findByLabelText('Workflow canvas');
    const nodeBrush = screen.getByRole('button', { name: 'Node' });
    fireEvent.click(nodeBrush);
    expect(nodeBrush).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(canvas, { clientX: 180, clientY: 140 });

    let dialog = screen.getByRole('dialog', { name: 'Configure new node' });
    expect(within(dialog).getByRole('radio', { name: 'From scratch' })).toBeChecked();
    expect(within(dialog).getByRole('radio', { name: /Existing role/ })).toBeDisabled();
    expect(screen.getByText('Draft')).toBeVisible();
    await waitFor(() => expect(within(dialog).getByLabelText('Harness name')).toBeEnabled());
    expect(within(dialog).queryByRole('button', { name: 'Activate node' })).toBeNull();

    fireEvent.change(within(dialog).getByLabelText('Harness name'), {
      target: { value: 'Architecture reviewer' },
    });
    fireEvent.change(within(dialog).getByLabelText('Node name'), {
      target: { value: 'Review architecture' },
    });
    expect(within(dialog).getByRole('checkbox', { name: /Starting node/ })).toBeChecked();
    fireEvent.click(canvas);

    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(screen.getByText('Draft')).toBeVisible();
    expect(nodeBrush).toHaveAttribute('aria-pressed', 'true');
    const savedNode = vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1];
    expect(savedNode).toMatchObject({
      name: 'Review architecture',
      harnessName: 'Architecture reviewer',
      roleName: null,
      isStartingPoint: true,
    });

    fireEvent.click(screen.getByRole('button', { name: 'Configure Review architecture' }));
    dialog = screen.getByRole('dialog', { name: 'Configure Review architecture' });
    fireEvent.click(await within(dialog).findByRole('button', { name: 'Activate node' }));
    await waitFor(() =>
      expect(client.activateChanges).toHaveBeenCalledWith('workflow-1', [
        { kind: 'node', id: savedNode!.id },
      ]),
    );
  });

  it('places and persists a node from the keyboard-accessible canvas', async () => {
    const client = workflowClient(emptyDefinition());
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const canvas = await screen.findByLabelText('Workflow canvas');
    fireEvent.click(screen.getByRole('button', { name: 'Node' }));
    canvas.focus();
    fireEvent.keyDown(canvas, { key: 'Enter' });

    expect(screen.getByRole('dialog', { name: 'Configure new node' })).toBeVisible();
    await waitFor(() => expect(client.saveNodeDraft).toHaveBeenCalledOnce());
    expect(vi.mocked(client.saveNodeDraft).mock.calls[0][1]).toMatchObject({
      roleName: null,
      isStartingPoint: true,
    });
  });

  it('flushes the latest serialized draft before an editor is navigated away and reopened', async () => {
    const client = workflowClient(emptyDefinition());
    const rendered = render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const canvas = await screen.findByLabelText('Workflow canvas');
    fireEvent.click(screen.getByRole('button', { name: 'Node' }));
    fireEvent.keyDown(canvas, { key: 'Enter' });
    const dialog = screen.getByRole('dialog', { name: 'Configure new node' });
    await waitFor(() => expect(within(dialog).getByLabelText('Harness name')).toBeEnabled());
    fireEvent.change(within(dialog).getByLabelText('Harness name'), {
      target: { value: 'Security reviewer' },
    });
    fireEvent.change(within(dialog).getByLabelText('Node name'), {
      target: { value: 'Review security' },
    });

    rendered.rerender(
      <WorkflowScreen client={client} workflowTypeId={null} onOpenWorkflowType={() => undefined} />,
    );
    await waitFor(() =>
      expect(vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1]).toMatchObject({
        name: 'Review security',
        harnessName: 'Security reviewer',
        roleName: null,
      }),
    );

    rendered.rerender(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );
    expect(await screen.findByRole('button', { name: 'Configure Review security' })).toBeVisible();
  });

  it('locks configuration and dismissal while the initial durable draft save is pending', async () => {
    let resolveSave!: (definition: WorkflowDefinition) => void;
    const saveResult = new Promise<WorkflowDefinition>((resolve) => {
      resolveSave = resolve;
    });
    const client: WorkflowApplicationClient = {
      ...workflowClient(emptyDefinition()),
      saveNodeDraft: vi.fn(() => saveResult),
    };
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const canvas = await screen.findByLabelText('Workflow canvas');
    fireEvent.click(screen.getByRole('button', { name: 'Node' }));
    fireEvent.keyDown(canvas, { key: 'Enter' });
    const dialog = screen.getByRole('dialog', { name: 'Configure new node' });
    expect(within(dialog).getByLabelText('Harness name')).toBeDisabled();
    expect(within(dialog).getByRole('button', { name: 'Close node configuration' })).toBeDisabled();
    fireEvent.click(canvas);
    expect(screen.getByRole('dialog', { name: 'Configure new node' })).toBeVisible();

    await waitFor(() => expect(client.saveNodeDraft).toHaveBeenCalledOnce());
    const node = vi.mocked(client.saveNodeDraft).mock.calls[0][1];
    resolveSave(withNode(emptyDefinition(), node));
    await waitFor(() => expect(within(dialog).getByLabelText('Harness name')).toBeEnabled());
  });

  it('surfaces an unmount save failure once and recovers the last durable definition', async () => {
    const failure = new Error('Unable to persist Workflow draft.');
    const client: WorkflowApplicationClient = {
      ...workflowClient(emptyDefinition()),
      saveNodeDraft: vi.fn(async () => {
        throw failure;
      }),
    };
    const rendered = render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const canvas = await screen.findByLabelText('Workflow canvas');
    fireEvent.click(screen.getByRole('button', { name: 'Node' }));
    fireEvent.keyDown(canvas, { key: 'Enter' });
    await waitFor(() => expect(client.saveNodeDraft).toHaveBeenCalled());
    rendered.rerender(
      <WorkflowScreen client={client} workflowTypeId={null} onOpenWorkflowType={() => undefined} />,
    );

    fireEvent.click(screen.getByRole('tab', { name: 'Workflow types' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(failure.message);
    rendered.rerender(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );
    expect(await screen.findByLabelText('Workflow canvas')).toBeVisible();
    expect(screen.queryByRole('button', { name: /Configure/ })).toBeNull();
  });
});

function workflowClient(initial: WorkflowDefinition): WorkflowApplicationClient {
  let definition = initial;
  return {
    listWorkflowTypes: vi.fn(async () => [definition.workflowType]),
    createWorkflowType: vi.fn(async ({ name }) => {
      definition = {
        ...definition,
        workflowType: { ...definition.workflowType, name },
      };
      return definition;
    }),
    loadWorkflowType: vi.fn(async () => definition),
    saveNodeDraft: vi.fn(async (_workflowTypeId: string, node: WorkflowNodeConfig) => {
      definition = {
        ...definition,
        workflowType: { ...definition.workflowType, editedElementCount: 1 },
        nodes: [{ id: node.id, draft: node, live: null, hasUnpublishedChanges: true }],
      };
      return definition;
    }),
    activateChanges: vi.fn(async () => {
      const draft = definition.nodes[0]?.draft ?? null;
      definition = {
        ...definition,
        workflowType: {
          ...definition.workflowType,
          activeRecipeId: 'recipe-1',
          editedElementCount: 0,
        },
        nodes: draft ? [{ id: draft.id, draft, live: draft, hasUnpublishedChanges: false }] : [],
        activeRecipe: draft
          ? {
              id: 'recipe-1',
              workflowTypeId: definition.workflowType.id,
              ordinal: 1,
              createdAt: '2026-08-08T21:00:00.000Z',
              nodes: [draft],
              connections: [],
            }
          : null,
      };
      return definition;
    }),
  };
}

function emptyDefinition(): WorkflowDefinition {
  return {
    workflowType: {
      id: 'workflow-1',
      name: 'Workflow type',
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

function withNode(definition: WorkflowDefinition, node: WorkflowNodeConfig): WorkflowDefinition {
  return {
    ...definition,
    workflowType: { ...definition.workflowType, editedElementCount: 1 },
    nodes: [{ id: node.id, draft: node, live: null, hasUnpublishedChanges: true }],
  };
}
