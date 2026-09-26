import { formatAgentSessionContext } from './sessionClipboard';
import { render, screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import { AgentSessionTranscript } from './AgentSessionTranscript';
import {
  projectAgentSessionTranscript,
  selectLatestFinalAgentResponseRange,
  selectTranscriptRange,
} from './transcriptProjector';
import { sessionDetails, fixtureTime } from './testFixtures';
import type { AgentRuntimeEventDto } from '../../application/agentSessions';

it('renders imported user, assistant, steering and activity in source order, with precise final excerpts', () => {
  const items = [
    { kind: 'user', text: 'Initial input' },
    { kind: 'assistant', text: 'Working response' },
    { kind: 'user', text: 'Steering input' },
    { kind: 'activity', text: 'Historical command' },
    { kind: 'assistant', text: 'Final response' },
  ];
  const events: AgentRuntimeEventDto[] = [null, ...items].map((item, sequence) => ({
    id: 'event-' + sequence,
    invocationId: 'invocation-1',
    sequence,
    source: 'runtime',
    recordedAt: fixtureTime,
    rawPayload: {},
    normalized: {
      kind: item?.kind === 'assistant' ? 'agent_message' : 'unknown',
      text: item?.text ?? null,
      externalContextId: null,
      usage: null,
      toolActivity: null,
      details: item
        ? {
            importedContent: item,
            ...(sequence === 5 ? { role: 'final' } : { role: 'intermediate' }),
          }
        : null,
    },
  }));
  const details = sessionDetails('completed', events);
  details.invocations[0].importProvenance = {
    sourceTurnId: 'turn-1',
    ordinal: 0,
    sourceStartedAt: null,
    sourceCompletedAt: null,
  };
  const transcript = projectAgentSessionTranscript(details);
  const exported = formatAgentSessionContext(details, transcript);
  expect(exported.indexOf('Working response')).toBeLessThan(exported.indexOf('Steering input'));
  expect(exported.match(/Initial input/g)).toHaveLength(1);
  expect(exported).toContain('Imported from Codex');
  const props = {
    transcript,
    loading: false,
    expandedProcessing: new Set<string>(),
    onToggleProcessing: () => {},
  };
  const { container, rerender } = render(<AgentSessionTranscript {...props} />);
  const text = container.textContent!;
  expect(text.indexOf('Initial input')).toBeLessThan(text.indexOf('Working response'));
  expect(text.indexOf('Working response')).toBeLessThan(text.indexOf('Steering input'));
  expect(text.indexOf('Steering input')).toBeLessThan(text.indexOf('Historical command'));
  expect(screen.getByText('Imported from Codex')).toBeInTheDocument();
  rerender(<AgentSessionTranscript {...props} safeActivityDetails />);
  expect(screen.queryByText('Historical command')).not.toBeInTheDocument();
  const range = selectLatestFinalAgentResponseRange(transcript)!;
  rerender(
    <AgentSessionTranscript {...props} content={selectTranscriptRange(transcript, range)} />,
  );
  expect(screen.getByText('Final response')).toBeInTheDocument();
  expect(screen.queryByText('Initial input')).not.toBeInTheDocument();
});
