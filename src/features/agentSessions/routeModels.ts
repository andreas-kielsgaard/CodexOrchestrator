import { agentProviderLabel } from '../../application/agentProviders';
import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import type { ExecutionBindingDto } from '../../application/executionTargets/contracts';

type QuickModel = AgentSessionQuickFeatures['models'][number];

/** The reasoning order Capability Profile allowances are validated against. */
const REASONING_ORDER = ['none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'];

function reasoningRange(minimum: string, maximum: string): string[] {
  const first = REASONING_ORDER.indexOf(minimum);
  const last = REASONING_ORDER.indexOf(maximum);
  if (first < 0 || last < first) return [...new Set([minimum, maximum])];
  return REASONING_ORDER.slice(first, last + 1);
}

/**
 * Models the Capability Profile offers on the same device through its other provider routes. The
 * provider follows from the device and model: sending with one of these moves the Session to that
 * route when the message is prepared.
 */
export function otherRouteModels(
  profile: CapabilityProfileDto | null | undefined,
  execution: ExecutionBindingDto | null | undefined,
): QuickModel[] {
  if (!profile || !execution) return [];
  return (profile.routePolicies ?? [])
    .filter(
      (route) =>
        route.execution.deviceId === execution.deviceId &&
        route.execution.provider !== execution.provider,
    )
    .flatMap((route) =>
      route.modelAllowances.map((allowance) => {
        const reasoningModes =
          allowance.minimumReasoning && allowance.maximumReasoning
            ? reasoningRange(allowance.minimumReasoning, allowance.maximumReasoning)
            : [];
        return {
          id: allowance.modelId,
          label: `${allowance.modelId} · ${agentProviderLabel(route.execution.provider)}`,
          description: '',
          defaultReasoningMode: route.defaults.reasoningMode ?? null,
          reasoningModes: reasoningModes.map((id) => ({ id, description: '' })),
        };
      }),
    );
}

/** Adds other-route models the current route does not already offer. */
export function withOtherRouteModels(
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
