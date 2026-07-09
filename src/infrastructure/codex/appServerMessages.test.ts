import {
  CodexAppServerMessageParseError,
  parseCodexAppServerMessages,
  summarizeCodexAppServerMessages,
} from './appServerMessages';

describe('parseCodexAppServerMessages', () => {
  it('parses app-server responses and notifications while preserving raw objects', () => {
    const jsonl = [
      JSON.stringify({
        id: 1,
        result: { thread: { id: 'thread-123' } },
        future_field: true,
      }),
      JSON.stringify({
        method: 'turn/started',
        params: { turn: { id: 'turn-456' } },
      }),
      JSON.stringify({
        method: 'item/completed',
        params: { item: { type: 'agent_message', text: 'Done' } },
      }),
    ].join('\n');

    const messages = parseCodexAppServerMessages(jsonl);

    expect(messages).toEqual([
      {
        kind: 'response',
        lineNumber: 1,
        id: 1,
        result: { thread: { id: 'thread-123' } },
        raw: {
          id: 1,
          result: { thread: { id: 'thread-123' } },
          future_field: true,
        },
      },
      {
        kind: 'notification',
        lineNumber: 2,
        method: 'turn/started',
        params: { turn: { id: 'turn-456' } },
        raw: {
          method: 'turn/started',
          params: { turn: { id: 'turn-456' } },
        },
      },
      {
        kind: 'notification',
        lineNumber: 3,
        method: 'item/completed',
        params: { item: { type: 'agent_message', text: 'Done' } },
        raw: {
          method: 'item/completed',
          params: { item: { type: 'agent_message', text: 'Done' } },
        },
      },
    ]);
  });

  it('preserves unknown message envelopes without throwing', () => {
    expect(parseCodexAppServerMessages(JSON.stringify({ heartbeat: true }))).toEqual([
      {
        kind: 'unknown',
        lineNumber: 1,
        raw: { heartbeat: true },
      },
    ]);
  });

  it('ignores blank lines', () => {
    expect(parseCodexAppServerMessages(`${JSON.stringify({ method: 'ready' })}\n\n`)).toEqual([
      {
        kind: 'notification',
        lineNumber: 1,
        method: 'ready',
        raw: { method: 'ready' },
      },
    ]);
  });

  it('throws line-numbered errors for malformed JSON-RPC-ish envelopes', () => {
    expectParseErrorContaining('{', 1, 'Invalid JSON');
    expectParseError('[]', 1, 'Message line must be a JSON object');
    expectParseError(JSON.stringify({ id: true }), 1, 'Response id must be a string or number');
    expectParseError(
      JSON.stringify({ id: 1, result: 'ok' }),
      1,
      'Response result must be a JSON object',
    );
    expectParseError(JSON.stringify({ method: 42 }), 1, 'Notification method must be a string');
    expectParseError(
      JSON.stringify({ method: 'turn/started', params: 'bad' }),
      1,
      'Notification params must be a JSON object',
    );
  });
});

describe('summarizeCodexAppServerMessages', () => {
  it('extracts thread, turn, terminal, notification counts, and token usage updates', () => {
    const messages = parseCodexAppServerMessages(
      [
        JSON.stringify({ id: 1, result: { thread: { id: 'thread-abc' } } }),
        JSON.stringify({ id: 2, result: {} }),
        JSON.stringify({ method: 'turn/started', params: { turn: { id: 'turn-def' } } }),
        JSON.stringify({
          method: 'thread/tokenUsage/updated',
          params: {
            threadId: 'thread-abc',
            usage: { input_tokens: 10, cached_input_tokens: 8 },
          },
        }),
        JSON.stringify({
          method: 'turn/completed',
          params: { threadId: 'thread-abc', turnId: 'turn-def', status: 'completed' },
        }),
      ].join('\n'),
    );

    expect(summarizeCodexAppServerMessages(messages)).toEqual({
      threadId: 'thread-abc',
      turnId: 'turn-def',
      terminalTurnStatus: { threadId: 'thread-abc', turnId: 'turn-def', status: 'completed' },
      responseCount: 2,
      errorResponseCount: 0,
      notificationCountsByMethod: {
        'turn/started': 1,
        'thread/tokenUsage/updated': 1,
        'turn/completed': 1,
      },
      tokenUsageUpdates: [
        {
          threadId: 'thread-abc',
          usage: { input_tokens: 10, cached_input_tokens: 8 },
        },
      ],
    });
  });

  it('counts error responses', () => {
    const messages = parseCodexAppServerMessages(
      JSON.stringify({ id: 1, error: { code: -32601, message: 'Method not found' } }),
    );

    expect(summarizeCodexAppServerMessages(messages).errorResponseCount).toBe(1);
  });
});

function expectParseError(jsonl: string, lineNumber: number, message: string): void {
  expect(() => parseCodexAppServerMessages(jsonl)).toThrow(
    new CodexAppServerMessageParseError(lineNumber, message),
  );
}

function expectParseErrorContaining(jsonl: string, lineNumber: number, message: string): void {
  try {
    parseCodexAppServerMessages(jsonl);
  } catch (error) {
    expect(error).toBeInstanceOf(CodexAppServerMessageParseError);
    expect((error as CodexAppServerMessageParseError).lineNumber).toBe(lineNumber);
    expect((error as CodexAppServerMessageParseError).message).toContain(message);
    return;
  }

  throw new Error('Expected CodexAppServerMessageParseError');
}
