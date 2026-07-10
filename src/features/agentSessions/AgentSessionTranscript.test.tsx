import { fireEvent, render, screen } from '@testing-library/react';
import { AgentSessionTranscript } from './AgentSessionTranscript';
import { projectAgentSessionTranscript } from './transcriptProjector';
import { runtimeEvent, sessionDetails } from './testFixtures';

describe('AgentSessionTranscript', () => {
  it('shows running work and intermediate agent messages openly', () => {
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
    expect(screen.getByText('Read files')).not.toBeVisible();
    fireEvent.click(screen.getByText('Processing'));
    expect(onToggle).toHaveBeenCalledWith('invocation-1');
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

  it('retains technical detail after a durable reload projection', () => {
    const unknown = runtimeEvent(1, 'unknown', null, { diagnostic: 'malformed' });
    unknown.rawPayload = 'malformed jsonl';
    renderTranscript(projectAgentSessionTranscript(sessionDetails('completed', [unknown])));
    fireEvent.click(screen.getByText('Technical details (1)'));
    expect(screen.getByText('malformed jsonl')).toBeVisible();
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
