import { codexProviderDescriptor } from './codex/descriptor';
import type { AgentProviderDescriptor } from './contracts';

/** Registered provider descriptors. Explicit composition; there is no dynamic provider loading. */
const DESCRIPTORS: readonly AgentProviderDescriptor[] = [codexProviderDescriptor];

/** Display name for a provider identity. An unregistered provider shows its identity. */
export function agentProviderLabel(provider: string): string {
  return DESCRIPTORS.find((descriptor) => descriptor.id === provider)?.label ?? provider;
}
