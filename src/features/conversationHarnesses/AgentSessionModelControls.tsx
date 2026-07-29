import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementSnapshot,
  HarnessModelPolicy,
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
  const constraints = policyForAppliedRevision(snapshot);
  const override = snapshot.modelChoices.sessionOverride;
  const selectedModel = override?.model ?? '';
  const selectedReasoning = override?.reasoning ?? '';
  const modelConfiguration = constraints?.models.find(
    (model) => model.modelId === selectedModel && model.allowed,
  );
  const catalogModel = snapshot.catalogs.models.items.find((model) => model.id === selectedModel);
  const reasoningOptions =
    modelConfiguration && catalogModel
      ? catalogModel.reasoningLevels.slice(
          catalogModel.reasoningLevels.indexOf(modelConfiguration.minReasoning),
          catalogModel.reasoningLevels.indexOf(modelConfiguration.maxReasoning) + 1,
        )
      : [];
  const controlsDisabled = disabled || !onCommand || !constraints;

  const chooseModel = (modelId: string) => {
    if (!constraints) return;
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
    if (!constraints || !selectedModel) return;
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
          {snapshot.catalogs.models.items
            .filter((model) =>
              constraints?.models.some(
                (configuration) => configuration.modelId === model.id && configuration.allowed,
              ),
            )
            .map((model) => (
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
      <small title="This Session choice is stored separately and stays within the applied Harness model and effort constraints.">
        This Session Â· v{snapshot.sessionBinding.appliedRevision ?? 'untracked'} constraints
      </small>
      {error && <span role="alert">{error}</span>}
    </section>
  );
}

function policyForAppliedRevision(
  snapshot: ConversationHarnessManagementSnapshot,
): HarnessModelPolicy | null {
  const revision = snapshot.sessionBinding.appliedRevision;
  if (revision === null) return null;
  const version = snapshot.versionControl.versions.find(
    (candidate) => candidate.revision === revision,
  );
  if (!version) return null;
  if (version.configuration.runtime.modelPolicyMode === 'delegated_shared')
    return (
      snapshot.modelChoices.delegatedPolicies.find((policy) => policy.revision === revision)
        ?.policy ?? policyFromConfiguration(version.configuration)
    );
  return policyFromConfiguration(version.configuration);
}

function policyFromConfiguration(
  configuration: ConversationHarnessManagementSnapshot['versionControl']['versions'][number]['configuration'],
): HarnessModelPolicy {
  return {
    models: configuration.runtime.models,
    defaultModel: configuration.runtime.defaultModel,
    defaultReasoning: configuration.runtime.defaultReasoning,
  };
}
