import { codexProviderDescriptor } from './codex/descriptor';
import type { AgentProviderDescriptor } from './contracts';

/** Registered provider descriptors. Explicit composition; there is no dynamic provider loading. */
const DESCRIPTORS: readonly AgentProviderDescriptor[] = [codexProviderDescriptor];

/** A provider's descriptor. An unregistered provider is described by its identity. */
export function agentProviderDescriptor(provider: string): AgentProviderDescriptor {
  return (
    DESCRIPTORS.find((descriptor) => descriptor.id === provider) ?? {
      id: provider,
      label: provider,
      configurationLabel: `${provider} setup`,
      harnessLabel: provider,
      inferenceLabel: provider,
    }
  );
}

/** Display name for a provider identity. An unregistered provider shows its identity. */
export function agentProviderLabel(provider: string): string {
  return agentProviderDescriptor(provider).label;
}
