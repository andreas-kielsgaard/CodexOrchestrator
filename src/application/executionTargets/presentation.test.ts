import { localExecutionBinding, type SessionExecutionTargetDto } from './contracts';
import { presentExecutionTarget, toSessionExecutionTarget } from './presentation';

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
