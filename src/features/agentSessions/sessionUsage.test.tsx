import { render, screen } from '@testing-library/react';
import { contextUsage, turnUsage } from './sessionUsage';
import { ContextUsageIndicator } from './ContextUsageIndicator';
import { runtimeEvent } from './testFixtures';

function report(
  sequence: number,
  input: number,
  output: number,
  cached: number,
  totalInput: number,
) {
  const event = runtimeEvent(sequence, 'usage', null);
  const last = {
    inputTokens: input,
    outputTokens: output,
    cachedInputTokens: cached,
    totalTokens: input + output,
  };
  event.normalized!.usage = last;
  event.rawPayload = {
    tokenUsage: { last, total: { ...last, inputTokens: totalInput }, modelContextWindow: 100000 },
  };
  return event;
}

it('uses request context rather than cumulative thread tokens and displays a ring', () => {
  const events = [report(1, 49000, 1000, 40000, 900000)];
  const usage = contextUsage(events);
  expect(usage).toEqual({ used: 50000, capacity: 100000 });
  render(<ContextUsageIndicator usage={usage} />);
  expect(screen.getByLabelText(/Context window: 50% used/)).toHaveAttribute(
    'title',
    expect.stringContaining('50,000'),
  );
});

it('estimates turn totals without charging earlier turns or counting repeated snapshots twice', () => {
  const first = report(1, 100, 20, 50, 1100);
  const second = report(2, 300, 60, 150, 1400);
  expect(turnUsage([first, second, second])).toEqual({
    inputTokens: 400,
    outputTokens: 60,
    cachedInputTokens: 150,
  });
});

it('does not invent an unknown context capacity or missing token counts', () => {
  expect(contextUsage([])).toBeNull();
  expect(turnUsage([])).toBeNull();
  const event = runtimeEvent(1, 'usage', null);
  event.normalized!.usage = { inputTokens: 100, outputTokens: null, cachedInputTokens: null };
  expect(contextUsage([event])).toEqual({ used: 100, capacity: null });
  render(<ContextUsageIndicator usage={null} />);
  expect(screen.getByLabelText('Context window usage unavailable')).toBeVisible();
});

it('chooses the newest recorded report even when history arrives out of order', () => {
  const older = report(1, 50000, 100, 40000, 800000);
  older.recordedAt = '2026-10-01T12:00:00Z';
  const newer = report(1, 9000, 1000, 8000, 900000);
  newer.recordedAt = '2026-10-01T12:01:00Z';
  expect(contextUsage([newer, older])).toEqual({ used: 10000, capacity: 100000 });
});

it('excludes previous-turn usage replayed when resuming a native thread', () => {
  const previous = report(1, 50000, 1000, 40000, 900000);
  previous.rawPayload = {
    ...(previous.rawPayload as object),
    native: { params: { turnId: 'previous' } },
  };
  const started = runtimeEvent(2, 'processing_started', null);
  started.rawPayload = { native: { method: 'turn/started', params: { turn: { id: 'current' } } } };
  const current = report(3, 100, 20, 50, 900100);
  current.rawPayload = {
    ...(current.rawPayload as object),
    native: { params: { turnId: 'current' } },
  };
  expect(turnUsage([previous, started])).toBeNull();
  expect(turnUsage([previous, started, current])).toEqual({
    inputTokens: 100,
    outputTokens: 20,
    cachedInputTokens: 50,
  });
});
