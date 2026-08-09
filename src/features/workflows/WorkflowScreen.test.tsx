import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { vi } from 'vitest';
import type {
  WorkflowApplicationClient,
  WorkflowConnectionConfig,
  WorkflowDefinition,
  WorkflowElementRef,
  WorkflowInstance,
  WorkflowNodeConfig,
} from '../../application/workflows';
import {
  createRecordedAgentSessionClient,
  createRecordedAgentSessionStore,
} from '../../dev/agentSessions';
import { recordedAgentSessionDetails } from '../../dev/orchestrationSection/recordedPresentationAdjunct';
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

  it('launches an activated type with the exact starting prompt and opens its instance', async () => {
    const definition = definitionWithNodes();
    const instance = workflowInstance();
    const client: WorkflowApplicationClient = {
      ...workflowClient(definition),
      launchWorkflowInstance: vi.fn(async () => instance),
    };
    const onOpenInstance = vi.fn();
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        onOpenWorkflowType={() => undefined}
        onOpenWorkflowInstance={onOpenInstance}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Start workflow' }));
    const dialog = screen.getByRole('dialog', { name: 'Start Workflow type' });
    fireEvent.change(within(dialog).getByLabelText('Starting prompt'), {
      target: { value: 'Review the current architecture.' },
    });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Start workflow' }));

    await waitFor(() =>
      expect(client.launchWorkflowInstance).toHaveBeenCalledWith({
        workflowTypeId: 'workflow-1',
        name: null,
        startingPrompt: 'Review the current architecture.',
      }),
    );
    expect(onOpenInstance).toHaveBeenCalledWith('instance-1');
  });

  it('shows the immutable launch recipe, Session activity, and launch acceptance separately', async () => {
    const instance = workflowInstance();
    const client: WorkflowApplicationClient = {
      ...workflowClient(definitionWithNodes()),
      loadWorkflowInstance: vi.fn(async () => instance),
    };
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId={null}
        workflowInstanceId="instance-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    expect(
      await screen.findByRole('main', { name: 'Workflow instance Architecture review' }),
    ).toBeVisible();
    expect(screen.getByLabelText('Workflow instance graph')).toBeVisible();
    expect(screen.getByText('Sender Harness')).toBeVisible();
    expect(screen.getByText('1 Session · 1 active · 0 idle')).toBeVisible();
    expect(screen.getAllByText('Runtime launch accepted').length).toBeGreaterThan(0);
    expect(screen.getByText('Fresh · no inheritance · no compression')).toBeVisible();
  });

  it('keeps node configuration and newest-first Agent Sessions inside a dismissible popup', async () => {
    const oldestDetails = recordedAgentSessionDetails[0]!;
    const newestDetails = recordedAgentSessionDetails[1]!;
    const misleadingSummary = 'Submitted prompt is not the latest agent turn.';
    const instance: WorkflowInstance = {
      ...workflowInstance(),
      summary: {
        ...workflowInstance().summary,
        sessionCount: 2,
        activeSessionCount: 1,
        idleSessionCount: 1,
      },
      sessions: [
        {
          nodeId: 'sender',
          sessionId: oldestDetails.session.id,
          title: 'Older node Session',
          activity: 'idle',
          latestTurnSummary: null,
          associatedAt: '2026-08-09T01:00:01.000Z',
        },
        {
          nodeId: 'sender',
          sessionId: newestDetails.session.id,
          title: 'Newest node Session',
          activity: 'active',
          latestTurnSummary: misleadingSummary,
          associatedAt: '2026-08-09T02:00:01.000Z',
        },
      ],
    };
    const client: WorkflowApplicationClient = {
      ...workflowClient(definitionWithNodes()),
      loadWorkflowInstance: vi.fn(async () => instance),
    };
    const agentSessionClient = createRecordedAgentSessionClient({
      store: createRecordedAgentSessionStore([oldestDetails, newestDetails]),
    });
    render(
      <WorkflowScreen
        client={client}
        agentSessionClient={agentSessionClient}
        workflowTypeId={null}
        workflowInstanceId="instance-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const nodeTrigger = await screen.findByRole('button', {
      name: 'Open Sender Harness node Sender',
    });
    expect(screen.queryByText(misleadingSummary)).toBeNull();
    fireEvent.click(nodeTrigger);
    const dialog = screen.getByRole('dialog', { name: 'Sender Harness node details' });
    expect(dialog).toHaveFocus();
    expect(within(dialog).getByRole('heading', { name: 'Sender Harness' })).toBeVisible();
    expect(within(dialog).getByText('Sender')).toBeVisible();
    expect(within(dialog).queryByText(misleadingSummary)).toBeNull();
    const activity = within(dialog).getByLabelText('Node Session activity');
    expect(within(activity).getByText('2 Sessions')).toBeVisible();
    expect(within(activity).getByText('1 active')).toBeVisible();
    expect(within(activity).getByText('1 idle')).toBeVisible();

    const agentSessionsAction = within(dialog).getByRole('button', { name: 'Agent Sessions' });
    fireEvent.keyDown(dialog, { key: 'Tab', shiftKey: true });
    expect(agentSessionsAction).toHaveFocus();
    fireEvent.keyDown(agentSessionsAction, { key: 'Tab' });
    expect(within(dialog).getByRole('button', { name: 'Close node details' })).toHaveFocus();

    fireEvent.click(agentSessionsAction);
    expect(screen.getByRole('button', { name: 'Return to node' })).toHaveFocus();
    expect(screen.queryByText(misleadingSummary)).toBeNull();
    const sessionList = screen.getByRole('navigation', { name: 'Sender Agent Sessions' });
    const sessionButtons = within(sessionList).getAllByRole('button');
    expect(sessionButtons.map((button) => button.textContent)).toEqual([
      expect.stringContaining('Newest node Session'),
      expect.stringContaining('Older node Session'),
    ]);
    expect(sessionButtons[0]).toHaveAttribute('aria-pressed', 'true');
    expect(
      await within(screen.getByLabelText('Selected Agent Session')).findByRole('heading', {
        name: newestDetails.session.title,
      }),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'View Agent Session' }).parentElement).toHaveClass(
      'agent-session-header__actions',
    );

    fireEvent.click(screen.getByRole('button', { name: 'View Agent Session' }));
    expect(screen.getByRole('button', { name: 'Return to Session list' })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Return to Session list' }));
    expect(screen.getByRole('button', { name: 'Return to node' })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Return to node' }));
    expect(screen.getByRole('button', { name: 'Agent Sessions' })).toBeVisible();

    fireEvent.click(screen.getByLabelText('Workflow instance graph'));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(nodeTrigger).toHaveFocus();

    fireEvent.click(nodeTrigger);
    const reopenedDialog = screen.getByRole('dialog', { name: 'Sender Harness node details' });
    fireEvent.keyDown(reopenedDialog, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(nodeTrigger).toHaveFocus();
  });

  it('bundles instance connections and inspects exact durable activation endpoints', async () => {
    const sourceDetails = recordedAgentSessionDetails[0]!;
    const targetDetails = recordedAgentSessionDetails[1]!;
    const sourceInvocation = sourceDetails.invocations[0]!.invocation.id;
    const targetInvocation = targetDetails.invocations[0]!.invocation.id;
    const firstConnection = connection('edge-a');
    const secondConnection = connection('edge-b');
    const base = workflowInstance();
    const instance: WorkflowInstance = {
      ...base,
      recipe: { ...base.recipe, connections: [firstConnection, secondConnection] },
      connectionActivations: [
        {
          id: 'activation-newest',
          recipeId: base.recipe.id,
          connectionId: firstConnection.id,
          senderNodeId: 'sender',
          receiverNodeId: 'receiver',
          sourceSessionId: sourceDetails.session.id,
          sourceInvocationId: sourceInvocation,
          targetSessionId: targetDetails.session.id,
          targetInvocationId: targetInvocation,
          deliveryKind: 'direct_prompt_runtime_v1',
          sessionMode: 'fresh',
          contextInheritance: 'none',
          compression: 'none',
          resolvedFilePath: 'handoffs/newest.md',
          status: 'launch_accepted',
          requestedAt: '2026-08-09T03:00:00.000Z',
          resolvedAt: '2026-08-09T03:00:01.000Z',
          associatedAt: '2026-08-09T03:00:02.000Z',
          launchRequestedAt: '2026-08-09T03:00:03.000Z',
          launchAcceptedAt: '2026-08-09T03:00:04.000Z',
          failedAt: null,
        },
        {
          id: 'activation-older',
          recipeId: base.recipe.id,
          connectionId: firstConnection.id,
          senderNodeId: 'sender',
          receiverNodeId: 'receiver',
          sourceSessionId: targetDetails.session.id,
          sourceInvocationId: targetInvocation,
          targetSessionId: null,
          targetInvocationId: null,
          deliveryKind: 'direct_prompt_runtime_v1',
          sessionMode: null,
          contextInheritance: 'none',
          compression: 'none',
          resolvedFilePath: null,
          status: 'failed',
          requestedAt: '2026-08-09T02:00:00.000Z',
          resolvedAt: null,
          associatedAt: null,
          launchRequestedAt: null,
          launchAcceptedAt: null,
          failedAt: '2026-08-09T02:00:01.000Z',
        },
      ],
    };
    const client: WorkflowApplicationClient = {
      ...workflowClient(definitionWithNodes()),
      loadWorkflowInstance: vi.fn(async () => instance),
    };
    const agentSessionClient = createRecordedAgentSessionClient({
      store: createRecordedAgentSessionStore([sourceDetails, targetDetails]),
    });
    render(
      <WorkflowScreen
        client={client}
        agentSessionClient={agentSessionClient}
        workflowTypeId={null}
        workflowInstanceId="instance-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    const edge = await screen.findByRole('button', {
      name: '2 connections from Sender to Receiver',
    });
    edge.focus();
    fireEvent.keyDown(edge, { key: 'Enter' });
    const list = screen.getByRole('dialog', { name: '2 connections from Sender to Receiver' });
    const firstEntry = within(list).getByRole('button', { name: /Connection edge-a/ });
    expect(within(list).getByRole('button', { name: /Connection edge-b/ })).toBeVisible();
    expect(firstEntry).toHaveFocus();
    expect(edge).toHaveClass('is-highlighted');
    fireEvent.blur(firstEntry);
    expect(edge).not.toHaveClass('is-highlighted');
    fireEvent.mouseEnter(firstEntry);
    expect(edge).toHaveClass('is-highlighted');
    fireEvent.click(firstEntry);

    let detail = screen.getByRole('dialog', { name: 'Connection edge-a activity' });
    expect(within(detail).getByText('Activation 1 of 2')).toBeVisible();
    expect(within(detail).getByText('handoffs/newest.md')).toBeVisible();
    expect(within(detail).getByText('Launch accepted')).toBeVisible();

    fireEvent.click(within(detail).getByRole('button', { name: 'Show older activation' }));
    expect(within(detail).getByText('Activation 2 of 2')).toBeVisible();
    expect(within(detail).getByText('Not resolved')).toBeVisible();
    expect(within(detail).getByRole('button', { name: 'Receiving Agent Session' })).toBeDisabled();
    fireEvent.click(within(detail).getByRole('button', { name: 'All activations' }));
    const all = within(detail).getAllByRole('button', { name: /Launch accepted/ });
    all[0]!.focus();
    fireEvent.click(all[0]!);
    expect(within(detail).getByText('Activation 1 of 2')).toBeVisible();
    expect(all[0]).toHaveFocus();

    fireEvent.click(within(detail).getByRole('button', { name: 'Sending Agent Session' }));
    expect(screen.getByRole('button', { name: 'Return to activation' })).toHaveFocus();
    const sourceInspector = await screen.findByLabelText('Supporting Agent Session passage');
    expect(sourceInspector).toHaveAttribute('data-session-id', sourceDetails.session.id);
    expect(sourceInspector).toHaveAttribute('data-invocation-id', sourceInvocation);
    fireEvent.click(screen.getByRole('button', { name: 'Return to activation' }));
    detail = screen.getByRole('dialog', { name: 'Connection edge-a activity' });
    await waitFor(() =>
      expect(within(detail).getByRole('button', { name: 'Sending Agent Session' })).toHaveFocus(),
    );

    fireEvent.click(within(detail).getByRole('button', { name: 'Receiving Agent Session' }));
    expect(screen.getByRole('button', { name: 'Return to activation' })).toHaveFocus();
    const targetInspector = await screen.findByLabelText('Supporting Agent Session passage');
    expect(targetInspector).toHaveAttribute('data-session-id', targetDetails.session.id);
    expect(targetInspector).toHaveAttribute('data-invocation-id', targetInvocation);
    fireEvent.click(screen.getByRole('button', { name: 'Return to activation' }));
    detail = screen.getByRole('dialog', { name: 'Connection edge-a activity' });
    await waitFor(() =>
      expect(within(detail).getByRole('button', { name: 'Receiving Agent Session' })).toHaveFocus(),
    );
    fireEvent.keyDown(detail, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(edge).toHaveFocus();

    fireEvent.click(edge);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: /edge-b/ }));
    expect(
      screen.getByText('This connection has not fired in this workflow instance.'),
    ).toBeVisible();
    fireEvent.click(screen.getByLabelText('Workflow instance graph'));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(edge).toHaveFocus();
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
    listWorkflowInstances: vi.fn(async () => []),
    launchWorkflowInstance: vi.fn(async () => {
      throw new Error('not configured');
    }),
    loadWorkflowInstance: vi.fn(async () => {
      throw new Error('not configured');
    }),
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

function workflowInstance(): WorkflowInstance {
  const definition = definitionWithNodes();
  return {
    summary: {
      id: 'instance-1',
      workflowTypeId: 'workflow-1',
      workflowTypeName: 'Workflow type',
      recipeId: 'recipe-existing',
      name: 'Architecture review',
      sessionCount: 1,
      activeSessionCount: 1,
      idleSessionCount: 0,
      launchStatus: 'launch_accepted',
      createdAt: '2026-08-09T01:00:00.000Z',
    },
    startingPrompt: 'Review the current architecture.',
    workingDirectory: 'C:\\workflow-instances\\instance-1',
    recipe: {
      ...definition.activeRecipe!,
      connections: [connection('launch-edge')],
    },
    sessions: [
      {
        nodeId: 'sender',
        sessionId: 'session-1',
        title: 'Architecture review',
        activity: 'active',
        latestTurnSummary: 'Review the current architecture.',
        associatedAt: '2026-08-09T01:00:01.000Z',
      },
    ],
    launchActivation: {
      id: 'activation-1',
      sourceKind: 'human',
      targetNodeId: 'sender',
      targetSessionId: 'session-1',
      targetInvocationId: 'invocation-1',
      deliveryKind: 'direct_prompt_runtime_v1',
      sessionMode: 'fresh',
      contextInheritance: 'none',
      compression: 'none',
      status: 'launch_accepted',
      requestedAt: '2026-08-09T01:00:00.000Z',
      associatedAt: '2026-08-09T01:00:01.000Z',
      launchRequestedAt: '2026-08-09T01:00:02.000Z',
      launchAcceptedAt: '2026-08-09T01:00:03.000Z',
      failedAt: null,
      failureStage: null,
      failureReason: null,
    },
    connectionActivations: [],
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
