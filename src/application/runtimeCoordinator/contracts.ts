import type { ExecutionDeviceConfigurationDto } from '../executionTargets/contracts';

export type CoordinatorLocation =
  | { readonly view: 'list' | 'setup' | 'settings' }
  | { readonly view: 'device' | 'legacy'; readonly deviceId: string };
export interface CloudServer {
  readonly id: number;
  readonly name: string;
  readonly status: string;
  readonly address: string | null;
  readonly location: string | null;
}
export interface PowerOperation {
  readonly id: string;
  readonly deviceId: string;
  readonly command: 'start' | 'stop';
  readonly status: string;
  readonly providerActionId: number | null;
  readonly requestedAt: string;
  readonly detail: string | null;
}
export interface CoordinatorDevice {
  readonly id: string;
  readonly displayName: string;
  readonly server: CloudServer;
  readonly observedAt: string;
  readonly credentialConfigured: boolean;
  readonly operation: PowerOperation | null;
  readonly observationError: string | null;
}
export interface CoordinatorBinding {
  readonly coordinatorId: string;
  readonly deviceId: string;
  readonly executionDeviceId: string;
}
export interface CoordinatorOverview {
  readonly mode: 'local' | 'external';
  readonly endpoint: string | null;
  readonly coordinatorId: string | null;
  readonly error: string | null;
  readonly devices: readonly CoordinatorDevice[];
  readonly bindings: readonly CoordinatorBinding[];
  readonly legacyDevices: readonly ExecutionDeviceConfigurationDto[];
}
export interface Discovery {
  readonly credentialId: string;
  readonly servers: readonly CloudServer[];
}
export interface DeviceSetupDraft {
  readonly coordinatorId: string;
  readonly displayName: string;
  readonly serverId: string;
  readonly executionDeviceId: string;
  readonly discovery?: Discovery;
}
export interface RuntimeCoordinatorClient {
  overview(): Promise<CoordinatorOverview>;
  dockerSettings(): Promise<CoordinatorOverview>;
  settings(input: {
    externalUrl: string | null;
    token: string | null;
  }): Promise<CoordinatorOverview>;
  connect(token: string): Promise<Discovery>;
  register(input: {
    credentialId: string;
    serverId: number;
    displayName: string;
    executionDeviceId: string | null;
  }): Promise<CoordinatorDevice>;
  refresh(id: string): Promise<CoordinatorDevice>;
  rename(id: string, name: string): Promise<CoordinatorDevice>;
  power(
    id: string,
    input: { operationId: string; command: 'start' | 'stop' },
  ): Promise<PowerOperation>;
}
