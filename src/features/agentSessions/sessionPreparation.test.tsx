import { act, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import type { SessionPreparationDto } from '../../application/agentSessions/preparation';
import { localExecutionBinding } from '../../application/executionTargets/contracts';
import { useAgentSession } from './useAgentSession';
import { AgentSessionWorkspace } from './AgentSessionWorkspace';
import { SessionTargetDialog } from './SessionTargetDialog';
import { repairSessionClients } from './profileTestFixtures';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';
import { selectionForTarget } from './useSessionTarget';

it('confirms a missing branch instance at the published SHA and does no preparation on selection', async () => {
  const fixture = sessionTargetFixtures();
  const commit = 'a'.repeat(40);
  fixture.client.resolvePublishedTip = vi.fn().mockResolvedValue({ commit });
  const onSelectSelection = vi.fn();
  const onSelect = vi.fn();
  render(
    <SessionTargetDialog
      client={fixture.client}
      source={fixture.source}
      selected={remoteTarget}
      deviceId="local"
      capabilityProfileId="local-profile"
      onClose={() => {}}
      onSelect={onSelect}
      onSelectSelection={onSelectSelection}
    />,
  );
  expect(await screen.findByRole('heading', { name: 'Create a worktree on Send?' })).toBeVisible();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Use this commit' })).toBeEnabled(),
  );
  expect(screen.getByTitle(commit)).toHaveTextContent(commit.slice(0, 12));
  expect(fixture.client.loadRuntime).not.toHaveBeenCalled();
  expect(onSelectSelection).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(onSelect).not.toHaveBeenCalled();
  expect(onSelectSelection).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Create worktree on Send…' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Use this commit' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Use this commit' }));
  expect(onSelectSelection).toHaveBeenCalledWith({
    capabilityProfileId: 'local-profile',
    capabilityProfileRevision: 1,
    execution: localExecutionBinding,
    workspace: {
      kind: 'create',
      repositoryId: remoteTarget.repositoryId,
      branchRef: remoteTarget.branchRef,
      commit,
      attachment: 'branch',
    },
  });
});

it('keeps the focused next draft through first-session acceptance and disables another Send during setup', async () => {
  const fixture = repairSessionClients(false);
  const selection = selectionForTarget(remoteTarget);
  let acknowledge!: (value: { sessionId: string; invocationId: string }) => void;
  const sendPreparedMessage = vi.fn(
    () =>
      new Promise<{ sessionId: string; invocationId: string }>((resolve) => {
        acknowledge = resolve;
      }),
  );
  const preparation: SessionPreparationDto = {
    sessionId: 'session-1',
    invocationId: 'invocation-1',
    phase: 'preparing',
    selection,
    steps: [{ id: 'history', label: 'Copy conversation history', status: 'running' }],
    canRetry: false,
    error: null,
  };
  fixture.details.invocations[0].invocation.status = 'pending';
  const cancelPreparation = vi.fn(async () => {});
  const profileClient = {
    ...fixture.profiles,
    sendPreparedMessage,
    loadPreparation: vi.fn(async () => preparation),
    cancelPreparation,
  };
  const steerSession = vi.fn();
  const client = { ...fixture.sessions, steerSession };
  function Host() {
    const [sessionId, setSessionId] = useState<string | null>(null);
    const controller = useAgentSession(client, {
      selectedSessionId: sessionId,
      draftId: sessionId ? undefined : 'draft',
      onSessionCreated: setSessionId,
      preparedExecution: true,
      executionSelection: selection,
      execution: {
        client: profileClient,
        selection: { model: null, reasoningMode: null },
        afterAccepted: () => {},
      },
    });
    return <AgentSessionWorkspace controller={controller} />;
  }
  render(<Host />);
  const input = await screen.findByRole('textbox', { name: 'Message' });
  input.focus();
  fireEvent.change(input, { target: { value: 'First submitted prompt' } });
  fireEvent.keyDown(input, { key: 'Enter' });
  await waitFor(() => expect(sendPreparedMessage).toHaveBeenCalledTimes(1));
  expect(input).toBeEnabled();
  fireEvent.change(input, { target: { value: 'My next draft' } });
  await act(async () => acknowledge({ sessionId: 'session-1', invocationId: 'invocation-1' }));
  await waitFor(() => expect(screen.getAllByText('Waiting for setup').length).toBeGreaterThan(0));
  expect(screen.getByRole('textbox', { name: 'Message' })).toBe(input);
  expect(input).toHaveFocus();
  expect(input).toHaveValue('My next draft');
  expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled();
  fireEvent.keyDown(input, { key: 'Enter' });
  expect(sendPreparedMessage).toHaveBeenCalledTimes(1);
  expect(steerSession).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(cancelPreparation).toHaveBeenCalledWith('invocation-1'));
  expect(input).toHaveValue('My next draft');
});

it('does not steer changed execution settings into the active prepared turn', async () => {
  const fixture = repairSessionClients();
  fixture.details.invocations[0].invocation.status = 'running';
  const selection = selectionForTarget(remoteTarget);
  const preparation: SessionPreparationDto = {
    sessionId: 'session-1',
    invocationId: 'invocation-1',
    phase: 'ready',
    selection,
    steps: [],
    canRetry: false,
    error: null,
    resolution: fixture.result.invocationResolution,
    currentResolution: fixture.profile.creationResolution,
  };
  const profileClient = { ...fixture.profiles, loadPreparation: vi.fn(async () => preparation) };
  const steerSession = vi.fn();
  const client = { ...fixture.sessions, steerSession };
  const { result } = renderHook(() =>
    useAgentSession(client, {
      selectedSessionId: 'session-1',
      preparedExecution: true,
      executionSelection: selection,
      execution: {
        client: profileClient,
        selection: { model: 'a-different-model', reasoningMode: null },
        afterAccepted: () => {},
      },
    }),
  );
  await waitFor(() => expect(result.current.loading).toBe(false));
  expect(result.current.steeringAvailable).toBe(false);
  act(() => result.current.setDraft('For the next model'));
  await act(async () => result.current.send());
  expect(steerSession).not.toHaveBeenCalled();
  expect(result.current.draft).toBe('For the next model');
});
