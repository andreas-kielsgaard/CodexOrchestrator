import { fireEvent, render, screen } from '@testing-library/react';
import type { SessionExecutionTargetDto } from '../../application/executionTargets/contracts';
import type {
  SessionQueueViewDto,
  SessionTargetOperationDto,
} from '../../application/agentSessions/sessionQueue';
import { sessionDetails } from './testFixtures';
import { projectAgentSessionTranscript } from './transcriptProjector';
import { projectSessionHistory } from './sessionHistoryProjector';
import { AgentSessionTranscript } from './AgentSessionTranscript';

const target: SessionExecutionTargetDto = {
  capabilityProfileId: 'profile',
  capabilityProfileRevision: 1,
  execution: {
    deviceId: 'local',
    deviceName: 'Laptop',
    provider: 'codex',
    configurationRef: 'default',
    connection: { kind: 'local' },
  },
  repositoryId: 'repo',
  branchRef: 'refs/heads/main',
  worktreeId: 'tree',
  path: 'C:/work',
  head: null,
};
const receipt = {
  operationId: 'switch',
  intent: 'change_conversation_target' as const,
  source: null,
  destination: target,
  actions: [
    { kind: 'activate_target' as const, status: 'completed' as const, detail: null, error: null },
  ],
  completedAt: '2026-09-30T12:00:00Z',
};
function details() {
  const base = sessionDetails('completed');
  const original = base.invocations[0];
  return {
    ...base,
    invocations: ['second', 'first', 'third'].map((id) => ({
      ...original,
      invocation: { ...original.invocation, id },
    })),
    timeline: {
      positions: [
        { ordinal: 0, invocationId: 'first', operationId: null },
        { ordinal: 1, invocationId: null, operationId: 'switch' },
        { ordinal: 2, invocationId: 'second', operationId: null },
        { ordinal: 3, invocationId: 'third', operationId: null },
      ],
      transitions: [receipt],
      providers: [
        { invocationId: 'first', provider: 'codex' },
        { invocationId: 'second', provider: 'claude' },
        { invocationId: 'third', provider: 'claude' },
      ],
    },
  };
}
it('uses durable execution order and places a workspace outcome before a changed provider', () => {
  const value = details();
  const result = projectSessionHistory(value, projectAgentSessionTranscript(value));
  expect(result.map((item) => item.id)).toEqual(['first', 'switch', 'second', 'third']);
  expect(result[2]).toMatchObject({
    kind: 'invocation',
    providerChange: { previous: 'codex', provider: 'claude' },
  });
  expect(result[3]).not.toHaveProperty('providerChange');
  expect(
    projectSessionHistory(JSON.parse(JSON.stringify(value)), projectAgentSessionTranscript(value)),
  ).toEqual(result);
});
it('does not invent a provider change across unknown provenance', () => {
  const value = details();
  value.timeline.providers = value.timeline.providers.filter((p) => p.invocationId !== 'first');
  expect(
    projectSessionHistory(value, projectAgentSessionTranscript(value)).filter(
      (item) => item.kind === 'invocation' && item.providerChange,
    ),
  ).toEqual([]);
});
it('renders a transition-only session and opens its outcome before expandable diagnostics', () => {
  const value = {
    ...sessionDetails(),
    timeline: {
      positions: [{ ordinal: 0, invocationId: null, operationId: 'switch' }],
      transitions: [receipt],
      providers: [],
    },
  };
  const transcript = projectAgentSessionTranscript(value);
  render(
    <AgentSessionTranscript
      transcript={transcript}
      history={projectSessionHistory(value, transcript)}
      loading={false}
      expandedProcessing={new Set()}
      onToggleProcessing={() => {}}
    />,
  );
  const button = screen.getByRole('button', { name: 'Switching target to Laptop main' });
  expect(button.closest('.session-transition')).toHaveClass('session-transition--workspace');
  fireEvent.click(button);
  expect(screen.getByRole('dialog')).toBeVisible();
  expect(screen.getByText('Completed successfully.')).toBeVisible();
  expect(screen.getByText(/Activate target/)).not.toBeVisible();
  fireEvent.click(screen.getByText('Actual steps and diagnostics'));
  expect(screen.getByText(/Activate target/)).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Close transition details' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
it('removes the temporary blocked indicator and its open modal when the queue is resolved', () => {
  const value = sessionDetails();
  const transcript = projectAgentSessionTranscript(value);
  const operation = {
    id: 'blocked-switch',
    error: 'Destination unavailable',
    actions: [],
  } as unknown as SessionTargetOperationDto;
  const queue = {
    entries: [{ entry: { state: 'blocked' }, targetOperation: operation }],
  } as unknown as SessionQueueViewDto;
  const props = {
    transcript,
    loading: false,
    expandedProcessing: new Set<string>(),
    onToggleProcessing: () => {},
  };
  const view = render(
    <AgentSessionTranscript {...props} history={projectSessionHistory(value, transcript, queue)} />,
  );
  fireEvent.click(
    screen.getByRole('button', { name: 'Target switch blocked: Destination unavailable' }),
  );
  expect(screen.getByRole('dialog')).toBeVisible();
  view.rerender(
    <AgentSessionTranscript {...props} history={projectSessionHistory(value, transcript)} />,
  );
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(
    screen.queryByText('Target switch blocked: Destination unavailable'),
  ).not.toBeInTheDocument();
});
