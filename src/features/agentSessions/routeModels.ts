import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
type QuickModel = AgentSessionQuickFeatures['models'][number];

export { capabilityProfileDeviceModels } from '../../application/executionConfiguration';

/** Adds device models the selected runtime catalogue does not already contain. */
export function withCapabilityProfileDeviceModels(
  capabilities: AgentSessionQuickFeatures | undefined,
  models: readonly QuickModel[],
): AgentSessionQuickFeatures | undefined {
  if (!capabilities || models.length === 0) return capabilities;
  const known = new Set(capabilities.models.map((model) => model.id));
  return {
    ...capabilities,
    models: [...capabilities.models, ...models.filter((model) => !known.has(model.id))],
  };
}
