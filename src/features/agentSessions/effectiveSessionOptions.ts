import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';

/**
 * A reasoning level is chosen by the user, the profile or the model's reported default. Without
 * one it stays unset and the provider applies its own default.
 */
export interface EffectiveSessionOptions {
  readonly model: AgentSessionQuickFeatures['models'][number] | null;
  readonly reasoningMode: string | null;
}

export function effectiveSessionOptions(
  capabilities: AgentSessionQuickFeatures | undefined,
  selection: PerMessageRuntimeSelection,
): EffectiveSessionOptions {
  if (!capabilities) return { model: null, reasoningMode: null };
  const model =
    capabilities.models.find((item) => item.id === selection.model) ??
    capabilities.models.find((item) => item.id === capabilities.defaults.model) ??
    capabilities.models[0] ??
    null;
  const reasoningMode =
    supportedReasoning(model, selection.reasoningMode) ??
    supportedReasoning(model, capabilities.defaults.reasoningMode) ??
    supportedReasoning(model, model?.defaultReasoningMode ?? null);
  return { model, reasoningMode };
}

export function selectionForModel(
  capabilities: AgentSessionQuickFeatures,
  selection: PerMessageRuntimeSelection,
  modelId: string,
): PerMessageRuntimeSelection {
  const current = effectiveSessionOptions(capabilities, selection);
  const model = capabilities.models.find((item) => item.id === modelId);
  const reasoningMode =
    supportedReasoning(model ?? null, current.reasoningMode) ??
    supportedReasoning(model ?? null, model?.defaultReasoningMode ?? null);
  return { model: modelId, reasoningMode };
}

function supportedReasoning(
  model: AgentSessionQuickFeatures['models'][number] | null,
  value: string | null | undefined,
): string | null {
  return value && model?.reasoningModes.some((mode) => mode.id === value) ? value : null;
}
