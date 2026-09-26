import { CapabilityProfileDialog } from './CapabilityProfileDialog';

interface CapabilityModelPickerDialogProps {
  readonly models: readonly string[];
  readonly observed: readonly string[];
  readonly selected: readonly string[];
  onAdd(model: string): void;
  onRemove(model: string): void;
  onClose(): void;
}

export function CapabilityModelPickerDialog({
  models,
  observed,
  selected,
  onAdd,
  onRemove,
  onClose,
}: CapabilityModelPickerDialogProps) {
  return (
    <CapabilityProfileDialog title="Add model" onClose={onClose}>
      <p>
        Choose from the latest observed options for this route. Current availability is checked when
        a session starts.
      </p>
      {models.length === 0 ? <p>No models are available.</p> : null}
      <ul className="capability-dialog__list">
        {models.map((model) => (
          <li key={model}>
            <span>
              {model}
              {!observed.includes(model) ? <small>Saved; not currently observed</small> : null}
            </span>
            {selected.includes(model) ? (
              <button type="button" onClick={() => onRemove(model)}>
                Remove
              </button>
            ) : (
              <button type="button" onClick={() => onAdd(model)}>
                Add
              </button>
            )}
          </li>
        ))}
      </ul>
    </CapabilityProfileDialog>
  );
}
