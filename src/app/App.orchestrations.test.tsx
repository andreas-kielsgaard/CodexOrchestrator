import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { vi } from 'vitest';
import type {
  AgentSessionClient,
  SendAgentSessionMessageCommandDto,
} from '../application/agentSessions';
import type {
  EpicInitiationCapability,
  OrchestrationApplicationClient,
  ProductReadModelsV1,
} from '../application/orchestrations';
import { createRecordedAgentSessionClient } from '../dev/agentSessions';
import { createMutableRecordedEpicPlanProposalSource } from '../dev/orchestrationSection/recordedEpicPlanProposalSource';
import {
  createRecordedDevelopmentApplicationComposition,
  recordedDevelopmentOrchestrationClient,
  recordedDevelopmentOrchestrationPresentation,
  recordedPlanBuilderAgentIdentity,
} from '../dev/orchestrationSection/recordedOrchestrationClient';
import { sessionDetails } from '../features/agentSessions/testFixtures';
import { createTauriEpicPlanningDraftLifecycleClient } from '../infrastructure/orchestrations/tauriEpicPlanningDraftLifecycle';
import { App } from './App';
import { productOrchestrationPresentationAdapter } from './orchestrationPresentation';
import { BUILD_EPIC_PLAN_PROMPT } from '../features/orchestrations/EpicPlanBuilder';

describe('App orchestration loading', () => {
  it('reconciles same-draft initiation capability when a user-origin Plan Builder run records a proposal', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const binding = { draftId: 'draft-plan-completion', sessionId: sessionDetails().session.id };
    const proposal = createMutableRecordedEpicPlanProposalSource({
      kind: 'unavailable',
      reason: 'No current Epic Plan Proposal has been recorded.',
    });
    let proposalAvailable = false;
    const proposalSource = {
      ...proposal,
      refresh: vi.fn(async () => {
        if (!proposalAvailable || proposal.getSnapshot().kind === 'available') return;
        proposal.setSnapshot({
          kind: 'available',
          suggestedEpicName: 'Minimal Epic Plan Test',
          revision: {
            id: 'proposal-revision-17c056a5-0e98-41ac-bdd7-ad5e1870d06f',
            recordedAt: '2026-07-28T12:00:00.000Z',
          },
          sprints: [
            {
              title: 'Reconciled Sprint',
              intendedMovement: 'Enable initiation from current durable proposal authority.',
              concernSummaries: [],
            },
          ],
        });
      }),
    };
    const sessionClient: AgentSessionClient = {
      ...agentClient(),
      loadSession: async () => sessionDetails('completed'),
      reloadSession: async () => sessionDetails('completed'),
    };
    const requestPlan = vi.fn(async () => {
      proposalAvailable = true;
      return { sessionId: binding.sessionId, invocationId: 'plan-invocation' };
    });
    const initiationCapabilityForDraft = vi.fn(
      async (draftId: string): Promise<EpicInitiationCapability> =>
        proposal.getSnapshot().kind === 'available'
          ? {
              status: 'ready',
              request: {
                epicPlanningDraftId: draftId,
                expectedRevisionToken: 'revision-token-plan-completion',
                idempotencyKey: `initiate:${draftId}:proposal-revision-plan-completion`,
              },
            }
          : {
              status: 'blocked',
              reason: 'A current active Epic Plan Proposal is required before initiation.',
            },
    );
    const list = vi.fn().mockResolvedValue([
      {
        epicPlanningDraftId: binding.draftId,
        agentSessionId: binding.sessionId,
        title: 'Plan completion draft',
        status: 'active' as const,
        createdAt: '2026-07-28T11:00:00.000Z',
        updatedAt: '2026-07-28T11:00:00.000Z',
      },
    ]);
    render(
      <App
        {...composition}
        agentSessionClient={sessionClient}
        managedPlanBuilderSessionClient={{ ...sessionClient, requestPlan }}
        epicPlanProposalSourceForDraft={() => proposalSource}
        epicInitiationCapabilityForDraft={initiationCapabilityForDraft}
        epicPlanningDraftLifecycleClient={{
          reconcile: vi.fn(),
          list,
          updateTitle: vi.fn(),
          cancel: vi.fn(),
        }}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: /Plan completion draft/ }));
    await waitFor(() => expect(initiationCapabilityForDraft).toHaveBeenCalledOnce());
    expect(screen.getByRole('button', { name: 'Initiate Epic' })).toBeDisabled();
    expect(
      screen.getByText('A current active Epic Plan Proposal is required before initiation.'),
    ).toBeVisible();

    fireEvent.click(await screen.findByRole('button', { name: 'Plan Epic' }));

    await waitFor(() => expect(requestPlan).toHaveBeenCalledOnce());
    expect(await screen.findByText('Reconciled Sprint')).toBeVisible();
    await waitFor(() => expect(initiationCapabilityForDraft).toHaveBeenCalledTimes(2));
    expect(screen.getByRole('button', { name: 'Initiate Epic' })).toBeEnabled();
    expect(screen.getByRole('main', { name: 'Plan an Epic' })).toBeVisible();
    expect(list).toHaveBeenCalledOnce();
  });

  it('does not let an older capability response overwrite a newer proposal reconciliation', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const binding = { draftId: 'draft-capability-race', sessionId: sessionDetails().session.id };
    const proposal = createMutableRecordedEpicPlanProposalSource({ kind: 'unavailable' });
    let resolveInitialCapability!: (capability: EpicInitiationCapability) => void;
    const initialCapability = new Promise<EpicInitiationCapability>((resolve) => {
      resolveInitialCapability = resolve;
    });
    const readyCapability: EpicInitiationCapability = {
      status: 'ready',
      request: {
        epicPlanningDraftId: binding.draftId,
        expectedRevisionToken: 'current-revision-token',
        idempotencyKey: 'initiate:draft-capability-race:current-revision',
      },
    };
    const initiationCapabilityForDraft = vi
      .fn<(draftId: string) => Promise<EpicInitiationCapability>>()
      .mockReturnValueOnce(initialCapability)
      .mockResolvedValue(readyCapability);
    render(
      <App
        {...composition}
        epicPlanProposalSourceForDraft={() => proposal}
        epicInitiationCapabilityForDraft={initiationCapabilityForDraft}
        epicPlanningDraftLifecycleClient={{
          reconcile: vi.fn(),
          list: vi.fn().mockResolvedValue([
            {
              epicPlanningDraftId: binding.draftId,
              agentSessionId: binding.sessionId,
              title: 'Capability race draft',
              status: 'active' as const,
              createdAt: '2026-07-28T11:00:00.000Z',
              updatedAt: '2026-07-28T11:00:00.000Z',
            },
          ]),
          updateTitle: vi.fn(),
          cancel: vi.fn(),
        }}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: /Capability race draft/ }));
    await waitFor(() => expect(initiationCapabilityForDraft).toHaveBeenCalledOnce());
    await act(async () => {
      proposal.setSnapshot({
        kind: 'available',
        revision: {
          id: 'current-revision',
          recordedAt: '2026-07-28T12:00:00.000Z',
        },
        sprints: [],
      });
    });
    await waitFor(() => expect(initiationCapabilityForDraft).toHaveBeenCalledTimes(2));
    expect(screen.getByRole('button', { name: 'Initiate Epic' })).toBeEnabled();

    await act(async () => {
      resolveInitialCapability({
        status: 'blocked',
        reason: 'A current active Epic Plan Proposal is required before initiation.',
      });
      await initialCapability;
    });

    expect(screen.getByRole('button', { name: 'Initiate Epic' })).toBeEnabled();
    expect(
      screen.queryByText('A current active Epic Plan Proposal is required before initiation.'),
    ).toBeNull();
  });

  it('loads selected-draft initiation authority, confirms it durably, and keeps the builder mounted', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const binding = { draftId: 'draft-initiate', sessionId: sessionDetails().session.id };
    let initiated = false;
    const requestConfirmation = vi.fn().mockResolvedValue({
      requestId: 'confirmation-1',
      source: { kind: 'button' as const },
      epicPlanningDraftId: binding.draftId,
      state: 'requested' as const,
    });
    const resolveConfirmation = vi.fn().mockImplementation(async () => {
      initiated = true;
      return {
        requestId: 'confirmation-1',
        state: 'projected' as const,
        initiation: {
          initiationId: 'initiation-1',
          epicId: 'epic-1',
          proposalRevisionId: 'revision-1',
          materialSnapshotHash: 'hash-1',
          idempotentReplay: false,
        },
      };
    });
    const initiationCapabilityForDraft = vi.fn(async (draftId: string) => {
      expect(draftId).toBe(binding.draftId);
      return initiated
        ? {
            status: 'already_initiated' as const,
            reason: 'This Epic Planning Draft has already been initiated.',
          }
        : {
            status: 'ready' as const,
            request: {
              epicPlanningDraftId: binding.draftId,
              expectedRevisionToken: 'revision-token-1',
              idempotencyKey: 'initiate:draft-initiate:revision-1',
            },
          };
    });
    const load = vi.fn(composition.orchestrationClient.load);
    render(
      <App
        {...composition}
        orchestrationClient={{ load }}
        epicInitiationConfirmationClient={{
          request: requestConfirmation,
          resolve: resolveConfirmation,
          subscribe: vi.fn().mockResolvedValue(vi.fn()),
          describe: vi
            .fn()
            .mockResolvedValue({ title: 'Initiable draft', sprintTitles: ['Sprint 1'] }),
        }}
        epicInitiationCapabilityForDraft={initiationCapabilityForDraft}
        epicPlanningDraftLifecycleClient={{
          reconcile: vi.fn(),
          list: vi.fn().mockImplementation(async () => [
            {
              epicPlanningDraftId: binding.draftId,
              agentSessionId: binding.sessionId,
              title: 'Initiable draft',
              status: initiated ? ('initiated' as const) : ('active' as const),
              createdAt: '2026-07-15T12:00:00Z',
              updatedAt: '2026-07-15T12:00:00Z',
            },
          ]),
          updateTitle: vi.fn(),
          cancel: vi.fn(),
        }}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: /Initiable draft/ }));
    expect(await screen.findByRole('button', { name: 'Initiate Epic' })).toBeEnabled();
    fireEvent.click(screen.getByRole('button', { name: 'Initiate Epic' }));
    expect(await screen.findByRole('dialog', { name: 'Initiate this Epic?' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Confirm initiation' }));

    await waitFor(() => expect(requestConfirmation).toHaveBeenCalledOnce());
    await waitFor(() => expect(resolveConfirmation).toHaveBeenCalledOnce());
    await waitFor(() => expect(initiationCapabilityForDraft).toHaveBeenCalledTimes(2));
    expect(await screen.findByRole('main', { name: 'Plan an Epic' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Epic already initiated' })).toBeDisabled();
    expect(screen.getByText('Epic initiation confirmed')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Cancel draft' })).toBeNull();
    expect(screen.queryByRole('button', { name: /Initiable draft/ })).toBeNull();
    expect(load).toHaveBeenCalledTimes(2);
  });

  it('keeps durable confirmation successful while clearing stale state after refresh rejection', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const initial = await composition.orchestrationClient.load();
    const load = vi
      .fn()
      .mockResolvedValueOnce(initial)
      .mockRejectedValueOnce(new Error('orchestration refresh unavailable'));
    const binding = { draftId: 'draft-refresh-failure', sessionId: sessionDetails().session.id };
    const proposal = createMutableRecordedEpicPlanProposalSource({
      kind: 'available',
      suggestedEpicName: 'Stale proposal',
      sprints: [{ title: 'Stale Sprint', intendedMovement: 'Must clear', concernSummaries: [] }],
    });
    let proposalRefreshes = 0;
    const proposalSource = {
      ...proposal,
      refresh: vi.fn(async () => {
        proposalRefreshes += 1;
        if (proposalRefreshes > 1)
          proposal.setSnapshot({ kind: 'unavailable', reason: 'refresh unavailable' });
      }),
    };
    const resolveConfirmation = vi.fn().mockResolvedValue({
      requestId: 'confirmation-refresh-failure',
      state: 'projected' as const,
      initiation: {
        initiationId: 'initiation-refresh-failure',
        epicId: 'epic-refresh-failure',
        proposalRevisionId: 'revision-refresh-failure',
        materialSnapshotHash: 'hash-refresh-failure',
        idempotentReplay: false,
      },
    });
    const capability = vi
      .fn()
      .mockResolvedValueOnce({
        status: 'ready' as const,
        request: {
          epicPlanningDraftId: binding.draftId,
          expectedRevisionToken: 'revision-token',
          idempotencyKey: 'refresh-failure',
        },
      })
      .mockRejectedValueOnce(new Error('capability refresh unavailable'));
    const list = vi
      .fn()
      .mockResolvedValueOnce([
        {
          epicPlanningDraftId: binding.draftId,
          agentSessionId: binding.sessionId,
          title: 'Refresh failure draft',
          status: 'active' as const,
          createdAt: '2026-07-16T12:00:00Z',
          updatedAt: '2026-07-16T12:00:00Z',
        },
      ])
      .mockRejectedValueOnce(new Error('draft refresh unavailable'));
    render(
      <App
        {...composition}
        orchestrationClient={{ load }}
        epicPlanProposalSourceForDraft={() => proposalSource}
        epicInitiationCapabilityForDraft={capability}
        epicPlanningDraftLifecycleClient={{
          reconcile: vi.fn(),
          list,
          updateTitle: vi.fn(),
          cancel: vi.fn(),
        }}
        epicInitiationConfirmationClient={{
          request: vi.fn().mockResolvedValue({
            requestId: 'confirmation-refresh-failure',
            source: { kind: 'button' as const },
            epicPlanningDraftId: binding.draftId,
            state: 'requested' as const,
          }),
          resolve: resolveConfirmation,
          subscribe: vi.fn().mockResolvedValue(vi.fn()),
          describe: vi
            .fn()
            .mockResolvedValue({ title: 'Refresh failure draft', sprintTitles: ['Stale Sprint'] }),
        }}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: /Refresh failure draft/ }));
    fireEvent.click(await screen.findByRole('button', { name: 'Initiate Epic' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Confirm initiation' }));
    expect(
      await screen.findByText(/initiation was confirmed.*could not be refreshed/i),
    ).toBeVisible();
    expect(resolveConfirmation).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByRole('button', { name: 'Initiate Epic' })).toBeDisabled();
    expect(screen.getByText(/initiation state is unavailable/i)).toBeVisible();
    expect(screen.queryByText('Stale Sprint')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Back to orchestration overview' }));
    expect(await screen.findByText('Orchestration data could not be loaded.')).toBeVisible();
    expect(resolveConfirmation).toHaveBeenCalledOnce();
  });

  it('cancels a durable planning draft from the Plan Builder toolbar and returns to overview', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const binding = { draftId: 'draft-cancel', sessionId: sessionDetails().session.id };
    const cancel = vi.fn().mockResolvedValue(undefined);
    render(
      <App
        {...composition}
        epicPlanningDraftLifecycleClient={{
          reconcile: vi.fn().mockResolvedValue(binding),
          list: vi.fn().mockResolvedValue([
            {
              epicPlanningDraftId: binding.draftId,
              agentSessionId: binding.sessionId,
              title: 'Cancelable draft',
              status: 'active',
              createdAt: '2026-07-15T12:00:00Z',
              updatedAt: '2026-07-15T12:00:00Z',
            },
          ]),
          updateTitle: vi.fn(),
          cancel,
        }}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: /Cancelable draft/ }));
    await screen.findByRole('main', { name: 'Plan an Epic' });
    fireEvent.click(await screen.findByRole('button', { name: 'Cancel draft' }));
    const cancelDialog = await screen.findByRole('dialog', { name: 'Cancel this planning draft?' });
    expect(cancelDialog).toHaveTextContent('Agent Session history will be kept');
    fireEvent.click(within(cancelDialog).getByRole('button', { name: 'Cancel draft' }));
    await waitFor(() =>
      expect(cancel).toHaveBeenCalledWith({ ...binding, title: 'Cancelable draft' }),
    );
    expect(await screen.findByRole('button', { name: 'Plan an Epic' })).toBeVisible();
  });

  it('keeps the draft open and reports a durable cancellation failure', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const binding = { draftId: 'draft-failure', sessionId: sessionDetails().session.id };
    let rejectCancellation!: (reason: Error) => void;
    const cancellation = new Promise<void>((_resolve, reject) => {
      rejectCancellation = reject;
    });
    render(
      <App
        {...composition}
        epicPlanningDraftLifecycleClient={{
          reconcile: vi.fn().mockResolvedValue(binding),
          list: vi.fn().mockResolvedValue([
            {
              epicPlanningDraftId: binding.draftId,
              agentSessionId: binding.sessionId,
              title: 'Uncancelable draft',
              status: 'active',
              createdAt: '2026-07-15T12:00:00Z',
              updatedAt: '2026-07-15T12:00:00Z',
            },
          ]),
          updateTitle: vi.fn(),
          cancel: vi.fn().mockReturnValue(cancellation),
        }}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: /Uncancelable draft/ }));
    await screen.findByRole('main', { name: 'Plan an Epic' });
    fireEvent.click(await screen.findByRole('button', { name: 'Cancel draft' }));
    const failureDialog = await screen.findByRole('dialog', {
      name: 'Cancel this planning draft?',
    });
    fireEvent.click(within(failureDialog).getByRole('button', { name: 'Cancel draft' }));
    expect(within(failureDialog).getByRole('button', { name: 'Canceling draft…' })).toBeDisabled();
    rejectCancellation(new Error('persistence unavailable'));
    expect(await screen.findByText(/could not be canceled.*remains active/i)).toBeVisible();
    expect(screen.getByRole('main', { name: 'Plan an Epic' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Cancel draft' })).toBeEnabled();
  });

  it('lists an active durable planning draft and reopens its bound Agent Session', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const invoke = vi.fn();
    const lifecycle = createTauriEpicPlanningDraftLifecycleClient(invoke, {
      load: vi.fn().mockResolvedValue({
        planningDrafts: [
          {
            epicPlanningDraftId: 'draft-reopen',
            title: 'Restartable plan',
            status: 'active',
            createdAt: '2026-07-15T12:00:00Z',
            updatedAt: '2026-07-15T12:00:00Z',
            currentProposal: { status: 'empty' },
          },
        ],
        agentSessionAssociations: [
          { epicPlanningDraftId: 'draft-reopen', agentSessionId: sessionDetails().session.id },
        ],
      }),
    } as never);
    render(<App {...composition} epicPlanningDraftLifecycleClient={lifecycle} />);
    fireEvent.click(await screen.findByRole('button', { name: /Restartable plan/ }));
    expect(screen.getByRole('main', { name: 'Plan an Epic' })).toBeVisible();
    expect(screen.getByRole('textbox', { name: 'Epic name' })).toHaveValue('Restartable plan');
    expect(invoke).not.toHaveBeenCalledWith(
      'reconcile_managed_plan_builder_session',
      expect.anything(),
    );
  });

  it('opens an empty Plan Builder first, then binds the draft created by its first send', async () => {
    const client = createRecordedAgentSessionClient();
    const reconcile = vi.fn().mockResolvedValue({
      draftId: 'draft-created-after-send',
      sessionId: 'recorded-session-1',
      title: 'Local title before send',
    });
    const updateTitle = vi.fn().mockResolvedValue(undefined);
    render(
      <App
        agentSessionClient={client}
        orchestrationClient={{
          load: async () => ({ kind: 'unavailable', reason: 'No product read.' }),
        }}
        epicPlanningDraftLifecycleClient={{
          reconcile,
          list: vi.fn().mockResolvedValue([]),
          updateTitle,
          cancel: vi.fn(),
        }}
      />,
    );

    await screen.findByRole('alert');
    fireEvent.click(screen.getByRole('button', { name: 'Plan an Epic' }));
    expect(client.store.sessions.size).toBe(0);
    expect(reconcile).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'Cancel draft' })).toBeNull();
    fireEvent.change(screen.getByRole('textbox', { name: 'Epic name' }), {
      target: { value: 'Local title before send' },
    });
    fireEvent.blur(screen.getByRole('textbox', { name: 'Epic name' }));
    expect(updateTitle).not.toHaveBeenCalled();

    fireEvent.change(screen.getByRole('textbox', { name: 'Describe what we are working on' }), {
      target: { value: 'First durable discussion turn' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    await screen.findByText('First durable discussion turn');
    await waitFor(() =>
      expect(reconcile).toHaveBeenCalledWith(
        'recorded-session-1',
        'Epic builder session for Local title before send',
      ),
    );
    expect(await screen.findByRole('button', { name: 'Cancel draft' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Describe what we are working on' }), {
      target: { value: 'A later discussion turn' },
    });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Send' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    await screen.findByText('A later discussion turn');
    expect(reconcile).toHaveBeenCalledTimes(1);
    fireEvent.blur(screen.getByRole('textbox', { name: 'Epic name' }));
    await waitFor(() =>
      expect(updateTitle).toHaveBeenCalledWith(
        {
          draftId: 'draft-created-after-send',
          sessionId: 'recorded-session-1',
          title: 'Local title before send',
        },
        'Local title before send',
      ),
    );
  });

  it('backs out of an empty Plan Builder without a session or durable draft', async () => {
    const client = createRecordedAgentSessionClient();
    const reconcile = vi.fn();
    render(
      <App
        agentSessionClient={client}
        orchestrationClient={{
          load: async () => ({ kind: 'unavailable', reason: 'No product read.' }),
        }}
        epicPlanningDraftLifecycleClient={{
          reconcile,
          list: vi.fn().mockResolvedValue([]),
          updateTitle: vi.fn(),
          cancel: vi.fn(),
        }}
      />,
    );

    await screen.findByRole('alert');
    fireEvent.click(screen.getByRole('button', { name: 'Plan an Epic' }));
    fireEvent.click(screen.getByRole('button', { name: 'Back to orchestration overview' }));
    expect(await screen.findByRole('button', { name: 'Plan an Epic' })).toBeVisible();
    expect(client.store.sessions.size).toBe(0);
    expect(reconcile).not.toHaveBeenCalled();
  });

  it('does not reconcile a durable draft when the first normal send fails', async () => {
    const client = createRecordedAgentSessionClient();
    client.sendMessage = vi.fn().mockRejectedValue(new Error('managed send unavailable'));
    const reconcile = vi.fn();
    render(
      <App
        agentSessionClient={client}
        orchestrationClient={{
          load: async () => ({ kind: 'unavailable', reason: 'No product read.' }),
        }}
        epicPlanningDraftLifecycleClient={{
          reconcile,
          list: vi.fn().mockResolvedValue([]),
          updateTitle: vi.fn(),
          cancel: vi.fn(),
        }}
      />,
    );

    await screen.findByRole('alert');
    fireEvent.click(screen.getByRole('button', { name: 'Plan an Epic' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Describe what we are working on' }), {
      target: { value: 'This send will fail' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(await screen.findByText('managed send unavailable')).toBeVisible();
    expect(reconcile).not.toHaveBeenCalled();
    expect(client.store.sessions.size).toBe(0);
    expect(screen.queryByRole('button', { name: 'Cancel draft' })).toBeNull();
  });
  it('does not retain or invent planning drafts when the durable draft catalog fails', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const list = vi.fn().mockRejectedValue(new Error('native query unavailable'));
    render(
      <App
        {...composition}
        epicPlanningDraftLifecycleClient={{
          reconcile: vi.fn(),
          list,
          updateTitle: vi.fn(),
          cancel: vi.fn(),
        }}
      />,
    );
    await waitFor(() => expect(list).toHaveBeenCalled());
    expect(screen.getByRole('button', { name: 'Plan an Epic' })).toBeVisible();
    await waitFor(() => expect(screen.queryByText('Pre-initiation planning draft')).toBeNull());
  });
  it('moves from loading to a recorded canonical composition through the shared OrchestrationSection', async () => {
    render(<App {...createRecordedDevelopmentApplicationComposition()} />);

    expect(screen.getByRole('status')).toHaveTextContent('Loading orchestration data');
    expect(
      await screen.findByRole('button', { name: /Open Codex Epic Runner workspace development/ }),
    ).toBeVisible();
  });

  it('opens one conversation-primary Plan Builder and only changes its proposal through the injected source', async () => {
    const proposalSource = createMutableRecordedEpicPlanProposalSource({
      kind: 'available',
      suggestedEpicName: 'Suggested Epic',
      sprints: [
        {
          title: 'First predicted Sprint',
          intendedMovement: 'Clarify the smallest planning direction.',
          concernSummaries: ['Keep proposal state separate from conversation prose.'],
        },
      ],
    });
    const client = createRecordedAgentSessionClient();
    const sent: SendAgentSessionMessageCommandDto[] = [];
    const sendMessage = client.sendMessage.bind(client);
    client.sendMessage = async (command) => {
      sent.push(command);
      return sendMessage(command);
    };
    render(
      <App
        agentSessionClient={client}
        managedPlanBuilderSessionClient={{
          ...client,
          requestPlan: (command) =>
            client.sendMessage({ ...command, submittedText: BUILD_EPIC_PLAN_PROMPT }),
        }}
        orchestrationClient={{
          load: async () => ({ kind: 'unavailable', reason: 'No product read.' }),
        }}
        managedPlanBuilderAgentIdentity={recordedPlanBuilderAgentIdentity}
        epicPlanProposalSource={proposalSource}
      />,
    );

    await screen.findByRole('alert');
    fireEvent.click(screen.getByRole('button', { name: 'Plan an Epic' }));
    expect(screen.getByRole('main', { name: 'Plan an Epic' })).toBeVisible();
    expect(screen.getByRole('heading', { level: 1, name: 'Suggested Epic' })).toBeVisible();
    expect(screen.getByText('Epic planning')).toBeVisible();
    expect(screen.queryByRole('heading', { name: 'Plan an Epic' })).toBeNull();
    const viewHeader = screen
      .getByRole('heading', { level: 1, name: 'Suggested Epic' })
      .closest('.product-view-header');
    expect(viewHeader).not.toBeNull();
    expect(
      within(viewHeader as HTMLElement).getByRole('group', {
        name: 'Epic planning view actions',
      }),
    ).toContainElement(screen.getByRole('button', { name: 'Back to orchestration overview' }));
    expect(
      screen.getByRole('region', { name: 'Planning conversation and plan preview' }),
    ).toBeVisible();
    expect(screen.getByLabelText('Epic Plan Builder conversation')).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Avery: Epic Plan Builder' })).toBeVisible();
    expect(screen.getByRole('heading', { name: "Avery's Proposed Plan:" })).toBeVisible();
    expect(
      screen.getByRole('heading', { name: "Avery's Proposed Plan:" }).closest('header'),
    ).toHaveClass('epic-plan-builder__proposal-header');
    const identityMarkers = document.querySelectorAll('[data-visual-identity-token="sunflower"]');
    expect(identityMarkers).toHaveLength(2);
    expect(screen.getByRole('textbox', { name: 'Epic name' })).toHaveValue('Suggested Epic');
    expect(screen.getByText('Epic name')).toBeVisible();
    expect(screen.queryByText('Agent Session')).toBeNull();
    expect(screen.queryByText('New Agent Session')).toBeNull();
    expect(await screen.findByText('Let’s build a plan')).toBeVisible();
    expect(screen.getByText(/Paste a prepared Epic description or begin discussing/)).toBeVisible();
    expect(
      screen.getByRole('textbox', { name: 'Describe what we are working on' }),
    ).toHaveAttribute('placeholder', 'Describe what we are working on');
    expect(screen.queryByRole('textbox', { name: 'Working directory' })).toBeNull();
    const sprintToggle = screen.getByRole('button', { name: 'Sprint 1 First predicted Sprint' });
    expect(sprintToggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('Clarify the smallest planning direction.')).toBeVisible();
    fireEvent.click(sprintToggle);
    expect(sprintToggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByText('Clarify the smallest planning direction.')).not.toBeVisible();
    fireEvent.click(sprintToggle);
    fireEvent.click(screen.getByText('Clarify the smallest planning direction.'));
    expect(sprintToggle).toHaveAttribute('aria-expanded', 'false');

    fireEvent.keyDown(sprintToggle, { key: 'Enter' });
    expect(sprintToggle).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(sprintToggle);
    expect(sprintToggle).toHaveAttribute('aria-expanded', 'true');
    fireEvent.click(screen.getByText('Clarify the smallest planning direction.'));
    fireEvent.keyDown(sprintToggle, { key: ' ' });
    expect(sprintToggle).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(sprintToggle);
    expect(sprintToggle).toHaveAttribute('aria-expanded', 'true');
    expect(
      screen.getByText('First predicted Sprint').closest('.epic-plan-builder__sprint-heading'),
    ).toBeInTheDocument();
    fireEvent.change(screen.getByRole('textbox', { name: 'Epic name' }), {
      target: { value: 'User named Epic' },
    });
    expect(screen.getByRole('button', { name: 'Initiate Epic' })).toBeDisabled();
    expect(
      screen.getByText(
        'Select an active Epic Planning Draft with a current proposal before initiation.',
      ),
    ).toBeVisible();

    fireEvent.change(screen.getByRole('textbox', { name: 'Describe what we are working on' }), {
      target: { value: 'First intake message' },
    });
    const send = screen.getByRole('button', { name: 'Send' });
    expect(send).toHaveAttribute('aria-describedby', 'composer-keyboard-hint');
    expect(screen.getByRole('tooltip')).toHaveTextContent('Enter to send');
    fireEvent.click(send);
    expect(await screen.findByText('First intake message')).toBeVisible();
    expect([...client.store.sessions.values()][0].session.title).toBe(
      'Epic builder session for User named Epic',
    );
    fireEvent.change(screen.getByRole('textbox', { name: 'Epic name' }), {
      target: { value: 'Renamed after creation' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Send' })).toBeVisible());
    fireEvent.change(screen.getByRole('textbox', { name: 'Describe what we are working on' }), {
      target: { value: 'Continue the same plan' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(await screen.findByText('Continue the same plan')).toBeVisible();
    const planBuilderSession = [...client.store.sessions.values()][0];
    expect(planBuilderSession.invocations).toHaveLength(2);
    expect(
      new Set(planBuilderSession.invocations.map(({ invocation }) => invocation.sessionId)),
    ).toEqual(new Set([planBuilderSession.session.id]));
    expect(planBuilderSession.session.title).toBe('Epic builder session for User named Epic');
    expect(sent).toEqual([
      { submittedText: 'First intake message', title: 'Epic builder session for User named Epic' },
      { sessionId: planBuilderSession.session.id, submittedText: 'Continue the same plan' },
    ]);
    expect(screen.getByText('First predicted Sprint')).toBeVisible();

    await act(async () => {
      proposalSource.setSnapshot({
        kind: 'available',
        suggestedEpicName: 'Changed source name',
        sprints: [
          {
            title: 'Updated predicted Sprint',
            intendedMovement: 'Show a source-owned update.',
            concernSummaries: ['No transcript parsing occurred.'],
          },
        ],
      });
    });
    expect(screen.getByText('Updated predicted Sprint')).toBeVisible();
    expect(screen.queryByText('First predicted Sprint')).toBeNull();
    expect(screen.getByRole('textbox', { name: 'Epic name' })).toHaveValue(
      'Renamed after creation',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Back to orchestration overview' }));
    expect(screen.getByRole('button', { name: 'Plan an Epic' })).toBeVisible();
  });

  it('gates Plan and Rebuild actions on conversation evidence and sends the exact build prompt', async () => {
    const proposalSource = createMutableRecordedEpicPlanProposalSource({ kind: 'unavailable' });
    const client = createRecordedAgentSessionClient();
    const sent: SendAgentSessionMessageCommandDto[] = [];
    const sendMessage = client.sendMessage.bind(client);
    client.sendMessage = async (command) => {
      sent.push(command);
      return sendMessage(command);
    };
    render(
      <App
        agentSessionClient={client}
        managedPlanBuilderSessionClient={{
          ...client,
          requestPlan: (command) =>
            client.sendMessage({ ...command, submittedText: BUILD_EPIC_PLAN_PROMPT }),
        }}
        orchestrationClient={{
          load: async () => ({ kind: 'unavailable', reason: 'No product read.' }),
        }}
        epicPlanProposalSource={proposalSource}
      />,
    );

    await screen.findByRole('alert');
    fireEvent.click(screen.getByRole('button', { name: 'Plan an Epic' }));
    expect(screen.getByRole('button', { name: 'Plan Epic' })).toBeDisabled();
    fireEvent.change(screen.getByRole('textbox', { name: 'Describe what we are working on' }), {
      target: { value: 'Discuss the product boundary first' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    await screen.findByText('Discuss the product boundary first');
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Plan Epic' })).toBeEnabled());

    fireEvent.click(screen.getByRole('button', { name: 'Plan Epic' }));
    await waitFor(() => expect(sent.at(-1)?.submittedText).toBe(BUILD_EPIC_PLAN_PROMPT));
    expect(screen.getByRole('button', { name: 'Plan Epic' })).toBeDisabled();

    await act(async () => {
      proposalSource.setSnapshot({
        kind: 'available',
        sprints: [
          {
            title: 'Structured Sprint',
            intendedMovement: 'Capture the discussed direction.',
            concernSummaries: [],
          },
        ],
      });
    });
    expect(screen.getByRole('button', { name: 'Rebuild plan' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Sprint 1 Structured Sprint' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
  });

  it.each([
    [
      { kind: 'available' as const, suggestedEpicName: 'Suggested Epic', sprints: [] },
      'Epic builder session for Suggested Epic',
    ],
    [{ kind: 'unavailable' as const }, 'Epic builder session'],
  ])('captures %o as the first-session title', async (initialProposal, expectedTitle) => {
    const client = createRecordedAgentSessionClient();
    render(
      <App
        agentSessionClient={client}
        orchestrationClient={{
          load: async () => ({ kind: 'unavailable', reason: 'No product read.' }),
        }}
        epicPlanProposalSource={createMutableRecordedEpicPlanProposalSource(initialProposal)}
      />,
    );

    await screen.findByRole('alert');
    fireEvent.click(screen.getByRole('button', { name: 'Plan an Epic' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Describe what we are working on' }), {
      target: { value: 'First intake message' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    await screen.findByText('First intake message');
    expect([...client.store.sessions.values()][0].session.title).toBe(expectedTitle);
  });

  it('keeps an unavailable product proposal as an honest before-plan state', async () => {
    render(
      <App
        agentSessionClient={agentClient()}
        orchestrationClient={{
          load: async () => ({ kind: 'unavailable', reason: 'No product read.' }),
        }}
      />,
    );
    await screen.findByRole('alert');
    fireEvent.click(screen.getByRole('button', { name: 'Plan an Epic' }));
    await screen.findByRole('textbox', { name: 'Describe what we are working on' });
    expect(
      screen.getByText(
        /will organize the emerging plan into proposed Sprints with bounded objectives/,
      ),
    ).toBeVisible();
  });

  it('uses the same section and Sprint tree for an independent product read client', async () => {
    render(
      <App
        agentSessionClient={agentClient()}
        orchestrationClient={directProductClient(productReadModels())}
        orchestrationPresentation={productOrchestrationPresentationAdapter}
      />,
    );

    expect(await screen.findByRole('button', { name: /Open Product Epic/ })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: /Open Product Epic/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Open Sprint: Product Sprint' }));
    expect(screen.getByRole('region', { name: 'Sprint planning workspace' })).toBeVisible();
  });

  it.each(['pending', 'unavailable', 'unsupported'] as const)(
    'renders %s overview authority without inventing movement or state',
    async (status) => {
      const reads = productReadModels(status);
      render(
        <App agentSessionClient={agentClient()} orchestrationClient={directProductClient(reads)} />,
      );

      expect((await screen.findAllByText(`${label(status)}: ${status} from product`)).length).toBe(
        2,
      );
      expect(screen.queryByText('Planning next work')).toBeNull();
      expect(screen.queryByText('Paused')).toBeNull();
    },
  );

  it('keeps canonical recorded facts over conflicting recorded adjunct lifecycle and state', async () => {
    const loaded = await recordedDevelopmentOrchestrationClient.load();
    if (loaded.kind !== 'ready') throw new Error('Recorded client did not compose.');
    const reads = structuredClone(loaded.readModels) as ProductReadModelsV1;
    const epic = reads.epics[0];
    (epic.overview as { state: { source: unknown; value: 'running' } }).state.value = 'running';
    const sprint = epic.sprints.find(({ sprintId }) => sprintId === 'sprint-control-surface')!;
    (sprint as { lifecycle: { source: unknown; value: 'in_progress' } }).lifecycle.value =
      'in_progress';

    const view = recordedDevelopmentOrchestrationPresentation.present(reads);
    expect(view.epics[0].state).toBe('running');
    expect(view.epics[0].plan.items.find(({ id }) => id === sprint.sprintId)?.status).toBe(
      'in_progress',
    );
  });

  it.each([
    [{ kind: 'empty', reason: 'No orchestration records.' }],
    [{ kind: 'unavailable', reason: 'No durable source is connected.' }],
    [{ kind: 'failed', message: 'Source rejected the request.' }],
  ] as const)('renders %s without recorded fallback', async (result) => {
    render(
      <App agentSessionClient={agentClient()} orchestrationClient={{ load: async () => result }} />,
    );
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'reason' in result ? result.reason : result.message,
    );
    expect(screen.queryByRole('button', { name: /Open Codex Epic Runner/ })).toBeNull();
  });

  it('keeps disposable and compatibility inputs outside production app and feature modules', () => {
    for (const file of [
      'src/app/App.tsx',
      'src/app/orchestrationPresentation.ts',
      'src/app/useOrchestrationLoad.ts',
      'src/bootstrap/productApplicationComposition.ts',
      'src/features/orchestrations/index.ts',
      'src/features/orchestrations/EpicPlanBuilder.tsx',
      'src/features/orchestrations/OrchestrationSection.tsx',
      'src/features/orchestrations/orchestrationModel.ts',
    ]) {
      const source = readFileSync(resolve(file), 'utf8');
      expect(source).not.toMatch(
        /disposableRecordedOrchestrationView|recordedEpicPlanProposalSource|SprintPlannerOutputV1|SprintExecutionSnapshotV1/,
      );
    }
  });

  it('owns scrolling inside bounded product surfaces without elastic overscroll', () => {
    const rootStyles = readFileSync(resolve('src/styles.css'), 'utf8');
    const planStyles = readFileSync(
      resolve('src/features/orchestrations/styles/epicPlanBuilder.css'),
      'utf8',
    );
    const agentStyles = readFileSync(
      resolve('src/features/agentSessions/agentSession.css'),
      'utf8',
    );

    expect(rootStyles).toMatch(/html,\s*#root\s*\{[\s\S]*height: 100%;[\s\S]*overflow: hidden;/);
    expect(rootStyles).toMatch(
      /body\s*\{[\s\S]*overflow: hidden;[\s\S]*overscroll-behavior: none;/,
    );
    expect(planStyles).toMatch(
      /\.epic-plan-builder__proposal-body\s*\{[\s\S]*overflow-y: auto;[\s\S]*overscroll-behavior: none;/,
    );
    expect(planStyles).toMatch(
      /\.epic-plan-builder\s*\{[\s\S]*position: fixed;[\s\S]*inset: 48px 0 0;[\s\S]*overflow: hidden;[\s\S]*overscroll-behavior: none;/,
    );
    expect(planStyles).toMatch(
      /\.epic-plan-builder__body\s*\{[\s\S]*display: flex;[\s\S]*flex-direction: column;[\s\S]*overflow: hidden;/,
    );
    expect(planStyles).toMatch(
      /\.epic-plan-builder__layout\s*\{[\s\S]*min-height: 0;[\s\S]*flex: 1 1 auto;[\s\S]*grid-template-columns: minmax\(200px, 230px\) minmax\(0, 1fr\);[\s\S]*overflow: hidden;/,
    );
    expect(planStyles).toMatch(
      /\.epic-plan-builder__workspace\s*\{[\s\S]*grid-template-columns: minmax\(400px, 1fr\) minmax\(320px, 420px\);[\s\S]*overflow: hidden;/,
    );
    expect(planStyles).toMatch(
      /\.epic-plan-builder__proposal-header\s*\{[\s\S]*height: var\(--epic-plan-builder-workspace-header-height\);[\s\S]*min-height: var\(--epic-plan-builder-workspace-header-height\);/,
    );
    expect(planStyles).toMatch(
      /\.epic-plan-builder__conversation \.agent-session-identity-header\s*\{[\s\S]*height: var\(--epic-plan-builder-workspace-header-height\);[\s\S]*min-height: var\(--epic-plan-builder-workspace-header-height\);/,
    );
    expect(planStyles).toMatch(
      /@media \(max-width: 1100px\)\s*\{[\s\S]*\.epic-plan-builder__layout\s*\{[\s\S]*grid-template-columns: minmax\(180px, 210px\) minmax\(0, 1fr\);[\s\S]*\.epic-plan-builder__workspace\s*\{[\s\S]*grid-template-columns: minmax\(300px, 1fr\) minmax\(280px, 340px\);/,
    );
    expect(planStyles).toMatch(
      /@media \(max-width: 900px\)\s*\{[\s\S]*\.epic-plan-builder__body\s*\{[\s\S]*overflow-x: hidden;[\s\S]*overflow-y: auto;/,
    );
    expect(planStyles).toMatch(
      /@media \(max-width: 900px\)\s*\{[\s\S]*\.epic-plan-builder__layout\s*\{[\s\S]*grid-template-columns: minmax\(0, 1fr\);[\s\S]*overflow: visible;/,
    );
    expect(planStyles).toMatch(
      /@media \(max-width: 900px\)\s*\{[\s\S]*\.epic-plan-builder__workspace\s*\{[\s\S]*grid-template-columns: minmax\(0, 1fr\);[\s\S]*overflow: visible;/,
    );
    expect(agentStyles).toMatch(
      /\.agent-session-scroll-region\s*\{[\s\S]*overflow-y: auto;[\s\S]*overscroll-behavior: none;/,
    );
    expect(
      [
        'detailWorkspace.css',
        'orchestrationSection.css',
        'orchestrationSubdetail.css',
        'sharedAgentSessionPanel.css',
        'sprintInformationSurfaces.css',
        'sprintFlowMap.css',
        'sprintPlan.css',
      ].some((file) =>
        readFileSync(resolve('src/features/orchestrations/styles', file), 'utf8').includes(
          'overscroll-behavior: contain',
        ),
      ),
    ).toBe(false);
  });
});

function directProductClient(readModels: ProductReadModelsV1): OrchestrationApplicationClient {
  return { load: async () => ({ kind: 'ready', readModels }) };
}

function productReadModels(
  status?: 'pending' | 'unavailable' | 'unsupported',
): ProductReadModelsV1 {
  const authority = status
    ? { status, reason: `${status} from product` }
    : { status: 'available' as const, sourceKind: 'repository' as const, sourceReferences: [] };
  const availableSource = {
    status: 'available' as const,
    sourceKind: 'repository' as const,
    sourceReferences: [],
  };
  return {
    epics: [
      {
        epicId: 'product-epic',
        title: 'Product Epic',
        goal: 'Read through the product boundary.',
        source: authority,
        overview: {
          currentMovement: status
            ? { source: { status, reason: `${status} from product` } }
            : { source: availableSource, value: { kind: 'planning_next_work' } },
          state: status
            ? { source: { status, reason: `${status} from product` } }
            : { source: availableSource, value: 'running' },
        },
        sprints: [
          {
            sprintId: 'product-sprint',
            epicId: 'product-epic',
            title: 'Product Sprint',
            summary: 'Canonical Sprint summary.',
            details: 'Canonical Sprint details.',
            source: authority,
            lifecycle: status
              ? { source: { status, reason: `${status} from product` } }
              : { source: availableSource, value: 'in_progress' },
            sprintPlan: {
              sprintPlanId: 'product-plan',
              currentSprintPlanRevisionId: 'product-revision-1',
              selectedSprintPlanRevisionId: 'product-revision-1',
              revisions: [
                {
                  sprintPlanRevisionId: 'product-revision-1',
                  revision: 1,
                  summary: 'Product revision',
                  source: authority,
                  isCurrent: true,
                  isSelected: true,
                  workUnitScopes: [],
                },
              ],
            },
            plannerActivities: [],
            revisionViews: [
              {
                sprintPlanRevisionId: 'product-revision-1',
                revision: 1,
                summary: 'Product revision',
                source: authority,
                isCurrent: true,
                isSelected: true,
                workUnitScopes: [],
                plannerActivityGroups: [],
                workUnits: [],
                gates: [],
                reviews: [],
              },
            ],
            concerns: [],
            reviews: [],
            documents: [],
            internalArtifacts: [],
            workspacePresentation: {
              plannerActivityMembership: [],
              gates: [],
              documents: [],
              narratives: {
                progress: status
                  ? { source: { status, reason: `${status} from product` } }
                  : { source: availableSource, value: 'No work Units.' },
              },
            },
            agentSessionReferences: [],
            continuation: {
              level: 'sprint',
              policy: null,
              eligibility: null,
              commandResults: [],
              eventEligibilityFacts: [],
              continuationRequests: [],
              observedContinuationIds: [],
              initiationObserved: false,
            },
          },
        ],
        agentSessionReferences: [],
        continuation: {
          level: 'epic',
          policy: null,
          eligibility: null,
          commandResults: [],
          eventEligibilityFacts: [],
          continuationRequests: [],
          observedContinuationIds: [],
          initiationObserved: false,
        },
      },
    ],
    unassociatedAgentSessionReferences: [],
  };
}

function label(status: 'pending' | 'unavailable' | 'unsupported') {
  return { pending: 'Pending', unavailable: 'Unavailable', unsupported: 'Unsupported' }[status];
}

function agentClient(): AgentSessionClient {
  return {
    createSession: async () => sessionDetails().session,
    listSessions: async () => [],
    loadSession: async () => sessionDetails(),
    reloadSession: async () => sessionDetails(),
    subscribeUpdates: async () => () => undefined,
    sendMessage: async () => ({ sessionId: 'session-1', invocationId: 'invocation-1' }),
    cancelInvocation: async () => sessionDetails('canceled').invocations[0].invocation,
    disconnectUpdates: async () => undefined,
  };
}
