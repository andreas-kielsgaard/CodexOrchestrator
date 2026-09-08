import { ResolvedValueField } from '../../components/ResolvedValueField';
import type { CapabilitySetViewModel } from './types';
import { describeMcpTools } from './types';
import './executionConfiguration.css';

export function CapabilitySetInspector({
  value,
  source,
  inherited = false,
  locked = false,
}: {
  readonly value: CapabilitySetViewModel;
  readonly source: string;
  readonly inherited?: boolean;
  readonly locked?: boolean;
}) {
  const fields = [
    ['Models', value.models],
    ['Reasoning modes', value.reasoningModes],
    ['MCP tools', describeMcpTools(value.mcpTools)],
    ['Skills', value.skills],
    ['Sandbox modes', value.sandboxModes],
  ] as const;
  return (
    <dl className="execution-configuration__resolved-grid">
      {fields.map(([label, entries]) => (
        <ResolvedValueField
          key={label}
          label={label}
          value={entries.length ? entries.join(', ') : 'None'}
          source={source}
          inherited={inherited}
          locked={locked}
          empty={entries.length === 0}
        />
      ))}
    </dl>
  );
}
