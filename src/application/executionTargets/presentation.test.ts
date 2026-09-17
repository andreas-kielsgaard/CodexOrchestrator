import { localExecutionBinding, type SessionExecutionTargetDto } from './contracts';
import { presentExecutionTarget, toSessionExecutionTarget } from './presentation';
import { orderTargetBranches } from './presentation';
import type { ReviewBranch } from '../branches';

const target: SessionExecutionTargetDto = {
  repositoryId: 'repository',
  capabilityProfileId: 'profile',
  capabilityProfileRevision: 3,
  execution: localExecutionBinding,
  branchRef: 'refs/heads/feature/Remote-Development',
  worktreeId: 'worktree-observation-123456789abc',
  path: 'C:\\worktrees\\remote-development',
  head: null,
};

it('retains complete target identity without parsing presentation labels', () => {
  expect(toSessionExecutionTarget(target.repositoryId, target, target)).toEqual(target);
});

it.each([
  ['C:\\worktrees\\remote-development', 'remote-development'],
  ['/root/worktrees/remote-development/', 'remote-development'],
])('renders the branch and directory for %s', (path, directory) => {
  const choice = presentExecutionTarget({ ...target, path }, 'Orchid', 'Codex workstation');
  expect(choice.label).toBe(`feature/Remote-Development · ${directory}`);
  expect(choice.description).toBe(`Orchid · Codex workstation · ${path} · #3456789abc`);
  expect(choice.keywords).toEqual(expect.arrayContaining([path, target.worktreeId, 'Orchid']));
});

it('distinguishes identically named instances across repositories, devices, and profiles', () => {
  const variants = [
    target,
    { ...target, repositoryId: 'another-repository' },
    { ...target, capabilityProfileId: 'another-profile' },
    { ...target, execution: { ...target.execution, deviceId: 'another-device' } },
    { ...target, worktreeId: 'another-instance' },
  ].map((value) => presentExecutionTarget(value, 'Orchid', 'Codex'));
  expect(new Set(variants.map((choice) => choice.id)).size).toBe(variants.length);
  expect(new Set(variants.map((choice) => choice.label)).size).toBe(1);
});

it('orders the default branch, instantiated branches, dirty instances, then commit recency', () => {
  const branch = (name: string, committedAt: string): ReviewBranch =>
    ({
      target: { kind: 'branch', repositoryId: 'repository', branchRef: `refs/heads/${name}` },
      availableWorktreeCount: 0,
      worktreeIds: [],
      activity: null,
      repositoryId: 'repository',
      branchRef: `refs/heads/${name}`,
      displayName: name,
      tip: { objectId: name.padEnd(40, '0'), abbreviatedObjectId: name, subject: name, author: 'Test', committedAt },
      aheadOfDefault: 0,
      behindDefault: 0,
      associatedWorktreeCount: 0,
    }) as ReviewBranch;
  const branches = [
    branch('old-clean', '2026-01-01T00:00:00Z'),
    branch('dirty', '2026-01-02T00:00:00Z'),
    branch('main', '2025-01-01T00:00:00Z'),
    branch('no-worktree', '2026-03-01T00:00:00Z'),
  ];
  const profiles = [
    {
      ...target,
      capabilityProfileName: 'Laptop',
      instances: [
        { ...target, branchRef: 'refs/heads/old-clean', dirty: false, headCommittedAt: '2026-01-01T00:00:00Z' },
        { ...target, branchRef: 'refs/heads/dirty', dirty: true, headCommittedAt: '2026-01-02T00:00:00Z' },
        { ...target, branchRef: 'refs/heads/main', dirty: false, headCommittedAt: '2025-01-01T00:00:00Z' },
      ],
      error: null,
    },
  ];
  expect(
    orderTargetBranches(branches, { kind: 'branch', repositoryId: 'repository', branchRef: 'refs/heads/main' }, profiles).map(
      (item) => item.displayName,
    ),
  ).toEqual(['main', 'dirty', 'old-clean', 'no-worktree']);
});
