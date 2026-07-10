import type {
  AgentInvocationStatusDto,
  AgentRuntimeEventDto,
  AgentSessionDetailsDto,
} from '../../application/agentSessions';
import { projectAgentSessionTranscript } from './transcriptProjector';

const timestamp = '2026-07-10T12:00:00.000Z';

describe('projectAgentSessionTranscript', () => {
  it('projects an empty durable session', () => {
    expect(projectAgentSessionTranscript(details())).toEqual({
      sessionId: 'session-1',
      invocations: [],
      activeInvocationId: null,
    });
  });

  it('keeps live processing, tools, and intermediate messages visible while running', () => {
    const projected = projectAgentSessionTranscript(
      details('running', [
        event(3, 'agent_message', 'A partial answer', { role: 'intermediate' }),
        event(1, 'processing_started', null),
        event(2, 'tool_activity', 'npm test'),
      ]),
    );

    expect(projected.activeInvocationId).toBe('invocation-1');
    expect(projected.invocations[0].processing.map((item) => item.text)).toEqual([
      'Processing started',
      'npm test',
      'A partial answer',
    ]);
    expect(projected.invocations[0].finalResponse).toBeNull();
  });

  it('keeps only the last backend-final message prominent and groups earlier agent messages', () => {
    const projected = projectAgentSessionTranscript(
      details('completed', [
        event(1, 'agent_message', 'First update', { role: 'intermediate' }),
        event(2, 'agent_message', 'Earlier candidate', { role: 'final' }),
        event(3, 'tool_activity', 'Checked files'),
        event(4, 'agent_message', 'Comprehensive final response', { role: 'final' }),
      ]),
    );

    expect(projected.invocations[0].finalResponse).toBe('Comprehensive final response');
    expect(projected.invocations[0].processing.map((item) => item.text)).toEqual([
      'First update',
      'Earlier candidate',
      'Checked files',
    ]);
  });

  it('does not fabricate a final response when completion has none', () => {
    const invocation = projectAgentSessionTranscript(
      details('completed', [event(1, 'processing_update', 'Did work')]),
    ).invocations[0];
    expect(invocation.finalResponse).toBeNull();
    expect(invocation.outcome).toEqual({ status: 'completed', label: 'Completed', message: null });
  });

  it.each([
    ['failed', 'Runtime exploded', 'Failed'],
    ['canceled', null, 'Canceled'],
    ['interrupted', null, 'Interrupted'],
  ] as const)('projects explicit %s outcomes without a final answer', (status, error, label) => {
    const invocation = projectAgentSessionTranscript(details(status, [], error)).invocations[0];
    expect(invocation.finalResponse).toBeNull();
    expect(invocation.outcome.label).toBe(label);
    expect(invocation.outcome.message).toBeTruthy();
  });

  it('preserves malformed, unknown, and stderr activity as technical detail', () => {
    const malformed = event(1, 'unknown', null, { diagnostic: 'bad json' });
    malformed.rawPayload = '{bad';
    const stderr = event(2, 'processing_update', 'warning');
    stderr.source = 'stderr';
    const invocation = projectAgentSessionTranscript(details('failed', [malformed, stderr]))
      .invocations[0];
    expect(invocation.technical.map((item) => item.text)).toEqual(['{bad', 'warning']);
  });

  it('orders invocations and their events deterministically', () => {
    const value = details();
    value.invocations = [
      invocation('invocation-b', 'Second', '2026-07-10T12:02:00.000Z', [
        event(2, 'agent_message', 'Second final', { role: 'final' }, 'invocation-b'),
      ]),
      invocation('invocation-a', 'First', '2026-07-10T12:01:00.000Z', [
        event(1, 'agent_message', 'First final', { role: 'final' }, 'invocation-a'),
      ]),
    ];
    expect(
      projectAgentSessionTranscript(value).invocations.map((item) => item.submittedText),
    ).toEqual(['First', 'Second']);
  });
});

function details(
  status?: AgentInvocationStatusDto,
  events: AgentRuntimeEventDto[] = [],
  runtimeError: string | null = null,
): AgentSessionDetailsDto {
  return {
    session: {
      id: 'session-1',
      title: 'Session',
      availability: 'available',
      runtimeBinding: { kind: 'codex_cli', externalContextId: null, runtimeVersion: null },
      workingDirectory: null,
      requestedOptions: { model: null, sandbox: null },
      createdAt: timestamp,
      updatedAt: timestamp,
    },
    invocations: status
      ? [invocation('invocation-1', 'Please do it', timestamp, events, status, runtimeError)]
      : [],
  };
}

function invocation(
  id: string,
  submittedText: string,
  createdAt: string,
  events: AgentRuntimeEventDto[],
  status: AgentInvocationStatusDto = 'completed',
  runtimeError: string | null = null,
) {
  return {
    invocation: {
      id,
      sessionId: 'session-1',
      submittedText,
      status,
      requestedOptions: { model: null, sandbox: null },
      effectiveOptions: null,
      startedAt: createdAt,
      completedAt: status === 'running' ? null : createdAt,
      exitCode: status === 'completed' ? 0 : null,
      signal: null,
      runtimeError: runtimeError
        ? { code: 'runtime_failed', message: runtimeError, details: null }
        : null,
      diagnostics: [],
      createdAt,
      updatedAt: createdAt,
    },
    events,
  };
}

function event(
  sequence: number,
  kind: NonNullable<AgentRuntimeEventDto['normalized']>['kind'],
  text: string | null,
  details: unknown = null,
  invocationId = 'invocation-1',
): AgentRuntimeEventDto {
  return {
    id: `event-${invocationId}-${sequence}`,
    invocationId,
    sequence,
    source: 'stdout',
    rawPayload: { kind },
    normalized: { kind, text, externalContextId: null, usage: null, details },
    recordedAt: new Date(Date.parse(timestamp) + sequence * 1000).toISOString(),
  };
}
