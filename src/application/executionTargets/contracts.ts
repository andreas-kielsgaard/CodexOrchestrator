import type {
  RuntimeProfileSnapshotDto,
} from '../executionConfiguration';

export interface ExecutionBindingDto {
  readonly deviceId: string;
  readonly deviceName: string;
  readonly provider: 'codex';
  readonly configurationRef: string;
  readonly connection:
    | { readonly kind: 'local' }
    | { readonly kind: 'ssh'; readonly target: string; readonly hostExecutable: string };
}
export const localExecutionBinding: ExecutionBindingDto = {
  deviceId: 'local',
  deviceName: 'This laptop',
  provider: 'codex',
  configurationRef: 'selected',
  connection: { kind: 'local' },
};
export interface SessionExecutionTargetDto {
  readonly capabilityProfileId: string;
  readonly capabilityProfileRevision: number;
  readonly execution: ExecutionBindingDto;
  readonly repositoryId: string;
  readonly branchRef: string;
  readonly worktreeId: string;
  readonly path: string;
  readonly head: string | null;
}
export interface SessionExecutionSelectionDto {
  readonly capabilityProfileId: string;
  readonly capabilityProfileRevision: number;
  readonly execution: ExecutionBindingDto;
  readonly workspace:
    | { readonly kind: 'existing'; readonly target: SessionExecutionTargetDto }
    | {
        readonly kind: 'create';
        readonly repositoryId: string;
        readonly branchRef: string;
        readonly commit: string;
        readonly attachment: 'branch';
      }
    | { readonly kind: 'auxiliary' };
}
export interface ExecutionTargetProfileDto {
  readonly capabilityProfileId: string;
  readonly capabilityProfileRevision: number;
  readonly capabilityProfileName: string;
  readonly execution: ExecutionBindingDto;
}
export interface TargetWorktreeDto {
  readonly worktreeId: string;
  readonly path: string;
  readonly head: string | null;
  readonly dirty?: boolean;
  readonly headCommittedAt?: string | null;
  readonly branchRef: string;
  readonly sisterLock?: SisterWorktreeLockDto | null;
  readonly isSister?: boolean | null;
}
export interface SisterWorktreeLockDto {
  readonly sisterGroupId: string;
  readonly activeDeviceId: string;
  readonly ownerSessionId: string;
}
export interface ProfileWorktreeTargetsDto extends ExecutionTargetProfileDto {
  readonly instances: readonly TargetWorktreeDto[];
  readonly error: string | null;
  readonly sisterLock?: SisterWorktreeLockDto | null;
}
export interface ConfiguredExecutionDeviceDto {
  readonly deviceId: string;
  readonly deviceName: string;
  readonly profiles: readonly ExecutionTargetProfileDto[];
}
export interface DeviceCommandSpecDto {
  readonly program: string;
  readonly arguments: readonly string[];
  readonly workingDirectory: string | null;
  readonly timeoutSeconds: number;
}
export interface DeviceLifecyclePolicyDto {
  readonly start: DeviceCommandSpecDto | null;
  readonly stop: DeviceCommandSpecDto | null;
  readonly idleShutdownSeconds: number | null;
}
export interface ExecutionDeviceConfigurationDto {
  readonly deviceId: string;
  readonly displayName: string;
  readonly connectionSummary: string;
  readonly lifecycle: DeviceLifecyclePolicyDto;
  readonly activeLeases: number;
  readonly lastOrchidActivityAt: string | null;
  readonly keepAwakeUntil: string | null;
  readonly lastLifecycleMessage: string | null;
}
export interface ExecutionTargetDeviceDto extends ConfiguredExecutionDeviceDto {
  readonly profiles: readonly ProfileWorktreeTargetsDto[];
}
export type WorktreeChoiceScopeDto =
  { readonly kind: 'local' } | { readonly kind: 'device'; readonly deviceId: string };
export interface RepositoryWorktreeChoicesDto {
  readonly repositoryId: string;
  readonly repositoryName: string;
  readonly profiles: readonly ProfileWorktreeTargetsDto[];
}
export interface RepositoryDeviceLocationDto {
  readonly repositoryId: string;
  readonly deviceId: string;
  readonly repositoryRoot: string;
}
export interface ExecutionTargetRuntimeDto {
  readonly runtimeProfile: RuntimeProfileSnapshotDto;
}
export interface ExecutionTargetClient {
  listConfiguredDevices?(): Promise<readonly ExecutionDeviceConfigurationDto[]>;
  saveConfiguredDevice?(input: {
    readonly deviceId: string;
    readonly displayName: string;
    readonly lifecycle: DeviceLifecyclePolicyDto;
  }): Promise<readonly ExecutionDeviceConfigurationDto[]>;
  startConfiguredDevice?(deviceId: string): Promise<void>;
  stopConfiguredDevice?(deviceId: string): Promise<void>;
  holdConfiguredDeviceAwake?(
    deviceId: string,
    seconds: number,
  ): Promise<readonly ExecutionDeviceConfigurationDto[]>;
  resolvePublishedTip?(input: {
    repositoryId: string;
    branchRef: string;
    execution: ExecutionBindingDto;
  }): Promise<{ commit: string }>;
  listDevices(): Promise<readonly ConfiguredExecutionDeviceDto[]>;
  listWorktreeChoices(
    scope: WorktreeChoiceScopeDto,
  ): Promise<readonly RepositoryWorktreeChoicesDto[]>;
  listTargets(
    repositoryId: string,
    branchRef: string,
  ): Promise<readonly ExecutionTargetDeviceDto[]>;
  loadRuntime(
    execution: ExecutionBindingDto,
    workingDirectory?: string,
  ): Promise<ExecutionTargetRuntimeDto>;
  listRepositoryLocations(): Promise<readonly RepositoryDeviceLocationDto[]>;
  saveRepositoryLocation(input: RepositoryDeviceLocationDto): Promise<void>;
}
