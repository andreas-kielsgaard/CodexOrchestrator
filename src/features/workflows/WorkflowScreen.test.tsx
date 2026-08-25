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
import type {
  RepoBranchWorktreeTargetSelectorProps,
  ResolvedRepoBranchWorktreeTarget,
} from '../../application/worktreeTargets';
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

    expect(await screen.findByText('No Workflow instances')).toBeVisible();
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

  it('derives Ready to begin for an instance without Sessions', async () => {
    const client = workflowClient(definitionWithNodes());
    const summary = workflowInstance().summary;
    client.listWorkflowInstances = vi.fn(async () => [
      { ...summary, sessionCount: 0, activeSessionCount: 0, idleSessionCount: 0 },
    ]);

    render(
      <WorkflowScreen client={client} workflowTypeId={null} onOpenWorkflowType={() => undefined} />,
    );

    expect(await screen.findByText('Ready to begin')).toBeVisible();
    expect(screen.queryByText('0 Sessions')).toBeNull();
  });

  it('creates a named instance for the exact selected target and opens it', async () => {
    const definition = definitionWithNodes();
    const instance = workflowInstance();
    const client: WorkflowApplicationClient = {
      ...workflowClient(definition),
      createWorkflowInstance: vi.fn(async () => instance),
    };
    const onOpenInstance = vi.fn();
    render(
      <WorkflowScreen
        client={client}
        workflowTypeId="workflow-1"
        targetSelector={TestTargetSelector}
        onOpenWorkflowType={() => undefined}
        onOpenWorkflowInstance={onOpenInstance}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Create instance' }));
    const dialog = screen.getByRole('dialog', { name: 'Create Workflow instance' });
    fireEvent.change(within(dialog).getByLabelText('Workflow type'), {
      target: { value: 'workflow-1' },
    });
    fireEvent.change(within(dialog).getByLabelText('Instance name'), {
      target: { value: '  Architecture review  ' },
    });
    fireEvent.click(within(dialog).getByLabelText('Repository and branch'));
    fireEvent.click(within(dialog).getByRole('button', { name: 'Create Workflow instance' }));

    await waitFor(() =>
      expect(client.createWorkflowInstance).toHaveBeenCalledWith({
        workflowTypeId: 'workflow-1',
        name: 'Architecture review',
        target: workflowTarget,
      }),
    );
    expect(onOpenInstance).toHaveBeenCalledWith('instance-1');
  });

  it('shows the immutable recipe, registered target, and Session activity separately', async () => {
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
    expect(screen.getByText('Review repo')).toBeVisible();
    expect(screen.getByText('C:\\worktrees\\review')).toBeVisible();
  });

  it('opens the start node before a Session exists and sends the first message through its Workflow binding', async () => {
    const recordedDetails = recordedAgentSessionDetails[0]!;
    const base = workflowInstance();
    const emptyInstance: WorkflowInstance = {
      ...base,
      summary: {
        ...base.summary,
        sessionCount: 0,
        activeSessionCount: 0,
        idleSessionCount: 0,
      },
      sessions: [],
    };
    const associatedInstance: WorkflowInstance = {
      ...emptyInstance,
      summary: { ...emptyInstance.summary, sessionCount: 1, activeSessionCount: 1 },
      sessions: [
        {
          nodeId: 'sender',
          sessionId: recordedDetails.session.id,
          title: recordedDetails.session.title,
          activity: 'active',
          latestTurnSummary: null,
          associatedAt: '2026-08-25T12:00:00.000Z',
        },
      ],
    };
    const firstInvocation = recordedDetails.invocations[0]!.invocation;
    const client: WorkflowApplicationClient = {
      ...workflowClient(definitionWithNodes()),
      loadWorkflowInstance: vi
        .fn()
        .mockResolvedValueOnce(emptyInstance)
        .mockResolvedValue(associatedInstance),
      sendWorkflowNodeMessage: vi.fn(async () => ({
        sessionId: recordedDetails.session.id,
        invocationId: firstInvocation.id,
      })),
    };
    const agentSessionClient = createRecordedAgentSessionClient({
      store: createRecordedAgentSessionStore([recordedDetails]),
    });
    const ordinarySend = vi.spyOn(agentSessionClient, 'sendMessage');
    render(
      <WorkflowScreen
        client={client}
        agentSessionClient={agentSessionClient}
        workflowTypeId={null}
        workflowInstanceId="instance-1"
        onOpenWorkflowType={() => undefined}
      />,
    );

    expect(await screen.findByText('Ready to begin')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Open Sender Harness node Sender' }));
    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    fireEvent.change(await screen.findByLabelText('Initial message'), {
      target: { value: 'Review the selected worktree.' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));

    await waitFor(() =>
      expect(client.sendWorkflowNodeMessage).toHaveBeenCalledWith({
        workflowInstanceId: 'instance-1',
        nodeId: 'sender',
        submittedText: 'Review the selected worktree.',
      }),
    );
    expect(ordinarySend).not.toHaveBeenCalled();
    await waitFor(() => expect(client.loadWorkflowInstance).toHaveBeenCalledTimes(2));
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
    await waitFor(() => expect(within(dialog).getByLabelText('Harness name')).toBeEnabled());
    expect(within(dialog).getByRole('combobox', { name: 'Harness source' })).toHaveValue(
      'From scratch',
    );
    expect(within(dialog).queryByRole('option', { name: /Existing role/ })).toBeNull();
    expect(screen.getByText('Draft')).toBeVisible();
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

  it('discovers only sender-exposed native MCP handoffs and persists the selected warning', async () => {
    const base = definitionWithNodes();
    const senderHarness = {
      ...emptyHarness('Sender Harness'),
      tools: {
        ...emptyHarness('Sender Harness').tools,
        mcpServers: [
          {
            serverName: 'workflow_handoff',
            access: { kind: 'selected_tools' as const, toolNames: ['handoff_to_agent'] },
          },
          { serverName: 'other_workflow', access: { kind: 'entire_server' as const } },
        ],
      },
    };
    const definition: WorkflowDefinition = {
      ...base,
      nodes: base.nodes.map((element) =>
        element.id === 'sender' ? { ...element, draftEffectiveHarness: senderHarness } : element,
      ),
    };
    const client = workflowClient(definition);
    vi.mocked(client.listWorkflowMcpComponents).mockResolvedValue([
      {
        serverName: 'workflow_handoff',
        toolName: 'handoff_to_agent',
        title: 'Handoff to agent',
        participationMode: 'native',
        interfaceId: 'prompt_agent_files_and_text/v1',
      },
      {
        serverName: 'workflow_handoff',
        toolName: 'hidden_tool',
        title: 'Hidden tool',
        participationMode: 'native',
        interfaceId: 'prompt_agent_files_and_text/v1',
      },
      {
        serverName: 'other_workflow',
        toolName: 'wrong_interface',
        title: 'Wrong interface',
        participationMode: 'native',
        interfaceId: 'other/v1',
      },
    ]);
    render(
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
      target: { value: 'mcp_native_prompt_agent' },
    });

    const component = within(dialog).getByLabelText('MCP component');
    expect(within(component).getByRole('option', { name: /Handoff to agent/ })).toBeVisible();
    expect(within(component).queryByRole('option', { name: /Hidden tool/ })).toBeNull();
    expect(within(component).queryByRole('option', { name: /Wrong interface/ })).toBeNull();
    fireEvent.change(component, {
      target: { value: JSON.stringify(['workflow_handoff', 'handoff_to_agent']) },
    });
    await waitFor(() =>
      expect(vi.mocked(client.saveConnectionDraft).mock.calls.at(-1)?.[1]).toMatchObject({
        mechanism: {
          kind: 'mcp_native_prompt_agent',
          serverName: 'workflow_handoff',
          toolName: 'handoff_to_agent',
          warningText: null,
        },
      }),
    );

    fireEvent.change(within(dialog).getByLabelText('Connection warning'), {
      target: { value: 'The handoff ran after this tool call.' },
    });
    await waitFor(() =>
      expect(vi.mocked(client.saveConnectionDraft).mock.calls.at(-1)?.[1]).toMatchObject({
        mechanism: {
          kind: 'mcp_native_prompt_agent',
          warningText: 'The handoff ran after this tool call.',
        },
      }),
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
      promptPrefix: {
        ...emptyHarness('Review Harness').promptPrefix,
        content: 'Inherited instructions',
      },
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
    const promptSection = within(dialog)
      .getByRole('heading', { name: 'Prompt prefix' })
      .closest('section')!;
    fireEvent.click(within(promptSection).getByRole('button', { name: 'Plain' }));
    const instructions = within(promptSection).getByLabelText('Prompt prefix plain Markdown');
    const promptField = instructions.closest('.harness-management__field') as HTMLElement;
    expect(instructions).toHaveValue('Inherited instructions');
    fireEvent.change(instructions, { target: { value: 'Security-only instructions' } });
    await waitFor(() =>
      expect(vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1].harness).toMatchObject({
        kind: 'role',
        roleId: role.id,
        overrides: {
          promptPrefixContent: 'Security-only instructions',
        },
      }),
    );
    expect(
      (
        vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1].harness as unknown as {
          overrides: Record<string, unknown>;
        }
      ).overrides,
    ).toEqual({ promptPrefixContent: 'Security-only instructions' });
    await waitFor(() => expect(promptField).toHaveClass('is-overridden'));
    fireEvent.click(within(promptField).getByRole('button', { name: 'Use inherited value' }));
    await waitFor(() =>
      expect(
        (
          vi.mocked(client.saveNodeDraft).mock.calls.at(-1)?.[1].harness as {
            overrides: { promptPrefixContent?: unknown };
          }
        ).overrides.promptPrefixContent,
      ).toBeUndefined(),
    );
    const harnessSource = await waitFor(() =>
      within(dialog).getByRole('combobox', { name: 'Harness source' }),
    );
    fireEvent.click(harnessSource);
    fireEvent.click(within(dialog).getByRole('option', { name: /From scratch/ }));
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
      harness: {
        kind: 'standalone',
        config: { identity: { name: 'Sender Harness' } },
      },
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
          draftEffectiveHarness: {
            ...role.harness,
            promptPrefix: { ...role.harness.promptPrefix, content: 'Local review scope' },
          },
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
    expect(within(catalog).getByTestId('harness-definition-editor')).toBeVisible();
    fireEvent.keyDown(within(catalog).getByLabelText('Role name'), { key: ' ' });
    expect(client.saveNodeDraft).not.toHaveBeenCalled();
    fireEvent.change(within(catalog).getByLabelText('Role name'), {
      target: { value: 'Security reviewer' },
    });
    fireEvent.change(within(catalog).getByLabelText('Harness name'), {
      target: { value: 'Security Harness' },
    });
    fireEvent.change(within(catalog).getByLabelText('Authority summary'), {
      target: { value: 'Review security boundaries.' },
    });
    fireEvent.click(within(catalog).getByRole('button', { name: 'Create Role' }));
    await waitFor(() =>
      expect(client.createRole).toHaveBeenCalledWith({
        name: 'Security reviewer',
        harness: expect.objectContaining({
          identity: expect.objectContaining({ name: 'Security Harness' }),
          runtime: expect.objectContaining({ authoritySummary: 'Review security boundaries.' }),
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
    listWorkflowMcpComponents: vi.fn(async () => []),
    createWorkflowInstance: vi.fn(async () => {
      throw new Error('not configured');
    }),
    sendWorkflowNodeMessage: vi.fn(async () => {
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
    identity: {
      name: harnessName,
      machineKey: harnessName.toLowerCase().replaceAll(' ', '_'),
      permittedAgentNames: null,
      visualIdentity: null,
    },
    promptPrefix: {
      content: '',
      initialDelivery: 'prepend' as const,
      contextCompressionDelivery: 'deferred' as const,
    },
    skills: { availableDiscoveryPolicy: 'whitelist' as const, items: [] },
    tools: {
      availableDiscoveryPolicy: 'whitelist' as const,
      items: [],
      schemaBoundary: 'Tool schemas remain runtime-owned.',
      mcpServers: [],
    },
    runtime: {
      modelPolicyMode: 'revision_owned' as const,
      models: [
        {
          modelId: 'gpt-5.6-terra',
          allowed: true,
          minReasoning: 'low' as const,
          maxReasoning: 'xhigh' as const,
        },
        {
          modelId: 'gpt-5.6-sol',
          allowed: true,
          minReasoning: 'medium' as const,
          maxReasoning: 'xhigh' as const,
        },
      ],
      defaultModel: null,
      defaultReasoning: null,
      sandbox: 'workspace_write' as const,
      sandboxOptions: ['read_only', 'workspace_write', 'danger_full_access'] as const,
      approvalPolicy: 'never' as const,
      approvalPolicyOptions: ['never'] as const,
      authoritySummary: '',
    },
    hooks: [],
    updatePolicy: {
      status: 'not_configured' as const,
      reason: 'Session replacement applies activated Workflow Harness changes.',
    },
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
      createdAt: '2026-08-09T01:00:00.000Z',
    },
    target: workflowTarget,
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
    connectionActivations: [],
  };
}

function TestTargetSelector({
  id,
  value,
  disabled,
  onChange,
}: RepoBranchWorktreeTargetSelectorProps) {
  return (
    <button type="button" id={id} disabled={disabled} onClick={() => onChange(workflowTarget)}>
      {value ? 'Review repo · feature/workflow' : 'Choose review target'}
    </button>
  );
}

const workflowTarget: ResolvedRepoBranchWorktreeTarget = {
  repository: {
    id: 'repo-1',
    name: 'Review repo',
    rootPath: 'C:\\repos\\review',
  },
  branch: { id: 'branch-1', name: 'feature/workflow' },
  worktree: { id: 'worktree-1', path: 'C:\\worktrees\\review' },
};

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
