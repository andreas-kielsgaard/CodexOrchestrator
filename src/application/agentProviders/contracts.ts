import type { ExecutionBindingDto } from '../executionTargets/contracts';

export interface AgentProviderDescriptor {
  readonly id: string;
  readonly label: string;
  readonly configurationLabel: string;
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
