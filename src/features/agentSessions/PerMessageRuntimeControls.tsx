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
  readonly defaultModelLabel?: string;
  readonly defaultReasoningLabel?: string;
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
  defaultModelLabel = 'Use Session default',
  defaultReasoningLabel = 'Use Session default',
  disabled,
  onChange,
}: PerMessageRuntimeControlsProps) {
  return (
    <fieldset className="per-message-runtime-controls" disabled={disabled}>
      <legend>Options for this message</legend>
      <p>These choices apply only to the next user message.</p>
      <div className="per-message-runtime-controls__fields">
        <label>
          <span>Model</span>
          <select
            value={value.model ?? ''}
            onChange={(event) =>
              onChange({ ...value, model: event.target.value ? event.target.value : null })
            }
          >
            <option value="">{defaultModelLabel}</option>
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
            value={value.reasoningMode ?? ''}
            onChange={(event) =>
              onChange({
                ...value,
                reasoningMode: event.target.value ? event.target.value : null,
              })
            }
          >
            <option value="">{defaultReasoningLabel}</option>
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
