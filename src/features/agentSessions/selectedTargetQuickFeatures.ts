import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import type {
  CapabilityProfileDto,
  RuntimeProfileSnapshotDto,
} from '../../application/executionConfiguration';
export function selectedTargetQuickFeatures(
  runtime: RuntimeProfileSnapshotDto | null,
  profile: CapabilityProfileDto | null,
): AgentSessionQuickFeatures | undefined {
  if (!runtime || !profile) return undefined;
  const reasoningModes = runtime.exposure.reasoningModes.filter((mode) =>
    profile.allowedCapabilities.reasoningModes.includes(mode),
  );
  return {
    profileRef: runtime.profileRef,
    defaults: {
      model: profile.defaults?.model ?? runtime.locked.model,
      reasoningMode: profile.defaults?.reasoningMode ?? runtime.locked.reasoningMode,
    },
    models: runtime.exposure.models
      .filter((model) => profile.allowedCapabilities.models.includes(model))
      .map((id) => ({
        id,
        label: id,
        description: '',
        defaultReasoningMode: profile.defaults?.reasoningMode ?? runtime.locked.reasoningMode,
        reasoningModes: reasoningModes.map((id) => ({ id, description: '' })),
      })),
    skills: [],
    limitations: [],
  };
}
export function mergeSelectedQuickFeatures(
  selected: AgentSessionQuickFeatures,
  native: AgentSessionQuickFeatures,
): AgentSessionQuickFeatures {
  return {
    ...native,
    defaults: selected.defaults,
    models: selected.models.map((model) => {
      const discovered = native.models.find((item) => item.id === model.id);
      return discovered
        ? {
            ...discovered,
            reasoningModes: discovered.reasoningModes.filter((mode) =>
              model.reasoningModes.some((allowed) => allowed.id === mode.id),
            ),
          }
        : model;
    }),
  };
}
