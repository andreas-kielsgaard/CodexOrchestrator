import type { CapabilitySetDto, NodeProfileDto } from '../../application/executionConfiguration';
import type { CatalogState } from '../../components/CatalogSelect';
import { selectedMcpToolValues, type RuntimeCapabilityCatalogs } from './types';

function restrict<T extends string>(
  catalog: CatalogState<T>,
  allowed: readonly T[],
): CatalogState<T> {
  return {
    ...catalog,
    options: catalog.options.filter((option) => allowed.includes(option.value)),
  };
}

export function nodeProfileCatalogs(
  runtime: RuntimeCapabilityCatalogs,
  ceiling?: CapabilitySetDto,
): RuntimeCapabilityCatalogs {
  return {
    models: restrict(runtime.models, ceiling?.models ?? []),
    reasoningModes: restrict(runtime.reasoningModes, ceiling?.reasoningModes ?? []),
    sandboxModes: restrict(runtime.sandboxModes, ceiling?.sandboxModes ?? []),
    mcpTools: restrict(runtime.mcpTools, selectedMcpToolValues(ceiling?.mcpTools ?? {})),
    skills: restrict(runtime.skills, ceiling?.skills ?? []),
  };
}

export function nodeProfileValidationErrors(
  profile: NodeProfileDto,
  ceiling?: CapabilitySetDto,
): readonly string[] {
  if (!ceiling) return ['Choose an available Capability Profile.'];
  const errors: string[] = [];
  for (const [label, allowed, selected, pinned] of [
    ['model', ceiling.models, profile.allowedCapabilities.models, profile.pinnedDefaults.model],
    [
      'reasoning mode',
      ceiling.reasoningModes,
      profile.allowedCapabilities.reasoningModes,
      profile.pinnedDefaults.reasoningMode,
    ],
    [
      'sandbox mode',
      ceiling.sandboxModes,
      profile.allowedCapabilities.sandboxModes,
      profile.pinnedDefaults.sandboxMode,
    ],
  ] as const) {
    if (selected.some((choice) => !(allowed as readonly string[]).includes(choice))) {
      errors.push(`An exposed ${label} is outside the selected profile.`);
    }
    if (pinned && !(selected as readonly string[]).includes(pinned)) {
      errors.push(
        `Default ${label} “${pinned}” is not exposed. Choose another default or expose it.`,
      );
    }
  }
  return errors;
}
