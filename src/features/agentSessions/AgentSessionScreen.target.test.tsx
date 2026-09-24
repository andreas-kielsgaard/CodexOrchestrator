import { useState } from 'react';
import type { SessionNavigationSelection } from '../../application/agentSessions/navigation';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { StandaloneAgentSessionScreen } from './AgentSessionScreen';
import { repairSessionClients } from './profileTestFixtures';
import { repairClients } from '../workflowAuthoring/testFixtures';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';
import { sessionDetails } from './testFixtures';

beforeEach(() => window.localStorage.clear());

it('preserves the prompt while choosing a remote worktree and uses the shared target selection for the profiled send', async () => {
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
  await user.type(
    await screen.findByRole('textbox', { name: 'Message' }),
    'Work on the remote checkout',
  );
  await user.click(screen.getByRole('button', { name: /Destination device/ }));
  await user.click(await screen.findByRole('button', { name: /Remote server/ }));
  await user.click(screen.getByRole('button', { name: 'Use device' }));
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
    await screen.findByRole('button', { name: /codex\/durable-review.*Remote server/ }),
  ).toBeEnabled();
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

it('uses the device command to select the same target as the modal and retains draft folder placement', async () => {
  const user = userEvent.setup();
  const sessions = repairSessionClients(false);
  const config = repairClients();
  const targets = sessionTargetFixtures();
  const folderTarget = { kind: 'repository' as const, repositoryId: 'repository-one' };
  const originalStart = sessions.profiles.startDirectUserSession;
  const start = vi
    .spyOn(sessions.profiles, 'startDirectUserSession')
    .mockImplementation(async (input) => {
      sessions.details.session.executionTarget = input.executionTarget;
      sessions.details.session.workingDirectory = input.executionTarget?.path ?? null;
      return originalStart(input);
    });
  function DraftScreen() {
    const [selection, setSelection] = useState<SessionNavigationSelection>({
      kind: 'draft',
      draftId: 'target-command-draft',
      folderTarget,
    });
    return (
      <StandaloneAgentSessionScreen
        client={sessions.sessions}
        profileClient={sessions.profiles}
        executionConfigurationClient={config.configuration}
        executionTargetClient={targets.client}
        branchSource={targets.source}
        selection={selection}
        onSelectionChange={setSelection}
      />
    );
  }
  render(<DraftScreen />);
  const input = await screen.findByRole('textbox', { name: 'Message' });
  await user.type(input, '/device{Enter}');
  await screen.findByRole('option', { name: /Remote server/ });
  await user.type(input, 'Remote{Enter}');
  await screen.findByRole('option', { name: /codex\/durable-review · review/ });
  expect(screen.getByRole('button', { name: 'Target worktree' })).toHaveTextContent(
    'Empty workspace',
  );
  expect(start).not.toHaveBeenCalled();
  await user.type(input, 'codex/durable-review{Enter}');
  expect(
    await screen.findByRole('button', { name: /codex\/durable-review.*Remote server/ }),
  ).toBeEnabled();
  await waitFor(() =>
    expect(targets.client.loadRuntime).toHaveBeenCalledWith(
      remoteTarget.execution,
      remoteTarget.path,
    ),
  );
  expect(input).toHaveValue('');
  expect(start).not.toHaveBeenCalled();
  await user.type(input, 'Inspect the selected checkout');
  await waitFor(() => expect(screen.getByRole('button', { name: 'Send' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'Send' }));
  await waitFor(() =>
    expect(start).toHaveBeenCalledWith(
      expect.objectContaining({
        executionTarget: remoteTarget,
        workingDirectory: remoteTarget.path,
        folderTarget,
        submittedText: 'Inspect the selected checkout',
      }),
    ),
  );
  expect(
    await screen.findByRole('button', { name: /codex\/durable-review.*Remote server/ }),
  ).toBeEnabled();
  await user.type(screen.getByRole('textbox', { name: 'Message' }), '/worktree');
  expect(await screen.findByRole('option', { name: /^Worktree / })).toHaveAttribute(
    'aria-disabled',
    'false',
  );
  await user.keyboard('{Enter}');
  await waitFor(() => expect(targets.client.listWorktreeChoices).toHaveBeenCalledTimes(2));
  expect(start).toHaveBeenCalledTimes(1);
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
  await user.click(screen.getByRole('button', { name: /Destination device/ }));
  await user.click(await screen.findByRole('button', { name: /Remote server/ }));
  await user.click(screen.getByRole('button', { name: 'Use device' }));
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
    screen.getByRole('button', { name: /codex\/durable-review.*Remote server/ }),
  ).toBeEnabled();
  expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue(
    'Preserve this remote prompt',
  );
  await user.click(screen.getByRole('button', { name: 'New session' }));
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: /codex\/durable-review.*Remote server/ }),
    ).toBeEnabled(),
  );
  expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue(
    'Preserve this remote prompt',
  );
});

it('allows a failed existing Session to choose a new target in the command menu', async () => {
  const sessions = repairSessionClients(true);
  sessions.details.invocations = sessionDetails('failed').invocations;
  sessions.details.session.executionTarget = remoteTarget;
  const config = repairClients();
  const targets = sessionTargetFixtures();
  render(
    <StandaloneAgentSessionScreen
      client={sessions.sessions}
      profileClient={sessions.profiles}
      executionConfigurationClient={config.configuration}
      executionTargetClient={targets.client}
      branchSource={targets.source}
      selection={{ kind: 'session', sessionId: 'session-1' }}
    />,
  );
  const user = userEvent.setup();
  const input = await screen.findByRole('textbox', { name: 'Message' });
  await user.type(input, '/device');
  expect(await screen.findByRole('option', { name: /^Device / })).toHaveAttribute(
    'aria-disabled',
    'false',
  );
  await user.keyboard('{Enter}');
  await user.keyboard('{Escape}');
  fireEvent.change(input, { target: { value: '/worktree' } });
  expect(await screen.findByRole('option', { name: /^Worktree / })).toHaveAttribute(
    'aria-disabled',
    'false',
  );
  await user.keyboard('{Enter}');
  expect(targets.client.listDevices).toHaveBeenCalled();
  await waitFor(() => expect(targets.client.listWorktreeChoices).toHaveBeenCalled());
});
