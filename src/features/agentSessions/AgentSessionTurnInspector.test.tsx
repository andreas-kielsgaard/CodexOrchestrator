import { fireEvent, render, screen } from '@testing-library/react';
import { AgentSessionTurnInspector } from './AgentSessionTurnInspector';
import { projectAgentSessionTranscript } from './transcriptProjector';
import { runtimeEvent, sessionDetails } from './testFixtures';

describe('AgentSessionTurnInspector', () => {
  it('selects one explicit turn, keeps complete output, and shows authoritative timing', () => {
    const details = sessionDetails('completed', [
      runtimeEvent(1, 'agent_message', 'The complete recorded answer.', { role: 'final' }),
    ]);
    details.invocations[0].invocation.startedAt = '2026-08-05T10:00:00.000Z';
    details.invocations[0].invocation.completedAt = '2026-08-05T10:01:05.000Z';

    render(
      <AgentSessionTurnInspector
        sessionId="session-1"
        invocationId="invocation-1"
        transcript={projectAgentSessionTranscript(details)}
      />,
    );

    expect(screen.getByText('Do the work')).toBeVisible();
    expect(screen.getByText('The complete recorded answer.')).toBeVisible();
    expect(screen.getByText('Started')).toBeVisible();
    expect(screen.getByText('Duration')).toBeVisible();
    expect(screen.getByText('1m 5s')).toBeVisible();
  });

  it('keeps provider text, raw payloads, and free-form status out of safe tool steps', () => {
    const step = runtimeEvent(1, 'tool_activity', 'provider secret query');
    step.rawPayload = { private: 'raw secret result' };
    step.normalized!.toolActivity = {
      phase: 'completed',
      itemId: 'item-1',
      server: 'workspace',
      tool: 'read_file',
      status: 'free-form status secret',
      resultClassification: 'succeeded',
    };
    const transcript = projectAgentSessionTranscript(sessionDetails('completed', [step]));

    render(
      <AgentSessionTurnInspector
        sessionId="session-1"
        invocationId="invocation-1"
        transcript={transcript}
      />,
    );

    const recordedSteps = screen.getByText('Recorded steps');
    expect(recordedSteps).toBeVisible();
    fireEvent.click(recordedSteps);
    expect(screen.getByText('workspace / read_file · completed · succeeded')).toBeVisible();
    expect(screen.queryByText('provider secret query')).toBeNull();
    expect(screen.queryByText('raw secret result')).toBeNull();
    expect(screen.queryByText('free-form status secret')).toBeNull();
    expect(screen.queryByText('Raw event')).toBeNull();
  });

  it.each([
    ['missing-session', 'invocation-1'],
    ['session-1', 'missing-invocation'],
  ])('renders an unavailable state for stale identity %s/%s', (sessionId, invocationId) => {
    render(
      <AgentSessionTurnInspector
        sessionId={sessionId}
        invocationId={invocationId}
        transcript={projectAgentSessionTranscript(sessionDetails('completed'))}
      />,
    );

    expect(screen.getByRole('alert')).toHaveTextContent('Agent Session turn unavailable');
    expect(screen.queryByText('Do the work')).toBeNull();
  });

  it('does not show timing or duration without the authoritative facts', () => {
    const details = sessionDetails('running');
    details.invocations[0].invocation.startedAt = null;
    details.invocations[0].invocation.completedAt = null;
    render(
      <AgentSessionTurnInspector
        sessionId="session-1"
        invocationId="invocation-1"
        transcript={projectAgentSessionTranscript(details)}
      />,
    );

    expect(screen.queryByText('Started')).toBeNull();
    expect(screen.queryByText('Duration')).toBeNull();
  });
});
