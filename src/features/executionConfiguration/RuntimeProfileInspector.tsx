import { CollapsibleSection } from '../../components/CollapsibleSection';
import { ResolvedValueField } from '../../components/ResolvedValueField';
import { CapabilitySetInspector } from './CapabilitySetInspector';
import type { RuntimeProfileViewModel } from './types';
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
      <CapabilitySetInspector
        value={runtime.exposure}
        source={runtime.sourceLabel}
        inherited
        locked
      />
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
