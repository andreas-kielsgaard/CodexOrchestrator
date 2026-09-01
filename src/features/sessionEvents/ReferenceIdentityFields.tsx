import type { ReferenceIdentity, SessionLogicalAddress } from './types';

interface ReferenceIdentityFieldsProps {
  readonly legend: string;
  readonly value: ReferenceIdentity;
  readonly disabled?: boolean;
  readonly onChange: (value: ReferenceIdentity) => void;
}

export function ReferenceIdentityFields({
  legend,
  value,
  disabled,
  onChange,
}: ReferenceIdentityFieldsProps) {
  return (
    <fieldset className="session-event-editor__reference" disabled={disabled}>
      <legend>{legend}</legend>
      <label>
        <span>Namespace</span>
        <input
          value={value.namespace}
          onChange={(event) => onChange({ ...value, namespace: event.target.value })}
        />
      </label>
      <label>
        <span>Kind</span>
        <input
          value={value.kind}
          onChange={(event) => onChange({ ...value, kind: event.target.value })}
        />
      </label>
      <label>
        <span>ID</span>
        <input
          value={value.id}
          onChange={(event) => onChange({ ...value, id: event.target.value })}
        />
      </label>
    </fieldset>
  );
}

interface LogicalAddressFieldsProps {
  readonly legend: string;
  readonly value: SessionLogicalAddress;
  readonly disabled?: boolean;
  readonly onChange: (value: SessionLogicalAddress) => void;
}

export function LogicalAddressFields({
  legend,
  value,
  disabled,
  onChange,
}: LogicalAddressFieldsProps) {
  return (
    <fieldset className="session-event-editor__address" disabled={disabled}>
      <legend>{legend}</legend>
      <ReferenceIdentityFields
        legend="Scope"
        value={value.scope}
        onChange={(scope) => onChange({ ...value, scope })}
      />
      <ReferenceIdentityFields
        legend="Subject"
        value={value.subject}
        onChange={(subject) => onChange({ ...value, subject })}
      />
    </fieldset>
  );
}
