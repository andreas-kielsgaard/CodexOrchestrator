import { fireEvent, render, screen } from '@testing-library/react';
import { AgentSessionExcerpt } from './AgentSessionExcerpt';
import {
  projectAgentSessionTranscript,
  selectLatestFinalAgentResponseRange,
} from './transcriptProjector';
import { runtimeEvent, sessionDetails } from './testFixtures';

describe('AgentSessionExcerpt', () => {
  it('renders an anchored final response through the shared transcript presentation', () => {
    const transcript = projectAgentSessionTranscript(
      sessionDetails('completed', [
        runtimeEvent(1, 'processing_update', 'Older processing'),
        runtimeEvent(2, 'agent_message', 'Reusable final response', { role: 'final' }),
      ]),
    );
    const range = selectLatestFinalAgentResponseRange(transcript);
    if (!range) throw new Error('Expected a final response range.');
    const onActivate = vi.fn();

    render(
      <AgentSessionExcerpt
        transcript={transcript}
        range={range}
        actionLabel="Open session"
        onActivate={onActivate}
      />,
    );

    expect(screen.getByLabelText('Agent Session transcript')).toBeVisible();
    expect(screen.getByText('Reusable final response')).toBeVisible();
    expect(screen.queryByText('Do the work')).toBeNull();
    expect(screen.queryByText('Older processing')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Open session' }));
    expect(onActivate).toHaveBeenCalledOnce();
  });
});
