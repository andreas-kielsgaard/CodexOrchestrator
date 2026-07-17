import { describe, expect, it, vi } from 'vitest';
import { createTauriEpicPlanningDraftLifecycleClient } from './tauriEpicPlanningDraftLifecycle';

describe('Tauri Epic Planning Draft lifecycle', () => {
  it('reconciles an acknowledged Agent Session and lists strict durable bindings', async () => {
    const invoke = vi.fn().mockResolvedValue({ draftId: 'draft-a', sessionId: 'session-a' });
    const nativeQuery = {
      load: vi.fn().mockResolvedValue({
        planningDrafts: [
          {
            epicPlanningDraftId: 'draft-a',
            title: 'A',
            status: 'active',
            createdAt: 'c',
            updatedAt: 'u',
            currentProposal: { status: 'empty' },
          },
        ],
        agentSessionAssociations: [{ epicPlanningDraftId: 'draft-a', agentSessionId: 'session-a' }],
      }),
    };
    const client = createTauriEpicPlanningDraftLifecycleClient(invoke, nativeQuery as never);
    await expect(client.reconcile('session-a', 'A')).resolves.toEqual({
      draftId: 'draft-a',
      sessionId: 'session-a',
    });
    expect(invoke).toHaveBeenCalledWith('reconcile_managed_plan_builder_session', {
      input: { sessionId: 'session-a', title: 'A' },
    });
    await expect(client.list()).resolves.toEqual([
      {
        epicPlanningDraftId: 'draft-a',
        agentSessionId: 'session-a',
        title: 'A',
        status: 'active',
        createdAt: 'c',
        updatedAt: 'u',
      },
    ]);
  });

  it('uses application-generated idempotency for title and stable cancellation identity', async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const client = createTauriEpicPlanningDraftLifecycleClient(invoke, { load: vi.fn() });
    const binding = { draftId: 'draft-a', sessionId: 'session-a' };
    await client.updateTitle(binding, 'Renamed draft');
    await client.cancel(binding);
    expect(invoke.mock.calls[0][1].input).toMatchObject({
      epicPlanningDraftId: 'draft-a',
      agentSessionId: 'session-a',
      title: 'Renamed draft',
    });
    expect(invoke.mock.calls[1][1].input).toMatchObject({ idempotencyKey: 'cancel:draft-a' });
  });

  it('preserves initiated as a durable terminal lifecycle status', async () => {
    const client = createTauriEpicPlanningDraftLifecycleClient(vi.fn(), {
      load: vi.fn().mockResolvedValue({
        planningDrafts: [
          {
            epicPlanningDraftId: 'draft-initiated',
            status: 'initiated',
            createdAt: 'c',
            updatedAt: 'u',
            currentProposal: { status: 'available', proposalRevisionId: 'revision-1' },
          },
        ],
        agentSessionAssociations: [
          { epicPlanningDraftId: 'draft-initiated', agentSessionId: 'session-initiated' },
        ],
      }),
    } as never);
    await expect(client.list()).resolves.toEqual([
      {
        epicPlanningDraftId: 'draft-initiated',
        agentSessionId: 'session-initiated',
        status: 'initiated',
        createdAt: 'c',
        updatedAt: 'u',
      },
    ]);
  });
});
