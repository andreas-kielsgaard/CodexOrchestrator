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
