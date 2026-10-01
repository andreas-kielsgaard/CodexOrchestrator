import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { SessionQueueViewDto } from '../../application/agentSessions/sessionQueue';
import type { AgentSessionWorkspaceController } from './useAgentSession';
import { SessionQueuePanel } from './SessionQueuePanel';

function fixture(): SessionQueueViewDto {
  const execution = {
    deviceId: 'local',
    deviceName: 'Laptop',
    provider: 'codex',
    configurationRef: 'default',
    connection: { kind: 'local' as const },
  };
  const target = {
    capabilityProfileId: 'profile',
    capabilityProfileRevision: 1,
    execution,
    repositoryId: 'repo',
    branchRef: 'refs/heads/main',
    worktreeId: 'tree',
    path: 'C:\\repo',
    head: 'a'.repeat(40),
  };
  const operation = {
    id: 'operation',
    sessionId: 'session',
    sourceTarget: target,
    needsResolution: false,
    receipt: null,
    projectedPlan: {
      calculatedAt: '2026-09-30T10:00:00Z',
      requestRevision: 0,
      stale: false,
      source: null,
      facts: { destination: null, destinationReady: null, unknown: [], blockers: [] },
      steps: [],
    },
    request: {
      intent: 'change_conversation_target' as const,
      migrationResolution: null,
      destination: {
        capabilityProfileId: 'profile',
        capabilityProfileRevision: 1,
        execution,
        workspace: { kind: 'existing' as const, target },
      },
    },
    phase: 'staged' as const,
    actions: [
      { kind: 'prepare_device' as const, status: 'completed' as const, detail: null, error: null },
    ],
    stagedTarget: target,
    error: null,
    generation: 0,
    createdAt: '',
    updatedAt: '',
  };
  return {
    sessionId: 'session',
    revision: 2,
    activeTarget: target,
    entries: [
      {
        entry: {
          id: 'target-entry',
          sessionId: 'session',
          ordinal: 0,
          state: 'pending',
          item: { kind: 'target_operation', operationId: 'operation' },
          blockedReason: null,
          createdAt: '',
          updatedAt: '',
        },
        message: null,
        targetOperation: operation,
      },
      {
        entry: {
          id: 'message-entry',
          sessionId: 'session',
          ordinal: 1,
          state: 'pending',
          item: { kind: 'message', invocationId: 'invocation' },
          blockedReason: null,
          createdAt: '',
          updatedAt: '',
        },
        message: {
          invocation: {
            id: 'invocation',
            sessionId: 'session',
            submittedText: 'Do the next task',
            inputProvenance: 'user',
            status: 'pending',
            requestedOptions: { model: 'gpt-test', sandbox: 'workspace_write' },
            effectiveOptions: null,
            startedAt: null,
            completedAt: null,
            exitCode: null,
            signal: null,
            runtimeError: null,
            diagnostics: [],
            createdAt: '',
            updatedAt: '',
          },
          preparation: {
            sessionId: 'session',
            invocationId: 'invocation',
            phase: 'accepted',
            selection: null,
            steps: [],
            error: null,
            canRetry: false,
            model: 'gpt-test',
            reasoningMode: 'high',
            sandboxMode: 'workspace_write',
          },
        },
        targetOperation: null,
      },
    ],
  };
}

function controller(overrides: Partial<AgentSessionWorkspaceController> = {}) {
  return {
    selectedSessionId: 'session',
    details: null,
    transcript: null,
    draft: '',
    workingDirectory: '',
    loading: false,
    sending: false,
    canceling: false,
    error: null,
    steeringQueue: [],
    expandedProcessing: new Set<string>(),
    setDraft: vi.fn(),
    setWorkingDirectory: vi.fn(),
    send: vi.fn(),
    cancel: vi.fn(),
    reload: vi.fn(),
    toggleProcessing: vi.fn(),
    clearError: vi.fn(),
    ...overrides,
  } satisfies AgentSessionWorkspaceController;
}

describe('SessionQueuePanel', () => {
  it('sends immediately from an icon without opening the editor', async () => {
    const sendQueueEntryNow = vi.fn(async () => true);
    render(
      <SessionQueuePanel
        queue={fixture()}
        controller={controller({ sendQueueEntryNow })}
        onChooseTarget={() => {}}
      />,
    );
    const button = screen.getByRole('button', { name: 'Steer Turn: Do the next task' });
    expect(button.textContent).toBe('');
    fireEvent.click(button);
    expect(sendQueueEntryNow).toHaveBeenCalledWith('message-entry');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await waitFor(() => expect(button).toBeEnabled());
  });

  it('sends the edited message from the modal and keeps it open on failure', async () => {
    const sendQueueEntryNow = vi.fn(async () => false);
    const updateQueuedMessage = vi.fn();
    render(
      <SessionQueuePanel
        queue={fixture()}
        controller={controller({ sendQueueEntryNow, updateQueuedMessage })}
        onChooseTarget={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Queued: Do the next task/ }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), {
      target: { value: 'Updated correction' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Steer Turn' }));
    await waitFor(() =>
      expect(sendQueueEntryNow).toHaveBeenCalledWith('message-entry', {
        submittedText: 'Updated correction',
        model: 'gpt-test',
        reasoningMode: 'high',
        sandboxMode: 'workspace_write',
      }),
    );
    expect(updateQueuedMessage).not.toHaveBeenCalled();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    sendQueueEntryNow.mockResolvedValue(true);
    fireEvent.click(screen.getByRole('button', { name: 'Steer Turn' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('reorders messages and target operation sets as peer entries', () => {
    const reorderQueue = vi.fn();
    render(
      <SessionQueuePanel
        queue={fixture()}
        controller={controller({ reorderQueue })}
        onChooseTarget={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Queued: Do the next task/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Move up' }));
    expect(reorderQueue).toHaveBeenCalledWith(['message-entry', 'target-entry']);
  });

  it('keeps later entries behind the executing queue boundary', () => {
    const reorderQueue = vi.fn();
    const base = fixture();
    const queue: SessionQueueViewDto = {
      ...base,
      entries: base.entries.map((item, index) =>
        index === 0
          ? {
              ...item,
              entry: { ...item.entry, state: 'executing' as const },
              targetOperation: item.targetOperation
                ? { ...item.targetOperation, phase: 'activating' as const }
                : null,
            }
          : item,
      ),
    };
    render(
      <SessionQueuePanel
        queue={queue}
        controller={controller({ reorderQueue })}
        onChooseTarget={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Queued: Do the next task/ }));
    expect(screen.getByRole('button', { name: 'Move up' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Move up' }));
    expect(reorderQueue).not.toHaveBeenCalled();
  });

  it('opens a queued message editor and saves message-local choices', async () => {
    const updateQueuedMessage = vi.fn(async () => {});
    render(
      <SessionQueuePanel
        queue={fixture()}
        controller={controller({ updateQueuedMessage })}
        onChooseTarget={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Queued: Do the next task/ }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), {
      target: { value: 'Revised task' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save message' }));
    await waitFor(() =>
      expect(updateQueuedMessage).toHaveBeenCalledWith({
        entryId: 'message-entry',
        submittedText: 'Revised task',
        model: 'gpt-test',
        reasoningMode: 'high',
        sandboxMode: 'workspace_write',
      }),
    );
  });

  it('opens target selection for a queued message without storing target state on the message', () => {
    const onChooseTarget = vi.fn();
    const queue = fixture();
    render(
      <SessionQueuePanel queue={queue} controller={controller()} onChooseTarget={onChooseTarget} />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Queued: Do the next task/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Change destination…' }));
    expect(onChooseTarget).toHaveBeenCalledWith(queue.entries[1]);
  });

  it('opens a target operation editor and delegates destination selection', () => {
    const onChooseTarget = vi.fn();
    const queue = fixture();
    render(
      <SessionQueuePanel queue={queue} controller={controller()} onChooseTarget={onChooseTarget} />,
    );
    fireEvent.click(
      screen.getByRole('button', { name: /Ready: Change conversation target to Laptop/ }),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choose destination…' }));
    expect(onChooseTarget).toHaveBeenCalledWith(queue.entries[0]);
  });

  it('shows one compact line with a more indicator while retaining all queue rows', () => {
    const { container } = render(
      <SessionQueuePanel queue={fixture()} controller={controller()} onChooseTarget={() => {}} />,
    );
    expect(screen.getByText('+1 more')).toBeVisible();
    expect(container.querySelectorAll('.session-queue li')).toHaveLength(2);
    expect(screen.queryByText('Prepare device')).not.toBeInTheDocument();
  });

  it('puts steering first and omits device and worktree details from compact rows', () => {
    const { container } = render(
      <SessionQueuePanel
        queue={fixture()}
        steeringEntries={[
          {
            id: 'steering-1',
            invocationId: 'active-invocation',
            sequence: 5,
            kind: 'steering',
            state: 'accepted',
            content: { text: 'Change direction now' },
            result: null,
          },
        ]}
        controller={controller()}
        onChooseTarget={() => {}}
      />,
    );

    const rows = container.querySelectorAll('.session-queue li');
    expect(rows[0]).toHaveClass('is-steering');
    expect(rows[0]).toHaveTextContent('Change direction now');
    expect(rows[0]).toHaveTextContent('Steering next');
    const messageRow = screen.getByText('Do the next task').closest('li');
    expect(messageRow).toHaveTextContent('Queued');
    expect(messageRow).not.toHaveTextContent('Laptop');
    expect(messageRow).not.toHaveTextContent('C:\\repo');
  });

  it('does not render an executing message as future queue work', () => {
    const base = fixture();
    const queue: SessionQueueViewDto = {
      ...base,
      entries: base.entries.map((item, index) =>
        index === 1 ? { ...item, entry: { ...item.entry, state: 'executing' as const } } : item,
      ),
    };
    render(<SessionQueuePanel queue={queue} controller={controller()} onChooseTarget={() => {}} />);
    expect(
      screen.queryByRole('button', { name: /Running: Do the next task/ }),
    ).not.toBeInTheDocument();
  });
});
