import type {
  AgentDiagnosticDto,
  AgentInvocationDto,
  AgentRuntimeEventDto,
  NormalizedRuntimeEventKindDto,
} from '../../application/agentSessions';

export type RecordedStep =
  | {
      readonly kind: 'event';
      readonly sessionId: string;
      readonly invocationId: string;
      readonly event: AgentRuntimeEventDto;
    }
  | {
      readonly kind: 'diagnostic';
      readonly sessionId: string;
      readonly invocationId: string;
      readonly diagnostic: AgentDiagnosticDto;
    }
  | {
      readonly kind: 'terminal';
      readonly sessionId: string;
      readonly invocationId: string;
      readonly invocation: AgentInvocationDto;
    };

export interface RecordedAgentSessionScenario {
  readonly name: string;
  readonly sessions: readonly {
    readonly sessionId: string;
    readonly invocationId: string;
    readonly submittedText: string;
  }[];
  readonly steps: readonly RecordedStep[];
  readonly failures?: Partial<Record<'subscribe' | 'load' | 'reload' | 'send' | 'cancel', string>>;
}

const time = '2026-07-10T12:00:00.000Z';
const event = (
  id: string,
  invocationId: string,
  sequence: number,
  kind: NormalizedRuntimeEventKindDto,
  text: string | null,
  source: AgentRuntimeEventDto['source'] = 'stdout',
  rawPayload: unknown = { fixture: id },
  details: unknown = null,
): AgentRuntimeEventDto => ({
  id,
  invocationId,
  sequence,
  source,
  rawPayload,
  normalized: { kind, text, externalContextId: null, usage: null, details },
  recordedAt: time,
});
const diagnostic = (invocationId: string, code: string, message: string): AgentDiagnosticDto => ({
  source: 'runtime',
  severity: 'warning',
  code,
  message,
  details: null,
  recordedAt: time,
});
const terminal = (
  id: string,
  sessionId: string,
  text: string,
  status: AgentInvocationDto['status'] = 'completed',
): AgentInvocationDto => ({
  id,
  sessionId,
  submittedText: text,
  status,
  requestedOptions: { model: null, sandbox: null },
  effectiveOptions: { model: null, sandbox: null },
  startedAt: time,
  completedAt: time,
  exitCode: status === 'completed' ? 0 : null,
  signal: null,
  runtimeError:
    status === 'interrupted'
      ? { code: 'interrupted', message: 'Interrupted by recorded runtime', details: null }
      : null,
  diagnostics: [],
  createdAt: time,
  updatedAt: time,
});

const liveSteps: RecordedStep[] = [
  {
    kind: 'event',
    sessionId: 'live-session',
    invocationId: 'live-invocation',
    event: event('live-1', 'live-invocation', 1, 'processing_started', 'Processing started'),
  },
  {
    kind: 'event',
    sessionId: 'live-session',
    invocationId: 'live-invocation',
    event: event('live-2', 'live-invocation', 2, 'tool_activity', 'Reading files'),
  },
  {
    kind: 'event',
    sessionId: 'live-session',
    invocationId: 'live-invocation',
    event: event('live-3', 'live-invocation', 3, 'processing_update', 'Considering the result'),
  },
  {
    kind: 'event',
    sessionId: 'live-session',
    invocationId: 'live-invocation',
    event: event(
      'live-4',
      'live-invocation',
      4,
      'agent_message',
      'The final answer',
      'stdout',
      { role: 'final', markdown: true },
      { role: 'final' },
    ),
  },
  {
    kind: 'terminal',
    sessionId: 'live-session',
    invocationId: 'live-invocation',
    invocation: terminal('live-invocation', 'live-session', 'Inspect the repository'),
  },
];

const diagnosticSteps: RecordedStep[] = [
  {
    kind: 'event',
    sessionId: 'diagnostic-session',
    invocationId: 'diagnostic-invocation',
    event: event('diag-1', 'diagnostic-invocation', 1, 'unknown', null, 'stdout', {
      future: 'opaque',
    }),
  },
  {
    kind: 'event',
    sessionId: 'diagnostic-session',
    invocationId: 'diagnostic-invocation',
    event: event(
      'diag-2',
      'diagnostic-invocation',
      2,
      'runtime_error',
      'stderr text',
      'stderr',
      'raw stderr',
    ),
  },
  {
    kind: 'diagnostic',
    sessionId: 'diagnostic-session',
    invocationId: 'diagnostic-invocation',
    diagnostic: diagnostic('diagnostic-invocation', 'FUTURE_EVENT', 'Unknown runtime event'),
  },
];

const markdownSteps: RecordedStep[] = [
  {
    kind: 'event',
    sessionId: 'markdown-session',
    invocationId: 'markdown-invocation',
    event: event(
      'markdown-1',
      'markdown-invocation',
      1,
      'agent_message',
      '# Result\n\n- **GFM**\n- <span>safe raw HTML</span>',
      'stdout',
      { markdown: 'gfm' },
      { role: 'final' },
    ),
  },
  {
    kind: 'terminal',
    sessionId: 'markdown-session',
    invocationId: 'markdown-invocation',
    invocation: terminal('markdown-invocation', 'markdown-session', 'Render Markdown'),
  },
];

const outcomeSteps: RecordedStep[] = [
  {
    kind: 'terminal',
    sessionId: 'outcomes-session',
    invocationId: 'canceled-invocation',
    invocation: terminal('canceled-invocation', 'outcomes-session', 'Cancel me', 'canceled'),
  },
  {
    kind: 'terminal',
    sessionId: 'outcomes-session',
    invocationId: 'interrupted-invocation',
    invocation: terminal(
      'interrupted-invocation',
      'outcomes-session',
      'Interrupt me',
      'interrupted',
    ),
  },
];

const twoSessionSteps: RecordedStep[] = [
  {
    kind: 'event',
    sessionId: 'session-b',
    invocationId: 'invocation-b',
    event: event('session-b-1', 'invocation-b', 1, 'processing_update', 'Background B update'),
  },
  {
    kind: 'event',
    sessionId: 'session-a',
    invocationId: 'invocation-a',
    event: event('session-a-1', 'invocation-a', 1, 'processing_update', 'Selected A update'),
  },
];

Object.freeze(liveSteps);
Object.freeze(diagnosticSteps);
Object.freeze(markdownSteps);
Object.freeze(outcomeSteps);
Object.freeze(twoSessionSteps);

export const recordedAgentSessionScenarios = {
  liveProcessing: Object.freeze({
    name: 'live-processing',
    sessions: [
      {
        sessionId: 'live-session',
        invocationId: 'live-invocation',
        submittedText: 'Inspect the repository',
      },
    ],
    steps: liveSteps,
  }),
  errors: Object.freeze({
    name: 'operation-errors',
    sessions: [],
    failures: {
      subscribe: 'Recorded subscription failed',
      load: 'Recorded load failed',
      reload: 'Recorded reload failed',
      send: 'Recorded send failed',
      cancel: 'Recorded cancellation failed',
    },
    steps: [],
  }),
  diagnostics: Object.freeze({
    name: 'diagnostics',
    sessions: [
      {
        sessionId: 'diagnostic-session',
        invocationId: 'diagnostic-invocation',
        submittedText: 'Show diagnostics',
      },
    ],
    steps: diagnosticSteps,
  }),
  markdownGfm: Object.freeze({
    name: 'markdown-gfm',
    sessions: [
      {
        sessionId: 'markdown-session',
        invocationId: 'markdown-invocation',
        submittedText: 'Render Markdown',
      },
    ],
    steps: markdownSteps,
  }),
  outcomes: Object.freeze({
    name: 'outcomes',
    sessions: [
      {
        sessionId: 'outcomes-session',
        invocationId: 'canceled-invocation',
        submittedText: 'Cancel me',
      },
      {
        sessionId: 'outcomes-session',
        invocationId: 'interrupted-invocation',
        submittedText: 'Interrupt me',
      },
    ],
    steps: outcomeSteps,
  }),
  twoSessions: Object.freeze({
    name: 'two-sessions',
    sessions: [
      { sessionId: 'session-a', invocationId: 'invocation-a', submittedText: 'First session' },
      { sessionId: 'session-b', invocationId: 'invocation-b', submittedText: 'Second session' },
    ],
    steps: twoSessionSteps,
  }),
  longContent: Object.freeze({
    name: 'long-content',
    sessions: [
      {
        sessionId: 'long-session',
        invocationId: 'long-invocation',
        submittedText: 'A'.repeat(4096),
      },
    ],
    steps: [],
  }),
} satisfies Record<string, RecordedAgentSessionScenario>;

export type RecordedAgentSessionScenarioName = keyof typeof recordedAgentSessionScenarios;
