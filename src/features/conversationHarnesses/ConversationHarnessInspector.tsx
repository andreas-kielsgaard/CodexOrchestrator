import {
  ArrowLeft,
  Check,
  GitCommitHorizontal,
  HelpCircle,
  Pencil,
  Search,
  Upload,
  Users,
  X,
} from 'lucide-react';
import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementRead,
  ConversationHarnessManagementSnapshot,
  HarnessEffectiveConfiguration,
  HarnessModelPolicy,
  HarnessVisualIdentity,
} from '../../application/conversationHarnesses';
import { AgentIdentityBadge } from '../../components/AgentIdentityBadge';
import { HarnessDefinitionEditor, SearchableSingleSelect } from './HarnessDefinitionEditor';

export interface ConversationHarnessManagementProps {
  readonly read: ConversationHarnessManagementRead | null;
  readonly commandPending?: boolean;
  readonly commandError?: string | null;
  onBack(): void;
  onCommand?(command: ConversationHarnessManagementCommand): void;
}

interface Confirmation {
  readonly title: string;
  readonly body: string;
  readonly confirmLabel: string;
  readonly command: ConversationHarnessManagementCommand;
}

type VersionSelection = `version:${number}` | 'draft';

export function ConversationHarnessManagement({
  read,
  commandPending = false,
  commandError,
  onBack,
  onCommand,
}: ConversationHarnessManagementProps) {
  if (!read)
    return (
      <ManagementShell onBack={onBack}>
        <p className="harness-management__loading" role="status">
          Loading harness...
        </p>
      </ManagementShell>
    );

  if (read.kind !== 'available')
    return (
      <ManagementShell onBack={onBack}>
        <div className="harness-management__unavailable" role="alert">
          <h2>{read.kind === 'unbound' ? 'No harness assigned' : 'Harness unavailable'}</h2>
          <p>{read.reason}</p>
        </div>
      </ManagementShell>
    );

  return (
    <AvailableHarnessManagement
      snapshot={read.snapshot}
      commandPending={commandPending}
      commandError={commandError}
      onBack={onBack}
      onCommand={onCommand}
    />
  );
}

function AvailableHarnessManagement({
  snapshot,
  commandPending,
  commandError,
  onBack,
  onCommand,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly commandPending: boolean;
  readonly commandError?: string | null;
  onBack(): void;
  onCommand?(command: ConversationHarnessManagementCommand): void;
}) {
  const initialRevision =
    snapshot.sessionBinding.appliedRevision ??
    snapshot.versionControl.pushedRevision ??
    snapshot.versionControl.versions.at(-1)?.revision ??
    0;
  const [selected, setSelected] = useState<VersionSelection>(`version:${initialRevision}`);
  const [editMode, setEditMode] = useState(false);
  const [identityDialogOpen, setIdentityDialogOpen] = useState(false);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const highestRevision = Math.max(
    0,
    ...snapshot.versionControl.versions.map((version) => version.revision),
  );
  const previousHighestRevision = useRef(highestRevision);

  useEffect(() => {
    if (highestRevision <= previousHighestRevision.current) return;
    previousHighestRevision.current = highestRevision;
    setSelected(`version:${highestRevision}`);
  }, [highestRevision]);

  useEffect(() => {
    if (selected === 'draft' && !snapshot.workingCopy && !editMode)
      setSelected(`version:${snapshot.sessionBinding.appliedRevision ?? highestRevision}`);
  }, [
    editMode,
    highestRevision,
    selected,
    snapshot.sessionBinding.appliedRevision,
    snapshot.workingCopy,
  ]);

  const selectedRevision =
    selected === 'draft' ? null : Number.parseInt(selected.replace('version:', ''), 10);
  const selectedVersion = snapshot.versionControl.versions.find(
    (version) => version.revision === selectedRevision,
  );
  const fallbackVersion =
    snapshot.versionControl.versions.find(
      (version) => version.revision === snapshot.sessionBinding.appliedRevision,
    ) ?? snapshot.versionControl.versions.at(-1);
  const configuration =
    selected === 'draft' && snapshot.workingCopy
      ? snapshot.workingCopy.configuration
      : (selectedVersion?.configuration ?? fallbackVersion?.configuration);
  if (!configuration)
    return (
      <ManagementShell onBack={onBack}>
        <div className="harness-management__unavailable" role="alert">
          <h2>Harness unavailable</h2>
          <p>No harness version can be displayed.</p>
        </div>
      </ManagementShell>
    );

  const editable = Boolean(editMode && selected === 'draft' && snapshot.workingCopy && onCommand);
  const saveConfiguration = (next: HarnessEffectiveConfiguration) => {
    if (!snapshot.workingCopy || !editable) return;
    onCommand?.({
      kind: 'save_working_copy',
      configuration: next,
    });
  };
  const beginEdit = () => {
    setEditMode(true);
    setSelected('draft');
    if (!snapshot.workingCopy && selectedRevision !== null)
      onCommand?.({ kind: 'start_edit', baseRevision: selectedRevision });
  };
  const openConfirmation = (next: Confirmation) => setConfirmation(next);

  const selectedIsCurrentPushed =
    selectedRevision !== null && selectedRevision === snapshot.versionControl.pushedRevision;
  const pushedVersion = snapshot.versionControl.versions.find(
    (version) => version.revision === snapshot.versionControl.pushedRevision,
  );
  const selectedDiffersFromSession =
    selectedRevision !== null && selectedRevision !== snapshot.sessionBinding.appliedRevision;
  const selectedAlreadyQueued =
    selectedRevision !== null && selectedRevision === snapshot.sessionBinding.desiredRevision;
  const configuredModelPolicy = policyFromConfiguration(configuration);
  const delegatedModelPolicy = snapshot.modelChoices.delegatedPolicies.find(
    (policy) => policy.revision === selectedRevision,
  );
  const displayedModelPolicy =
    configuration.runtime.modelPolicyMode === 'delegated_shared' && delegatedModelPolicy
      ? delegatedModelPolicy.policy
      : configuredModelPolicy;
  const displayedRuntime = {
    ...configuration.runtime,
    models: displayedModelPolicy.models,
    defaultModel: displayedModelPolicy.defaultModel,
    defaultReasoning: displayedModelPolicy.defaultReasoning,
  };
  const modelPolicyEditable = Boolean(
    editable ||
    (!editMode &&
      selectedRevision !== null &&
      configuration.runtime.modelPolicyMode === 'delegated_shared' &&
      onCommand),
  );
  const saveDisplayedRuntime = (runtime: HarnessEffectiveConfiguration['runtime']) => {
    if (editable) {
      saveConfiguration({ ...configuration, runtime });
      return;
    }
    if (selectedRevision !== null && configuration.runtime.modelPolicyMode === 'delegated_shared')
      onCommand?.({
        kind: 'save_delegated_model_policy',
        revision: selectedRevision,
        policy: policyFromConfiguration({ ...configuration, runtime }),
      });
  };

  return (
    <section className="harness-management" aria-label="Harness Management">
      <header
        className={`harness-management__toolbar${editMode ? ' is-editing' : ''}`}
        aria-label="Harness Management controls"
      >
        <button className="harness-management__back" type="button" onClick={onBack}>
          <ArrowLeft size={14} aria-hidden="true" />
          Back to conversation
        </button>
        <div className="harness-management__toolbar-context">
          {snapshot.agentIdentity && (
            <AgentIdentityBadge identity={snapshot.agentIdentity} compact />
          )}
          <strong>{configuration.identity.name}</strong>
        </div>
        <div className="harness-management__version-select">
          <SearchableSingleSelect<VersionSelection>
            label="Viewed harness version"
            options={[
              ...(snapshot.workingCopy || selected === 'draft'
                ? [
                    {
                      value: 'draft' as const,
                      label: snapshot.workingCopy
                        ? `Working draft${snapshot.workingCopy.dirty ? ' · uncommitted' : ''}`
                        : 'Starting working draft...',
                    },
                  ]
                : []),
              ...[...snapshot.versionControl.versions]
                .sort((left, right) => right.revision - left.revision)
                .map((version) => ({
                  value: `version:${version.revision}` as const,
                  label: `v${version.revision} · ${version.label}`,
                })),
            ]}
            value={selected}
            editable
            onChange={(value) => {
              if (!value) return;
              setSelected(value);
              setEditMode(value === 'draft' && editMode);
            }}
          />
        </div>
        <div className="harness-management__toolbar-actions">
          {onCommand && !editMode && selectedDiffersFromSession && (
            <button
              className="is-primary"
              type="button"
              disabled={selectedAlreadyQueued || commandPending}
              onClick={() => {
                if (selectedRevision === null) return;
                openConfirmation({
                  title: `Change this Session to v${selectedRevision}?`,
                  body: `This queues v${selectedRevision} (${selectedVersion?.label ?? 'selected version'}) for this Session. Its applied version changes only when the recorded next-prompt update is consumed.`,
                  confirmLabel: `Queue v${selectedRevision}`,
                  command: {
                    kind: 'queue_version',
                    revision: selectedRevision,
                    scope: 'current_session',
                  },
                });
              }}
            >
              {selectedAlreadyQueued
                ? `v${selectedRevision} queued`
                : `Use v${selectedRevision} for this Session`}
            </button>
          )}
          {onCommand && !editMode && (
            <button type="button" onClick={() => beginEdit()}>
              <Pencil size={15} aria-hidden="true" />
              {snapshot.workingCopy ? 'Edit draft' : 'Edit harness'}
            </button>
          )}
          {onCommand && editMode && (
            <>
              <button type="button" onClick={() => setEditMode(false)}>
                Finish editing
              </button>
              <button
                type="button"
                disabled={!snapshot.workingCopy?.dirty || commandPending}
                onClick={() => {
                  if (!snapshot.workingCopy) return;
                  openConfirmation({
                    title: 'Commit this harness version?',
                    body: 'Commit records the working draft as a new local version. It does not push the version or update any Sessions.',
                    confirmLabel: 'Commit version',
                    command: {
                      kind: 'commit',
                      expectedDraftRevision: snapshot.workingCopy.draftRevision,
                    },
                  });
                }}
              >
                <GitCommitHorizontal size={15} aria-hidden="true" />
                Commit
              </button>
              <button
                className="is-primary"
                type="button"
                disabled={
                  selectedRevision === null ||
                  selectedIsCurrentPushed ||
                  commandPending ||
                  !selectedVersion
                }
                onClick={() => {
                  if (selectedRevision === null) return;
                  openConfirmation({
                    title: `Push harness v${selectedRevision}?`,
                    body: `Push makes v${selectedRevision} the local active version and queues it for every relevant Session at the next prompt. It does not contact a remote or interrupt a running invocation.`,
                    confirmLabel: `Push v${selectedRevision}`,
                    command: { kind: 'push', revision: selectedRevision },
                  });
                }}
              >
                <Upload size={15} aria-hidden="true" />
                Push
              </button>
            </>
          )}
        </div>
      </header>

      <div className="harness-management__scroll">
        <h1 className="visually-hidden">{configuration.identity.name} Harness Management</h1>
        {commandError && (
          <p className="harness-management__command-error" role="alert">
            {commandError}
          </p>
        )}
        <div className="harness-management__version-cues" aria-live="polite">
          {snapshot.workingCopy?.dirty && (
            <StateBadge tone="caution">
              {selected === 'draft'
                ? 'Working draft · uncommitted'
                : 'Working draft has uncommitted changes'}
            </StateBadge>
          )}
          {pushedVersion && pushedVersion.revision !== selectedRevision && (
            <button type="button" onClick={() => setSelected(`version:${pushedVersion.revision}`)}>
              Newest pushed: v{pushedVersion.revision} · {pushedVersion.label}
            </button>
          )}
        </div>

        <div className="harness-management__grid">
          {snapshot.agentIdentity ? (
            <ManagementCard
              title="Session attachment"
              description={snapshot.sessionBinding.reason}
              wide
            >
              <button
                className="harness-management__identity-policy-button is-agent"
                type="button"
                aria-label={`Edit Agent identity for ${snapshot.agentIdentity.name}`}
                onClick={() => setIdentityDialogOpen(true)}
              >
                <span>Current Agent</span>
                <AgentIdentityBadge identity={snapshot.agentIdentity} />
                <small>Change this Session only.</small>
              </button>
            </ManagementCard>
          ) : null}
          <HarnessDefinitionEditor
            configuration={configuration}
            catalogs={snapshot.catalogs}
            editable={editable}
            modelPolicy={displayedRuntime}
            modelPolicyEditable={modelPolicyEditable}
            modelPolicyBoundaryKey={`${selected}:${editMode ? 'editing' : 'viewing'}:${configuration.runtime.modelPolicyMode}`}
            modelPolicyNote={
              <>
                <p className="harness-management__catalog-boundary">
                  {configuration.runtime.modelPolicyMode === 'revision_owned'
                    ? 'Fixed by this revision; edit the Harness to change it.'
                    : `Shared by recorded Sessions using v${selectedRevision ?? snapshot.workingCopy?.baseRevision ?? ''}.`}
                </p>
                {delegatedModelPolicy?.dirty ? (
                  <StateBadge tone="caution">Recorded shared adjustment</StateBadge>
                ) : null}
                <p className="harness-management__catalog-boundary">
                  <strong>Recorded model catalog.</strong> {snapshot.catalogs.models.reason}
                </p>
              </>
            }
            onChange={saveConfiguration}
            onModelPolicyChange={saveDisplayedRuntime}
          />

          <VersionHistory
            snapshot={snapshot}
            selected={selected}
            commandPending={commandPending}
            canCommand={Boolean(onCommand)}
            onSelect={setSelected}
            onConfirm={openConfirmation}
          />
        </div>
      </div>

      {identityDialogOpen && snapshot.agentIdentity && (
        <SessionIdentityDialog
          snapshot={snapshot}
          onApply={(name, visualIdentity) => {
            onCommand?.({
              kind: 'update_session_identity',
              name,
              visualIdentity,
            });
            setIdentityDialogOpen(false);
          }}
          onClose={() => setIdentityDialogOpen(false)}
        />
      )}
      {confirmation && (
        <ConfirmationDialog
          confirmation={confirmation}
          pending={commandPending}
          onCancel={() => setConfirmation(null)}
          onConfirm={() => {
            onCommand?.(confirmation.command);
            setConfirmation(null);
          }}
        />
      )}
    </section>
  );
}

function ManagementShell({ onBack, children }: { onBack(): void; readonly children: ReactNode }) {
  return (
    <section className="harness-management" aria-label="Harness Management">
      <header className="harness-management__toolbar">
        <button className="harness-management__back" type="button" onClick={onBack}>
          <ArrowLeft size={14} aria-hidden="true" />
          Back to conversation
        </button>
      </header>
      <div className="harness-management__scroll">{children}</div>
    </section>
  );
}

function ManagementCard({
  title,
  description,
  help,
  wide = false,
  action,
  children,
}: {
  readonly title: string;
  readonly description?: string;
  readonly help?: string;
  readonly wide?: boolean;
  readonly action?: ReactNode;
  readonly children: ReactNode;
}) {
  const helpId = useId();
  return (
    <section className={`harness-management__card${wide ? ' is-wide' : ''}`}>
      <header>
        <div>
          <div className="harness-management__card-title">
            <h2>{title}</h2>
            {help && (
              <span className="harness-management__help">
                <button type="button" aria-label={`About ${title}`} aria-describedby={helpId}>
                  <HelpCircle size={14} aria-hidden="true" />
                </button>
                <span id={helpId} role="tooltip">
                  {help}
                </span>
              </span>
            )}
          </div>
          {description && <p>{description}</p>}
        </div>
        {action}
      </header>
      {children}
    </section>
  );
}

function ManagementField({
  label,
  children,
}: {
  readonly label: string;
  readonly children: ReactNode;
}) {
  return (
    <label className="harness-management__field">
      <span>{label}</span>
      {children}
    </label>
  );
}

function VersionHistory({
  snapshot,
  selected,
  commandPending,
  canCommand,
  onSelect,
  onConfirm,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly selected: VersionSelection;
  readonly commandPending: boolean;
  readonly canCommand: boolean;
  onSelect(selection: VersionSelection): void;
  onConfirm(confirmation: Confirmation): void;
}) {
  return (
    <ManagementCard
      title="Version history"
      description="Committed local versions and the Sessions currently using each one."
      wide
    >
      <div className="harness-management__history-wrap">
        <table className="harness-management__history">
          <thead>
            <tr>
              <th scope="col">Revision / status</th>
              <th scope="col">Active Sessions</th>
              <th scope="col">Selected Session</th>
              <th scope="col">
                <span className="visually-hidden">Session actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {snapshot.workingCopy && (
              <tr
                className={selected === 'draft' ? 'is-selected' : ''}
                tabIndex={0}
                aria-label="View working draft"
                onClick={() => onSelect('draft')}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') onSelect('draft');
                }}
              >
                <td>
                  <button type="button" onClick={() => onSelect('draft')}>
                    Working draft
                  </button>
                  <StateBadge tone={snapshot.workingCopy.dirty ? 'caution' : 'neutral'}>
                    {snapshot.workingCopy.dirty ? 'Uncommitted' : 'No changes'}
                  </StateBadge>
                </td>
                <td>—</td>
                <td>—</td>
                <td>Commit before changing Sessions</td>
              </tr>
            )}
            {[...snapshot.versionControl.versions]
              .sort((left, right) => right.revision - left.revision)
              .map((version) => {
                const isSessionVersion =
                  version.revision === snapshot.sessionBinding.appliedRevision;
                const isDesired = version.revision === snapshot.sessionBinding.desiredRevision;
                const allRelevantResolved =
                  snapshot.sessionBinding.relevantSessionCount !== null &&
                  version.activeSessionCount + version.queuedSessionCount >=
                    snapshot.sessionBinding.relevantSessionCount;
                return (
                  <tr
                    className={selected === `version:${version.revision}` ? 'is-selected' : ''}
                    key={version.revision}
                    tabIndex={0}
                    aria-label={`View v${version.revision} ${version.label}`}
                    onClick={() => onSelect(`version:${version.revision}`)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ')
                        onSelect(`version:${version.revision}`);
                    }}
                  >
                    <td>
                      <button type="button" onClick={() => onSelect(`version:${version.revision}`)}>
                        v{version.revision} · {version.label}
                      </button>
                      <StateBadge
                        tone={
                          version.revision === snapshot.versionControl.pushedRevision
                            ? 'positive'
                            : 'neutral'
                        }
                      >
                        {version.revision === snapshot.versionControl.pushedRevision
                          ? 'Current pushed'
                          : version.status === 'pushed'
                            ? 'Previously pushed'
                            : version.status === 'committed'
                              ? 'Committed'
                              : 'Inspected'}
                      </StateBadge>
                    </td>
                    <td>
                      {version.activeSessionCount} active
                      {version.queuedSessionCount > 0
                        ? ` · ${version.queuedSessionCount} queued`
                        : ''}
                    </td>
                    <td>
                      {isSessionVersion ? (
                        <span className="harness-management__session-indicator">
                          <Check size={14} aria-hidden="true" />
                          Using v{version.revision}
                        </span>
                      ) : isDesired ? (
                        `Queued for next prompt`
                      ) : (
                        '—'
                      )}
                    </td>
                    <td>
                      {canCommand && !allRelevantResolved && (
                        <button
                          type="button"
                          disabled={commandPending}
                          onClick={(event) => {
                            event.stopPropagation();
                            onConfirm({
                              title: `Change all relevant Sessions to v${version.revision}?`,
                              body: `This queues v${version.revision} (${version.label}) for every relevant Session not already using it. Applied versions change only when each recorded next-prompt update is consumed.`,
                              confirmLabel: `Queue v${version.revision} for all`,
                              command: {
                                kind: 'queue_version',
                                revision: version.revision,
                                scope: 'all_relevant_sessions',
                              },
                            });
                          }}
                        >
                          <Users size={14} aria-hidden="true" />
                          Change all to v{version.revision}
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
          </tbody>
        </table>
      </div>
      <p className="harness-management__card-footer">{snapshot.versionControl.reason}</p>
    </ManagementCard>
  );
}

function SessionIdentityDialog({
  snapshot,
  onApply,
  onClose,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  onApply(name: string, visualIdentity: HarnessVisualIdentity): void;
  onClose(): void;
}) {
  const identity = snapshot.agentIdentity;
  const [search, setSearch] = useState('');
  const [name, setName] = useState(identity?.name ?? '');
  const [visualIdentity, setVisualIdentity] = useState<HarnessVisualIdentity | null>(
    snapshot.catalogs.agentVisualIdentities.items.find(
      (entry) => entry.identity.token === identity?.visualIdentityToken,
    )?.identity ??
      snapshot.catalogs.agentVisualIdentities.items[0]?.identity ??
      null,
  );
  const names = snapshot.catalogs.agentNames.items.filter((candidate) =>
    fuzzyMatch(candidate, '', search),
  );
  return (
    <div className="harness-management__modal-backdrop">
      <section
        className="harness-management__modal is-identity"
        role="dialog"
        aria-modal="true"
        aria-labelledby="harness-session-identity-title"
      >
        <header>
          <div>
            <h2 id="harness-session-identity-title">Current Agent identity</h2>
            <p>
              This changes only this Agent Session after confirmation. Harness name pools are
              unchanged.
            </p>
          </div>
          <button type="button" aria-label="Close current Agent identity" onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>
        <div className="harness-management__identity-dialog-fields">
          <ManagementField label="Agent name">
            <input
              aria-label="Agent name"
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </ManagementField>
          <ManagementField label="Available names">
            <div className="harness-management__catalog-search">
              <Search size={15} aria-hidden="true" />
              <input
                aria-label="Search available Agent names"
                placeholder="Search available names"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>
          </ManagementField>
        </div>
        <div className="harness-management__modal-scroll">
          <div
            className="harness-management__identity-name-results"
            aria-label="Available Agent names"
          >
            {names.map((candidate) => (
              <button
                type="button"
                aria-pressed={candidate === name}
                key={candidate}
                onClick={() => setName(candidate)}
              >
                {candidate}
              </button>
            ))}
            {names.length === 0 && <p>No product names match this search.</p>}
          </div>
          <SearchableSingleSelect
            label="Visual identity"
            options={snapshot.catalogs.agentVisualIdentities.items.map((entry) => ({
              value: `${entry.identity.token}\u0000${entry.identity.accent}`,
              label: entry.label,
            }))}
            value={visualIdentity ? `${visualIdentity.token}\u0000${visualIdentity.accent}` : null}
            editable
            unavailableReason={snapshot.catalogs.agentVisualIdentities.reason}
            onChange={(value) => {
              const [token, accent] = value?.split('\u0000') ?? [];
              setVisualIdentity(token && accent ? { token, accent } : null);
            }}
          />
        </div>
        <div className="harness-management__modal-actions">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button
            className="is-primary"
            type="button"
            disabled={!name.trim() || !visualIdentity}
            onClick={() => visualIdentity && onApply(name.trim(), visualIdentity)}
          >
            Apply to this Session
          </button>
        </div>
      </section>
    </div>
  );
}

function ConfirmationDialog({
  confirmation,
  pending,
  onCancel,
  onConfirm,
}: {
  readonly confirmation: Confirmation;
  readonly pending: boolean;
  onCancel(): void;
  onConfirm(): void;
}) {
  return (
    <div className="harness-management__modal-backdrop">
      <section
        className="harness-management__modal is-confirmation"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="harness-confirmation-title"
        aria-describedby="harness-confirmation-body"
      >
        <h2 id="harness-confirmation-title">{confirmation.title}</h2>
        <p id="harness-confirmation-body">{confirmation.body}</p>
        <div className="harness-management__modal-actions">
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
          <button className="is-primary" type="button" disabled={pending} onClick={onConfirm}>
            {confirmation.confirmLabel}
          </button>
        </div>
      </section>
    </div>
  );
}

function StateBadge({
  tone,
  children,
}: {
  readonly tone: 'positive' | 'caution' | 'neutral';
  readonly children: ReactNode;
}) {
  return <span className={`harness-management__badge is-${tone}`}>{children}</span>;
}

function policyFromConfiguration(configuration: HarnessEffectiveConfiguration): HarnessModelPolicy {
  return {
    models: configuration.runtime.models,
    defaultModel: configuration.runtime.defaultModel,
    defaultReasoning: configuration.runtime.defaultReasoning,
  };
}

function fuzzyMatch(name: string, description: string, query: string): boolean {
  const normalizedName = name.toLocaleLowerCase();
  const normalizedDescription = description.toLocaleLowerCase();
  const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  return words.every((word) => {
    if (normalizedName.includes(word) || normalizedDescription.includes(word)) return true;
    let index = 0;
    for (const character of normalizedName) {
      if (character === word[index]) index += 1;
      if (index === word.length) return true;
    }
    return false;
  });
}
