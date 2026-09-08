import type { ReactNode } from 'react';
import './resolvedValueField.css';

export interface ResolvedValueFieldProps {
  readonly label: string;
  readonly value: ReactNode;
  readonly source?: string;
  readonly inherited?: boolean;
  readonly locked?: boolean;
  readonly empty?: boolean;
}

/** Read-only value with just enough provenance to explain how runtime truth was resolved. */
export function ResolvedValueField({
  label,
  value,
  source,
  inherited = false,
  locked = false,
  empty = false,
}: ResolvedValueFieldProps) {
  return (
    <div className="resolved-value-field">
      <dt>
        <span>{label}</span>
        <span className="resolved-value-field__states">
          {inherited ? <small>Inherited</small> : null}
          {locked ? <small>Locked</small> : null}
        </span>
      </dt>
      <dd className={empty ? 'is-empty' : undefined}>{value}</dd>
      {source ? <small className="resolved-value-field__source">From {source}</small> : null}
    </div>
  );
}
