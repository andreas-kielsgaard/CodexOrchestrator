import type { CapabilityProfileDto, ProfileModelCatalogueDto } from './contracts';
import type { ExecutionBindingDto } from '../executionTargets/contracts';
import { executionRouteKey } from '../executionTargets/contracts';

export type ProfileExposedModel = ProfileModelCatalogueDto['models'][number];

const REASONING_ORDER = ['none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'];

function reasoningRange(minimum: string, maximum: string): string[] {
  const first = REASONING_ORDER.indexOf(minimum);
  const last = REASONING_ORDER.indexOf(maximum);
  if (first < 0 || last < first) return [...new Set([minimum, maximum])];
  return REASONING_ORDER.slice(first, last + 1);
}

/** Models exposed by every provider route on the selected device. */
export function capabilityProfileDeviceModels(
  profile: CapabilityProfileDto | null | undefined,
  execution: ExecutionBindingDto | null | undefined,
  catalogues: Readonly<Record<string, ProfileModelCatalogueDto>> = {},
): ProfileExposedModel[] {
  if (!profile || !execution) return [];
  return (profile.routePolicies ?? [])
    .filter((route) => route.execution.deviceId === execution.deviceId)
    .flatMap((route) => {
      const observed = catalogues[executionRouteKey(route.execution)]?.models;
      if (observed?.length) return observed;
      return route.modelAllowances.map((allowance) => {
        const reasoningModes = reasoningRange(
          allowance.minimumReasoning,
          allowance.maximumReasoning,
        );
        return {
          id: allowance.modelId,
          label: allowance.modelId,
          description: '',
          defaultReasoningMode: route.defaults.reasoningMode ?? reasoningModes.at(-1) ?? null,
          reasoningModes: reasoningModes.map((id) => ({ id, description: '' })),
        };
      });
    });
}
