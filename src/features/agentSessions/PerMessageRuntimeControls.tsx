import './perMessageRuntimeControls.css';

export interface PerMessageRuntimeOption {
  readonly value: string;
  readonly label: string;
  readonly disabled?: boolean;
}

export interface PerMessageRuntimeSelection {
  readonly model: string | null;
  readonly reasoningMode: string | null;
}

export interface PerMessageRuntimeControlsProps {
  readonly value: PerMessageRuntimeSelection;
  readonly models: readonly PerMessageRuntimeOption[];
  readonly reasoningModes: readonly PerMessageRuntimeOption[];
  readonly defaultModel?: string | null;
  readonly defaultReasoning?: string | null;
  readonly disabled?: boolean;
  readonly onChange: (value: PerMessageRuntimeSelection) => void;
}

/**
 * Controlled options for one direct-user invocation. The parent owns the draft selection and
 * decides when to clear it; this component does not mutate the Session's pinned profile.
 */
export function PerMessageRuntimeControls({
  value,
  models,
  reasoningModes,
  defaultModel,
  defaultReasoning,
  disabled,
  onChange,
}: PerMessageRuntimeControlsProps) {
  const effectiveModel =
    value.model ?? defaultModel ?? models.find((option) => !option.disabled)?.value ?? '';
  const effectiveReasoning =
    value.reasoningMode ??
    defaultReasoning ??
    reasoningModes.find((option) => !option.disabled)?.value ??
    '';
  return (
    <fieldset className="per-message-runtime-controls" disabled={disabled}>
      <legend>Options for this message</legend>
      <p>These choices apply only to the next user message.</p>
      <div className="per-message-runtime-controls__fields">
        <label>
          <span>Model</span>
          <select
            value={effectiveModel}
            onChange={(event) => onChange({ ...value, model: event.target.value })}
          >
            {!effectiveModel && <option value="">No model available</option>}
            {models.map((model) => (
              <option key={model.value} value={model.value} disabled={model.disabled}>
                {model.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Reasoning</span>
          <select
            value={effectiveReasoning}
            onChange={(event) =>
              onChange({
                ...value,
                reasoningMode: event.target.value,
              })
            }
          >
            {!effectiveReasoning && <option value="">No reasoning available</option>}
            {reasoningModes.map((mode) => (
              <option key={mode.value} value={mode.value} disabled={mode.disabled}>
                {mode.label}
              </option>
            ))}
          </select>
        </label>
      </div>
    </fieldset>
  );
}
