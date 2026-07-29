import {
  ArrowLeft,
  Check,
  ChevronDown,
  ChevronRight,
  GitCommitHorizontal,
  Pencil,
  Search,
  Upload,
  Users,
  X,
} from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementRead,
  ConversationHarnessManagementSnapshot,
  HarnessEffectiveConfiguration,
  HarnessReasoningLevel,
  HarnessSkillPolicy,
  HarnessToolPolicy,
} from '../../application/conversationHarnesses';
import { AgentIdentityBadge } from '../../components/AgentIdentityBadge';
import { MarkdownContent } from '../../components/MarkdownContent';
import { MarkdownEditor } from '../../components/MarkdownEditor';

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
type CatalogDialog = 'names' | 'skills' | 'tools' | null;

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
  const [catalogDialog, setCatalogDialog] = useState<CatalogDialog>(null);
  const [selectedSkillName, setSelectedSkillName] = useState<string | null>(null);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [skillSections, setSkillSections] = useState({
    always_applicable: true,
    initial_ingestion: true,
    available: false,
  });
  const [toolSections, setToolSections] = useState({
    every_invocation: true,
    initial_invocation: true,
    available: false,
  });
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
  const beginEdit = (dialog: CatalogDialog = null) => {
    setEditMode(true);
    setSelected('draft');
    setCatalogDialog(dialog);
    if (!snapshot.workingCopy && selectedRevision !== null)
      onCommand?.({ kind: 'start_edit', baseRevision: selectedRevision });
  };
  const openConfirmation = (next: Confirmation) => setConfirmation(next);

  const selectedIsCurrentPushed =
    selectedRevision !== null && selectedRevision === snapshot.versionControl.pushedRevision;
  const selectedDiffersFromSession =
    selected === 'draft' || selectedRevision !== snapshot.sessionBinding.appliedRevision;

  return (
    <section className="harness-management" aria-label="Harness Management">
      <header className="harness-management__toolbar" aria-label="Harness Management controls">
        <button className="harness-management__back" type="button" onClick={onBack}>
          <ArrowLeft size={16} aria-hidden="true" />
          Back to conversation
        </button>
        <div className="harness-management__toolbar-context">
          {snapshot.agentIdentity && (
            <AgentIdentityBadge identity={snapshot.agentIdentity} compact />
          )}
          <span>Harness</span>
          <strong>{configuration.identity.name}</strong>
          {snapshot.versionControl.support === 'recorded_preview' && (
            <StateBadge tone="neutral">Recorded prototype</StateBadge>
          )}
        </div>
        <label className="harness-management__version-select">
          <span>Version</span>
          <select
            aria-label="Viewed harness version"
            value={selected}
            onChange={(event) => {
              setSelected(event.target.value as VersionSelection);
              setEditMode(event.target.value === 'draft' && editMode);
            }}
          >
            {snapshot.workingCopy && (
              <option value="draft">
                Working draft{snapshot.workingCopy.dirty ? ' · uncommitted' : ''}
              </option>
            )}
            {!snapshot.workingCopy && selected === 'draft' && (
              <option value="draft">Starting working draft...</option>
            )}
            {[...snapshot.versionControl.versions]
              .sort((left, right) => right.revision - left.revision)
              .map((version) => (
                <option value={`version:${version.revision}`} key={version.revision}>
                  v{version.revision}
                  {version.revision === snapshot.versionControl.pushedRevision
                    ? ' · current pushed'
                    : version.status === 'pushed'
                      ? ' · previously pushed'
                      : version.status === 'committed'
                        ? ' · committed'
                        : ' · inspected'}
                  {version.revision === snapshot.sessionBinding.appliedRevision
                    ? ' · this Session'
                    : ''}
                </option>
              ))}
          </select>
        </label>
        <div className="harness-management__toolbar-actions">
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
          {snapshot.sessionBinding.appliedRevision !== null && (
            <StateBadge tone="caution">
              Session version · v{snapshot.sessionBinding.appliedRevision}
            </StateBadge>
          )}
          {snapshot.versionControl.pushedRevision !== null && (
            <StateBadge tone="positive">
              Current pushed · v{snapshot.versionControl.pushedRevision}
            </StateBadge>
          )}
          {selectedVersion && (
            <StateBadge tone="neutral">
              Viewed v{selectedVersion.revision} ·{' '}
              {selectedVersion.revision === snapshot.versionControl.pushedRevision
                ? 'current pushed'
                : selectedVersion.status === 'pushed'
                  ? 'previously pushed'
                  : selectedVersion.status === 'committed'
                    ? 'committed, not pushed'
                    : 'push history not connected'}
            </StateBadge>
          )}
          {selectedDiffersFromSession && (
            <StateBadge tone="neutral">
              Viewing differs from Session v{snapshot.sessionBinding.appliedRevision ?? '—'}
            </StateBadge>
          )}
          {selectedRevision !== null &&
            selectedRevision !== snapshot.sessionBinding.appliedRevision &&
            onCommand && (
              <button
                type="button"
                onClick={() =>
                  openConfirmation({
                    title: `Change this Session to v${selectedRevision}?`,
                    body: `This queues v${selectedRevision} for this Session. The Session keeps its current version until its next prompt.`,
                    confirmLabel: `Queue v${selectedRevision}`,
                    command: {
                      kind: 'queue_version',
                      revision: selectedRevision,
                      scope: 'current_session',
                    },
                  })
                }
              >
                Change this Session to v{selectedRevision}
              </button>
            )}
        </div>

        <div className="harness-management__grid">
          <ManagementCard
            title="Harness details"
            description="Administrative identity and the stable Agent identity used by this Session."
            wide
          >
            <div className="harness-management__field-row">
              <ManagementField label="Harness name">
                <input
                  value={configuration.identity.name}
                  disabled={!editable}
                  onChange={(event) =>
                    saveConfiguration({
                      ...configuration,
                      identity: { ...configuration.identity, name: event.target.value },
                    })
                  }
                />
              </ManagementField>
              <ManagementField label="Machine key">
                <input
                  value={configuration.identity.machineKey}
                  disabled={!editable}
                  onChange={(event) =>
                    saveConfiguration({
                      ...configuration,
                      identity: { ...configuration.identity, machineKey: event.target.value },
                    })
                  }
                />
              </ManagementField>
            </div>
            <div className="harness-management__identity-policy">
              {snapshot.agentIdentity ? (
                <AgentIdentityBadge identity={snapshot.agentIdentity} />
              ) : (
                <p>This Session does not yet have a stored Agent identity.</p>
              )}
              <button
                className="harness-management__identity-policy-button"
                type="button"
                onClick={() => setCatalogDialog('names')}
              >
                <span>Permitted name pool</span>
                <strong>
                  {configuration.identity.permittedAgentNames
                    ? `Harness subset · ${configuration.identity.permittedAgentNames.length} names`
                    : 'Product default · 100 names'}
                </strong>
                <small>Existing Sessions keep their assigned names.</small>
              </button>
              <div>
                <span>Visual identity</span>
                <strong>
                  {configuration.identity.visualIdentity
                    ? humanize(configuration.identity.visualIdentity.token)
                    : 'Not configured'}
                </strong>
                <small>Stored with the Session’s applied harness revision.</small>
              </div>
            </div>
          </ManagementCard>

          <ManagementCard
            title="Prompt prefix"
            description="Prepended to the first prompt in a new Session and intended to be included again after context compression. Harness-aware compression is deferred."
            wide
          >
            {editable ? (
              <MarkdownEditor
                label="Prompt prefix"
                value={configuration.promptPrefix.content}
                editable
                onChange={(content) =>
                  saveConfiguration({
                    ...configuration,
                    promptPrefix: { ...configuration.promptPrefix, content },
                  })
                }
              />
            ) : (
              <MarkdownContent className="harness-management__markdown-view">
                {configuration.promptPrefix.content}
              </MarkdownContent>
            )}
          </ManagementCard>

          <PolicyCard
            title="Skills"
            editLabel="Edit skills"
            editable={Boolean(onCommand)}
            onEdit={() => beginEdit('skills')}
            onItemSelect={(skill) => setSelectedSkillName(skill.name)}
            groups={[
              {
                key: 'always_applicable',
                title: 'Always applicable',
                items: configuration.skills.items.filter(
                  (skill) => skill.policy === 'always_applicable',
                ),
                open: skillSections.always_applicable,
                onToggle: () =>
                  setSkillSections((current) => ({
                    ...current,
                    always_applicable: !current.always_applicable,
                  })),
              },
              {
                key: 'initial_ingestion',
                title: 'Initial ingestion only',
                items: configuration.skills.items.filter(
                  (skill) => skill.policy === 'initial_ingestion',
                ),
                open: skillSections.initial_ingestion,
                onToggle: () =>
                  setSkillSections((current) => ({
                    ...current,
                    initial_ingestion: !current.initial_ingestion,
                  })),
              },
              {
                key: 'available',
                title: 'Available',
                items: configuration.skills.items.filter((skill) => skill.policy === 'available'),
                open: skillSections.available,
                onToggle: () =>
                  setSkillSections((current) => ({
                    ...current,
                    available: !current.available,
                  })),
                suffix: humanize(configuration.skills.availableDiscoveryPolicy),
              },
            ]}
          />

          <PolicyCard
            title="Tools"
            editLabel="Edit tools"
            editable={Boolean(onCommand)}
            onEdit={() => beginEdit('tools')}
            groups={[
              {
                key: 'every_invocation',
                title: 'Always applicable',
                items: configuration.tools.items.filter(
                  (tool) => tool.policy === 'every_invocation',
                ),
                open: toolSections.every_invocation,
                onToggle: () =>
                  setToolSections((current) => ({
                    ...current,
                    every_invocation: !current.every_invocation,
                  })),
              },
              {
                key: 'initial_invocation',
                title: 'Initial ingestion only',
                items: configuration.tools.items.filter(
                  (tool) => tool.policy === 'initial_invocation',
                ),
                open: toolSections.initial_invocation,
                onToggle: () =>
                  setToolSections((current) => ({
                    ...current,
                    initial_invocation: !current.initial_invocation,
                  })),
              },
              {
                key: 'available',
                title: 'Available',
                items: configuration.tools.items.filter((tool) => tool.policy === 'available'),
                open: toolSections.available,
                onToggle: () =>
                  setToolSections((current) => ({
                    ...current,
                    available: !current.available,
                  })),
                suffix: humanize(configuration.tools.availableDiscoveryPolicy),
              },
            ]}
            footer={configuration.tools.schemaBoundary}
          />

          <ManagementCard
            title="Models and reasoning"
            description="Allow caller choices and constrain the reasoning range supported by each model."
            wide
          >
            <ModelPolicy
              configuration={configuration}
              snapshot={snapshot}
              editable={editable}
              onChange={saveConfiguration}
            />
          </ManagementCard>

          <ManagementCard
            title="Sandbox and authority"
            description={configuration.runtime.authoritySummary}
            wide
          >
            <div className="harness-management__field-row">
              <ManagementField label="Sandbox">
                <select
                  value={configuration.runtime.sandbox}
                  disabled={!editable}
                  onChange={(event) =>
                    saveConfiguration({
                      ...configuration,
                      runtime: {
                        ...configuration.runtime,
                        sandbox: event.target
                          .value as HarnessEffectiveConfiguration['runtime']['sandbox'],
                      },
                    })
                  }
                >
                  {configuration.runtime.sandboxOptions.map((option) => (
                    <option value={option} key={option}>
                      {humanize(option)}
                    </option>
                  ))}
                </select>
              </ManagementField>
              <ManagementField label="Approval policy">
                <select value={configuration.runtime.approvalPolicy} disabled>
                  <option value="never">Never</option>
                </select>
              </ManagementField>
            </div>
          </ManagementCard>

          <ManagementCard
            title="Application hooks"
            description="Hook references for this harness. Connection requires an Application hook registry."
            wide
          >
            <ul className="harness-management__hook-list">
              {configuration.hooks.map((hook) => (
                <li key={hook.name}>
                  <div>
                    <strong>{hook.name}</strong>
                    <p>{hook.detail}</p>
                  </div>
                  <StateBadge tone={hook.status === 'exposed' ? 'positive' : 'neutral'}>
                    {hook.status === 'exposed'
                      ? 'Exposed'
                      : hook.status === 'proposed'
                        ? 'Proposed'
                        : 'Not connected'}
                  </StateBadge>
                </li>
              ))}
            </ul>
          </ManagementCard>

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

      {catalogDialog === 'names' && (
        <NamePoolDialog
          snapshot={snapshot}
          configuration={configuration}
          editable={editable}
          canEdit={Boolean(onCommand)}
          onStartEdit={() => beginEdit('names')}
          onChange={saveConfiguration}
          onClose={() => setCatalogDialog(null)}
        />
      )}
      {catalogDialog === 'skills' && (
        <SkillCatalogDialog
          snapshot={snapshot}
          configuration={configuration}
          editable={editable}
          onChange={saveConfiguration}
          onClose={() => setCatalogDialog(null)}
        />
      )}
      {selectedSkillName && (
        <SkillDetailsDialog
          skillName={selectedSkillName}
          snapshot={snapshot}
          configuration={configuration}
          editable={editable}
          canEdit={Boolean(onCommand)}
          onStartEdit={() => beginEdit()}
          onChange={saveConfiguration}
          onClose={() => setSelectedSkillName(null)}
        />
      )}
      {catalogDialog === 'tools' && (
        <ToolCatalogDialog
          snapshot={snapshot}
          configuration={configuration}
          editable={editable}
          onChange={saveConfiguration}
          onClose={() => setCatalogDialog(null)}
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
          <ArrowLeft size={16} aria-hidden="true" />
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
  wide = false,
  action,
  children,
}: {
  readonly title: string;
  readonly description: string;
  readonly wide?: boolean;
  readonly action?: ReactNode;
  readonly children: ReactNode;
}) {
  return (
    <section className={`harness-management__card${wide ? ' is-wide' : ''}`}>
      <header>
        <div>
          <h2>{title}</h2>
          <p>{description}</p>
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

interface PolicyGroupItem {
  readonly name: string;
  readonly purpose?: string;
}

function PolicyCard({
  title,
  editLabel,
  editable,
  onEdit,
  onItemSelect,
  groups,
  footer,
}: {
  readonly title: string;
  readonly editLabel: string;
  readonly editable: boolean;
  onEdit(): void;
  onItemSelect?(item: PolicyGroupItem): void;
  readonly groups: readonly {
    readonly key: string;
    readonly title: string;
    readonly items: readonly PolicyGroupItem[];
    readonly open: boolean;
    onToggle(): void;
    readonly suffix?: string;
  }[];
  readonly footer?: string;
}) {
  return (
    <ManagementCard
      title={title}
      description=""
      action={
        editable ? (
          <button className="harness-management__card-action" type="button" onClick={onEdit}>
            <Pencil size={14} aria-hidden="true" />
            {editLabel}
          </button>
        ) : undefined
      }
    >
      <div className="harness-management__policy-groups">
        {groups.map((group, priority) => (
          <section
            className={`harness-management__policy-group priority-${priority + 1}`}
            key={group.key}
          >
            <button
              type="button"
              aria-expanded={group.open}
              aria-controls={`${title}-${group.key}-items`}
              onClick={group.onToggle}
            >
              {group.open ? (
                <ChevronDown size={15} aria-hidden="true" />
              ) : (
                <ChevronRight size={15} aria-hidden="true" />
              )}
              <strong>{group.title}</strong>
              <span>{group.items.length}</span>
              {group.suffix && <small>{group.suffix}</small>}
            </button>
            {group.open && (
              <ul id={`${title}-${group.key}-items`}>
                {group.items.length ? (
                  group.items.map((item) => (
                    <li key={item.name}>
                      {onItemSelect ? (
                        <button
                          type="button"
                          aria-label={`View ${item.name} skill details`}
                          onClick={() => onItemSelect(item)}
                        >
                          <span>
                            <strong>{item.name}</strong>
                            {item.purpose && <small>{item.purpose}</small>}
                          </span>
                          <ChevronRight size={15} aria-hidden="true" />
                        </button>
                      ) : (
                        <>
                          <strong>{item.name}</strong>
                          {item.purpose && <small>{item.purpose}</small>}
                        </>
                      )}
                    </li>
                  ))
                ) : (
                  <li className="is-empty">None selected</li>
                )}
              </ul>
            )}
          </section>
        ))}
      </div>
      {footer && <p className="harness-management__card-footer">{footer}</p>}
    </ManagementCard>
  );
}

function ModelPolicy({
  configuration,
  snapshot,
  editable,
  onChange,
}: {
  readonly configuration: HarnessEffectiveConfiguration;
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly editable: boolean;
  onChange(configuration: HarnessEffectiveConfiguration): void;
}) {
  const models = snapshot.catalogs.models.items;
  const updateModel = (
    modelId: string,
    next: Partial<HarnessEffectiveConfiguration['runtime']['models'][number]>,
  ) => {
    const updatedModels = configuration.runtime.models.map((model) =>
      model.modelId === modelId ? { ...model, ...next } : model,
    );
    let defaultModel = configuration.runtime.defaultModel;
    let defaultReasoning = configuration.runtime.defaultReasoning;
    if (next.allowed === false && defaultModel === modelId) {
      defaultModel = null;
      defaultReasoning = null;
    }
    onChange({
      ...configuration,
      runtime: {
        ...configuration.runtime,
        models: updatedModels,
        defaultModel,
        defaultReasoning,
      },
    });
  };
  const defaultModelConfiguration = configuration.runtime.models.find(
    (model) => model.modelId === configuration.runtime.defaultModel,
  );
  const defaultModelCatalog = models.find(
    (model) => model.id === configuration.runtime.defaultModel,
  );
  const defaultReasoningOptions =
    defaultModelCatalog && defaultModelConfiguration
      ? defaultModelCatalog.reasoningLevels.slice(
          defaultModelCatalog.reasoningLevels.indexOf(defaultModelConfiguration.minReasoning),
          defaultModelCatalog.reasoningLevels.indexOf(defaultModelConfiguration.maxReasoning) + 1,
        )
      : [];

  return (
    <>
      <div className="harness-management__model-list">
        {models.map((catalogModel) => {
          const model = configuration.runtime.models.find(
            (candidate) => candidate.modelId === catalogModel.id,
          );
          if (!model) return null;
          const minIndex = catalogModel.reasoningLevels.indexOf(model.minReasoning);
          const maxIndex = catalogModel.reasoningLevels.indexOf(model.maxReasoning);
          return (
            <div className="harness-management__model" key={catalogModel.id}>
              <label className="harness-management__model-allow">
                <input
                  type="checkbox"
                  checked={model.allowed}
                  disabled={!editable}
                  onChange={(event) =>
                    updateModel(model.modelId, { allowed: event.target.checked })
                  }
                />
                <span>
                  <strong>{catalogModel.label}</strong>
                  <small>{catalogModel.id}</small>
                </span>
              </label>
              <div className="harness-management__reasoning-range">
                <div>
                  <span>Allowed reasoning</span>
                  <strong>
                    {model.minReasoning} – {model.maxReasoning}
                  </strong>
                </div>
                <div className="harness-management__dual-range">
                  <input
                    type="range"
                    aria-label={`${catalogModel.label} minimum reasoning`}
                    min={0}
                    max={catalogModel.reasoningLevels.length - 1}
                    step={1}
                    value={minIndex}
                    disabled={!editable || !model.allowed}
                    onChange={(event) => {
                      const nextIndex = Math.min(Number(event.target.value), maxIndex);
                      updateModel(model.modelId, {
                        minReasoning: catalogModel.reasoningLevels[nextIndex],
                      });
                    }}
                  />
                  <input
                    type="range"
                    aria-label={`${catalogModel.label} maximum reasoning`}
                    min={0}
                    max={catalogModel.reasoningLevels.length - 1}
                    step={1}
                    value={maxIndex}
                    disabled={!editable || !model.allowed}
                    onChange={(event) => {
                      const nextIndex = Math.max(Number(event.target.value), minIndex);
                      updateModel(model.modelId, {
                        maxReasoning: catalogModel.reasoningLevels[nextIndex],
                      });
                    }}
                  />
                </div>
                <div className="harness-management__range-labels" aria-hidden="true">
                  {catalogModel.reasoningLevels.map((level) => (
                    <span key={level}>{level}</span>
                  ))}
                </div>
              </div>
            </div>
          );
        })}
      </div>
      <div className="harness-management__defaults">
        <ManagementField label="Default model">
          <select
            value={configuration.runtime.defaultModel ?? ''}
            disabled={!editable}
            onChange={(event) =>
              onChange({
                ...configuration,
                runtime: {
                  ...configuration.runtime,
                  defaultModel: event.target.value || null,
                  defaultReasoning: null,
                },
              })
            }
          >
            <option value="">Caller choice</option>
            {models
              .filter((catalogModel) =>
                configuration.runtime.models.some(
                  (model) => model.modelId === catalogModel.id && model.allowed,
                ),
              )
              .map((model) => (
                <option value={model.id} key={model.id}>
                  {model.label}
                </option>
              ))}
          </select>
        </ManagementField>
        <ManagementField label="Default reasoning">
          <select
            value={configuration.runtime.defaultReasoning ?? ''}
            disabled={!editable || !configuration.runtime.defaultModel}
            onChange={(event) =>
              onChange({
                ...configuration,
                runtime: {
                  ...configuration.runtime,
                  defaultReasoning: (event.target.value as HarnessReasoningLevel | '') || null,
                },
              })
            }
          >
            <option value="">Caller choice</option>
            {defaultReasoningOptions.map((level) => (
              <option value={level} key={level}>
                {level}
              </option>
            ))}
          </select>
        </ManagementField>
      </div>
      <p className="harness-management__catalog-boundary">
        <strong>Recorded model catalog.</strong> {snapshot.catalogs.models.reason}
      </p>
    </>
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
              <tr className={selected === 'draft' ? 'is-selected' : ''}>
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
                return (
                  <tr
                    className={selected === `version:${version.revision}` ? 'is-selected' : ''}
                    key={version.revision}
                  >
                    <td>
                      <button type="button" onClick={() => onSelect(`version:${version.revision}`)}>
                        v{version.revision}
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
                    <td>{version.activeSessionCount}</td>
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
                      {canCommand && (
                        <button
                          type="button"
                          disabled={commandPending}
                          onClick={() =>
                            onConfirm({
                              title: `Change all relevant Sessions to v${version.revision}?`,
                              body: `This queues v${version.revision} for every relevant Session. Each Session keeps its current version until its next prompt.`,
                              confirmLabel: `Queue v${version.revision} for all`,
                              command: {
                                kind: 'queue_version',
                                revision: version.revision,
                                scope: 'all_relevant_sessions',
                              },
                            })
                          }
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

function NamePoolDialog({
  snapshot,
  configuration,
  editable,
  canEdit,
  onStartEdit,
  onChange,
  onClose,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly configuration: HarnessEffectiveConfiguration;
  readonly editable: boolean;
  readonly canEdit: boolean;
  onStartEdit(): void;
  onChange(configuration: HarnessEffectiveConfiguration): void;
  onClose(): void;
}) {
  const [query, setQuery] = useState('');
  const catalog = snapshot.catalogs.agentNames;
  const subset = configuration.identity.permittedAgentNames;
  const selectedNames = new Set(subset ?? catalog.items);
  const filteredNames = catalog.items.filter((name) => fuzzyMatch(name, '', query));
  const updateNames = (names: readonly string[] | null) =>
    onChange({
      ...configuration,
      identity: {
        ...configuration.identity,
        permittedAgentNames: names,
      },
    });

  return (
    <div className="harness-management__modal-backdrop">
      <section
        className="harness-management__modal is-details"
        role="dialog"
        aria-modal="true"
        aria-labelledby="harness-name-pool-title"
      >
        <header>
          <div>
            <h2 id="harness-name-pool-title">Permitted name pool</h2>
            <p>Names available when this Harness creates a new Agent Session.</p>
          </div>
          <button type="button" aria-label="Close permitted name pool" onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>
        <div className="harness-management__details-actions">
          {!editable && canEdit && (
            <button type="button" onClick={onStartEdit}>
              <Pencil size={14} aria-hidden="true" />
              Edit name pool
            </button>
          )}
          <small>Existing Sessions keep their assigned names.</small>
        </div>
        <div className="harness-management__modal-scroll">
          <label className="harness-management__discovery-policy">
            <span>Pool</span>
            <select
              aria-label="Name pool source"
              value={subset ? 'harness_subset' : 'product_default'}
              disabled={!editable}
              onChange={(event) =>
                updateNames(
                  event.target.value === 'product_default'
                    ? null
                    : initialNameSubset(snapshot, catalog.items),
                )
              }
            >
              <option value="harness_subset">Harness subset</option>
              <option value="product_default">Full product pool</option>
            </select>
          </label>
          {catalog.source === 'not_connected' ? (
            <p>{catalog.reason}</p>
          ) : subset ? (
            <>
              <label className="harness-management__catalog-search">
                <Search size={16} aria-hidden="true" />
                <span className="visually-hidden">Search product names</span>
                <input
                  aria-label="Search product names"
                  value={query}
                  placeholder="Search product names"
                  onChange={(event) => setQuery(event.target.value)}
                />
              </label>
              <div className="harness-management__name-grid">
                {filteredNames.map((name) => {
                  const checked = selectedNames.has(name);
                  return (
                    <label key={name}>
                      <input
                        type="checkbox"
                        aria-label={`${name} permitted`}
                        checked={checked}
                        disabled={!editable || (checked && subset.length === 1)}
                        onChange={(event) =>
                          updateNames(
                            event.target.checked
                              ? [...subset, name]
                              : subset.filter((candidate) => candidate !== name),
                          )
                        }
                      />
                      <span>{name}</span>
                    </label>
                  );
                })}
              </div>
            </>
          ) : (
            <p className="harness-management__pool-summary">
              All {catalog.items.length} product names are permitted for new Sessions.
            </p>
          )}
        </div>
      </section>
    </div>
  );
}

function SkillDetailsDialog({
  skillName,
  snapshot,
  configuration,
  editable,
  canEdit,
  onStartEdit,
  onChange,
  onClose,
}: {
  readonly skillName: string;
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly configuration: HarnessEffectiveConfiguration;
  readonly editable: boolean;
  readonly canEdit: boolean;
  onStartEdit(): void;
  onChange(configuration: HarnessEffectiveConfiguration): void;
  onClose(): void;
}) {
  const skill = configuration.skills.items.find((item) => item.name === skillName);
  const catalogSkill = snapshot.catalogs.skills.items.find((item) => item.name === skillName);
  if (!skill) return null;

  return (
    <div className="harness-management__modal-backdrop">
      <section
        className="harness-management__modal is-details"
        role="dialog"
        aria-modal="true"
        aria-labelledby="harness-skill-details-title"
      >
        <header>
          <div>
            <h2 id="harness-skill-details-title">{skill.name}</h2>
            <p>{skill.path}</p>
          </div>
          <button type="button" aria-label={`Close ${skill.name} details`} onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>
        <div className="harness-management__details-actions">
          {!editable && canEdit && (
            <button type="button" onClick={onStartEdit}>
              <Pencil size={14} aria-hidden="true" />
              Edit skill policy
            </button>
          )}
          <label>
            <span>Applicability</span>
            <select
              aria-label={`${skill.name} details applicability`}
              value={skill.policy}
              disabled={!editable}
              onChange={(event) =>
                onChange({
                  ...configuration,
                  skills: {
                    ...configuration.skills,
                    items: configuration.skills.items.map((item) =>
                      item.name === skill.name
                        ? { ...item, policy: event.target.value as HarnessSkillPolicy }
                        : item,
                    ),
                  },
                })
              }
            >
              <option value="always_applicable">Always applicable</option>
              <option value="initial_ingestion">Initial ingestion only</option>
              <option value="available">Available</option>
            </select>
          </label>
        </div>
        <div className="harness-management__modal-scroll">
          <dl className="harness-management__skill-facts">
            <div>
              <dt>Purpose</dt>
              <dd>{skill.purpose}</dd>
            </div>
            <div>
              <dt>Use when</dt>
              <dd>{skill.useWhen}</dd>
            </div>
          </dl>
          <h3>Full skill text</h3>
          {catalogSkill?.text ? (
            <pre className="harness-management__skill-text">{catalogSkill.text}</pre>
          ) : (
            <p>{snapshot.catalogs.skills.reason}</p>
          )}
        </div>
      </section>
    </div>
  );
}

function SkillCatalogDialog({
  snapshot,
  configuration,
  editable,
  onChange,
  onClose,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly configuration: HarnessEffectiveConfiguration;
  readonly editable: boolean;
  onChange(configuration: HarnessEffectiveConfiguration): void;
  onClose(): void;
}) {
  const [query, setQuery] = useState('');
  const selectedNames = useMemo(
    () => new Set(configuration.skills.items.map((skill) => skill.name)),
    [configuration.skills.items],
  );
  const available = useMemo(
    () =>
      snapshot.catalogs.skills.items.filter(
        (skill) =>
          !selectedNames.has(skill.name) && fuzzyMatch(skill.name, skill.description, query),
      ),
    [query, selectedNames, snapshot.catalogs.skills.items],
  );
  return (
    <CatalogDialogShell
      title="Edit skills"
      searchLabel="Search all skills"
      query={query}
      count={snapshot.catalogs.skills.items.length}
      editable={editable}
      onQuery={setQuery}
      onClose={onClose}
    >
      <label className="harness-management__discovery-policy">
        <span>Available discovery</span>
        <select
          value={configuration.skills.availableDiscoveryPolicy}
          disabled={!editable}
          onChange={(event) =>
            onChange({
              ...configuration,
              skills: {
                ...configuration.skills,
                availableDiscoveryPolicy: event.target.value as 'whitelist' | 'blacklist',
              },
            })
          }
        >
          <option value="whitelist">Whitelist</option>
          <option value="blacklist">Blacklist</option>
        </select>
      </label>
      <h3>Selected skills</h3>
      <div className="harness-management__catalog-selected">
        {configuration.skills.items.map((skill) => (
          <div className="harness-management__catalog-row" key={skill.name}>
            <div>
              <strong>{skill.name}</strong>
              <small>{skill.purpose}</small>
            </div>
            <select
              aria-label={`${skill.name} applicability`}
              value={skill.policy}
              disabled={!editable}
              onChange={(event) =>
                onChange({
                  ...configuration,
                  skills: {
                    ...configuration.skills,
                    items: configuration.skills.items.map((item) =>
                      item.name === skill.name
                        ? { ...item, policy: event.target.value as HarnessSkillPolicy }
                        : item,
                    ),
                  },
                })
              }
            >
              <option value="always_applicable">Always applicable</option>
              <option value="initial_ingestion">Initial ingestion only</option>
              <option value="available">Available</option>
            </select>
            <button
              type="button"
              aria-label={`Remove ${skill.name}`}
              disabled={!editable}
              onClick={() =>
                onChange({
                  ...configuration,
                  skills: {
                    ...configuration.skills,
                    items: configuration.skills.items.filter((item) => item.name !== skill.name),
                  },
                })
              }
            >
              <X size={15} aria-hidden="true" />
            </button>
          </div>
        ))}
      </div>
      <h3>Skill catalog</h3>
      <div className="harness-management__catalog-results">
        {available.map((skill) => (
          <div className="harness-management__catalog-row" key={skill.name}>
            <div>
              <strong>{skill.name}</strong>
              <small>{skill.description}</small>
            </div>
            <button
              type="button"
              disabled={!editable}
              onClick={() =>
                onChange({
                  ...configuration,
                  skills: {
                    ...configuration.skills,
                    items: [
                      ...configuration.skills.items,
                      {
                        name: skill.name,
                        path: skill.path,
                        purpose: skill.description,
                        useWhen: skill.description,
                        policy: 'available',
                      },
                    ],
                  },
                })
              }
            >
              Add
            </button>
          </div>
        ))}
        {!available.length && <p>No matching unselected skills.</p>}
      </div>
    </CatalogDialogShell>
  );
}

function ToolCatalogDialog({
  snapshot,
  configuration,
  editable,
  onChange,
  onClose,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly configuration: HarnessEffectiveConfiguration;
  readonly editable: boolean;
  onChange(configuration: HarnessEffectiveConfiguration): void;
  onClose(): void;
}) {
  const [query, setQuery] = useState('');
  const selectedNames = useMemo(
    () => new Set(configuration.tools.items.map((tool) => tool.name)),
    [configuration.tools.items],
  );
  const available = useMemo(
    () =>
      snapshot.catalogs.tools.items.filter(
        (tool) => !selectedNames.has(tool.name) && fuzzyMatch(tool.name, tool.description, query),
      ),
    [query, selectedNames, snapshot.catalogs.tools.items],
  );
  return (
    <CatalogDialogShell
      title="Edit tools"
      searchLabel="Search all tools"
      query={query}
      count={snapshot.catalogs.tools.items.length}
      editable={editable}
      onQuery={setQuery}
      onClose={onClose}
    >
      <label className="harness-management__discovery-policy">
        <span>Available discovery</span>
        <select
          value={configuration.tools.availableDiscoveryPolicy}
          disabled={!editable}
          onChange={(event) =>
            onChange({
              ...configuration,
              tools: {
                ...configuration.tools,
                availableDiscoveryPolicy: event.target.value as 'whitelist' | 'blacklist',
              },
            })
          }
        >
          <option value="whitelist">Whitelist</option>
          <option value="blacklist">Blacklist</option>
        </select>
      </label>
      <h3>Selected tools</h3>
      <div className="harness-management__catalog-selected">
        {configuration.tools.items.map((tool) => (
          <div className="harness-management__catalog-row" key={tool.name}>
            <div>
              <strong>{tool.name}</strong>
            </div>
            <select
              aria-label={`${tool.name} exposure`}
              value={tool.policy}
              disabled={!editable}
              onChange={(event) =>
                onChange({
                  ...configuration,
                  tools: {
                    ...configuration.tools,
                    items: configuration.tools.items.map((item) =>
                      item.name === tool.name
                        ? { ...item, policy: event.target.value as HarnessToolPolicy }
                        : item,
                    ),
                  },
                })
              }
            >
              <option value="every_invocation">Always applicable</option>
              <option value="initial_invocation">Initial ingestion only</option>
              <option value="available">Available</option>
            </select>
            <button
              type="button"
              aria-label={`Remove ${tool.name}`}
              disabled={!editable}
              onClick={() =>
                onChange({
                  ...configuration,
                  tools: {
                    ...configuration.tools,
                    items: configuration.tools.items.filter((item) => item.name !== tool.name),
                  },
                })
              }
            >
              <X size={15} aria-hidden="true" />
            </button>
          </div>
        ))}
      </div>
      <h3>Tool catalog</h3>
      <div className="harness-management__catalog-results">
        {available.map((tool) => (
          <div className="harness-management__catalog-row" key={tool.name}>
            <div>
              <strong>{tool.name}</strong>
              <small>{tool.description}</small>
            </div>
            <button
              type="button"
              disabled={!editable}
              onClick={() =>
                onChange({
                  ...configuration,
                  tools: {
                    ...configuration.tools,
                    items: [...configuration.tools.items, { name: tool.name, policy: 'available' }],
                  },
                })
              }
            >
              Add
            </button>
          </div>
        ))}
        {!available.length && <p>No matching unselected tools.</p>}
      </div>
      <p className="harness-management__card-footer">{configuration.tools.schemaBoundary}</p>
    </CatalogDialogShell>
  );
}

function CatalogDialogShell({
  title,
  searchLabel,
  query,
  count,
  editable,
  onQuery,
  onClose,
  children,
}: {
  readonly title: string;
  readonly searchLabel: string;
  readonly query: string;
  readonly count: number;
  readonly editable: boolean;
  onQuery(query: string): void;
  onClose(): void;
  readonly children: ReactNode;
}) {
  return (
    <div className="harness-management__modal-backdrop">
      <section
        className="harness-management__modal is-catalog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="harness-catalog-title"
      >
        <header>
          <div>
            <h2 id="harness-catalog-title">{title}</h2>
            <p>{count} catalog entries</p>
          </div>
          <button type="button" aria-label={`Close ${title}`} onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>
        {!editable && <p role="status">Preparing the working draft...</p>}
        <label className="harness-management__catalog-search">
          <Search size={16} aria-hidden="true" />
          <span className="visually-hidden">{searchLabel}</span>
          <input
            autoFocus
            aria-label={searchLabel}
            value={query}
            placeholder={searchLabel}
            onChange={(event) => onQuery(event.target.value)}
          />
        </label>
        <div className="harness-management__modal-scroll">{children}</div>
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

function initialNameSubset(
  snapshot: ConversationHarnessManagementSnapshot,
  names: readonly string[],
): readonly string[] {
  const assignedName = snapshot.agentIdentity?.name;
  if (!assignedName || !names.includes(assignedName)) return names.slice(0, 10);
  return [assignedName, ...names.filter((name) => name !== assignedName)].slice(0, 10);
}

function humanize(value: string): string {
  return value.replaceAll('_', ' ');
}
