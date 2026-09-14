import type {
  RuntimeProfileSnapshotDto,
  NativeCapabilityInventoryDto,
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
export interface ExecutionTargetDeviceDto {
  readonly deviceId: string;
  readonly deviceName: string;
  readonly profiles: readonly {
    readonly capabilityProfileId: string;
    readonly capabilityProfileRevision: number;
    readonly capabilityProfileName: string;
    readonly execution: ExecutionBindingDto;
    readonly instances: readonly {
      readonly worktreeId: string;
      readonly path: string;
      readonly head: string | null;
      readonly branchRef: string;
    }[];
    readonly error: string | null;
  }[];
}
export interface RepositoryDeviceLocationDto {
  readonly repositoryId: string;
  readonly deviceId: string;
  readonly repositoryRoot: string;
}
export interface ExecutionTargetRuntimeDto {
  readonly runtimeProfile: RuntimeProfileSnapshotDto;
  readonly nativeInventory: NativeCapabilityInventoryDto;
}
export interface ExecutionTargetClient {
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
