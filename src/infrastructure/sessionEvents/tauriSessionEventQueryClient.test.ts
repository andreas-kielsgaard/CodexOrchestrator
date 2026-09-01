import { vi } from 'vitest';
import type { ReferenceIdentityDto } from '../../application/sessionEvents';
import { createTauriSessionEventQueryClient } from './tauriSessionEventQueryClient';

describe('Tauri Session Event query client', () => {
  it('preserves complete reference identities at each query boundary', async () => {
    const invoke = vi.fn().mockResolvedValue(null);
    const client = createTauriSessionEventQueryClient(invoke);
    const eventGroupId: ReferenceIdentityDto = {
      namespace: 'workflow',
      kind: 'event_group',
      id: 'event-group-1',
    };
    const session: ReferenceIdentityDto = {
      namespace: 'orchestrator.agent_sessions',
      kind: 'session',
      id: 'session-1',
    };

    await client.loadEventGroup(eventGroupId);
    await client.loadRecordedEvent(eventGroupId);
    await client.listDeliveriesForGroup(eventGroupId);
    await client.listDeliveriesForSession(session);

    expect(invoke.mock.calls).toEqual([
      ['load_session_event_group', { query: { eventGroupId } }],
      ['load_recorded_session_event', { query: { eventGroupId } }],
      ['list_session_event_deliveries_for_group', { query: { eventGroupId } }],
      ['list_session_event_deliveries_for_session', { query: { session } }],
    ]);
  });
});
