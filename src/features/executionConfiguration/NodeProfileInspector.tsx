import type {
  CapabilityProfileDto,
  NodeProfileDto,
  RuntimeSelectionsDto,
} from '../../application/executionConfiguration';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { ResolvedValueField } from '../../components/ResolvedValueField';
import { CapabilitySetInspector } from './CapabilitySetInspector';
import './executionConfiguration.css';

export function NodeProfileInspector({
  nodeProfile,
  capabilityProfile,
  capabilityProfileId,
  runtimeLockedSelections,
}: {
  readonly nodeProfile: NodeProfileDto;
  readonly capabilityProfile?: CapabilityProfileDto;
  readonly capabilityProfileId: string;
  readonly runtimeLockedSelections?: RuntimeSelectionsDto;
}) {
  return (
    <div className="execution-configuration" data-testid="node-profile-inspector">
      <CollapsibleSection
        title="Capability profile"
        description="The pinned profile reference and its current catalog entry. Session truth is shown in each Session."
        className="execution-configuration__section"
      >
        <dl className="execution-configuration__resolved-grid">
          <ResolvedValueField
            label="Profile ID"
            value={capabilityProfileId}
            source="Pinned workflow recipe"
          />
          <ResolvedValueField
            label="Current catalog name"
            value={capabilityProfile?.name ?? 'Unavailable'}
            source="Current profile catalog"
            empty={!capabilityProfile}
          />
          <ResolvedValueField
            label="Current revision"
            value={capabilityProfile ? String(capabilityProfile.revision) : 'Unavailable'}
            source="Current profile catalog"
            empty={!capabilityProfile}
          />
        </dl>
        <CapabilitySetInspector
          value={capabilityProfile?.allowedCapabilities ?? nodeProfile.allowedCapabilities}
          source={capabilityProfile ? 'Current Capability Profile' : 'Pinned node declaration'}
          inherited
        />
      </CollapsibleSection>

      <CollapsibleSection
        title="Node capabilities"
        description="The restrictions declared by this node. Session creation resolves and pins them."
        className="execution-configuration__section"
      >
        <CapabilitySetInspector
          value={nodeProfile.allowedCapabilities}
          source="Pinned node declaration"
        />
      </CollapsibleSection>

      <CollapsibleSection
        title="Pinned defaults"
        description="Workflow messages use these defaults after runtime locks are applied."
        className="execution-configuration__section"
      >
        <dl className="execution-configuration__resolved-grid">
          {(
            [
              ['Model', 'model'],
              ['Reasoning', 'reasoningMode'],
              ['Sandbox', 'sandboxMode'],
            ] as const
          ).map(([label, key]) => {
            const inherited = runtimeLockedSelections?.[key] ?? null;
            const value = inherited ?? nodeProfile.pinnedDefaults[key];
            return (
              <ResolvedValueField
                key={key}
                label={label}
                value={value ?? 'Not selected'}
                source={inherited ? 'Runtime lock' : 'Pinned node declaration'}
                inherited={Boolean(inherited)}
                locked={Boolean(inherited)}
                empty={!value}
              />
            );
          })}
        </dl>
      </CollapsibleSection>
    </div>
  );
}
