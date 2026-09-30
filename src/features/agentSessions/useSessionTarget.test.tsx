import { renderHook, waitFor } from '@testing-library/react';
import { repairClients, repairProfile } from '../workflowAuthoring/testFixtures';
import { composerDraftCacheKey, writeCachedComposerDraft } from './composerDraftCache';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';
import {
  selectionForTarget,
  selectionSatisfiedByTarget,
  useSessionTarget,
} from './useSessionTarget';

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

it('recognizes a create-worktree intent resolved to the active target without consuming a later choice', () => {
  const create = {
    capabilityProfileId: remoteTarget.capabilityProfileId,
    capabilityProfileRevision: remoteTarget.capabilityProfileRevision,
    execution: remoteTarget.execution,
    workspace: {
      kind: 'create' as const,
      repositoryId: remoteTarget.repositoryId,
      branchRef: remoteTarget.branchRef,
      commit: remoteTarget.head ?? '',
      attachment: 'branch' as const,
    },
  };
  expect(selectionSatisfiedByTarget(create, remoteTarget)).toBe(false);
  expect(
    selectionSatisfiedByTarget(
      create,
      { ...remoteTarget, head: 'newer-commit' },
      { selection: create, target: remoteTarget },
    ),
  ).toBe(true);
  expect(
    selectionSatisfiedByTarget(
      { ...create, workspace: { ...create.workspace, branchRef: 'refs/heads/another' } },
      remoteTarget,
    ),
  ).toBe(false);
});

it('keeps an existing worktree selected as its HEAD changes', () => {
  const selection = selectionForTarget(remoteTarget);
  expect(selectionSatisfiedByTarget(selection, { ...remoteTarget, head: null })).toBe(true);
  expect(selectionSatisfiedByTarget(selection, { ...remoteTarget, head: 'newer-commit' })).toBe(
    true,
  );
  expect(
    selectionSatisfiedByTarget(selection, { ...remoteTarget, branchRef: 'refs/heads/other' }),
  ).toBe(false);
  expect(selectionSatisfiedByTarget(selection, { ...remoteTarget, worktreeId: 'other-tree' })).toBe(
    false,
  );
});
