import { deferredEventIds, restoreDeferredPayload, fullActionDetails } from './deferredToolDetails';
import { runtimeEvent, sessionDetails } from './testFixtures';
it('restores both sides of coalesced actions by their exact durable event IDs', () => {
  const start = runtimeEvent(1, 'tool_activity', 'Start');
  start.rawPayload = { command: 'full command' };
  const end = runtimeEvent(2, 'tool_activity', 'End');
  end.rawPayload = { output: 'full output' };
  const payload = {
    started: { _orchidDeferredEventId: start.id, command: 'preview' },
    completed: { _orchidDeferredEventId: end.id, output: 'preview' },
  };
  expect(deferredEventIds(payload)).toEqual([start.id, end.id]);
  expect(
    restoreDeferredPayload(
      payload,
      new Map([
        [start.id, start],
        [end.id, end],
      ]),
    ),
  ).toEqual({ started: start.rawPayload, completed: end.rawPayload });
});
it('hydrates only deferred actions for the full-detail Markdown export', async () => {
  const event = runtimeEvent(1, 'tool_activity', 'command');
  event.rawPayload = { _orchidDeferredEventId: event.id, command: 'preview' };
  const details = sessionDetails('completed', [event]);
  const full = { ...event, rawPayload: { command: 'exact command', output: 'exact output' } };
  const load = vi.fn(async () => full);
  const result = await fullActionDetails(details, load);
  expect(load).toHaveBeenCalledWith(event.id);
  expect(result.invocations[0].events[0]).toEqual(full);
  expect(details.invocations[0].events[0]).toEqual(event);
});
