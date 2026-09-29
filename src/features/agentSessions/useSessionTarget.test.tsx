import { renderHook, waitFor } from '@testing-library/react';
import { repairClients, repairProfile } from '../workflowAuthoring/testFixtures';
import { composerDraftCacheKey, writeCachedComposerDraft } from './composerDraftCache';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';
import { selectionForTarget, useSessionTarget } from './useSessionTarget';

beforeEach(() => window.localStorage.clear());

it('requires a new choice when a cached worktree has disappeared', async () => {
  const folder = { kind: 'repository' as const, repositoryId: remoteTarget.repositoryId };
  writeCachedComposerDraft(composerDraftCacheKey(null, folder), {
    executionSelection: selectionForTarget(remoteTarget),
  });
  const fixture = sessionTargetFixtures();
  fixture.client.listTargets = vi.fn(async () =>
    fixture.devices.map((device) => ({
      ...device,
      profiles: device.profiles.map((profile) => ({ ...profile, instances: [] })),
    })),
  );
  const configuration = repairClients().configuration;
  configuration.listCapabilityProfiles = vi.fn(async () => [
    { ...repairProfile, execution: remoteTarget.execution, routePolicies: [] },
  ]);
  const { result } = renderHook(() =>
    useSessionTarget(
      fixture.client,
      configuration,
      null,
      'draft',
      folder,
      fixture.source,
      remoteTarget.repositoryId,
    ),
  );
  await waitFor(() => expect(result.current.branchChoice?.requiresWorktree).toBe(true));
  expect(result.current.selection?.workspace.kind).toBe('auxiliary');
  expect(result.current.target).toBeNull();
  expect(result.current.workspaceLabel).toContain('codex/durable-review');
});
