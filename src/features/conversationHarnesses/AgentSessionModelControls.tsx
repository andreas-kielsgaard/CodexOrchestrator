import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementSnapshot,
  HarnessReasoningLevel,
} from '../../application/conversationHarnesses';

export function AgentSessionModelControls({
  snapshot,
  disabled,
  error,
  onCommand,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly disabled: boolean;
  readonly error: string | null;
  onCommand?(command: ConversationHarnessManagementCommand): void;
}) {
  const override = snapshot.modelChoices.sessionOverride;
  const selectedModel = override?.model ?? '';
  const selectedReasoning = override?.reasoning ?? '';
  const catalogModel = snapshot.catalogs.models.items.find((model) => model.id === selectedModel);
  const reasoningOptions = catalogModel?.reasoningLevels ?? [];
  const controlsDisabled = disabled || !onCommand;

  const chooseModel = (modelId: string) => {
    if (!modelId) {
      onCommand?.({ kind: 'set_session_model_override', override: null });
      return;
    }
    onCommand?.({
      kind: 'set_session_model_override',
      override: {
        model: modelId,
        reasoning: null,
      },
    });
  };

  const chooseReasoning = (reasoning: HarnessReasoningLevel | '') => {
    if (!selectedModel) return;
    onCommand?.({
      kind: 'set_session_model_override',
      override: {
        model: selectedModel,
        reasoning: reasoning || null,
      },
    });
  };

  return (
    <section className="agent-session-model-controls" aria-label="Current Session model and effort">
      <label>
        <span>Model</span>
        <select
          aria-label="Session model"
          value={selectedModel}
          disabled={controlsDisabled}
          onChange={(event) => chooseModel(event.target.value)}
        >
          <option value="">Caller choice</option>
          {snapshot.catalogs.models.items.map((model) => (
            <option value={model.id} key={model.id}>
              {model.label}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>Effort</span>
        <select
          aria-label="Session effort"
          value={selectedReasoning}
          disabled={controlsDisabled || !selectedModel}
          onChange={(event) => chooseReasoning(event.target.value as HarnessReasoningLevel | '')}
        >
          <option value="">Caller choice</option>
          {reasoningOptions.map((level) => (
            <option value={level} key={level}>
              {level}
            </option>
          ))}
        </select>
      </label>
      <small title="This Session choice is stored independently and uses the application-wide model catalog.">
        This Session · application model catalog
      </small>
      {error && <span role="alert">{error}</span>}
    </section>
  );
}
