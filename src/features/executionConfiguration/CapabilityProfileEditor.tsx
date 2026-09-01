import { CollapsibleSection } from '../../components/CollapsibleSection';
import { ValidationSummary } from '../../components/ValidationSummary';
import { CapabilitySetFields } from './CapabilitySetFields';
import { RuntimeProfileInspector } from './RuntimeProfileInspector';
import type { CapabilityProfileDraft, RuntimeProfileViewModel } from './types';
import './executionConfiguration.css';

export interface CapabilityProfileEditorProps {
  readonly profile: CapabilityProfileDraft;
  readonly runtime: RuntimeProfileViewModel;
  readonly validationErrors?: readonly string[];
  readonly saving?: boolean;
  onChange(profile: CapabilityProfileDraft): void;
  onSave?(profile: CapabilityProfileDraft): void;
}

/** Controlled editor for the reusable capability ceiling applied before node restrictions. */
export function CapabilityProfileEditor({
  profile,
  runtime,
  validationErrors = [],
  saving = false,
  onChange,
  onSave,
}: CapabilityProfileEditorProps) {
  const existing = profile.revision !== null;
  return (
    <div className="execution-configuration" data-testid="capability-profile-editor">
      <header className="execution-configuration__header">
        <div>
          <span>Execution configuration</span>
          <h1>{existing ? profile.name : 'New capability profile'}</h1>
          <p>Choose the technical capabilities that workflow nodes may expose.</p>
        </div>
        {existing ? (
          <span className="execution-configuration__badge">Revision {profile.revision}</span>
        ) : null}
      </header>

      <ValidationSummary errors={validationErrors} />

      <CollapsibleSection
        title="Profile details"
        description="A stable identifier for this reusable capability definition."
        className="execution-configuration__section"
      >
        <label className="execution-configuration__field">
          <span>Name</span>
          <input
            aria-label="Capability profile name"
            value={profile.name}
            autoComplete="off"
            onChange={(event) => onChange({ ...profile, name: event.currentTarget.value })}
          />
        </label>
        <label className="execution-configuration__field">
          <span>Capability profile ID</span>
          <input
            aria-label="Capability profile ID"
            value={profile.capabilityProfileId}
            disabled={existing}
            autoComplete="off"
            onChange={(event) =>
              onChange({ ...profile, capabilityProfileId: event.currentTarget.value })
            }
          />
          {existing ? <small>The stable ID cannot be changed after creation.</small> : null}
        </label>
      </CollapsibleSection>

      <RuntimeProfileInspector runtime={runtime} defaultExpanded={false} />

      <CollapsibleSection
        title="Allowed capabilities"
        description="Selections must remain within the attached runtime exposure."
        className="execution-configuration__section"
      >
        <CapabilitySetFields
          catalogs={runtime.catalogs}
          value={profile.allowedCapabilities}
          scopeLabel="Nodes using this profile"
          onChange={(allowedCapabilities) => onChange({ ...profile, allowedCapabilities })}
        />
      </CollapsibleSection>

      {onSave ? (
        <footer className="execution-configuration__actions">
          <button
            type="button"
            className="is-primary"
            disabled={
              saving ||
              validationErrors.length > 0 ||
              !profile.capabilityProfileId.trim() ||
              !profile.name.trim()
            }
            onClick={() => onSave(profile)}
          >
            {saving ? 'Saving…' : existing ? 'Save new revision' : 'Create profile'}
          </button>
        </footer>
      ) : null}
    </div>
  );
}
