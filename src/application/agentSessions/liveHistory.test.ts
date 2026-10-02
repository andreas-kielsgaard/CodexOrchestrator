import { reduceAgentSessionHistory } from './liveHistory';
import { runtimeEvent, sessionDetails } from '../../features/agentSessions/testFixtures';

it('updates held steering in cached history and anchors it at handoff without a database reload', () => {
  let details = sessionDetails('running');
  const records = [
    { kind: 'session_steering_queued', inputId: 's', text: 'Original' },
    { kind: 'session_steering_result', id: 's', state: 'editing' },
    { kind: 'session_steering_queued', inputId: 's', text: 'Revised' },
    { kind: 'session_steering_pending', inputId: 's', text: 'Revised' },
    { kind: 'session_steering_result', id: 's', state: 'accepted' },
  ];
  records.forEach((raw, index) => {
    const event = runtimeEvent(index + 1, 'unknown', null);
    event.source = 'runtime';
    event.rawPayload = raw;
    const result = reduceAgentSessionHistory(details, {
      kind: 'event_persisted',
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      event,
    });
    expect(result.kind).toBe('changed');
    if (result.kind === 'changed') details = result.details;
    expect(details.interactions).toHaveLength(1);
  });
  expect(details.interactions![0]).toMatchObject({
    state: 'accepted',
    sequence: 4,
    content: { text: 'Revised' },
  });
});

it('makes a cached pending invocation steerable when its runtime starts', () => {
  const details = sessionDetails('pending');
  details.invocations[0].invocation.startedAt = null;
  const event = runtimeEvent(1, 'processing_started', null);
  const result = reduceAgentSessionHistory(details, {
    kind: 'event_persisted',
    sessionId: details.session.id,
    invocationId: event.invocationId,
    event,
  });
  expect(result.kind).toBe('changed');
  if (result.kind !== 'changed') throw new Error('Expected a changed history');
  expect(result.details.invocations[0].invocation.status).toBe('running');
  expect(result.details.invocations[0].invocation.startedAt).toBe(event.recordedAt);
  expect(details.invocations[0].invocation.status).toBe('pending');
});

it('does not revive a terminal invocation from a delayed start event', () => {
  const details = sessionDetails('completed');
  const event = runtimeEvent(1, 'processing_started', null);
  const result = reduceAgentSessionHistory(details, {
    kind: 'event_persisted',
    sessionId: details.session.id,
    invocationId: event.invocationId,
    event,
  });
  expect(result.kind).toBe('changed');
  if (result.kind === 'changed')
    expect(result.details.invocations[0].invocation.status).toBe('completed');
});
