import { useEffect, useId, useRef, useState, type FormEvent, type KeyboardEvent } from 'react';
import type { AssignedAgentIdentity, IdentityShape } from '../../application/identities';
import { AgentIdentityBadge } from './AgentIdentityBadge';
import { normalizedIdentityColor } from './identityPresentation';
import './identities.css';

const IDENTITY_SHAPES: readonly IdentityShape[] = ['circle', 'square', 'hexagon'];

export interface IdentityPickerDialogProps {
  readonly identity: AssignedAgentIdentity;
  readonly title?: string;
  readonly confirmLabel?: string;
  onSave(identity: AssignedAgentIdentity): void;
  onClose(): void;
}

/** Edits the independent identity value that will be owned by an Agent Session. */
export function IdentityPickerDialog({
  identity,
  title = 'Edit identity',
  confirmLabel = 'Apply identity',
  onSave,
  onClose,
}: IdentityPickerDialogProps) {
  const titleId = useId();
  const shapeName = useId();
  const dialogRef = useRef<HTMLFormElement>(null);
  const [displayName, setDisplayName] = useState(identity.displayName);
  const [color, setColor] = useState(() => normalizedIdentityColor(identity.color));
  const [shape, setShape] = useState<IdentityShape>(identity.shape);

  useEffect(() => {
    setDisplayName(identity.displayName);
    setColor(normalizedIdentityColor(identity.color));
    setShape(identity.shape);
  }, [identity]);

  useEffect(() => {
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialogRef.current?.focus();
    return () => opener?.focus();
  }, []);

  const draft: AssignedAgentIdentity = {
    originIdentityId: identity.originIdentityId,
    displayName: displayName.trim() || identity.displayName,
    color,
    shape,
  };

  const save = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmedName = displayName.trim();
    if (!trimmedName) return;
    onSave({ ...draft, displayName: trimmedName });
  };

  const handleDialogKeyDown = (event: KeyboardEvent<HTMLFormElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ) ?? [],
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div className="identity-picker__backdrop">
      <form
        ref={dialogRef}
        className="identity-picker"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onSubmit={save}
        onKeyDown={handleDialogKeyDown}
      >
        <header className="identity-picker__header">
          <div>
            <h2 id={titleId}>{title}</h2>
            <p>Name, color, and shape apply to this Agent Session.</p>
          </div>
          <button type="button" aria-label={`Close ${title}`} onClick={onClose}>
            <span aria-hidden="true">×</span>
          </button>
        </header>

        <div className="identity-picker__body">
          <div className="identity-picker__preview" aria-label="Identity preview">
            <AgentIdentityBadge identity={draft} />
          </div>

          <label className="identity-picker__field">
            <span>Display name</span>
            <input
              aria-label="Identity display name"
              value={displayName}
              onChange={(event) => setDisplayName(event.target.value)}
              autoComplete="off"
            />
          </label>

          <label className="identity-picker__field">
            <span>Color</span>
            <span className="identity-picker__color-field">
              <input
                type="color"
                aria-label="Identity color"
                value={color}
                onChange={(event) => setColor(event.target.value)}
              />
              <output>{color}</output>
            </span>
          </label>

          <fieldset className="identity-picker__shape-field">
            <legend>Shape</legend>
            <div className="identity-picker__shape-options">
              {IDENTITY_SHAPES.map((candidate) => (
                <label key={candidate}>
                  <input
                    type="radio"
                    name={shapeName}
                    value={candidate}
                    checked={shape === candidate}
                    onChange={() => setShape(candidate)}
                  />
                  <span
                    className={`identity-picker__shape-swatch is-${candidate}`}
                    aria-hidden="true"
                  />
                  <span>{capitalize(candidate)}</span>
                </label>
              ))}
            </div>
          </fieldset>
        </div>

        <footer className="identity-picker__actions">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="is-primary" disabled={!displayName.trim()}>
            {confirmLabel}
          </button>
        </footer>
      </form>
    </div>
  );
}

function capitalize(value: string): string {
  return `${value.charAt(0).toLocaleUpperCase()}${value.slice(1)}`;
}
