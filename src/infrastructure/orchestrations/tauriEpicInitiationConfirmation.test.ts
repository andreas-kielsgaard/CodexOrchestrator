import { describe, expect, it, vi } from 'vitest';
import { createTauriEpicInitiationConfirmationClient } from './tauriEpicInitiationConfirmation';

const request = {
  requestId: 'request-1',
  source: { kind: 'button' },
  epicPlanningDraftId: 'draft-1',
  state: 'requested',
};
const initiation = {
  initiationId: 'initiation-1',
  epicId: 'epic-1',
  proposalRevisionId: 'revision-1',
  materialSnapshotHash: 'hash',
  idempotentReplay: false,
};
const native = {
  load: vi.fn().mockResolvedValue({
    planningDrafts: [
      {
        epicPlanningDraftId: 'draft-1',
        title: 'Draft',
        currentProposal: { status: 'available', proposalRevisionId: 'revision-1' },
      },
    ],
    proposalRevisions: [
      {
        proposalRevisionId: 'revision-1',
        proposal: { suggestedEpicName: 'Epic', sprints: [{ title: 'Sprint' }] },
      },
    ],
  }),
};

describe('Tauri Epic initiation confirmation adapter', () => {
  it('uses only request, event, and resolve contracts', async () => {
    const invoke = vi.fn(async (command: string) =>
      command.startsWith('request_')
        ? request
        : { requestId: 'request-1', state: 'projected', initiation },
    );
    let handler: ((event: { payload: unknown }) => void) | undefined;
    const listen = vi.fn(async (_event: string, next: (event: { payload: unknown }) => void) => {
      handler = next;
      return vi.fn();
    });
    const client = createTauriEpicInitiationConfirmationClient(
      native as never,
      invoke as never,
      listen,
    );
    const input = {
      epicPlanningDraftId: 'draft-1',
      expectedRevisionToken: 'token',
      idempotencyKey: 'key',
    };
    await expect(client.request(input)).resolves.toEqual(request);
    await expect(client.resolve('request-1', 'confirmed')).resolves.toMatchObject({
      state: 'projected',
    });
    const received = vi.fn();
    const malformed = vi.fn();
    await client.subscribe(received, malformed);
    handler?.({ payload: { request, state: 'requested' } });
    handler?.({ payload: { request, state: 'unknown' } });
    expect(received).toHaveBeenCalledOnce();
    expect(malformed).toHaveBeenCalledOnce();
    expect(invoke).toHaveBeenNthCalledWith(1, 'request_epic_initiation_confirmation', { input });
    expect(invoke).toHaveBeenNthCalledWith(2, 'resolve_epic_initiation_confirmation', {
      input: { requestId: 'request-1', decision: 'confirmed' },
    });
    expect(invoke.mock.calls.flat()).not.toContain('initiate_epic');
  });
});
