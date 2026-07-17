import { act, fireEvent, render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import type {
  AgentInvocationDto,
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionUpdateDto,
  SendAgentSessionMessageCommandDto,
  SendAgentSessionMessageResultDto,
} from '../../application/agentSessions';
import type {
  EpicPlanProposalSnapshot,
  EpicPlanProposalSource,
} from '../../application/orchestrations';
import { EpicInitiationError } from '../../application/orchestrations';
import { sessionDetails } from '../agentSessions/testFixtures';
import { BUILD_EPIC_PLAN_PROMPT, EpicPlanBuilder } from './EpicPlanBuilder';

describe('EpicPlanBuilder', () => {
  it('refreshes the durable proposal after a normal send reaches the persisted terminal boundary', async () => {
    const source = createDurableProposalSource({ kind: 'unavailable' });
    const client = createPlanBuilderClient();
    render(
      <EpicPlanBuilder agentSessionClient={client} proposalSource={source} onBack={vi.fn()} />,
    );

    fireEvent.change(screen.getByRole('textbox', { name: 'Describe what we are working on' }), {
      target: { value: 'Normal planning discussion' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(await screen.findByText('Normal planning discussion')).toBeVisible();

    source.setDurableSnapshot(availableProposal('2026-07-10T12:01:00.000Z'));
    await act(async () => client.emitTerminal());

    expect(await screen.findByText('Durably saved Sprint')).toBeVisible();
    expect(source.refreshCalls).toBeGreaterThan(1);
  });

  it('re-queries durable state on mount so a missed update or restart restores the proposal', async () => {
    const source = createDurableProposalSource(availableProposal('2026-07-09T12:00:00.000Z'));
    const client = createPlanBuilderClient(sessionDetails('completed'));
    render(
      <EpicPlanBuilder
        agentSessionClient={client}
        proposalSource={source}
        draft={{ draftId: 'draft-1', sessionId: 'session-1' }}
        onBack={vi.fn()}
      />,
    );

    expect(await screen.findByText('Durably saved Sprint')).toBeVisible();
    expect(source.refreshCalls).toBeGreaterThan(0);
  });

  it('derives Rebuild from a durable proposal and later user discussion, without button history', async () => {
    const source = createDurableProposalSource(availableProposal('2026-07-09T12:00:00.000Z'));
    const client = createPlanBuilderClient(sessionDetails('completed'));
    render(
      <EpicPlanBuilder
        agentSessionClient={client}
        proposalSource={source}
        draft={{ draftId: 'draft-1', sessionId: 'session-1' }}
        onBack={vi.fn()}
      />,
    );

    expect(await screen.findByRole('button', { name: 'Rebuild plan' })).toBeEnabled();
    expect(client.requestPlanCalls).toBe(0);
  });

  it('does not treat an application-origin turn as later user discussion and clears stale state when refresh is unavailable', async () => {
    const details = sessionDetails('completed');
    details.invocations[0].invocation.inputProvenance = 'application';
    const source = createDurableProposalSource(availableProposal('2026-07-09T12:00:00.000Z'));
    const client = createPlanBuilderClient(details);
    render(
      <EpicPlanBuilder
        agentSessionClient={client}
        proposalSource={source}
        draft={{ draftId: 'draft-1', sessionId: 'session-1' }}
        onBack={vi.fn()}
      />,
    );

    expect(await screen.findByRole('button', { name: 'Rebuild plan' })).toBeDisabled();
    source.setDurableSnapshot({ kind: 'unavailable', reason: 'native query unavailable' });
    await act(async () => source.refresh());
    expect(screen.queryByText('Durably saved Sprint')).toBeNull();
    expect(screen.getByRole('button', { name: 'Plan Epic' })).toBeDisabled();
  });

  it('renders the product Plan action as Application and does not count it as later user discussion', async () => {
    const source = createDurableProposalSource({ kind: 'unavailable' });
    const client = createPlanBuilderClient(sessionDetails('completed'));
    render(
      <EpicPlanBuilder
        agentSessionClient={client}
        proposalSource={source}
        draft={{ draftId: 'draft-1', sessionId: 'session-1' }}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Plan Epic' }));
    expect(await screen.findByText(BUILD_EPIC_PLAN_PROMPT)).toBeVisible();
    expect(screen.getByText('Plan Builder / Application')).toBeVisible();
    expect(client.requestPlanCalls).toBe(1);
    await act(async () => client.emitTerminal());
    await act(async () => {
      source.setDurableSnapshot(availableProposal('2026-07-10T12:01:00.000Z'));
      await source.refresh();
    });
    expect(screen.getByRole('button', { name: 'Rebuild plan' })).toBeDisabled();
  });

  it('enables Initiate only through an injected capability', async () => {
    const requestInitiation = vi.fn().mockResolvedValue(undefined);
    const source = createDurableProposalSource({ kind: 'unavailable' });
    const client = createPlanBuilderClient();
    render(
      <EpicPlanBuilder
        agentSessionClient={client}
        proposalSource={source}
        initiationCapability={readyInitiation()}
        onRequestInitiation={requestInitiation}
        onBack={vi.fn()}
      />,
    );

    const button = screen.getByRole('button', { name: 'Initiate Epic' });
    expect(button).toBeEnabled();
    await act(async () => fireEvent.click(button));
    expect(requestInitiation).toHaveBeenCalledWith(readyInitiation().request);
    expect(
      screen.queryByText(
        'Select an active Epic Planning Draft with a current proposal before initiation.',
      ),
    ).toBeNull();
  });

  it('keeps the builder open while initiation is pending and requires durable confirmation before success', async () => {
    let resolveRequest!: () => void;
    const initiation = new Promise<void>((resolve) => {
      resolveRequest = resolve;
    });
    const requestInitiation = vi.fn().mockReturnValue(initiation);
    const source = createDurableProposalSource(availableProposal('2026-07-10T12:00:00.000Z'));
    render(
      <EpicPlanBuilder
        agentSessionClient={createPlanBuilderClient()}
        proposalSource={source}
        initiationCapability={readyInitiation()}
        onRequestInitiation={requestInitiation}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Initiate Epic' }));
    expect(screen.getByRole('button', { name: 'Requesting confirmation…' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Requesting confirmation…' }));
    expect(requestInitiation).toHaveBeenCalledOnce();

    await act(async () => resolveRequest());
    expect(screen.getByRole('main', { name: 'Plan an Epic' })).toBeVisible();
  });

  it('shows distinct canceled and already-initiated labels without offering another initiation', async () => {
    const { rerender } = render(
      <EpicPlanBuilder
        agentSessionClient={createPlanBuilderClient()}
        proposalSource={createDurableProposalSource({ kind: 'unavailable' })}
        initiationCapability={{
          status: 'blocked',
          reason: 'This Epic Planning Draft was canceled and cannot be initiated.',
        }}
        onBack={vi.fn()}
      />,
    );
    expect(screen.getByText(/was canceled and cannot be initiated/i)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Initiate Epic' })).toBeDisabled();

    await act(async () => undefined);

    rerender(
      <EpicPlanBuilder
        agentSessionClient={createPlanBuilderClient()}
        proposalSource={createDurableProposalSource({ kind: 'unavailable' })}
        initiationCapability={{
          status: 'already_initiated',
          reason: 'This Epic Planning Draft has already been initiated.',
        }}
        onBack={vi.fn()}
      />,
    );
    expect(screen.getByRole('button', { name: 'Epic already initiated' })).toBeDisabled();
    expect(screen.getByText(/has already been initiated/i)).toBeVisible();
    await act(async () => undefined);
  });

  it('does not offer cancellation after durable initiation is confirmed', async () => {
    render(
      <EpicPlanBuilder
        agentSessionClient={createPlanBuilderClient()}
        proposalSource={createDurableProposalSource({ kind: 'unavailable' })}
        draft={{ draftId: 'draft-1', sessionId: 'session-1' }}
        lifecycleClient={{
          cancel: vi.fn(),
          list: vi.fn(),
          reconcile: vi.fn(),
          updateTitle: vi.fn(),
        }}
        initiationCapability={{
          status: 'already_initiated',
          reason: 'This Epic Planning Draft has already been initiated.',
        }}
        onBack={vi.fn()}
      />,
    );

    await act(async () => undefined);
    expect(screen.getByText('Epic initiation confirmed')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Cancel draft' })).toBeNull();
  });

  it('keeps safe stale-proposal guidance visible and reloads capability authority for retry', async () => {
    const refreshAuthority = vi.fn().mockResolvedValue(undefined);
    render(
      <EpicPlanBuilder
        agentSessionClient={createPlanBuilderClient()}
        proposalSource={createDurableProposalSource(availableProposal('2026-07-10T12:00:00.000Z'))}
        initiationCapability={readyInitiation()}
        onRequestInitiation={vi.fn().mockRejectedValue(new EpicInitiationError('stale_proposal'))}
        onInitiationFailure={refreshAuthority}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Initiate Epic' }));
    expect(await screen.findByText(/proposal changed.*try initiation again/i)).toBeVisible();
    expect(refreshAuthority).toHaveBeenCalledOnce();
    expect(screen.getByRole('main', { name: 'Plan an Epic' })).toBeVisible();
  });
});

function readyInitiation() {
  return {
    status: 'ready' as const,
    request: {
      epicPlanningDraftId: 'draft-1',
      expectedRevisionToken: 'revision-token-1',
      idempotencyKey: 'initiate:draft-1:revision-1',
    },
  };
}

function availableProposal(recordedAt: string): EpicPlanProposalSnapshot {
  return {
    kind: 'available',
    revision: { id: `revision-${recordedAt}`, recordedAt },
    sprints: [
      {
        title: 'Durably saved Sprint',
        intendedMovement: 'Keep the proposal durable.',
        concernSummaries: [],
      },
    ],
  };
}

function createDurableProposalSource(initialDurable: EpicPlanProposalSnapshot) {
  let durableSnapshot = initialDurable;
  let snapshot: EpicPlanProposalSnapshot = { kind: 'unavailable' };
  let refreshCalls = 0;
  const listeners = new Set<() => void>();
  const source: EpicPlanProposalSource & {
    readonly refreshCalls: number;
    setDurableSnapshot(next: EpicPlanProposalSnapshot): void;
  } = {
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    async refresh() {
      refreshCalls += 1;
      snapshot = durableSnapshot;
      for (const listener of listeners) listener();
    },
    get refreshCalls() {
      return refreshCalls;
    },
    setDurableSnapshot(next) {
      durableSnapshot = next;
    },
  };
  return source;
}

function createPlanBuilderClient(
  initialDetails: AgentSessionDetailsDto = sessionDetails(),
): AgentSessionClient & {
  emitTerminal(): void;
  requestPlan(
    command: Omit<SendAgentSessionMessageCommandDto, 'submittedText'>,
  ): Promise<SendAgentSessionMessageResultDto>;
  readonly requestPlanCalls: number;
} {
  const details = structuredClone(initialDetails);
  const listeners = new Set<(update: AgentSessionUpdateDto) => void>();
  let requestPlanCalls = 0;
  let nextInvocation = details.invocations.length + 1;
  const client: AgentSessionClient & {
    emitTerminal(): void;
    requestPlan(
      command: Omit<SendAgentSessionMessageCommandDto, 'submittedText'>,
    ): Promise<SendAgentSessionMessageResultDto>;
    readonly requestPlanCalls: number;
  } = {
    createSession: async () => structuredClone(details.session),
    listSessions: async () => [],
    loadSession: async () => structuredClone(details),
    reloadSession: async () => structuredClone(details),
    subscribeUpdates: async (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    sendMessage: async (command) => {
      const invocation = makeInvocation(
        `invocation-${nextInvocation++}`,
        command.submittedText,
        'user',
      );
      details.invocations.push({ invocation, events: [] });
      return { sessionId: details.session.id, invocationId: invocation.id };
    },
    cancelInvocation: async () => details.invocations[0].invocation,
    disconnectUpdates: async () => undefined,
    requestPlan: async (_command) => {
      void _command;
      requestPlanCalls += 1;
      const invocation = makeInvocation(
        `invocation-${nextInvocation++}`,
        BUILD_EPIC_PLAN_PROMPT,
        'application',
      );
      details.invocations.push({ invocation, events: [] });
      return { sessionId: details.session.id, invocationId: invocation.id };
    },
    emitTerminal() {
      const entry = details.invocations.at(-1);
      if (!entry) throw new Error('No invocation to complete');
      entry.invocation.status = 'completed';
      entry.invocation.completedAt = '2026-07-10T12:02:00.000Z';
      entry.invocation.updatedAt = entry.invocation.completedAt;
      const update: AgentSessionUpdateDto = {
        kind: 'invocation_terminal',
        sessionId: details.session.id,
        invocationId: entry.invocation.id,
        invocation: structuredClone(entry.invocation),
      };
      for (const listener of listeners) listener(update);
    },
    get requestPlanCalls() {
      return requestPlanCalls;
    },
  };
  return client;
}

function makeInvocation(
  id: string,
  submittedText: string,
  inputProvenance: AgentInvocationDto['inputProvenance'],
): AgentInvocationDto {
  const timestamp = '2026-07-10T12:00:00.000Z';
  return {
    id,
    sessionId: 'session-1',
    submittedText,
    inputProvenance,
    status: 'pending',
    requestedOptions: { model: null, sandbox: null },
    effectiveOptions: null,
    startedAt: timestamp,
    completedAt: null,
    exitCode: null,
    signal: null,
    runtimeError: null,
    diagnostics: [],
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}
