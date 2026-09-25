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
  const reasoningModes = runtime.exposure.reasoningModes;
  return {
    configuration: runtime.configuration,
    defaults: {
      model: profile.defaults?.model ?? runtime.locked.model,
      reasoningMode: profile.defaults?.reasoningMode ?? runtime.locked.reasoningMode,
    },
    models: runtime.exposure.models.map((id) => ({
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
    defaults: {
      model: selected.defaults.model ?? native.defaults.model,
      reasoningMode: selected.defaults.reasoningMode ?? native.defaults.reasoningMode,
    },
    models: native.models.length ? native.models : selected.models,
  };
}
