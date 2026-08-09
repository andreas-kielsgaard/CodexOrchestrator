import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { vi } from 'vitest';
import type {
  WorkflowApplicationClient,
  WorkflowConnectionConfig,
  WorkflowDefinition,
  WorkflowElementRef,
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

  it('keeps the connection brush active and persists a complete turn-finished file mechanism', async () => {
    const client = workflowClient(definitionWithNodes());
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const brush = await screen.findByRole('button', { name: 'Connection' });
    fireEvent.click(brush);
    fireEvent.click(screen.getByRole('button', { name: 'Configure Sender' }));
    fireEvent.click(screen.getByRole('button', { name: 'Configure Receiver' }));

    const dialog = await screen.findByRole('dialog', { name: 'Configure Sender to Receiver' });
    expect(brush).toHaveAttribute('aria-pressed', 'true');
    expect(client.saveConnectionDraft).toHaveBeenCalledOnce();
    expect(within(dialog).queryByRole('button', { name: 'Activate connection' })).toBeNull();

    fireEvent.change(within(dialog).getByLabelText('Connecting mechanism'), {
      target: { value: 'turn_finished_expected_file' },
    });
    fireEvent.change(within(dialog).getByLabelText('Folder'), {
      target: { value: 'handoffs' },
    });
    fireEvent.change(within(dialog).getByLabelText('Filename pattern'), {
      target: { value: '*.md' },
    });
    fireEvent.change(within(dialog).getByLabelText('File description'), {
      target: { value: 'The sender handoff' },
    });
    fireEvent.change(within(dialog).getByLabelText('Fixed prompt'), {
      target: { value: 'Continue from this handoff.' },
    });

    await waitFor(() =>
      expect(vi.mocked(client.saveConnectionDraft).mock.calls.at(-1)?.[1]).toMatchObject({
        senderNodeId: 'sender',
        receiverNodeId: 'receiver',
        mechanism: {
          kind: 'turn_finished_expected_file',
          fileSelector: {
            kind: 'folder_filename_pattern',
            folder: 'handoffs',
            filenamePattern: '*.md',
          },
          descriptionText: 'The sender handoff',
          promptText: 'Continue from this handoff.',
          matchSelection: 'newest',
          initialCheck: 'once_immediately',
        },
      }),
    );
    fireEvent.click(await within(dialog).findByRole('button', { name: 'Activate connection' }));
    await waitFor(() =>
      expect(client.activateChanges).toHaveBeenCalledWith('workflow-1', [
        {
          kind: 'connection',
          id: vi.mocked(client.saveConnectionDraft).mock.calls[0]![1].id,
        },
      ]),
    );
  });

  it('creates parallel connections by drag and click, then opens their shared path list and preview', async () => {
    const client = workflowClient(definitionWithNodes());
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const brush = await screen.findByRole('button', { name: 'Connection' });
    fireEvent.click(brush);
    const sender = screen.getByRole('button', { name: 'Configure Sender' });
    const receiver = screen.getByRole('button', { name: 'Configure Receiver' });
    fireEvent.pointerDown(sender, { button: 0 });
    fireEvent.pointerUp(receiver);
    fireEvent.click(screen.getByLabelText('Workflow canvas'));
    let dialog = await screen.findByRole('dialog', { name: 'Configure Sender to Receiver' });
    await waitFor(() => expect(within(dialog).getByLabelText('Connection name')).toBeEnabled());
    fireEvent.click(within(dialog).getByRole('button', { name: 'Close connection configuration' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());

    fireEvent.click(sender);
    fireEvent.click(receiver);
    dialog = await screen.findByRole('dialog', { name: 'Configure Sender to Receiver' });
    await waitFor(() => expect(within(dialog).getByLabelText('Connection name')).toBeEnabled());
    fireEvent.click(within(dialog).getByRole('button', { name: 'Close connection configuration' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(brush).toHaveAttribute('aria-pressed', 'true');

    const bundle = screen.getByRole('button', { name: /2 connections from Sender to Receiver/ });
    fireEvent.click(bundle);
    const list = screen.getByRole('dialog', { name: /2 connections from Sender to Receiver/ });
    const entries = within(list).getAllByRole('button', { name: /Sender to Receiver/ });
    expect(entries).toHaveLength(2);
    fireEvent.mouseEnter(entries[0]!);
    expect(bundle).toHaveClass('is-highlighted');
    fireEvent.click(entries[0]!);

    const preview = screen.getByRole('dialog', { name: 'Preview Sender to Receiver' });
    expect(within(preview).getByText('Connecting mechanism not configured')).toBeVisible();
    fireEvent.click(within(preview).getByRole('button', { name: 'Open connection configuration' }));
    expect(screen.getByRole('dialog', { name: 'Configure Sender to Receiver' })).toBeVisible();
  });

  it('reopens a durable connection draft after leaving the workflow editor', async () => {
    const client = workflowClient(definitionWithNodes());
    const rendered = render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Connection' }));
    fireEvent.click(screen.getByRole('button', { name: 'Configure Sender' }));
    fireEvent.click(screen.getByRole('button', { name: 'Configure Receiver' }));
    const dialog = await screen.findByRole('dialog', { name: 'Configure Sender to Receiver' });
    fireEvent.change(within(dialog).getByLabelText('Connecting mechanism'), {
      target: { value: 'turn_finished_expected_file' },
    });
    fireEvent.change(within(dialog).getByLabelText('File locator'), {
      target: { value: 'folder_output_regex' },
    });
    fireEvent.change(within(dialog).getByLabelText('Folder'), {
      target: { value: 'handoffs' },
    });
    fireEvent.change(within(dialog).getByLabelText('Sender output regex'), {
      target: { value: 'handoff: (.+\\.md)' },
    });
    rendered.rerender(
      <WorkflowScreen client={client} workflowTypeId={null} onOpenWorkflowType={() => undefined} />,
    );
    fireEvent.click(await screen.findByRole('tab', { name: 'Workflow types' }));
    rendered.rerender(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const edge = await screen.findByRole('button', {
      name: '1 connection from Sender to Receiver',
    });
    fireEvent.click(edge);
    const list = screen.getByRole('dialog', { name: '1 connection from Sender to Receiver' });
    fireEvent.click(within(list).getByRole('button', { name: /Sender to Receiver/ }));
    const preview = screen.getByRole('dialog', { name: 'Preview Sender to Receiver' });
    expect(within(preview).getByText('Turn finished · expected file')).toBeVisible();
  });

  it('shows a node outgoing-connection list and bulk activates an explicit selection', async () => {
    const definition = definitionWithNodes();
    const first = connection('connection-a');
    const second = connection('connection-b');
    const client = workflowClient({
      ...definition,
      workflowType: { ...definition.workflowType, editedElementCount: 2 },
      connections: [
        { id: first.id, draft: first, live: null, hasUnpublishedChanges: true },
        { id: second.id, draft: second, live: null, hasUnpublishedChanges: true },
      ],
    });
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Configure Sender' }));
    expect(screen.getByRole('dialog', { name: 'Connections from Sender' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Close connection list' }));

    fireEvent.click(screen.getByRole('button', { name: 'Activate edits (2)' }));
    const bulk = screen.getByRole('dialog', { name: 'Activate edited elements' });
    fireEvent.click(within(bulk).getByRole('checkbox', { name: 'Select all edited elements' }));
    fireEvent.click(within(bulk).getByRole('button', { name: 'Activate selected (2)' }));
    await waitFor(() =>
      expect(client.activateChanges).toHaveBeenCalledWith('workflow-1', [
        { kind: 'connection', id: 'connection-a' },
        { kind: 'connection', id: 'connection-b' },
      ]),
    );
  });

  it('keeps a dangling incoming connection reachable and reconnects it by dragging to a node', async () => {
    const definition = definitionWithNodes();
    const dangling: WorkflowConnectionConfig = {
      ...connection('dangling'),
      receiverNodeId: null,
    };
    const client = workflowClient({
      ...definition,
      workflowType: { ...definition.workflowType, editedElementCount: 1 },
      connections: [{ id: dangling.id, draft: dangling, live: null, hasUnpublishedChanges: true }],
    });
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const edge = await screen.findByRole('button', {
      name: '1 connection from Sender to a dangling endpoint',
    });
    fireEvent.click(screen.getByRole('button', { name: 'Connection' }));
    fireEvent.pointerDown(edge);
    fireEvent.pointerUp(screen.getByRole('button', { name: 'Configure Receiver' }));
    fireEvent.click(screen.getByLabelText('Workflow canvas'));
    await waitFor(() =>
      expect(vi.mocked(client.saveConnectionDraft).mock.calls.at(-1)?.[1]).toMatchObject({
        id: 'dangling',
        receiverNodeId: 'receiver',
      }),
    );
    expect(screen.getByRole('dialog', { name: 'Configure Connection dangling' })).toBeVisible();
  });

  it('cancels a dangling-edge reattachment when the pointer is released off a node', async () => {
    const definition = definitionWithNodes();
    const dangling: WorkflowConnectionConfig = {
      ...connection('dangling'),
      receiverNodeId: null,
    };
    const client = workflowClient({
      ...definition,
      connections: [{ id: dangling.id, draft: dangling, live: null, hasUnpublishedChanges: true }],
    });
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Connection' }));
    fireEvent.pointerDown(
      screen.getByRole('button', { name: '1 connection from Sender to a dangling endpoint' }),
    );
    fireEvent.pointerUp(screen.getByLabelText('Workflow canvas'));
    fireEvent.pointerUp(screen.getByRole('button', { name: 'Configure Receiver' }));
    expect(client.saveConnectionDraft).not.toHaveBeenCalled();
  });

  it('edits and removes one inherited field override without detaching the Role', async () => {
    const role = workflowRole('role-reviewer', 'Reviewer', {
      ...emptyHarness('Review Harness'),
      instructions: 'Inherited instructions',
    });
    const node: WorkflowNodeConfig = {
      id: 'review',
      name: 'Review',
      harnessName: 'Review Harness',
      roleName: 'Reviewer',
      positionX: 100,
      positionY: 100,
      isStartingPoint: true,
      harness: { kind: 'role', roleId: role.id, overrides: {} },
    };
    const client = workflowClient({
      ...emptyDefinition(),
      nodes: [
        {
          id: node.id,
          draft: node,
          live: null,
          hasUnpublishedChanges: true,
          draftEffectiveHarness: role.harness,
          liveEffectiveHarness: null,
        },
      ],
    });
    vi.mocked(client.listRoles).mockResolvedValue([role]);
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Configure Review' }));
    const dialog = screen.getByRole('dialog', { name: 'Configure Review' });
    const instructions = within(dialog).getByLabelText('Instructions');
    expect(instructions).toHaveValue('Inherited instructions');
    fireEvent.change(instructions, { target: { value: 'Security-only instructions' } });
    await waitFor(() =>
      expect(vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1].harness).toMatchObject({
        kind: 'role',
        roleId: role.id,
        overrides: { instructions: 'Security-only instructions' },
      }),
    );
    const instructionsField = within(dialog).getByText('Instructions').closest('details')!;
    expect(instructionsField).toHaveClass('is-overridden');
    fireEvent.click(within(instructionsField).getByRole('button', { name: 'Use inherited value' }));
    await waitFor(() =>
      expect(
        (
          vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1].harness as {
            overrides: { instructions?: string };
          }
        ).overrides.instructions,
      ).toBeUndefined(),
    );
    fireEvent.click(within(dialog).getByRole('radio', { name: 'From scratch' }));
    await waitFor(() => expect(client.detachNodeRole).toHaveBeenCalledWith('workflow-1', 'review'));
    fireEvent.click(within(dialog).getByRole('button', { name: 'Edit saved Role' }));
    expect(
      within(screen.getByRole('dialog', { name: 'Saved Roles' })).getByLabelText('Role name'),
    ).toHaveValue('Reviewer');
  });

  it('keeps the copy brush active and copies only the selected node Harness as non-starting', async () => {
    const client = workflowClient(definitionWithNodes());
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );
    const canvas = await screen.findByLabelText('Workflow canvas');
    const brush = screen.getByRole('button', { name: 'Copy' });
    fireEvent.click(brush);
    fireEvent.click(screen.getByRole('button', { name: 'Configure Sender' }));
    fireEvent.click(canvas, { clientX: 300, clientY: 300 });
    expect(await screen.findByRole('dialog', { name: 'Configure Sender copy' })).toBeVisible();
    const copied = vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1];
    expect(copied).toMatchObject({
      name: 'Sender copy',
      harnessName: 'Sender Harness',
      isStartingPoint: false,
      harness: { kind: 'standalone', config: { harnessName: 'Sender Harness' } },
    });
    expect(brush).toHaveAttribute('aria-pressed', 'true');
  });

  it('copies a Role binding with its sparse overrides rather than materializing it', async () => {
    const role = workflowRole('role-reviewer', 'Reviewer', emptyHarness('Review Harness'));
    const node: WorkflowNodeConfig = {
      id: 'review',
      name: 'Review',
      harnessName: 'Review Harness',
      roleName: 'Reviewer',
      positionX: 100,
      positionY: 100,
      isStartingPoint: true,
      harness: {
        kind: 'role',
        roleId: role.id,
        overrides: { instructions: 'Local review scope' },
      },
    };
    const client = workflowClient({
      ...emptyDefinition(),
      nodes: [
        {
          id: node.id,
          draft: node,
          live: null,
          hasUnpublishedChanges: true,
          draftEffectiveHarness: { ...role.harness, instructions: 'Local review scope' },
        },
      ],
    });
    vi.mocked(client.listRoles).mockResolvedValue([role]);
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );
    const canvas = await screen.findByLabelText('Workflow canvas');
    fireEvent.click(screen.getByRole('button', { name: 'Copy' }));
    fireEvent.click(screen.getByRole('button', { name: 'Configure Review' }));
    fireEvent.click(canvas, { clientX: 350, clientY: 280 });
    await waitFor(() =>
      expect(vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1]).toMatchObject({
        isStartingPoint: false,
        harness: {
          kind: 'role',
          roleId: role.id,
          overrides: { instructions: 'Local review scope' },
        },
      }),
    );
  });

  it('creates a saved Role from the editor catalog with a keyboard-addressable form', async () => {
    const client = workflowClient(emptyDefinition());
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
      />,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Node' }));
    fireEvent.click(screen.getByRole('button', { name: 'Roles' }));
    const catalog = screen.getByRole('dialog', { name: 'Saved Roles' });
    fireEvent.keyDown(within(catalog).getByLabelText('Role name'), { key: ' ' });
    expect(client.saveNodeDraft).not.toHaveBeenCalled();
    fireEvent.change(within(catalog).getByLabelText('Role name'), {
      target: { value: 'Security reviewer' },
    });
    fireEvent.change(within(catalog).getByLabelText('Harness name'), {
      target: { value: 'Security Harness' },
    });
    fireEvent.change(within(catalog).getByLabelText('Role identity'), {
      target: { value: 'Review security boundaries.' },
    });
    fireEvent.click(within(catalog).getByRole('button', { name: 'Create Role' }));
    await waitFor(() =>
      expect(client.createRole).toHaveBeenCalledWith({
        name: 'Security reviewer',
        harness: expect.objectContaining({
          harnessName: 'Security Harness',
          roleIdentity: 'Review security boundaries.',
        }),
      }),
    );
  });
});

function workflowClient(initial: WorkflowDefinition): WorkflowApplicationClient {
  let definition = initial;
  return {
    listWorkflowTypes: vi.fn(async () => [definition.workflowType]),
    listRoles: vi.fn(async () => []),
    createRole: vi.fn(async ({ name, harness }) => workflowRole('role-created', name, harness)),
    updateRole: vi.fn(async ({ roleId, name, harness }) => workflowRole(roleId, name, harness)),
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
        workflowType: {
          ...definition.workflowType,
          editedElementCount: definition.workflowType.editedElementCount + 1,
        },
        nodes: [
          ...definition.nodes.filter((element) => element.id !== node.id),
          {
            id: node.id,
            draft: node,
            live: definition.nodes.find((element) => element.id === node.id)?.live ?? null,
            hasUnpublishedChanges: true,
          },
        ],
      };
      return definition;
    }),
    deleteNodeDraft: vi.fn(async (_workflowTypeId: string, nodeId: string) => {
      definition = {
        ...definition,
        nodes: definition.nodes.flatMap((element) =>
          element.id !== nodeId
            ? [element]
            : element.live
              ? [{ ...element, draft: null, hasUnpublishedChanges: true }]
              : [],
        ),
      };
      return definition;
    }),
    detachNodeRole: vi.fn(async () => definition),
    saveNodeAsRole: vi.fn(async () => definition),
    saveConnectionDraft: vi.fn(
      async (_workflowTypeId: string, connection: WorkflowConnectionConfig) => {
        definition = {
          ...definition,
          workflowType: {
            ...definition.workflowType,
            editedElementCount: definition.workflowType.editedElementCount + 1,
          },
          connections: [
            ...definition.connections.filter((element) => element.id !== connection.id),
            {
              id: connection.id,
              draft: connection,
              live:
                definition.connections.find((element) => element.id === connection.id)?.live ??
                null,
              hasUnpublishedChanges: true,
            },
          ],
        };
        return definition;
      },
    ),
    deleteConnectionDraft: vi.fn(async (_workflowTypeId: string, connectionId: string) => {
      definition = {
        ...definition,
        connections: definition.connections.flatMap((element) =>
          element.id !== connectionId
            ? [element]
            : element.live
              ? [{ ...element, draft: null, hasUnpublishedChanges: true }]
              : [],
        ),
      };
      return definition;
    }),
    activateChanges: vi.fn(
      async (_workflowTypeId: string, elements: readonly WorkflowElementRef[]) => {
        const selected = new Set(elements.map((element) => `${element.kind}:${element.id}`));
        const nodes = definition.nodes
          .map((element) =>
            selected.has(`node:${element.id}`)
              ? { ...element, live: element.draft, hasUnpublishedChanges: false }
              : element,
          )
          .filter((element) => element.draft || element.live);
        const connections = definition.connections
          .map((element) =>
            selected.has(`connection:${element.id}`)
              ? { ...element, live: element.draft, hasUnpublishedChanges: false }
              : element,
          )
          .filter((element) => element.draft || element.live);
        definition = {
          ...definition,
          workflowType: {
            ...definition.workflowType,
            activeRecipeId: 'recipe-1',
            editedElementCount: 0,
          },
          nodes,
          connections,
          activeRecipe: {
            id: 'recipe-1',
            workflowTypeId: definition.workflowType.id,
            ordinal: 1,
            createdAt: '2026-08-08T21:00:00.000Z',
            nodes: nodes.flatMap((element) =>
              element.live ? [materializeNode(element.live)] : [],
            ),
            connections: connections.flatMap((element) => (element.live ? [element.live] : [])),
          },
        };
        return definition;
      },
    ),
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

function definitionWithNodes(): WorkflowDefinition {
  const sender: WorkflowNodeConfig = {
    id: 'sender',
    name: 'Sender',
    harnessName: 'Sender Harness',
    roleName: null,
    positionX: 80,
    positionY: 100,
    isStartingPoint: true,
  };
  const receiver: WorkflowNodeConfig = {
    id: 'receiver',
    name: 'Receiver',
    harnessName: 'Receiver Harness',
    roleName: null,
    positionX: 480,
    positionY: 100,
    isStartingPoint: false,
  };
  return {
    ...emptyDefinition(),
    workflowType: {
      ...emptyDefinition().workflowType,
      activeRecipeId: 'recipe-existing',
    },
    nodes: [
      { id: sender.id, draft: sender, live: sender, hasUnpublishedChanges: false },
      { id: receiver.id, draft: receiver, live: receiver, hasUnpublishedChanges: false },
    ],
    activeRecipe: {
      id: 'recipe-existing',
      workflowTypeId: 'workflow-1',
      ordinal: 1,
      createdAt: '2026-08-08T21:00:00.000Z',
      nodes: [materializeNode(sender), materializeNode(receiver)],
      connections: [],
    },
  };
}

function materializeNode(node: WorkflowNodeConfig) {
  return {
    id: node.id,
    name: node.name,
    positionX: node.positionX,
    positionY: node.positionY,
    isStartingPoint: node.isStartingPoint,
    harness: emptyHarness(node.harnessName),
  };
}

function emptyHarness(harnessName = '') {
  return {
    harnessName,
    roleIdentity: '',
    instructions: '',
    skills: [],
    mcpServers: [],
    hooks: [],
    runtime: { provider: '', model: '', reasoningEffort: '' },
  };
}

function workflowRole(id: string, name: string, harness = emptyHarness(name)) {
  return {
    id,
    name,
    harness,
    createdAt: '2026-08-08T20:00:00.000Z',
    updatedAt: '2026-08-08T20:00:00.000Z',
  };
}

function connection(id: string): WorkflowConnectionConfig {
  return {
    id,
    name: `Connection ${id}`,
    senderNodeId: 'sender',
    receiverNodeId: 'receiver',
    mechanism: {
      kind: 'turn_finished_expected_file',
      fileSelector: {
        kind: 'folder_output_regex',
        folder: 'handoffs',
        outputRegex: 'path: (.+\\.md)',
      },
      descriptionText: 'The sender handoff',
      promptText: 'Continue from this handoff.',
      matchSelection: 'newest',
      initialCheck: 'once_immediately',
    },
  };
}
