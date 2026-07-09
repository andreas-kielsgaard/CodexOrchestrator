import type {
  AgentInvocationDto,
  AgentRuntimeEventDto,
  AgentSessionDto,
  SendAgentSessionMessageCommandDto,
  SendAgentSessionMessageResultDto,
} from './contracts';

const timestamp = '2026-07-10T12:00:00Z';

describe('Agent Session client contracts', () => {
  it('serializes local identity separately from nullable external runtime context identity', () => {
    const unbound = session(null);
    const bound = session('runtime-external');

    expect(JSON.parse(JSON.stringify(unbound))).toMatchObject({
      id: 'session-local',
      runtimeBinding: {
        kind: 'codex_cli',
        externalContextId: null,
      },
    });
    expect(JSON.parse(JSON.stringify(bound))).toMatchObject({
      id: 'session-local',
      runtimeBinding: {
        externalContextId: 'runtime-external',
      },
    });
  });

  it('represents terminal invocation status independently from session availability', () => {
    const completed: AgentInvocationDto = {
      id: 'invocation-1',
      sessionId: 'session-local',
      submittedText: 'Do the work',
      status: 'completed',
      requestedOptions: { model: null, sandbox: null },
      effectiveOptions: { model: 'runtime-default', sandbox: 'workspace_write' },
      startedAt: timestamp,
      completedAt: '2026-07-10T12:00:01Z',
      exitCode: 0,
      signal: null,
      runtimeError: null,
      diagnostics: [],
      createdAt: timestamp,
      updatedAt: '2026-07-10T12:00:01Z',
    };

    expect({ session: session(null), invocation: completed }).toMatchObject({
      session: { availability: 'available' },
      invocation: { status: 'completed' },
    });
  });

  it('keeps raw runtime payload alongside optional provider-neutral normalization', () => {
    const event: AgentRuntimeEventDto = {
      id: 'event-1',
      invocationId: 'invocation-1',
      sequence: 9,
      source: 'stdout',
      rawPayload: { type: 'provider.future_event', untouched: true },
      normalized: {
        kind: 'unknown',
        text: null,
        externalContextId: null,
        usage: null,
        details: null,
      },
      recordedAt: timestamp,
    };

    expect(JSON.parse(JSON.stringify(event))).toEqual(event);
  });

  it('correlates send acknowledgements with both session and invocation IDs', () => {
    const command: SendAgentSessionMessageCommandDto = {
      sessionId: 'session-local',
      submittedText: 'Continue',
    };
    const acknowledgement: SendAgentSessionMessageResultDto = {
      sessionId: command.sessionId!,
      invocationId: 'invocation-2',
    };

    expect(acknowledgement).toEqual({
      sessionId: 'session-local',
      invocationId: 'invocation-2',
    });
  });
});

function session(externalContextId: string | null): AgentSessionDto {
  return {
    id: 'session-local',
    title: 'Agent session',
    availability: 'available',
    runtimeBinding: {
      kind: 'codex_cli',
      externalContextId,
      runtimeVersion: 'runtime-test',
    },
    workingDirectory: 'C:/work/session-local',
    requestedOptions: { model: null, sandbox: null },
    effectiveOptions: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}
