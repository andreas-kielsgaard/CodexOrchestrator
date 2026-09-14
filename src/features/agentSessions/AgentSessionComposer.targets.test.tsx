import { useState } from 'react';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import type {
  ExecutionTargetClient,
  RepositoryWorktreeChoicesDto,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import { localExecutionBinding } from '../../application/executionTargets/contracts';
import { AgentSessionComposer } from './AgentSessionComposer';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';

function TargetComposer({
  client,
  choose,
  send,
  load,
  contextKey = 'draft-1',
  fixed = false,
}: {
  client: ExecutionTargetClient;
  choose: (target: SessionExecutionTargetDto) => void;
  send: (draft: string) => void;
  load?: () => Promise<AgentSessionQuickFeatures>;
  contextKey?: string;
  fixed?: boolean;
}) {
  const [draft, setDraft] = useState('');
  const [target, setTarget] = useState<SessionExecutionTargetDto | null>(null);
  return (
    <AgentSessionComposer
      draft={draft}
      onDraftChange={setDraft}
      workingDirectory=""
      isNewSession={!fixed}
      sending={false}
      active={false}
      canceling={false}
      showWorkingDirectory={false}
      keyboardHint="tooltip"
      onWorkingDirectoryChange={() => {}}
      onCancel={() => {}}
      onSend={() => send(draft)}
      targetSource={{
        contextKey,
        client,
        target,
        disabledReason: fixed ? 'This Session target is fixed after its first prompt.' : undefined,
        onSelectTarget: (next) => {
          setTarget(next);
          choose(next);
        },
      }}
      quickFeatures={
        load
          ? {
              contextKey,
              load,
              selection: { model: null, reasoningMode: null },
              setSelection: () => {},
            }
          : undefined
      }
    />
  );
}

function setupTargets(load?: () => Promise<AgentSessionQuickFeatures>) {
  const fixture = sessionTargetFixtures();
  const localTarget = {
    ...remoteTarget,
    execution: localExecutionBinding,
    path: 'C:/worktrees/local-review',
    worktreeId: 'local-instance',
    capabilityProfileId: 'local-profile',
  };
  const localInventory: readonly RepositoryWorktreeChoicesDto[] = [
    {
      repositoryId: localTarget.repositoryId,
      repositoryName: 'Codex Orchestrator',
      profiles: [{ ...fixture.devices[0].profiles[0], instances: [localTarget] }],
    },
  ];
  const remoteInventory: readonly RepositoryWorktreeChoicesDto[] = [
    {
      repositoryId: remoteTarget.repositoryId,
      repositoryName: 'Codex Orchestrator',
      profiles: fixture.devices[1].profiles,
    },
  ];
  vi.mocked(fixture.client.listWorktreeChoices).mockImplementation(async (scope) =>
    scope.kind === 'local' ? localInventory : remoteInventory,
  );
  const choose = vi.fn();
  const send = vi.fn();
  const result = render(
    <TargetComposer client={fixture.client} choose={choose} send={send} load={load} />,
  );
  return {
    ...result,
    ...fixture,
    choose,
    send,
    localTarget,
    localInventory,
    remoteInventory,
    user: userEvent.setup(),
    input: screen.getByRole('textbox', { name: 'Message' }),
  };
}

it('selects a local worktree with slash branch filtering despite native discovery failure', async () => {
  const load = vi.fn().mockRejectedValue(new Error('Remote native quick features are unavailable'));
  const { input, user, choose, send, client, localTarget } = setupTargets(load);
  await user.type(input, '/worktree');
  await screen.findByText('Remote native quick features are unavailable');
  await user.keyboard('{Enter}');
  expect(input).toHaveValue('/');
  await screen.findByRole('option', { name: /codex\/durable-review · local-review/ });
  await user.type(input, 'codex/durable-review{Enter}');
  expect(choose).toHaveBeenCalledWith(localTarget);
  expect(input).toHaveValue('');
  expect(
    screen.getByText(/Target: codex\/durable-review · local-review on This laptop/),
  ).toBeVisible();
  expect(client.listWorktreeChoices).toHaveBeenCalledWith({ kind: 'local' });
  expect(client.listDevices).not.toHaveBeenCalled();
  expect(send).not.toHaveBeenCalled();
  await user.type(input, 'Inspect this checkout{Enter}');
  expect(send).toHaveBeenCalledWith('Inspect this checkout');
});

it('keeps devices provisional and returns to the previous query without inserting identity keys', async () => {
  const { input, user, choose, send, client } = setupTargets();
  await user.type(input, '/device{Enter}');
  await screen.findByRole('option', { name: /Remote server/ });
  await user.type(input, 'Remote{Tab}');
  await screen.findByRole('option', { name: /codex\/durable-review · review/ });
  expect(choose).not.toHaveBeenCalled();
  await user.keyboard('{Backspace}');
  expect(input).toHaveValue('/Remote');
  expect(screen.getByRole('listbox', { name: 'Devices' })).toBeVisible();
  await user.keyboard('{Enter}');
  await screen.findByRole('option', { name: /codex\/durable-review · review/ });
  await user.keyboard('{Escape}{Escape}');
  expect(input).toHaveValue('/device');
  expect(screen.getByRole('listbox', { name: 'Quick features' })).toBeVisible();
  expect(client.listWorktreeChoices).toHaveBeenCalledTimes(2);
  expect(choose).not.toHaveBeenCalled();
  expect(send).not.toHaveBeenCalled();
});

it('distinguishes profile-qualified copies of a remote worktree and applies the chosen full binding', async () => {
  const { input, user, choose, send, client, remoteInventory } = setupTargets();
  vi.mocked(client.listWorktreeChoices).mockResolvedValue([
    {
      ...remoteInventory[0],
      profiles: [
        ...remoteInventory[0].profiles,
        {
          ...remoteInventory[0].profiles[0],
          capabilityProfileId: 'alternate',
          capabilityProfileRevision: 7,
          capabilityProfileName: 'Alternate Codex',
        },
        {
          ...remoteInventory[0].profiles[0],
          capabilityProfileId: 'unavailable',
          capabilityProfileName: 'Offline Codex',
          instances: [],
          error: 'Host unavailable',
        },
      ],
    },
  ]);
  await user.type(input, '/device{Enter}');
  await screen.findByRole('option', { name: /Remote server/ });
  await user.type(input, 'Remote{Enter}');
  await screen.findByRole('option', { name: /Alternate Codex/ });
  expect(screen.getAllByRole('option')).toHaveLength(2);
  expect(screen.getByText('Codex Orchestrator · Offline Codex: Host unavailable')).toBeVisible();
  await user.type(input, 'Alternate{Enter}');
  expect(choose).toHaveBeenCalledWith({
    ...remoteTarget,
    capabilityProfileId: 'alternate',
    capabilityProfileRevision: 7,
  });
  expect(client.listWorktreeChoices).toHaveBeenCalledWith({ kind: 'device', deviceId: 'remote' });
  expect(send).not.toHaveBeenCalled();
});

it('keeps slash queries local during inventory loading and ignores results after backing out', async () => {
  const { input, user, choose, send, client, localInventory } = setupTargets();
  let resolve!: (value: readonly RepositoryWorktreeChoicesDto[]) => void;
  vi.mocked(client.listWorktreeChoices).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  await user.type(input, '/worktree{Enter}');
  await screen.findByText('Loading available choices…');
  await user.type(input, 'feature/Remote Development{Enter}');
  await user.click(screen.getByRole('button', { name: 'Send' }));
  expect(input).toHaveValue('/feature/Remote Development');
  expect(send).not.toHaveBeenCalled();
  await user.keyboard('{Escape}');
  expect(input).toHaveValue('/worktree');
  await act(async () => resolve(localInventory));
  expect(screen.getByRole('listbox', { name: 'Quick features' })).toBeVisible();
  expect(screen.queryByRole('option', { name: /local-review/ })).not.toBeInTheDocument();
  expect(choose).not.toHaveBeenCalled();
});

it('retains the query through an inventory error and empty retry without submitting it', async () => {
  const { input, user, choose, send, client } = setupTargets();
  vi.mocked(client.listWorktreeChoices)
    .mockRejectedValueOnce(new Error('Device unavailable'))
    .mockResolvedValueOnce([]);
  await user.type(input, '/worktree{Enter}');
  await screen.findByText('Device unavailable');
  await user.type(input, 'feature/missing{Enter}');
  expect(send).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Retry' }));
  await screen.findByText('No matching choices');
  fireEvent.change(input, { target: { value: '/' } });
  await screen.findByText(
    'No existing branch worktrees are available in the configured repositories.',
  );
  await user.keyboard('{Enter}');
  expect(choose).not.toHaveBeenCalled();
  expect(send).not.toHaveBeenCalled();
});

it('rejects late inventory after a draft context changes and keeps existing Sessions fixed', async () => {
  const { input, user, choose, send, client, rerender, localInventory } = setupTargets();
  let resolve!: (value: readonly RepositoryWorktreeChoicesDto[]) => void;
  vi.mocked(client.listWorktreeChoices).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  await user.type(input, '/worktree{Enter}');
  await waitFor(() => expect(client.listWorktreeChoices).toHaveBeenCalledOnce());
  rerender(<TargetComposer client={client} choose={choose} send={send} contextKey="draft-2" />);
  await act(async () => resolve(localInventory));
  expect(screen.queryByRole('option', { name: /local-review/ })).not.toBeInTheDocument();
  rerender(
    <TargetComposer
      client={client}
      choose={choose}
      send={send}
      contextKey="completed-session"
      fixed
    />,
  );
  fireEvent.change(input, { target: { value: '/worktree' } });
  expect(await screen.findByRole('option', { name: /Worktree/ })).toHaveAttribute(
    'aria-disabled',
    'true',
  );
  await user.keyboard('{Enter}');
  fireEvent.change(input, { target: { value: '/device' } });
  expect(await screen.findByRole('option', { name: /^Device / })).toHaveAttribute(
    'aria-disabled',
    'true',
  );
  await user.keyboard('{Enter}');
  expect(client.listDevices).not.toHaveBeenCalled();
  expect(client.listWorktreeChoices).toHaveBeenCalledOnce();
  expect(choose).not.toHaveBeenCalled();
  expect(send).not.toHaveBeenCalled();
});
