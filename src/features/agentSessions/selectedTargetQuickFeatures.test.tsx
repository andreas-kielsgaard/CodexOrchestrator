import type { SessionExecutionSelectionDto } from '../../application/executionTargets/contracts';
import { localExecutionBinding } from '../../application/executionTargets/contracts';
import { act, renderHook, waitFor } from '@testing-library/react';
import { repairProfile, repairRuntime } from '../workflowAuthoring/testFixtures';
import { repairSessionClients } from './profileTestFixtures';
import { remoteExecution } from './sessionTargetFixtures';
import { selectedTargetQuickFeatures } from './selectedTargetQuickFeatures';
import { useAgentSession } from './useAgentSession';

it('keeps the route model catalogue available to direct users outside profile allowances', () => {
  expect(
    selectedTargetQuickFeatures(repairRuntime, repairProfile)?.models.map((model) => model.id),
  ).toEqual(['model-a', 'model-b']);
});

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

it('discovers skills from the selected local configuration before creating a worktree', async () => {
  const fixture = repairSessionClients(false);
  const loadQuickFeatures = vi.fn(fixture.profiles.loadQuickFeatures);
  const profiles = { ...fixture.profiles, loadQuickFeatures };
  const facts = selectedTargetQuickFeatures(repairRuntime, repairProfile);
  const selection: SessionExecutionSelectionDto = {
    capabilityProfileId: repairProfile.capabilityProfileId,
    capabilityProfileRevision: repairProfile.revision,
    execution: { ...localExecutionBinding, configurationRef: 'profile-two' },
    workspace: {
      kind: 'create',
      repositoryId: 'repo',
      branchRef: 'refs/heads/main',
      commit: 'a'.repeat(40),
      attachment: 'branch',
    },
  };
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
  await act(async () => {
    await result.current.quickFeatures!.load();
  });
  expect(loadQuickFeatures).toHaveBeenCalledWith({
    sessionId: null,
    workingDirectory: null,
    configuration: { provider: 'codex', configurationId: 'profile-two' },
  });
});
