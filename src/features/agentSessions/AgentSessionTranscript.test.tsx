import { fireEvent, render, screen } from '@testing-library/react';
import { AgentSessionTranscript } from './AgentSessionTranscript';
import { projectAgentSessionTranscript } from './transcriptProjector';
import { runtimeEvent, sessionDetails } from './testFixtures';

describe('AgentSessionTranscript', () => {
  it('opens running work by default', () => {
    renderTranscript(
      projectAgentSessionTranscript(
        sessionDetails('running', [
          runtimeEvent(1, 'processing_update', 'Inspecting files'),
          runtimeEvent(2, 'agent_message', 'I found the issue', { role: 'intermediate' }),
        ]),
      ),
    );
    expect(screen.getByText('Inspecting files')).toBeVisible();
    expect(screen.getByText('I found the issue')).toBeVisible();
    expect(screen.getByText('Working')).toBeInTheDocument();
  });

  it('renders steering as a user message between the work before and after it', () => {
    const details = sessionDetails('running', [
      runtimeEvent(1, 'processing_update', 'Work before steering'),
      runtimeEvent(4, 'processing_update', 'Work after steering'),
    ]);
    details.interactions = [
      {
        id: 'steering-1',
        invocationId: 'invocation-1',
        sequence: 3,
        kind: 'steering',
        state: 'accepted',
        content: { text: 'Change direction' },
        result: null,
      },
    ];
    const { container } = render(
      <AgentSessionTranscript
        transcript={projectAgentSessionTranscript(details)}
        loading={false}
        expandedProcessing={new Set(['invocation-1:activity:0', 'invocation-1:activity:1'])}
        onToggleProcessing={() => undefined}
      />,
    );

    const text = container.textContent ?? '';
    expect(text.indexOf('Work before steering')).toBeLessThan(text.indexOf('Change direction'));
    expect(text.indexOf('Change direction')).toBeLessThan(text.indexOf('Work after steering'));
    expect(screen.getByText('Change direction').closest('article')).toHaveClass('user-message');
    expect(screen.queryByText(/Steering/)).toBeNull();
  });

  it('keeps a tool that started before steering fully visible while it is still running', () => {
    const command = runtimeEvent(1, 'tool_activity', 'command execution');
    command.rawPayload = { item: { id: 'live-command', command: 'npm test' } };
    command.normalized!.toolActivity = {
      kind: 'command',
      phase: 'started',
      itemId: 'live-command',
      server: null,
      tool: null,
      status: 'in_progress',
      resultClassification: 'unknown',
    };
    const details = sessionDetails('running', [command]);
    details.interactions = [
      {
        id: 'steer',
        invocationId: 'invocation-1',
        sequence: 2,
        kind: 'steering',
        state: 'accepted',
        content: { text: 'Keep going' },
        result: null,
      },
    ];
    renderTranscript(projectAgentSessionTranscript(details));
    expect(screen.getByText('npm test')).toBeVisible();
    expect(screen.getByText('Running')).toBeVisible();
    expect(screen.getByText('Keep going')).toBeVisible();
  });

  it('collapses completed processing while keeping the final response prominent and expandable', () => {
    const onToggle = vi.fn();
    render(
      <AgentSessionTranscript
        transcript={projectAgentSessionTranscript(
          sessionDetails('completed', [
            runtimeEvent(1, 'tool_activity', 'Read files'),
            runtimeEvent(2, 'agent_message', 'Finished result', { role: 'final' }),
          ]),
        )}
        loading={false}
        expandedProcessing={new Set()}
        onToggleProcessing={onToggle}
      />,
    );
    expect(screen.getByText('Finished result')).toBeVisible();
    expect(screen.queryByText('Read files')).not.toBeInTheDocument();
    fireEvent.click(screen.getByText('Worked'));
    expect(onToggle).toHaveBeenCalledWith('invocation-1');
  });

  it('presents agent updates as messages and tools as compact actions', () => {
    const started = runtimeEvent(
      2,
      'tool_activity',
      '"C:\\tools\\pwsh.exe" -Command "npm test -- --run"',
    );
    started.recordedAt = '2026-07-10T12:00:00.000Z';
    started.rawPayload = {
      type: 'item.started',
      item: {
        id: 'command-1',
        type: 'command_execution',
        command: '"C:\\tools\\pwsh.exe" -Command "npm test -- --run"',
        status: 'in_progress',
      },
    };
    started.normalized!.toolActivity = {
      kind: 'command',
      phase: 'started',
      itemId: 'command-1',
      server: null,
      tool: null,
      status: 'in_progress',
      resultClassification: 'unknown',
    };
    const completed = runtimeEvent(3, 'tool_activity', 'npm test -- --run');
    completed.recordedAt = '2026-07-10T12:00:02.000Z';
    completed.rawPayload = {
      type: 'item.completed',
      item: {
        id: 'command-1',
        type: 'command_execution',
        command: 'npm test -- --run',
        aggregated_output: '812 tests passed',
        exit_code: 0,
        status: 'completed',
      },
    };
    completed.normalized!.toolActivity = {
      kind: 'command',
      phase: 'completed',
      itemId: 'command-1',
      server: null,
      tool: null,
      status: 'completed',
      resultClassification: 'succeeded',
    };

    render(
      <AgentSessionTranscript
        transcript={projectAgentSessionTranscript(
          sessionDetails('running', [
            runtimeEvent(1, 'agent_message', 'I found the relevant files.', {
              role: 'intermediate',
            }),
            started,
            completed,
          ]),
        )}
        loading={false}
        expandedProcessing={new Set(['invocation-1'])}
        onToggleProcessing={() => undefined}
      />,
    );

    expect(screen.getByText('I found the relevant files.')).toBeVisible();
    fireEvent.click(screen.getByText('Show operations'));
    expect(screen.getByText('Ran npm test -- --run')).toBeVisible();
    expect(screen.queryByText('Agent update')).not.toBeInTheDocument();
    expect(screen.queryByText('Tool')).not.toBeInTheDocument();
    expect(screen.getByText('1 action · 1 update')).toBeVisible();
    const commandSummary = screen.getByText('Ran npm test -- --run').closest('summary');
    expect(commandSummary).not.toBeNull();
    fireEvent.click(commandSummary!);
    expect(commandSummary!.parentElement).toHaveAttribute('open');
    expect(screen.getByText('812 tests passed')).toBeVisible();
    expect(screen.getByText('Exit code')).toBeVisible();
    expect(screen.getByText('2.0 s')).toBeVisible();
  });

  it('shows edited files and explicit command batches in expandable details', () => {
    const files = runtimeEvent(1, 'tool_activity', 'file change', {
      itemType: 'file_change',
      fileChanges: [
        { path: 'src/app.ts', operation: 'edit' },
        { path: 'src/new.ts', operation: 'create' },
      ],
    });
    files.rawPayload = {
      type: 'item.completed',
      item: { id: 'files-1', type: 'file_change', status: 'completed' },
    };
    files.normalized!.toolActivity = {
      kind: 'file_change',
      phase: 'completed',
      itemId: 'files-1',
      server: null,
      tool: null,
      status: 'completed',
      resultClassification: 'succeeded',
    };
    const batch = runtimeEvent(2, 'tool_activity', 'command execution');
    batch.rawPayload = {
      type: 'item.completed',
      item: {
        id: 'batch-1',
        type: 'command_execution',
        commands: ['npm test', 'npm run lint'],
        status: 'completed',
      },
    };
    batch.normalized!.toolActivity = {
      kind: 'command',
      phase: 'completed',
      itemId: 'batch-1',
      server: null,
      tool: null,
      status: 'completed',
      resultClassification: 'succeeded',
    };

    render(
      <AgentSessionTranscript
        transcript={projectAgentSessionTranscript(sessionDetails('running', [files, batch]))}
        loading={false}
        expandedProcessing={new Set(['invocation-1'])}
        onToggleProcessing={() => undefined}
      />,
    );

    fireEvent.click(screen.getByText('2 actions · 2 file edits'));
    fireEvent.click(screen.getByText('Edited 2 files'));
    expect(screen.getByText(/src\/app\.ts/)).toHaveTextContent('src/app.ts — Edit');
    expect(screen.getByText(/src\/new\.ts/)).toHaveTextContent('src/new.ts — Create');
    expect(screen.getByText('Ran 2 commands')).toBeVisible();
    expect(screen.getByText('Batch · 2 commands')).toBeVisible();
    fireEvent.click(screen.getByText('Ran 2 commands'));
    expect(
      screen.getByText(
        (_, element) =>
          element?.tagName === 'PRE' && element.textContent === 'npm test\nnpm run lint',
      ),
    ).toBeVisible();
  });

  it.each([
    ['failed', 'Failed. Runtime failed'],
    ['canceled', 'Canceled. This invocation was canceled.'],
    ['interrupted', 'Interrupted. This invocation was interrupted before completion.'],
  ] as const)('renders %s terminal truth without inventing an agent reply', (status, text) => {
    renderTranscript(projectAgentSessionTranscript(sessionDetails(status)));
    expect(screen.getByRole('status')).toHaveTextContent(text);
    expect(screen.queryByText('Agent')).not.toBeInTheDocument();
  });

  it('renders empty and completed-without-final states explicitly', () => {
    const { rerender } = renderTranscript(projectAgentSessionTranscript(sessionDetails()));
    expect(screen.getByText('Start with a message')).toBeInTheDocument();
    rerender(
      <AgentSessionTranscript
        transcript={projectAgentSessionTranscript(sessionDetails('completed'))}
        loading={false}
        expandedProcessing={new Set()}
        onToggleProcessing={() => undefined}
      />,
    );
    expect(screen.getByText('Completed without a final response.')).toBeInTheDocument();
  });

  it('does not expose routine technical detail in the conversation', () => {
    const unknown = runtimeEvent(1, 'unknown', null, { diagnostic: 'malformed' });
    unknown.rawPayload = 'malformed jsonl';
    renderTranscript(projectAgentSessionTranscript(sessionDetails('completed', [unknown])));
    expect(screen.queryByText('Technical details (1)')).not.toBeInTheDocument();
    expect(screen.queryByText('malformed jsonl')).not.toBeInTheDocument();
  });

  it('keeps live diagnostics out of the ordinary transcript across updates', () => {
    const unknown = runtimeEvent(1, 'unknown', 'Unclassified runtime output');
    const { rerender } = renderTranscript(
      projectAgentSessionTranscript(sessionDetails('running', [unknown])),
    );

    expect(screen.queryByText('Unclassified runtime output')).not.toBeInTheDocument();

    rerender(
      <AgentSessionTranscript
        transcript={projectAgentSessionTranscript(sessionDetails('completed', [unknown]))}
        loading={false}
        expandedProcessing={new Set()}
        onToggleProcessing={() => undefined}
      />,
    );

    expect(screen.queryByText('Unclassified runtime output')).not.toBeInTheDocument();
  });
});

function renderTranscript(transcript: ReturnType<typeof projectAgentSessionTranscript>) {
  return render(
    <AgentSessionTranscript
      transcript={transcript}
      loading={false}
      expandedProcessing={new Set()}
      onToggleProcessing={() => undefined}
    />,
  );
}
