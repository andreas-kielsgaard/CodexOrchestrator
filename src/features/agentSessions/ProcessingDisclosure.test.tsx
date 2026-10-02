import { act, fireEvent, render, screen } from '@testing-library/react';
import { ProcessingDisclosure } from './ProcessingDisclosure';
import { projectAgentSessionTranscript } from './transcriptProjector';
import { runtimeEvent, sessionDetails } from './testFixtures';
import { ToolDetailsContext } from './ToolDetailsContext';

it('loads full durable action data only when the action is expanded', async () => {
  const event = runtimeEvent(1, 'tool_activity', 'command');
  event.rawPayload = {
    _orchidDeferredEventId: event.id,
    item: {
      id: 'command-1',
      type: 'command_execution',
      command: 'npm test',
      aggregatedOutput: 'preview',
    },
  };
  event.normalized!.toolActivity = {
    kind: 'command',
    phase: 'completed',
    itemId: 'command-1',
    server: null,
    tool: null,
    status: 'completed',
    resultClassification: 'succeeded',
  };
  const full = {
    ...event,
    rawPayload: {
      item: {
        id: 'command-1',
        type: 'command_execution',
        command: 'npm test',
        aggregatedOutput: 'complete durable command output',
      },
    },
  };
  const load = vi.fn(async () => full);
  const activity = projectAgentSessionTranscript(sessionDetails('completed', [event]))
    .invocations[0].processing;
  const { container } = render(
    <ToolDetailsContext.Provider value={load}>
      <ProcessingDisclosure
        invocationId="deferred"
        activity={activity}
        expanded
        onToggle={() => {}}
        running={false}
      />
    </ToolDetailsContext.Provider>,
  );
  expect(load).not.toHaveBeenCalled();
  fireEvent.click(container.querySelector('.turn-operation-group summary')!);
  const detail = container.querySelector('.turn-tool-event details') as HTMLDetailsElement;
  detail.open = true;
  fireEvent(detail, new Event('toggle'));
  expect(await screen.findByText('complete durable command output')).toBeVisible();
  expect(load).toHaveBeenCalledWith(event.id);
});

it('groups completed actions around messages, keeps live command details visible, and collapses on completion', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-10-01T12:00:05Z'));
  const tool = (
    sequence: number,
    phase: 'started' | 'completed',
    kind: 'command' | 'file_change' = 'command',
  ) => {
    const event = runtimeEvent(sequence, 'tool_activity', 'command execution');
    event.recordedAt = '2026-10-01T12:00:00Z';
    event.rawPayload = {
      item: {
        id: `tool-${sequence}`,
        type: kind === 'command' ? 'command_execution' : 'file_change',
        command: `npm test ${sequence}`,
      },
    };
    event.normalized!.toolActivity = {
      kind,
      phase,
      itemId: `tool-${sequence}`,
      server: null,
      tool: null,
      status: phase === 'started' ? 'in_progress' : 'completed',
      resultClassification: 'succeeded',
    };
    return event;
  };
  const activity = projectAgentSessionTranscript(
    sessionDetails('running', [
      tool(1, 'completed'),
      tool(2, 'completed', 'file_change'),
      runtimeEvent(3, 'agent_message', 'Next batch', { role: 'commentary' }),
      tool(4, 'completed'),
      tool(5, 'started'),
    ]),
  ).invocations[0].processing;
  const props = {
    invocationId: 'turn',
    activity,
    expanded: false,
    onToggle: vi.fn(),
    startedAt: '2026-10-01T12:00:00Z',
    usage: { inputTokens: 100, outputTokens: 30, cachedInputTokens: 40 },
  };
  try {
    const { rerender, container } = render(<ProcessingDisclosure {...props} running />);
    expect(container.querySelector('.processing-disclosure')).toHaveAttribute('open');
    expect(screen.getByText('1 action · 1 file edit')).toBeVisible();
    expect(screen.getByText('Next batch')).toBeVisible();
    expect(screen.getByText('1 action')).toBeVisible();
    expect(screen.getByText('npm test 5')).toBeVisible();
    expect(screen.queryByText('npm test 1')).not.toBeInTheDocument();
    act(() => vi.advanceTimersByTime(61000));
    expect(screen.getAllByText('1m 6s')).toHaveLength(2);
    fireEvent.click(screen.getByText('1 action · 1 file edit'));
    expect(screen.getByText('Ran npm test 1')).toBeVisible();
    rerender(
      <ProcessingDisclosure {...props} running={false} completedAt="2026-10-01T12:01:10Z" />,
    );
    expect(container.querySelector('.processing-disclosure')).not.toHaveAttribute('open');
    expect(screen.getByText('1m 10s')).toBeVisible();
    expect(screen.queryByText('Next batch')).not.toBeInTheDocument();
    rerender(
      <ProcessingDisclosure
        {...props}
        expanded
        running={false}
        completedAt="2026-10-01T12:01:10Z"
      />,
    );
    expect(screen.getByLabelText('Estimated turn tokens')).toHaveTextContent(
      'Input: 100 · Output: 30 · Cached input: 40',
    );
  } finally {
    vi.useRealTimers();
  }
});

it('places a counted failure indicator beside the action count', () => {
  const events = [1, 2, 3].map((sequence) => {
    const event = runtimeEvent(sequence, 'tool_activity', 'command');
    event.normalized!.toolActivity = {
      kind: 'command',
      phase: 'completed',
      itemId: `tool-${sequence}`,
      server: null,
      tool: null,
      status: sequence < 3 ? 'failed' : 'completed',
      resultClassification: sequence < 3 ? 'failed' : 'succeeded',
    };
    return event;
  });
  const activity = projectAgentSessionTranscript(sessionDetails('completed', events)).invocations[0]
    .processing;
  const { container } = render(
    <ProcessingDisclosure
      invocationId="failed-turn"
      activity={activity}
      expanded
      onToggle={() => {}}
      running={false}
    />,
  );
  const summary = container.querySelector('.turn-operation-group summary')!;
  expect(summary).toHaveTextContent('3 actions2 Failed');
  expect(summary.children[1]).toHaveTextContent('2 Failed');
});
