import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import './otpElementPicker.css';

export interface OtpPickerItem {
  readonly id: string;
  readonly label: string;
  readonly disabled?: boolean;
}
export interface OtpPickerGroup {
  readonly id: string;
  readonly label: string;
  readonly items: readonly OtpPickerItem[];
}

/** Selection is local until confirmed; the caller owns eligibility and persistence. */
export function OtpElementPicker({
  title,
  groups,
  selected,
  multiple = false,
  renderDetails,
  onConfirm,
  onClose,
}: {
  readonly title: string;
  readonly groups: readonly OtpPickerGroup[];
  readonly selected: readonly string[];
  readonly multiple?: boolean;
  readonly renderDetails: (id: string) => ReactNode;
  readonly onConfirm: (ids: readonly string[]) => void;
  readonly onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useId();
  const [preview, setPreview] = useState<string | null>(selected[0] ?? null);
  const [checked, setChecked] = useState(selected);
  const items = groups.flatMap((group) => group.items);
  const [expanded, setExpanded] = useState(
    () =>
      new Set(
        groups
          .filter((group) => group.items.some((item) => selected.includes(item.id)))
          .map((g) => g.id),
      ),
  );
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const element = dialog.current!;
    element.showModal();
    return () => {
      element.close();
      previous?.focus();
    };
  }, []);
  const missing = checked.filter((id) => !items.some((item) => item.id === id));
  const candidate = items.find((item) => item.id === preview);
  return (
    <dialog
      ref={dialog}
      className="otp-picker"
      aria-labelledby={heading}
      onCancel={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onClose();
      }}
      onKeyDown={(event) => event.stopPropagation()}
    >
      <header>
        <h2 id={heading}>{title}</h2>
        <button type="button" onClick={onClose} aria-label="Close picker">
          ×
        </button>
      </header>
      <div className="otp-picker__body">
        <nav aria-label="OTP packages">
          {groups.map((group) => (
            <div key={group.id} className="otp-picker__group">
              <button
                type="button"
                aria-expanded={expanded.has(group.id)}
                onClick={() =>
                  setExpanded((old) => {
                    const next = new Set(old);
                    if (next.has(group.id)) next.delete(group.id);
                    else next.add(group.id);
                    return next;
                  })
                }
              >
                <span aria-hidden="true">{expanded.has(group.id) ? '▾' : '▸'}</span> {group.label}
              </button>
              {expanded.has(group.id) && (
                <ul>
                  {group.items.map((item) => (
                    <li key={item.id}>
                      {multiple && (
                        <input
                          type="checkbox"
                          aria-label={`Include ${item.label}`}
                          checked={checked.includes(item.id)}
                          disabled={item.disabled && !checked.includes(item.id)}
                          onChange={() =>
                            setChecked((old) =>
                              old.includes(item.id)
                                ? old.filter((id) => id !== item.id)
                                : [...old, item.id],
                            )
                          }
                        />
                      )}
                      <button
                        type="button"
                        aria-pressed={preview === item.id}
                        onClick={() => setPreview(item.id)}
                      >
                        {item.label}
                        {item.disabled ? ' (unavailable)' : ''}
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ))}
          {items.length === 0 && <p>No eligible elements are available.</p>}
          {missing.length > 0 && (
            <div className="otp-picker__missing">
              <strong>Unavailable selections</strong>
              {missing.map((id) => (
                <div key={id}>
                  <span>{id}</span>
                  {multiple && (
                    <button
                      type="button"
                      onClick={() => setChecked((old) => old.filter((value) => value !== id))}
                    >
                      Remove
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </nav>
        <section className="otp-picker__details" aria-label="Element details">
          {candidate ? (
            renderDetails(candidate.id)
          ) : (
            <p>Select an element to inspect its details.</p>
          )}
        </section>
      </div>
      <footer>
        {multiple && <span>{checked.length} selected</span>}
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button
          type="button"
          disabled={!multiple && (!candidate || candidate.disabled)}
          onClick={() => onConfirm(multiple ? checked : [candidate!.id])}
        >
          {multiple ? 'Apply selection' : title}
        </button>
      </footer>
    </dialog>
  );
}
