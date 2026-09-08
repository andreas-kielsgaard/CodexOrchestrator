import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementSnapshot,
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

  return (
    <section className="agent-session-model-controls" aria-label="Current Session model">
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
      <small title="The selected model is stored for this Session and comes from the application-wide catalog.">
        Saved for this Session · application model catalog
      </small>
      {error && <span role="alert">{error}</span>}
    </section>
  );
}
