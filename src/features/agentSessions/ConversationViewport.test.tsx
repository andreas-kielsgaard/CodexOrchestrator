import { render, screen } from '@testing-library/react';
import { ConversationViewport } from './ConversationViewport';
import { projectAgentSessionTranscript, projectedTranscriptContent } from './transcriptProjector';
import { runtimeEvent, sessionDetails } from './testFixtures';

describe('ConversationViewport', () => {
  it('renders a complete Agent Session and an inclusive historical excerpt', () => {
    const transcript = projectAgentSessionTranscript(
      sessionDetails('completed', [
        runtimeEvent(1, 'processing_update', 'Inspecting'),
        runtimeEvent(2, 'agent_message', 'Final answer', { role: 'final' }),
      ]),
    );
    const { rerender } = renderViewport([{ id: 'whole', transcript }]);
    expect(screen.getByText('Do the work')).toBeVisible();
    expect(screen.getByText('Final answer')).toBeVisible();

    const content = projectedTranscriptContent(transcript);
    rerender(
      <ConversationViewport
        segments={[
          {
            id: 'excerpt',
            transcript,
            range: { start: content[1].anchor, end: content[2].anchor },
          },
        ]}
        loading={false}
        expandedProcessing={new Set(['invocation-1'])}
        onToggleProcessing={() => undefined}
      />,
    );
    expect(screen.queryByText('Do the work')).not.toBeInTheDocument();
    expect(screen.getByText('Inspecting')).toBeVisible();
    expect(screen.getByText('Final answer')).toBeVisible();
  });

  it('is read-only unless a composer target is explicitly supplied', () => {
    const transcript = projectAgentSessionTranscript(sessionDetails('completed'));
    const { rerender } = renderViewport([{ id: 'read-only', transcript }]);
    expect(screen.queryByLabelText('Message')).not.toBeInTheDocument();

    rerender(
      <ConversationViewport
        segments={[{ id: 'writable', transcript }]}
        loading={false}
        expandedProcessing={new Set()}
        onToggleProcessing={() => undefined}
        composerTarget={{
          sessionId: 'session-1',
          draft: '',
          workingDirectory: '',
          sending: false,
          active: false,
          canceling: false,
          setDraft: () => undefined,
          setWorkingDirectory: () => undefined,
          send: async () => undefined,
          cancel: async () => undefined,
        }}
      />,
    );
    expect(screen.getByLabelText('Message')).toBeInTheDocument();
  });

  it('renders no transcript content for stale or reversed excerpts', () => {
    const transcript = projectAgentSessionTranscript(
      sessionDetails('completed', [
        runtimeEvent(1, 'processing_update', 'Inspecting'),
        runtimeEvent(2, 'agent_message', 'Final answer', { role: 'final' }),
      ]),
    );
    const content = projectedTranscriptContent(transcript);
    const { rerender } = renderViewport([
      {
        id: 'stale',
        transcript,
        range: { start: { ...content[0].anchor, sessionId: 'missing' }, end: content[1].anchor },
      },
    ]);
    expect(screen.queryByText('Do the work')).not.toBeInTheDocument();
    expect(screen.queryByText('Start with a message')).not.toBeInTheDocument();

    rerender(
      <ConversationViewport
        segments={[
          {
            id: 'reversed',
            transcript,
            range: { start: content[2].anchor, end: content[1].anchor },
          },
        ]}
        loading={false}
        expandedProcessing={new Set()}
        onToggleProcessing={() => undefined}
      />,
    );
    expect(screen.queryByText('Final answer')).not.toBeInTheDocument();
    expect(screen.queryByText('Start with a message')).not.toBeInTheDocument();
  });

  it('renders multi-session segments in caller-provided causal order', () => {
    const first = projectAgentSessionTranscript(
      sessionDetails('completed', [
        runtimeEvent(1, 'agent_message', 'First causal result', { role: 'final' }),
      ]),
    );
    const secondDetails = sessionDetails('completed', [
      runtimeEvent(1, 'agent_message', 'Second causal result', { role: 'final' }),
    ]);
    secondDetails.session.id = 'session-2';
    secondDetails.invocations[0].invocation.sessionId = 'session-2';
    const second = projectAgentSessionTranscript(secondDetails);
    renderViewport([
      { id: 'second', transcript: second },
      { id: 'first', transcript: first },
    ]);
    const text = screen.getByLabelText('Conversation').textContent ?? '';
    expect(text.indexOf('Second causal result')).toBeLessThan(text.indexOf('First causal result'));
  });
});

function renderViewport(segments: Parameters<typeof ConversationViewport>[0]['segments']) {
  return render(
    <ConversationViewport
      segments={segments}
      loading={false}
      expandedProcessing={new Set(['invocation-1'])}
      onToggleProcessing={() => undefined}
    />,
  );
}
