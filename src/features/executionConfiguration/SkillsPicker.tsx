import { useState } from 'react';
import type { CatalogState } from '../../components/CatalogSelect';
import {
  GroupedElementPicker,
  type GroupedElementPickerNode,
} from '../../components/otp/GroupedElementPicker';

export function SkillsPicker({
  catalog,
  values,
  disabled,
  onChange,
}: {
  readonly catalog: CatalogState;
  readonly values: readonly string[];
  readonly disabled?: boolean;
  onChange(values: readonly string[]): void;
}) {
  const [open, setOpen] = useState(false);
  const groups = new Map<string, GroupedElementPickerNode & { items: typeof catalog.options }>();
  for (const option of catalog.options) {
    const separator = option.value.includes(':') ? ':' : option.value.includes('/') ? '/' : null;
    const source = separator ? option.value.split(separator)[0] : 'Codex profile';
    const existing = groups.get(source) ?? {
      id: `skills:${source}`,
      label: source,
      items: [],
    };
    groups.set(source, { ...existing, items: [...existing.items, option] });
  }
  return (
    <div>
      <strong>Skills</strong>
      <div className="otp-selection">
        <span className="otp-selection__summary">
          {values.length ? values.join(', ') : 'No skills selected'}
        </span>
        <button
          type="button"
          disabled={disabled || catalog.availability === 'unavailable'}
          onClick={() => setOpen(true)}
        >
          Set skills
        </button>
      </div>
      {catalog.reason ? <p>{catalog.reason}</p> : null}
      {open ? (
        <GroupedElementPicker
          title="Set skills"
          groups={[...groups.values()]}
          selected={values}
          selectionMode="multiple"
          renderDetails={(id) => (
            <>
              <h3>{id}</h3>
              <p>Skill available to Sessions created by this node.</p>
            </>
          )}
          onClose={() => setOpen(false)}
          onConfirm={(next) => {
            onChange(next);
            setOpen(false);
          }}
        />
      ) : null}
    </div>
  );
}
