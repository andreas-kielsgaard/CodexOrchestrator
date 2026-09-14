import { useState } from 'react';
import type { SessionNavigationSelection } from '../../application/agentSessions/navigation';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { StandaloneAgentSessionScreen } from './AgentSessionScreen';
import { repairSessionClients } from './profileTestFixtures';
import { repairClients } from '../workflowAuthoring/testFixtures';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';
it('preserves the prompt while choosing a remote worktree and binds only the first profiled send', async () => {
  const user = userEvent.setup();
  const sessions = repairSessionClients(false);
  const config = repairClients();
  const targets = sessionTargetFixtures();
  const originalStart = sessions.profiles.startDirectUserSession;
  const start = vi
    .spyOn(sessions.profiles, 'startDirectUserSession')
    .mockImplementation(async (input) => {
      sessions.details.session.executionTarget = input.executionTarget;
      sessions.details.session.workingDirectory = input.executionTarget?.path ?? null;
      return originalStart(input);
    });
  const send = vi.spyOn(sessions.profiles, 'sendDirectUserMessage');
  const generic = vi.spyOn(sessions.sessions, 'sendMessage');
  render(
    <StandaloneAgentSessionScreen
      client={sessions.sessions}
      profileClient={sessions.profiles}
      executionConfigurationClient={config.configuration}
      executionTargetClient={targets.client}
      branchSource={targets.source}
    />,
  );
  fireEvent.change(await screen.findByRole('textbox', { name: 'Message' }), {
    target: { value: 'Work on the remote checkout' },
  });
  await user.click(screen.getByRole('button', { name: 'Target worktree' }));
  await screen.findByRole('option', { name: /Codex Orchestrator/ });
  fireEvent.change(screen.getByLabelText('Target repository'), {
    target: { value: 'repository-one' },
  });
  await user.click(await screen.findByRole('button', { name: /^codex\/durable-review / }));
  await user.click(await screen.findByRole('button', { name: new RegExp(remoteTarget.path) }));
  await user.click(screen.getByRole('button', { name: 'Use worktree' }));
  expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue(
    'Work on the remote checkout',
  );
  await waitFor(() =>
    expect(targets.client.loadRuntime).toHaveBeenCalledWith(
      remoteTarget.execution,
      remoteTarget.path,
    ),
  );
  await waitFor(() => expect(screen.getByRole('button', { name: 'Send' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'Send' }));
  await waitFor(() =>
    expect(start).toHaveBeenCalledWith(
      expect.objectContaining({
        executionTarget: remoteTarget,
        workingDirectory: remoteTarget.path,
        submittedText: 'Work on the remote checkout',
      }),
    ),
  );
  expect(
    await screen.findByRole('button', { name: /codex\/durable-review · Remote server/ }),
  ).toBeDisabled();
  fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), {
    target: { value: 'Continue there' },
  });
  await waitFor(() => expect(screen.getByRole('button', { name: 'Send' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'Send' }));
  expect(send).toHaveBeenCalledWith({
    sessionId: 'session-1',
    submittedText: 'Continue there',
    model: null,
    reasoningMode: null,
  });
  expect(generic).not.toHaveBeenCalled();
});

it('keeps New session and its remote target draft open while existing history refreshes', async () => {
  const user = userEvent.setup();
  const sessions = repairSessionClients(true);
  const config = repairClients();
  const targets = sessionTargetFixtures();
  const list = vi.spyOn(sessions.sessions, 'listSessions');
  function ControlledScreen() {
    const [selected, setSelected] = useState<SessionNavigationSelection>({
      kind: 'session',
      sessionId: 'session-1',
    });
    return (
        <StandaloneAgentSessionScreen
          client={sessions.sessions}
          profileClient={sessions.profiles}
          executionConfigurationClient={config.configuration}
          executionTargetClient={targets.client}
          branchSource={targets.source}
          selection={selected}
          onSelectionChange={setSelected}
        />
    );
  }
  render(<ControlledScreen />);
  await screen.findByText('Do the work');
  await user.click(screen.getAllByRole('button', { name: 'New session' })[0]);
  await screen.findByRole('heading', { name: 'New Agent Session' });
  expect(screen.getByRole('button', { name: 'Target worktree' })).toBeEnabled();
  fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), {
    target: { value: 'Preserve this remote prompt' },
  });
  await user.click(screen.getByRole('button', { name: 'Target worktree' }));
  await screen.findByRole('option', { name: /Codex Orchestrator/ });
  fireEvent.change(screen.getByLabelText('Target repository'), {
    target: { value: 'repository-one' },
  });
  await user.click(await screen.findByRole('button', { name: /^codex\/durable-review / }));
  await user.click(await screen.findByRole('button', { name: new RegExp(remoteTarget.path) }));
  await user.click(screen.getByRole('button', { name: 'Use worktree' }));
  const previousCalls = list.mock.calls.length;
  await user.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(list.mock.calls.length).toBeGreaterThan(previousCalls));
  expect(screen.getByRole('heading', { name: 'New Agent Session' })).toBeVisible();
  expect(
    screen.getByRole('button', { name: /codex\/durable-review · Remote server/ }),
  ).toBeEnabled();
  expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue(
    'Preserve this remote prompt',
  );
  await user.click(screen.getByRole('button', { name: 'New session' }));
  await waitFor(() => expect(screen.getByRole('button', { name: 'Target worktree' })).toBeEnabled());
  expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue('');
});
