import type { PinnedAgentSessionProfileDto } from '../../application/agentSessionProfiles';
import type { EventDeliveryRecordDto } from '../../application/sessionEvents';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { SessionProfileInspector, sessionProfileViewModel } from '../executionConfiguration';
import { EventDeliveryList } from '../sessionEvents';
import {
  PerMessageRuntimeControls,
  type PerMessageRuntimeSelection,
} from './PerMessageRuntimeControls';

export interface AgentSessionExecutionSettingsProps {
  readonly profile: PinnedAgentSessionProfileDto | null;
  readonly profileError: string | null;
  readonly deliveries: readonly EventDeliveryRecordDto[];
  readonly selection: PerMessageRuntimeSelection;
  readonly disabled?: boolean;
  readonly onSelectionChange: (selection: PerMessageRuntimeSelection) => void;
}

/** Session-facing projection of pinned creation truth and message-local user authority. */
export function AgentSessionExecutionSettings({
  profile,
  profileError,
  deliveries,
  selection,
  disabled,
  onSelectionChange,
}: AgentSessionExecutionSettingsProps) {
  if (!profile) {
    return profileError ? (
      <p className="agent-session-execution-settings__unavailable" role="status">
        Pinned Session Profile unavailable. Direct user messages require a resolved Session Profile.
        <small>{profileError}</small>
      </p>
    ) : (
      <p className="agent-session-execution-settings__unavailable" role="status">
        Loading pinned Session configuration…
      </p>
    );
  }

  const resolved = profile.creationResolution.sessionProfile;
  return (
    <div className="agent-session-execution-settings">
      <CollapsibleSection
        title="Message and Session configuration"
        description="The Session Profile is pinned; model and reasoning may be selected for the next direct user message."
        defaultExpanded={false}
      >
        <PerMessageRuntimeControls
          value={selection}
          models={resolved.attachedRuntimeCapabilities.models.map((model) => ({
            value: model,
            label: model,
          }))}
          reasoningModes={resolved.attachedRuntimeCapabilities.reasoningModes.map((mode) => ({
            value: mode,
            label: mode,
          }))}
          defaultModelLabel={
            resolved.pinnedDefaults.model
              ? `Use Session default · ${resolved.pinnedDefaults.model}`
              : 'Use Session default'
          }
          defaultReasoningLabel={
            resolved.pinnedDefaults.reasoningMode
              ? `Use Session default · ${resolved.pinnedDefaults.reasoningMode}`
              : 'Use Session default'
          }
          disabled={disabled}
          onChange={onSelectionChange}
        />
        <SessionProfileInspector profile={sessionProfileViewModel(profile.creationResolution)} />
        <CollapsibleSection
          title="Session Event deliveries"
          description="Recorded workflow deliveries that addressed this Session."
          defaultExpanded={false}
          className="execution-configuration__section"
        >
          <EventDeliveryList
            deliveries={deliveries}
            emptyMessage="No Workflow Session Event deliveries have addressed this Session yet."
          />
        </CollapsibleSection>
      </CollapsibleSection>
    </div>
  );
}
