import { providerConfigurationLabel } from '../../application/agentProviders';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { ResolvedValueField } from '../../components/ResolvedValueField';
import { CapabilitySetInspector } from './CapabilitySetInspector';
import type { SessionProfileViewModel } from './types';
import './executionConfiguration.css';

export interface SessionProfileInspectorProps {
  readonly profile: SessionProfileViewModel;
}

/** Read-only presentation of the configuration pinned when an Agent Session was created. */
export function SessionProfileInspector({ profile }: SessionProfileInspectorProps) {
  return (
    <div className="execution-configuration" data-testid="session-profile-inspector">
      <header className="execution-configuration__header">
        <div>
          <span>Agent Session</span>
          <h1>Session configuration</h1>
          <p>Resolved runtime truth pinned at Session creation.</p>
        </div>
        <span className="execution-configuration__badge">Read only</span>
      </header>

      <CollapsibleSection
        title="Resolution"
        description="The exact definitions attached when this Session was created."
        className="execution-configuration__section"
      >
        <dl className="execution-configuration__resolved-grid">
          <ResolvedValueField
            label="Provider configuration"
            value={providerConfigurationLabel(profile.configuration)}
            source="Session creation"
          />
          <ResolvedValueField
            label="Capability profile"
            value={`${profile.capabilityProfileId} · revision ${profile.capabilityProfileRevision}`}
            source="Session creation"
          />
          {profile.resolutionDigest ? (
            <ResolvedValueField
              label="Resolution digest"
              value={<code>{profile.resolutionDigest}</code>}
              source="Session creation"
            />
          ) : null}
        </dl>
      </CollapsibleSection>

      <CollapsibleSection
        title="Pinned defaults"
        description="Workflow-addressed messages use these resolved defaults."
        className="execution-configuration__section"
      >
        <dl className="execution-configuration__resolved-grid">
          <SelectionField label="Model" value={profile.pinnedDefaults.model} />
          <SelectionField label="Reasoning" value={profile.pinnedDefaults.reasoningMode} />
          <SelectionField label="Sandbox" value={profile.pinnedDefaults.sandboxMode} />
        </dl>
      </CollapsibleSection>

      <CollapsibleSection
        title="Node capabilities"
        description="Capabilities exposed to workflow-triggered operation in this Session."
        className="execution-configuration__section"
      >
        <CapabilitySetInspector value={profile.nodeCapabilities} source="Resolved Node Profile" />
      </CollapsibleSection>

      <CollapsibleSection
        title="Attached runtime"
        description="Full runtime exposure retained for direct user messages."
        className="execution-configuration__section"
        defaultExpanded={false}
      >
        <dl className="execution-configuration__resolved-grid">
          <ResolvedValueField
            label="Model lock"
            value={profile.attachedRuntimeLocked.model ?? 'Not locked'}
            inherited
            locked={profile.attachedRuntimeLocked.model !== null}
            empty={profile.attachedRuntimeLocked.model === null}
          />
          <ResolvedValueField
            label="Reasoning lock"
            value={profile.attachedRuntimeLocked.reasoningMode ?? 'Not locked'}
            inherited
            locked={profile.attachedRuntimeLocked.reasoningMode !== null}
            empty={profile.attachedRuntimeLocked.reasoningMode === null}
          />
          <ResolvedValueField
            label="Sandbox lock"
            value={profile.attachedRuntimeLocked.sandboxMode ?? 'Not locked'}
            inherited
            locked={profile.attachedRuntimeLocked.sandboxMode !== null}
            empty={profile.attachedRuntimeLocked.sandboxMode === null}
          />
        </dl>
        <CapabilitySetInspector
          value={profile.attachedRuntimeCapabilities}
          source="Attached runtime"
          inherited
          locked
        />
      </CollapsibleSection>
    </div>
  );
}

function SelectionField({
  label,
  value,
}: {
  readonly label: string;
  readonly value: string | null;
}) {
  return (
    <ResolvedValueField
      label={label}
      value={value ?? 'Not selected'}
      source="Resolved Node Profile"
      empty={value === null}
    />
  );
}
