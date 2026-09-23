import type { PinnedAgentSessionProfileDto } from '../../application/agentSessions';
import { useState } from 'react';
import type { AssignedAgentIdentity } from '../../application/identities';
import { AgentIdentityBadge, IdentityPickerDialog } from '../identities';
import type { EventDeliveryRecordDto } from '../../application/sessionEvents';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { SessionProfileInspector, sessionProfileViewModel } from '../executionConfiguration';
import { EventDeliveryList } from '../sessionEvents';
import {
  PerMessageRuntimeControls,
  type PerMessageRuntimeSelection,
} from './PerMessageRuntimeControls';

export interface AgentSessionExecutionSettingsProps {
  readonly showMessageControls?: boolean;
  readonly profile: PinnedAgentSessionProfileDto | null;
  readonly profileError: string | null;
  readonly identity?: AssignedAgentIdentity | null;
  readonly onIdentityChange?: (identity: AssignedAgentIdentity) => Promise<void>;
  readonly deliveryError?: string | null;
  readonly onReloadDeliveries?: () => void;
  readonly deliveries: readonly EventDeliveryRecordDto[];
  readonly selection: PerMessageRuntimeSelection;
  readonly disabled?: boolean;
  readonly onSelectionChange: (selection: PerMessageRuntimeSelection) => void;
}

/** Session-facing projection of pinned creation truth and message-local user authority. */
export function AgentSessionExecutionSettings({
  profile,
  showMessageControls = true,
  profileError,
  deliveries,
  selection,
  disabled,
  onSelectionChange,
  identity,
  onIdentityChange,
  deliveryError,
  onReloadDeliveries,
}: AgentSessionExecutionSettingsProps) {
  const [editingIdentity, setEditingIdentity] = useState(false);
  const [identityError, setIdentityError] = useState<string | null>(null);
  const resolved = profile?.creationResolution.sessionProfile;
  return (
    <div className="agent-session-execution-settings">
      <div className="agent-session-execution-settings__identity">
        {identity ? <AgentIdentityBadge identity={identity} /> : <span>No assigned identity</span>}
        {onIdentityChange ? (
          <button type="button" disabled={disabled} onClick={() => setEditingIdentity(true)}>
            {identity ? 'Edit identity' : 'Set identity'}
          </button>
        ) : null}
        {identityError ? <p role="alert">{identityError}</p> : null}
      </div>
      {editingIdentity && onIdentityChange ? (
        <IdentityPickerDialog
          identity={
            identity ?? {
              originIdentityId: null,
              displayName: 'Agent',
              color: '#39745a',
              shape: 'circle',
            }
          }
          onClose={() => setEditingIdentity(false)}
          onSave={(next) => {
            setIdentityError(null);
            void onIdentityChange(next).then(
              () => setEditingIdentity(false),
              (cause) => setIdentityError(String(cause)),
            );
          }}
        />
      ) : null}
      {profile && resolved ? (
        <CollapsibleSection title="Message and Session configuration" defaultExpanded={false}>
          {showMessageControls && (
            <>
              <p>
                Model and reasoning choices apply to your next message. Other settings come from the
                Session Profile.
              </p>
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
                defaultModel={resolved.pinnedDefaults.model}
                defaultReasoning={resolved.pinnedDefaults.reasoningMode}
                disabled={disabled}
                onChange={onSelectionChange}
              />
            </>
          )}
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
            {deliveryError ? <p role="alert">{deliveryError}</p> : null}
            {onReloadDeliveries ? (
              <button type="button" onClick={onReloadDeliveries}>
                Refresh deliveries
              </button>
            ) : null}
          </CollapsibleSection>
        </CollapsibleSection>
      ) : (
        <p className="agent-session-execution-settings__unavailable" role="status">
          {profileError
            ? 'Pinned Session Profile unavailable. Direct user messages require a resolved Session Profile.'
            : 'Loading pinned Session configuration…'}
          {profileError ? <small>{profileError}</small> : null}
        </p>
      )}
    </div>
  );
}
