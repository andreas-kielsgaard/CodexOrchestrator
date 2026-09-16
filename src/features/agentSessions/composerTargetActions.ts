import type {
  ExecutionTargetClient,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import {
  presentExecutionTarget,
  toSessionExecutionTarget,
} from '../../application/executionTargets/presentation';
import type { ComposerQuickAction, ComposerQuickPage } from './composerQuickMenuTypes';

export interface ComposerTargetSource {
  readonly contextKey: string;
  readonly client: Pick<ExecutionTargetClient, 'listDevices' | 'listWorktreeChoices'>;
  readonly target: SessionExecutionTargetDto | null;
  readonly disabledReason?: string;
  onBrowseBranches?(deviceId?: string): void;
  onSelectTarget(target: SessionExecutionTargetDto): void;
}

export function composerTargetActions(
  source: ComposerTargetSource,
): readonly ComposerQuickAction[] {
  const selectedId = source.target ? presentExecutionTarget(source.target, '', '').id : null;
  const worktrees = async (
    scope: Parameters<ExecutionTargetClient['listWorktreeChoices']>[0],
    title: string,
  ): Promise<ComposerQuickPage> => {
    const repositories = await source.client.listWorktreeChoices(scope);
    const limitations: string[] = [];
    const items = repositories.flatMap((repository) =>
      repository.profiles.flatMap((profile) => {
        if (profile.error)
          limitations.push(
            `${repository.repositoryName} · ${profile.capabilityProfileName}: ${profile.error}`,
          );
        return profile.instances.map((instance): ComposerQuickAction => {
          const target = toSessionExecutionTarget(repository.repositoryId, profile, instance);
          const choice = presentExecutionTarget(
            target,
            repository.repositoryName,
            profile.capabilityProfileName,
          );
          return {
            ...choice,
            selected: choice.id === selectedId,
            run: () => {
              source.onSelectTarget(target);
              return {
                replacement: '',
                notice: `Target: ${choice.label} on ${target.execution.deviceName}`,
              };
            },
          };
        });
      }),
    );
    if (source.onBrowseBranches)
      items.push({
        id: 'browse-branches',
        label: 'Browse branches…',
        description: 'Choose a branch or confirm a new worktree',
        run: () => {
          source.onBrowseBranches?.(scope.kind === 'device' ? scope.deviceId : 'local');
          return { replacement: '', notice: '' };
        },
      });
    return {
      title,
      items,
      limitations,
      emptyMessage: 'No existing branch worktrees are available in the configured repositories.',
    };
  };
  return [
    {
      id: 'worktree',
      label: 'Worktree',
      description: 'Choose an existing worktree on this laptop',
      disabledReason: source.disabledReason,
      acceptsSlash: true,
      loadChildren: () => worktrees({ kind: 'local' }, 'Local worktrees'),
    },
    {
      id: 'device',
      label: 'Device',
      description: 'Choose a configured device, then a worktree',
      disabledReason: source.disabledReason,
      loadChildren: async () => ({
        title: 'Devices',
        emptyMessage: 'No devices are configured in Capability Profiles.',
        items: (await source.client.listDevices()).map((device) => ({
          id: `device:${device.deviceId}`,
          label: device.deviceName,
          description: device.profiles.map((profile) => profile.capabilityProfileName).join(' · '),
          keywords: [device.deviceId],
          acceptsSlash: true,
          loadChildren: () =>
            worktrees(
              { kind: 'device', deviceId: device.deviceId },
              `${device.deviceName} worktrees`,
            ),
        })),
      }),
    },
  ];
}
