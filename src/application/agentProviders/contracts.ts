import type { ExecutionBindingDto } from '../executionTargets/contracts';

export interface AgentProviderDescriptor {
  readonly id: string;
  readonly label: string;
  readonly configurationLabel: string;
  /** The CLI harness a setup runs, for example "Codex CLI". */
  readonly harnessLabel: string;
  /** The account a setup's harness uses for inference. */
  readonly inferenceLabel: string;
}

/** A provider setup on a device: a native folder and the CLI executable that uses it. */
export interface ProviderSetupDto {
  readonly deviceId: string;
  readonly provider: string;
  readonly configurationId: string;
  readonly folder: string;
  readonly executable: string | null;
  readonly state: 'ready' | 'needs_login' | 'unavailable';
  readonly detail: string | null;
  readonly selected: boolean;
}

export interface HarnessInferenceRouteOption {
  readonly id: string;
  readonly selected: boolean;
  readonly label: string;
  readonly sourceLabel: string;
  readonly deviceLabel?: string;
  readonly harnessLabel?: string;
  readonly inferenceLabel?: string;
  readonly detail: string;
  readonly execution: ExecutionBindingDto;
}

/** One registered configuration of one agent provider. Compared as a pair, never parsed. */
export interface ProviderConfigurationRefDto {
  readonly provider: string;
  readonly configurationId: string;
}

/** Provider-native settings. Only the named provider's feature code reads `settings`. */
export interface ProviderNativeOptionsDto {
  readonly provider: string;
  readonly settings: unknown;
}

export function providerConfigurationLabel(configuration: ProviderConfigurationRefDto): string {
  return `${configuration.provider}/${configuration.configurationId}`;
}
