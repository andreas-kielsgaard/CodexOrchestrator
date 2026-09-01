import type {
  RuntimeProfileSnapshotDto,
  SessionCreationResolutionDto,
} from '../../application/executionConfiguration';
import type { CatalogState } from '../../components/CatalogSelect';
import {
  mcpToolCatalogValue,
  type RuntimeCapabilityCatalogs,
  type RuntimeProfileViewModel,
  type SessionProfileViewModel,
} from './types';

function availableCatalog<T extends string>(
  values: readonly T[],
  sourceLabel: string,
): CatalogState<T> {
  return {
    availability: 'available',
    options: values.map((value) => ({ value, label: value })),
    sourceLabel,
  };
}

/** Makes the provider-owned runtime snapshot consumable by controlled profile editors. */
export function runtimeProfileViewModel(
  runtime: RuntimeProfileSnapshotDto,
): RuntimeProfileViewModel {
  const sourceLabel = 'Selected native runtime';
  const mcpTools = Object.entries(runtime.exposure.mcpTools).flatMap(([connectionId, tools]) =>
    tools.map((toolId) => ({
      value: mcpToolCatalogValue(connectionId, toolId),
      label: `${connectionId}/${toolId}`,
    })),
  );
  const inheritedCatalog = <T extends string>(label: string): CatalogState<T> => ({
    availability: 'unavailable',
    options: [],
    sourceLabel,
    reason: `${label} are inherited from the selected runtime in this build.`,
  });

  const catalogs: RuntimeCapabilityCatalogs = {
    models: availableCatalog(runtime.exposure.models, sourceLabel),
    reasoningModes: availableCatalog(runtime.exposure.reasoningModes, sourceLabel),
    sandboxModes: availableCatalog(runtime.exposure.sandboxModes, sourceLabel),
    mcpTools:
      mcpTools.length > 0
        ? { availability: 'available', options: mcpTools, sourceLabel }
        : inheritedCatalog('MCP connections'),
    skills:
      runtime.exposure.skills.length > 0
        ? availableCatalog(runtime.exposure.skills, sourceLabel)
        : inheritedCatalog('Skills'),
  };

  return {
    profileRef: runtime.profileRef,
    sourceLabel,
    exposure: runtime.exposure,
    catalogs,
    lockedSelections: runtime.locked,
    notes: [
      'Provider connections remain managed separately; this profile selects from the one active native runtime.',
    ],
  };
}

export function sessionProfileViewModel(
  resolution: SessionCreationResolutionDto,
): SessionProfileViewModel {
  const profile = resolution.sessionProfile;
  return {
    runtimeProfileRef: profile.runtimeProfileRef,
    capabilityProfileId: profile.capabilityProfileId,
    capabilityProfileRevision: profile.capabilityProfileRevision,
    attachedRuntimeCapabilities: profile.attachedRuntimeCapabilities,
    attachedRuntimeLocked: profile.attachedRuntimeLocked,
    nodeCapabilities: profile.nodeCapabilities,
    pinnedDefaults: profile.pinnedDefaults,
    resolutionDigest: resolution.digest,
  };
}
