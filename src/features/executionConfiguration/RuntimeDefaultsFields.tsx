import { CatalogSingleSelect, type CatalogState } from '../../components/CatalogSelect';
import type {
  CapabilitySetViewModel,
  RuntimeCapabilityCatalogs,
  RuntimeSelectionsViewModel,
} from './types';

export function RuntimeDefaultsFields({
  catalogs,
  allowed,
  value,
  locked,
  onChange,
}: {
  catalogs: RuntimeCapabilityCatalogs;
  allowed: CapabilitySetViewModel;
  value: RuntimeSelectionsViewModel;
  locked?: RuntimeSelectionsViewModel;
  onChange(value: RuntimeSelectionsViewModel): void;
}) {
  return (
    <div className="execution-configuration__field-grid">
      <DefaultSelection
        label="Default model"
        catalog={withinExposure(catalogs.models, allowed.models)}
        value={value.model}
        lockedValue={locked?.model}
        onChange={(model) => onChange({ ...value, model })}
      />
      <DefaultSelection
        label="Default reasoning"
        catalog={withinExposure(catalogs.reasoningModes, allowed.reasoningModes)}
        value={value.reasoningMode}
        lockedValue={locked?.reasoningMode}
        onChange={(reasoningMode) => onChange({ ...value, reasoningMode })}
      />
    </div>
  );
}

function DefaultSelection<T extends string>({
  label,
  catalog,
  value,
  lockedValue,
  onChange,
}: {
  readonly label: string;
  readonly catalog: CatalogState<T>;
  readonly value: T | null;
  readonly lockedValue?: T | null;
  onChange(value: T | null): void;
}) {
  const locked = lockedValue !== null && lockedValue !== undefined;
  return (
    <CatalogSingleSelect
      label={label}
      catalog={catalog}
      value={locked ? lockedValue : value}
      disabled={locked}
      hint={locked ? `Inherited runtime lock: ${lockedValue}` : undefined}
      onChange={onChange}
    />
  );
}

function withinExposure<T extends string>(
  catalog: CatalogState<T>,
  exposed: readonly T[],
): CatalogState<T> {
  return {
    ...catalog,
    options: catalog.options.filter((option) => exposed.includes(option.value)),
  };
}
