import {
  CodexJsonlParseError,
  parseCodexJsonlEvents,
  summarizeCodexJsonlEvents,
} from './jsonlEvents';

describe('parseCodexJsonlEvents', () => {
  it('parses documented-like Codex JSONL events and preserves raw objects', () => {
    const jsonl = [
      JSON.stringify({
        type: 'thread.started',
        thread_id: 'thread-123',
        unexpected_future_field: true,
      }),
      JSON.stringify({ type: 'turn.started' }),
      JSON.stringify({
        type: 'item.completed',
        item: { type: 'agent_message', text: 'First answer', extra: 'preserved' },
      }),
      JSON.stringify({
        type: 'item.completed',
        item: { type: 'reasoning', summary: 'Reasoned briefly' },
      }),
      JSON.stringify({
        type: 'item.completed',
        item: { type: 'agent_message', text: 'Final answer' },
      }),
      JSON.stringify({
        type: 'turn.completed',
        usage: { input_tokens: 11, output_tokens: 7 },
      }),
    ].join('\n');

    const events = parseCodexJsonlEvents(jsonl);

    expect(events).toHaveLength(6);
    expect(events[0]).toMatchObject({
      type: 'thread.started',
      lineNumber: 1,
      threadId: 'thread-123',
      raw: {
        type: 'thread.started',
        thread_id: 'thread-123',
        unexpected_future_field: true,
      },
    });
    expect(events[2]).toMatchObject({
      type: 'item.completed',
      item: {
        type: 'agent_message',
        text: 'First answer',
        raw: { type: 'agent_message', text: 'First answer', extra: 'preserved' },
      },
    });
  });

  it('ignores trailing blank lines', () => {
    const events = parseCodexJsonlEvents(`${JSON.stringify({ type: 'turn.started' })}\n\n  \n`);

    expect(events).toEqual([
      {
        type: 'turn.started',
        lineNumber: 1,
        raw: { type: 'turn.started' },
      },
    ]);
  });

  it('preserves unknown event and item types without throwing', () => {
    const events = parseCodexJsonlEvents(
      [
        JSON.stringify({ type: 'future.event', payload: { ok: true } }),
        JSON.stringify({ type: 'item.completed', item: { type: 'future_item', value: 1 } }),
      ].join('\n'),
    );

    expect(events).toEqual([
      {
        type: 'future.event',
        lineNumber: 1,
        raw: { type: 'future.event', payload: { ok: true } },
        known: false,
      },
      {
        type: 'item.completed',
        lineNumber: 2,
        raw: { type: 'item.completed', item: { type: 'future_item', value: 1 } },
        item: {
          type: 'future_item',
          raw: { type: 'future_item', value: 1 },
          known: false,
        },
      },
    ]);
  });

  it('throws line-numbered errors for invalid JSON', () => {
    expectParseErrorContaining(`${JSON.stringify({ type: 'turn.started' })}\n{`, 2, 'Invalid JSON');
  });

  it('throws line-numbered errors for non-object JSON', () => {
    expectParseError('[]', 1, 'Event line must be a JSON object');
  });

  it('throws line-numbered errors for missing and malformed event types', () => {
    expectParseError(JSON.stringify({ item: {} }), 1, 'Event type is required');
    expectParseError(JSON.stringify({ type: 42 }), 1, 'Event type must be a string');
  });

  it('throws line-numbered errors for malformed known event envelopes', () => {
    expectParseError(
      JSON.stringify({ type: 'thread.started', thread_id: 42 }),
      1,
      'thread.started thread_id must be a string',
    );
    expectParseError(
      JSON.stringify({ type: 'turn.completed', usage: 'many tokens' }),
      1,
      'turn.completed usage must be a JSON object',
    );
    expectParseError(
      JSON.stringify({ type: 'item.completed', item: { text: 'missing type' } }),
      1,
      'Item type is required',
    );
    expectParseError(
      JSON.stringify({ type: 'item.completed', item: { type: 'agent_message', text: 42 } }),
      1,
      'agent_message text must be a string',
    );
  });
});

describe('summarizeCodexJsonlEvents', () => {
  it('extracts thread id, final message, terminal status, usage, and item counts', () => {
    const events = parseCodexJsonlEvents(
      [
        JSON.stringify({ type: 'thread.started', thread_id: 'thread-abc' }),
        JSON.stringify({
          type: 'item.started',
          item: { type: 'command_execution', command: 'npm test' },
        }),
        JSON.stringify({
          type: 'item.completed',
          item: { type: 'agent_message', text: 'Earlier response' },
        }),
        JSON.stringify({ type: 'item.completed', item: { type: 'web_search', query: 'docs' } }),
        JSON.stringify({
          type: 'item.completed',
          item: { type: 'agent_message', text: 'Final response' },
        }),
        JSON.stringify({ type: 'turn.completed', usage: { total_tokens: 42 } }),
      ].join('\n'),
    );

    expect(summarizeCodexJsonlEvents(events)).toEqual({
      threadId: 'thread-abc',
      finalAgentMessageText: 'Final response',
      terminalStatus: { kind: 'completed', lineNumber: 6 },
      tokenUsage: { total_tokens: 42 },
      itemCountsByType: {
        agent_message: 2,
        command_execution: 1,
        web_search: 1,
      },
    });
  });

  it('uses the last terminal status from completed, failed, or error events', () => {
    const events = parseCodexJsonlEvents(
      [
        JSON.stringify({ type: 'turn.completed' }),
        JSON.stringify({ type: 'turn.failed', error: { message: 'failed later' } }),
        JSON.stringify({ type: 'error', message: 'fatal later' }),
      ].join('\n'),
    );

    expect(summarizeCodexJsonlEvents(events).terminalStatus).toEqual({
      kind: 'error',
      lineNumber: 3,
    });
  });
});

function expectParseError(jsonl: string, lineNumber: number, message: string): void {
  expect(() => parseCodexJsonlEvents(jsonl)).toThrow(new CodexJsonlParseError(lineNumber, message));
}

function expectParseErrorContaining(jsonl: string, lineNumber: number, message: string): void {
  try {
    parseCodexJsonlEvents(jsonl);
  } catch (error) {
    expect(error).toBeInstanceOf(CodexJsonlParseError);
    expect((error as CodexJsonlParseError).lineNumber).toBe(lineNumber);
    expect((error as CodexJsonlParseError).message).toContain(message);
    return;
  }

  throw new Error('Expected CodexJsonlParseError');
}
