import type {
  AgentInvocationStatusDto,
  AgentRuntimeEventDto,
  AgentSessionDetailsDto,
  AgentSessionSummaryDto,
} from '../../application/agentSessions';

export const fixtureTime = '2026-07-10T12:00:00.000Z';

export function sessionDetails(
  status: AgentInvocationStatusDto | null = null,
  events: AgentRuntimeEventDto[] = [],
): AgentSessionDetailsDto {
  return {
    session: {
      id: 'session-1',
      title: 'Durable session',
      availability: 'available',
      runtimeBinding: { kind: 'codex_cli', externalContextId: 'thread-1', runtimeVersion: 'test' },
      workingDirectory: 'C:/workspace',
      requestedOptions: { model: null, sandbox: null },
      createdAt: fixtureTime,
      updatedAt: fixtureTime,
    },
    invocations: status
      ? [
          {
            invocation: {
              id: 'invocation-1',
              sessionId: 'session-1',
              submittedText: 'Do the work',
              status,
              requestedOptions: { model: null, sandbox: null },
              effectiveOptions: null,
              startedAt: fixtureTime,
              completedAt: status === 'running' ? null : fixtureTime,
              exitCode: status === 'completed' ? 0 : null,
              signal: null,
              runtimeError:
                status === 'failed'
                  ? { code: 'failed', message: 'Runtime failed', details: null }
                  : null,
              diagnostics: [],
              createdAt: fixtureTime,
              updatedAt: fixtureTime,
            },
            events,
          },
        ]
      : [],
  };
}

export function runtimeEvent(
  sequence: number,
  kind: NonNullable<AgentRuntimeEventDto['normalized']>['kind'],
  text: string | null,
  details: unknown = null,
): AgentRuntimeEventDto {
  return {
    id: `event-${sequence}`,
    invocationId: 'invocation-1',
    sequence,
    source: 'stdout',
    rawPayload: { kind },
    normalized: { kind, text, externalContextId: null, usage: null, details },
    recordedAt: fixtureTime,
  };
}

export function sessionSummary(active = false): AgentSessionSummaryDto {
  return {
    id: 'session-1',
    title: 'Durable session',
    availability: 'available',
    runtimeKind: 'codex_cli',
    hasActiveInvocation: active,
    createdAt: fixtureTime,
    updatedAt: fixtureTime,
  };
}
