import type {
  AgentInvocationStatusDto,
  AgentRuntimeEventDto,
  AgentSessionDetailsDto,
} from '../../application/agentSessions';
import {
  AgentSessionTranscriptProjectionCache,
  projectAgentSessionTranscript,
  projectedTranscriptContent,
  selectLatestFinalAgentResponseRange,
  selectTranscriptInvocation,
  selectTranscriptRange,
} from './transcriptProjector';

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
      'npm test',
      'A partial answer',
    ]);
    expect(projected.invocations[0].finalResponse).toBeNull();
  });

  it('projects paired tool lifecycle events as one logical activity while retaining both raw events', () => {
    const activity = (phase: 'started' | 'completed') => ({
      kind: 'mcp_tool' as const,
      phase,
      itemId: 'item-5',
      server: null,
      tool: null,
      status: null,
      resultClassification: 'unknown' as const,
    });
    const started = event(1, 'tool_activity', null);
    started.normalized!.toolActivity = activity('started');
    started.rawPayload = { provider: 'started evidence' };
    const completed = event(2, 'tool_activity', null);
    completed.normalized!.toolActivity = activity('completed');
    completed.rawPayload = { provider: 'completed evidence' };

    const invocation = projectAgentSessionTranscript(details('completed', [started, completed]))
      .invocations[0];

    expect(invocation.processing).toHaveLength(1);
    expect(invocation.processing[0]).toMatchObject({ text: 'mcp tool call' });
    expect(invocation.processing[0].rawPayload).toMatchObject({
      lifecycleEvents: [started.rawPayload, completed.rawPayload],
    });
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

    expect(projected.invocations[0].finalResponse).toMatchObject({
      eventId: 'event-invocation-1-4',
      text: 'Comprehensive final response',
      anchor: { sessionId: 'session-1', invocationId: 'invocation-1', kind: 'final_response' },
    });
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

  it('decodes legacy persisted stderr bytes for readable technical presentation', () => {
    const stderr = event(1, 'unknown', null);
    stderr.source = 'stderr';
    stderr.normalized = null;
    stderr.rawPayload = { bytes: [119, 97, 114, 110], lossyUtf8: 'warning\n' };

    const invocation = projectAgentSessionTranscript(details('failed', [stderr])).invocations[0];

    expect(invocation.technical[0].text).toBe('warning');
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

  it('creates durable anchors and selects an inclusive excerpt', () => {
    const projected = projectAgentSessionTranscript(
      details('completed', [
        event(1, 'processing_update', 'Thinking'),
        event(2, 'agent_message', 'Done', { role: 'final' }),
      ]),
    );
    const content = projectedTranscriptContent(projected);
    expect(content.map((item) => item.kind)).toEqual([
      'submitted_input',
      'activity',
      'final_response',
      'outcome',
    ]);
    expect(
      selectTranscriptRange(projected, { start: content[1].anchor, end: content[2].anchor }),
    ).toEqual(content.slice(1, 3));
  });

  it('returns no content for stale or reversed anchors', () => {
    const projected = projectAgentSessionTranscript(
      details('completed', [event(1, 'processing_update', 'Thinking')]),
    );
    const content = projectedTranscriptContent(projected);
    expect(
      selectTranscriptRange(projected, {
        start: { ...content[0].anchor, sessionId: 'missing' },
        end: content[1].anchor,
      }),
    ).toEqual([]);
    expect(
      selectTranscriptRange(projected, { start: content[1].anchor, end: content[0].anchor }),
    ).toEqual([]);
  });

  it('anchors only the latest final agent response without older turns or user input', () => {
    const value = details();
    value.invocations = [
      invocation('invocation-1', 'Older user input', '2026-07-10T12:01:00.000Z', [
        event(1, 'agent_message', 'Older final', { role: 'final' }, 'invocation-1'),
      ]),
      invocation('invocation-2', 'Latest user input', '2026-07-10T12:02:00.000Z', [
        event(1, 'agent_message', 'Latest final', { role: 'final' }, 'invocation-2'),
      ]),
    ];
    const projected = projectAgentSessionTranscript(value);
    const range = selectLatestFinalAgentResponseRange(projected);

    expect(range).not.toBeNull();
    expect(selectTranscriptRange(projected, range!)).toMatchObject([
      {
        kind: 'final_response',
        response: { text: 'Latest final' },
        anchor: { invocationId: 'invocation-2', kind: 'final_response' },
      },
    ]);
  });

  it('returns no latest-response range when the transcript has no final agent response', () => {
    expect(
      selectLatestFinalAgentResponseRange(projectAgentSessionTranscript(details())),
    ).toBeNull();
  });

  it('selects an invocation only with matching Session and invocation identities', () => {
    const projected = projectAgentSessionTranscript(details('completed'));

    expect(selectTranscriptInvocation(projected, 'session-1', 'invocation-1')?.id).toBe(
      'invocation-1',
    );
    expect(selectTranscriptInvocation(projected, 'other-session', 'invocation-1')).toBeNull();
    expect(selectTranscriptInvocation(projected, 'session-1', 'other-invocation')).toBeNull();
  });

  it('incrementally projects appended events while preserving unchanged invocations', () => {
    const first = details('running', [event(1, 'processing_update', 'First')]);
    first.invocations.push(invocation('invocation-2', 'Unchanged', '2026-07-10T12:01:00.000Z', []));
    const cache = new AgentSessionTranscriptProjectionCache();
    const before = cache.project(first);
    const next = {
      ...first,
      invocations: [
        {
          ...first.invocations[0],
          events: [...first.invocations[0].events, event(2, 'processing_update', 'Second')],
        },
        first.invocations[1],
      ],
    };

    const after = cache.project(next);
    expect(after.invocations[0].processing.map((item) => item.text)).toEqual(['First', 'Second']);
    expect(after.invocations[1]).toBe(before.invocations[1]);
  });

  it('projects a 10,000-event history and preserves the terminal response', () => {
    const events = Array.from({ length: 9_999 }, (_, index) =>
      event(index + 1, 'processing_update', `Update ${index + 1}`),
    );
    events.push(event(10_000, 'agent_message', 'Finished', { role: 'final' }));

    const projected = projectAgentSessionTranscript(details('completed', events));

    expect(projected.invocations[0].processing).toHaveLength(9_999);
    expect(projected.invocations[0].finalResponse?.text).toBe('Finished');
    expect(projected.invocations[0].outcome.status).toBe('completed');
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
      runtimeBinding: { externalContextId: null, runtimeVersion: null },
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
      inputProvenance: 'user' as const,
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
    observation: {
      launchAcceptedAt: null,
      externalContext: null,
      providerActivity: null,
      providerTerminal: null,
      processTerminal:
        status === 'running'
          ? null
          : {
              status,
              completedAt: createdAt,
              exitCode: status === 'completed' ? 0 : null,
              signal: null,
            },
      mcpToolActivities: [],
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
    normalized: { kind, text, externalContextId: null, usage: null, details, toolActivity: null },
    recordedAt: new Date(Date.parse(timestamp) + sequence * 1000).toISOString(),
  };
}
