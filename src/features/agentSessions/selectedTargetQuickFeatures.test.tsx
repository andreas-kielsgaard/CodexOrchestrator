import type { SessionExecutionSelectionDto } from '../../application/executionTargets/contracts';
import { act, renderHook, waitFor } from '@testing-library/react';
import { repairProfile, repairRuntime } from '../workflowAuthoring/testFixtures';
import { repairSessionClients } from './profileTestFixtures';
import { remoteExecution, remoteTarget } from './sessionTargetFixtures';
import { selectedTargetQuickFeatures } from './selectedTargetQuickFeatures';
import { useAgentSession } from './useAgentSession';
import { samePreparedConfiguration } from './sessionPreparationState';
import { selectionForTarget } from './useSessionTarget';

it('uses selected remote capabilities for a creation draft without querying laptop skills or defaults', async () => {
  const fixture = repairSessionClients(false);
  const loadQuickFeatures = vi.fn(fixture.profiles.loadQuickFeatures);
  const profiles = { ...fixture.profiles, loadQuickFeatures };
  const selection: SessionExecutionSelectionDto = {
    capabilityProfileId: repairProfile.capabilityProfileId,
    capabilityProfileRevision: repairProfile.revision,
    execution: remoteExecution,
    workspace: {
      kind: 'create',
      repositoryId: 'repo',
      branchRef: 'refs/heads/main',
      commit: 'a'.repeat(40),
      attachment: 'branch',
    },
  };
  const facts = selectedTargetQuickFeatures(repairRuntime, repairProfile)!;
  const { result } = renderHook(() =>
    useAgentSession(fixture.sessions, {
      selectedSessionId: null,
      preparedExecution: true,
      executionSelection: selection,
      executionQuickFeatures: facts,
      execution: {
        client: profiles,
        selection: { model: null, reasoningMode: null },
        setSelection: () => {},
        afterAccepted: () => {},
      },
    }),
  );
  await waitFor(() => expect(result.current.loading).toBe(false));
  let loaded;
  await act(async () => {
    loaded = await result.current.quickFeatures!.load();
  });
  expect(loaded).toEqual({
    ...facts,
    limitations: ['Native skill discovery is unavailable on remote devices.'],
  });
  expect(loadQuickFeatures).not.toHaveBeenCalled();
});

it('matches settled preparation after reload, but excludes profile revisions and delivery in progress', () => {
  const fixture = repairSessionClients();
  const selection = selectionForTarget(remoteTarget);
  const preparation = {
    sessionId: 'session-1',
    invocationId: 'invocation-1',
    phase: 'ready' as const,
    selection,
    resolvedTarget: remoteTarget,
    steps: [{ id: 'delivery', label: 'Deliver prompt', status: 'completed' as const }],
    canRetry: false,
    error: null,
    currentResolution: fixture.profile.creationResolution,
    resolution: fixture.result.invocationResolution,
  };
  const options = { model: null, reasoningMode: null };
  expect(samePreparedConfiguration(selection, options, preparation, null)).toBe(true);
  expect(
    samePreparedConfiguration(
      { ...selection, capabilityProfileRevision: 2 },
      options,
      preparation,
      null,
    ),
  ).toBe(false);
  expect(
    samePreparedConfiguration(
      selection,
      options,
      { ...preparation, steps: [{ id: 'delivery', label: 'Deliver prompt', status: 'running' }] },
      null,
    ),
  ).toBe(false);
});
