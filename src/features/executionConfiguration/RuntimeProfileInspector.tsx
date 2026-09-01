import { CollapsibleSection } from '../../components/CollapsibleSection';
import { ResolvedValueField } from '../../components/ResolvedValueField';
import type { CapabilitySetViewModel, RuntimeProfileViewModel } from './types';
import { describeMcpTools } from './types';
import './executionConfiguration.css';

export interface RuntimeProfileInspectorProps {
  readonly runtime: RuntimeProfileViewModel;
  readonly defaultExpanded?: boolean;
}

/** Read-only view of the native runtime facts available to capability resolution. */
export function RuntimeProfileInspector({
  runtime,
  defaultExpanded = true,
}: RuntimeProfileInspectorProps) {
  return (
    <CollapsibleSection
      title="Runtime profile"
      description="Observed from the globally selected native runtime. These values are inherited."
      defaultExpanded={defaultExpanded}
      className="execution-configuration__section"
      headerAccessory={<span className="execution-configuration__badge">Read only</span>}
    >
      <dl className="execution-configuration__resolved-grid">
        <ResolvedValueField
          label="Runtime reference"
          value={runtime.profileRef}
          source={runtime.sourceLabel}
          inherited
          locked
        />
        <ResolvedValueField
          label="Model lock"
          value={runtime.lockedSelections.model ?? 'Not locked'}
          source={runtime.sourceLabel}
          inherited
          locked={runtime.lockedSelections.model !== null}
          empty={runtime.lockedSelections.model === null}
        />
        <ResolvedValueField
          label="Reasoning lock"
          value={runtime.lockedSelections.reasoningMode ?? 'Not locked'}
          source={runtime.sourceLabel}
          inherited
          locked={runtime.lockedSelections.reasoningMode !== null}
          empty={runtime.lockedSelections.reasoningMode === null}
        />
        <ResolvedValueField
          label="Sandbox lock"
          value={runtime.lockedSelections.sandboxMode ?? 'Not locked'}
          source={runtime.sourceLabel}
          inherited
          locked={runtime.lockedSelections.sandboxMode !== null}
          empty={runtime.lockedSelections.sandboxMode === null}
        />
      </dl>
      <CapabilitySummary value={runtime.exposure} />
      {runtime.notes?.length ? (
        <ul className="execution-configuration__notes">
          {runtime.notes.map((note) => (
            <li key={note}>{note}</li>
          ))}
        </ul>
      ) : null}
    </CollapsibleSection>
  );
}

function CapabilitySummary({ value }: { readonly value: CapabilitySetViewModel }) {
  const rows = [
    ['Models', value.models],
    ['Reasoning', value.reasoningModes],
    ['MCP tools', describeMcpTools(value.mcpTools)],
    ['Skills', value.skills],
    ['Sandbox', value.sandboxModes],
  ] as const;
  return (
    <dl className="execution-configuration__capability-summary">
      {rows.map(([label, values]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{values.length ? values.join(', ') : 'None exposed'}</dd>
        </div>
      ))}
    </dl>
  );
}
