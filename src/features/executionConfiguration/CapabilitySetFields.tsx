import { CatalogMultiSelect } from '../../components/CatalogSelect';
import { OtpMcpToolsPicker } from './OtpMcpToolsPicker';
import type { CapabilitySetViewModel, RuntimeCapabilityCatalogs } from './types';
import { mcpToolsFromSelectedValues, selectedMcpToolValues } from './types';

interface CapabilitySetFieldsProps {
  readonly catalogs: RuntimeCapabilityCatalogs;
  readonly value: CapabilitySetViewModel;
  readonly disabled?: boolean;
  readonly scopeLabel: string;
  onChange(value: CapabilitySetViewModel): void;
}

export function CapabilitySetFields({
  catalogs,
  value,
  disabled = false,
  scopeLabel,
  onChange,
}: CapabilitySetFieldsProps) {
  return (
    <div className="execution-configuration__capability-fields">
      <CatalogMultiSelect
        label="Models"
        catalog={catalogs.models}
        values={value.models}
        disabled={disabled}
        hint={`${scopeLabel} may use these models.`}
        onChange={(models) => onChange({ ...value, models })}
      />
      <CatalogMultiSelect
        label="Reasoning modes"
        catalog={catalogs.reasoningModes}
        values={value.reasoningModes}
        disabled={disabled}
        hint={`${scopeLabel} may use these reasoning modes.`}
        onChange={(reasoningModes) => onChange({ ...value, reasoningModes })}
      />
      <OtpMcpToolsPicker
        packages={catalogs.otpPackages ?? []}
        catalog={catalogs.mcpTools}
        values={selectedMcpToolValues(value.mcpTools)}
        disabled={disabled}
        onChange={(values) => onChange({ ...value, mcpTools: mcpToolsFromSelectedValues(values) })}
      />
      <CatalogMultiSelect
        label="Skills"
        catalog={catalogs.skills}
        values={value.skills}
        disabled={disabled}
        hint={`${scopeLabel} may expose these skills.`}
        onChange={(skills) => onChange({ ...value, skills })}
      />
    </div>
  );
}
