import { ArrowLeft, Check, GitCommitHorizontal, RefreshCw, Upload } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import { MarkdownEditor } from '../../components/MarkdownEditor';
import { AgentIdentityBadge } from '../../components/AgentIdentityBadge';
import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementRead,
  ConversationHarnessManagementSnapshot,
  HarnessEffectiveConfiguration,
  HarnessSkillPolicy,
  HarnessToolGuidancePolicy,
  HarnessUpdateStrategy,
} from '../../application/conversationHarnesses';

export interface ConversationHarnessManagementProps {
  readonly read: ConversationHarnessManagementRead | null;
  readonly commandPending?: boolean;
  readonly commandError?: string | null;
  onBack(): void;
  onCommand?(command: ConversationHarnessManagementCommand): void;
}

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

  if (read.kind !== 'available') {
    return (
      <ManagementShell onBack={onBack}>
        <div className="harness-management__unavailable" role="alert">
          <div>
            <h2>{read.kind === 'unbound' ? 'No harness assigned' : 'Harness unavailable'}</h2>
            <p>{read.reason}</p>
          </div>
        </div>
      </ManagementShell>
    );
  }

  const { snapshot } = read;
  const configuration = snapshot.workingCopy.configuration;
  const editable = Boolean(onCommand);
  const saveConfiguration = (next: HarnessEffectiveConfiguration) =>
    onCommand?.({
      kind: 'save_working_copy',
      expectedDraftRevision: snapshot.workingCopy.draftRevision,
      configuration: next,
    });

  return (
    <ManagementShell snapshot={snapshot} onBack={onBack}>
      <h1 className="visually-hidden">{configuration.identity.name} Harness Management</h1>
      {commandError && (
        <p className="harness-management__command-error" role="alert">
          {commandError}
        </p>
      )}
      <SessionVersionNotice snapshot={snapshot} />

      <div className="harness-management__grid">
        <ManagementCard
          title="Harness details"
          description="Identity and version state for this harness."
          wide
        >
          <div className="harness-management__field-row is-three">
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
            <ManagementField label="Harness role">
              <input
                value={configuration.identity.role}
                disabled={!editable}
                onChange={(event) =>
                  saveConfiguration({
                    ...configuration,
                    identity: { ...configuration.identity, role: event.target.value },
                  })
                }
              />
            </ManagementField>
          </div>
          <VersionTiles snapshot={snapshot} />
          <div className="harness-management__identity-policy">
            {snapshot.agentIdentity ? (
              <AgentIdentityBadge identity={snapshot.agentIdentity} />
            ) : (
              <p>This session does not yet have a durable Agent identity.</p>
            )}
            <div>
              <span>Permitted name pool</span>
              <strong>
                {configuration.identity.permittedAgentNames
                  ? `Harness subset · ${configuration.identity.permittedAgentNames.length} names`
                  : 'Product default · 100 names'}
              </strong>
              <small>Existing sessions keep their assigned name when this pool changes.</small>
            </div>
            <div>
              <span>Visual identity</span>
              <strong>
                {configuration.identity.visualIdentity
                  ? humanize(configuration.identity.visualIdentity.token)
                  : 'Not configured'}
              </strong>
              <small>Stored with the applied Session revision.</small>
            </div>
          </div>
        </ManagementCard>

        <ManagementCard
          title="Prompt prefix"
          description="Prepended to the initial prompt for a new session. It is also intended to be re-ingested after context compression; the custom compression routine is deferred."
          wide
        >
          <MarkdownEditor
            label="Prompt prefix"
            value={configuration.promptPrefix.content}
            editable={editable}
            onChange={(content) =>
              saveConfiguration({
                ...configuration,
                promptPrefix: { ...configuration.promptPrefix, content },
              })
            }
          />
        </ManagementCard>

        <ManagementCard
          title="Skill policy"
          description="Choose when each skill is included and how available skills are discovered."
        >
          <PolicyGuide />
          <ManagementField label="Available-skill discovery">
            <select
              value={configuration.skills.discoveryPolicy}
              disabled={!editable}
              onChange={(event) =>
                saveConfiguration({
                  ...configuration,
                  skills: {
                    ...configuration.skills,
                    discoveryPolicy: event.target.value as 'whitelist' | 'blacklist',
                  },
                })
              }
            >
              <option value="whitelist">Whitelist</option>
              <option value="blacklist">Blacklist</option>
            </select>
          </ManagementField>
          <div className="harness-management__policy-list">
            {configuration.skills.items.map((skill) => (
              <div className="harness-management__policy-item" key={skill.name}>
                <div>
                  <strong>{skill.name}</strong>
                  <p>{skill.purpose}</p>
                  <code>{skill.path}</code>
                </div>
                <ManagementField label={`Policy for ${skill.name}`} compact>
                  <select
                    value={skill.policy}
                    disabled={!editable}
                    onChange={(event) =>
                      saveConfiguration({
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
                </ManagementField>
              </div>
            ))}
          </div>
        </ManagementCard>

        <ManagementCard title="Tool policy" description={configuration.tools.schemaBoundary}>
          <ManagementField label="Tool discovery">
            <select
              value={configuration.tools.discoveryPolicy}
              disabled={!editable}
              onChange={(event) =>
                saveConfiguration({
                  ...configuration,
                  tools: {
                    ...configuration.tools,
                    discoveryPolicy: event.target.value as 'whitelist' | 'blacklist',
                  },
                })
              }
            >
              <option value="whitelist">Whitelist</option>
              <option value="blacklist">Blacklist</option>
            </select>
          </ManagementField>
          <div className="harness-management__policy-list">
            {configuration.tools.items.map((tool) => (
              <div className="harness-management__policy-item is-tool" key={tool.name}>
                <label className="harness-management__check">
                  <input
                    type="checkbox"
                    checked={tool.exposed}
                    disabled={!editable}
                    onChange={(event) =>
                      saveConfiguration({
                        ...configuration,
                        tools: {
                          ...configuration.tools,
                          items: configuration.tools.items.map((item) =>
                            item.name === tool.name
                              ? { ...item, exposed: event.target.checked }
                              : item,
                          ),
                        },
                      })
                    }
                  />
                  <span>{tool.name}</span>
                </label>
                <ManagementField label={`Guidance for ${tool.name}`} compact>
                  <select
                    value={tool.guidancePolicy}
                    disabled={!editable}
                    onChange={(event) =>
                      saveConfiguration({
                        ...configuration,
                        tools: {
                          ...configuration.tools,
                          items: configuration.tools.items.map((item) =>
                            item.name === tool.name
                              ? {
                                  ...item,
                                  guidancePolicy: event.target.value as HarnessToolGuidancePolicy,
                                }
                              : item,
                          ),
                        },
                      })
                    }
                  >
                    <option value="none">Available when exposed</option>
                    <option value="initial_ingestion">Initial guidance only</option>
                    <option value="always_applicable">Initial + compression guidance</option>
                  </select>
                </ManagementField>
              </div>
            ))}
          </div>
        </ManagementCard>

        <ManagementCard
          title="Allowed model and reasoning"
          description="Callers may inherit the product default or choose any checked option."
        >
          <OptionGroup
            label="Models"
            inheritedLabel="Allow inherited/default model"
            inherited={configuration.runtime.allowInheritedModel}
            options={configuration.runtime.availableModels}
            selected={configuration.runtime.allowedModels}
            editable={editable}
            onInheritedChange={(allowInheritedModel) =>
              saveConfiguration({
                ...configuration,
                runtime: { ...configuration.runtime, allowInheritedModel },
              })
            }
            onSelectedChange={(allowedModels) =>
              saveConfiguration({
                ...configuration,
                runtime: { ...configuration.runtime, allowedModels },
              })
            }
          />
          <OptionGroup
            label="Reasoning levels"
            inheritedLabel="Allow inherited/default reasoning"
            inherited={configuration.runtime.allowInheritedReasoning}
            options={configuration.runtime.availableReasoningEfforts}
            selected={configuration.runtime.allowedReasoningEfforts}
            editable={editable}
            onInheritedChange={(allowInheritedReasoning) =>
              saveConfiguration({
                ...configuration,
                runtime: { ...configuration.runtime, allowInheritedReasoning },
              })
            }
            onSelectedChange={(allowedReasoningEfforts) =>
              saveConfiguration({
                ...configuration,
                runtime: { ...configuration.runtime, allowedReasoningEfforts },
              })
            }
          />
        </ManagementCard>

        <ManagementCard
          title="Sandbox and authority"
          description={configuration.runtime.authoritySummary}
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
              <select
                value={configuration.runtime.approvalPolicy}
                disabled={!editable}
                onChange={(event) =>
                  saveConfiguration({
                    ...configuration,
                    runtime: {
                      ...configuration.runtime,
                      approvalPolicy: event.target.value as 'never',
                    },
                  })
                }
              >
                {configuration.runtime.approvalPolicyOptions.map((option) => (
                  <option value={option} key={option}>
                    {humanize(option)}
                  </option>
                ))}
              </select>
            </ManagementField>
          </div>
          <p className="harness-management__deferred-note">
            Broader sandbox customization is deferred.
          </p>
        </ManagementCard>

        <ManagementCard
          title="Application hooks"
          description="Application-owned hooks exposed to this harness. Deeper hook design is deferred."
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
                  {hook.status === 'exposed' ? 'Exposed' : 'Not connected'}
                </StateBadge>
              </li>
            ))}
          </ul>
        </ManagementCard>

        <VersionControlCard
          snapshot={snapshot}
          commandPending={commandPending}
          onCommand={onCommand}
        />
        <SessionUpdateCard
          snapshot={snapshot}
          commandPending={commandPending}
          onCommand={onCommand}
        />
      </div>
    </ManagementShell>
  );
}

function ManagementShell({
  snapshot,
  onBack,
  children,
}: {
  readonly snapshot?: ConversationHarnessManagementSnapshot;
  onBack(): void;
  readonly children: ReactNode;
}) {
  const name = snapshot?.workingCopy.configuration.identity.name;
  return (
    <section className="harness-management" aria-label="Harness Management">
      <div className="harness-management__toolbar" aria-label="Harness Management controls">
        <button className="harness-management__back" type="button" onClick={onBack}>
          <ArrowLeft size={16} aria-hidden="true" />
          Back to conversation
        </button>
        <div className="harness-management__toolbar-context">
          {snapshot?.agentIdentity && (
            <AgentIdentityBadge identity={snapshot.agentIdentity} compact />
          )}
          <span>Harness</span>
          <strong>{name ?? 'Management'}</strong>
        </div>
        {snapshot && (
          <div className="harness-management__toolbar-status" aria-live="polite">
            <StateBadge tone={workingStateTone(snapshot.workingCopy.state)}>
              {workingStateLabel(snapshot.workingCopy.state)}
            </StateBadge>
            {snapshot.versionControl.support === 'recorded_preview' && (
              <StateBadge tone="neutral">Preview only</StateBadge>
            )}
          </div>
        )}
      </div>
      <div className="harness-management__scroll">{children}</div>
    </section>
  );
}

function SessionVersionNotice({
  snapshot,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
}) {
  const binding = snapshot.sessionBinding;
  if (binding.state === 'untracked')
    return (
      <section
        className="harness-management__notice is-neutral"
        aria-label="Session harness version"
      >
        <strong>Session version is not tracked yet.</strong>
        <span>{binding.reason}</span>
      </section>
    );
  if (binding.state === 'current')
    return (
      <section
        className="harness-management__notice is-current"
        aria-label="Session harness version"
      >
        <Check size={18} aria-hidden="true" />
        <strong>This Agent Session uses the active harness version.</strong>
      </section>
    );
  return (
    <section className="harness-management__notice" aria-label="Session harness version">
      <RefreshCw size={18} aria-hidden="true" />
      <div>
        <strong>
          This Agent Session uses v{binding.appliedRevision}; v{binding.desiredRevision} is
          available.
        </strong>
        <span>{binding.reason}</span>
      </div>
    </section>
  );
}

function ManagementCard({
  title,
  description,
  wide = false,
  children,
}: {
  readonly title: string;
  readonly description: string;
  readonly wide?: boolean;
  readonly children: ReactNode;
}) {
  return (
    <section className={`harness-management__card${wide ? ' is-wide' : ''}`}>
      <header>
        <h2>{title}</h2>
        <p>{description}</p>
      </header>
      {children}
    </section>
  );
}

function ManagementField({
  label,
  compact = false,
  children,
}: {
  readonly label: string;
  readonly compact?: boolean;
  readonly children: ReactNode;
}) {
  return (
    <label className={`harness-management__field${compact ? ' is-compact' : ''}`}>
      <span>{label}</span>
      {children}
    </label>
  );
}

function VersionTiles({ snapshot }: { readonly snapshot: ConversationHarnessManagementSnapshot }) {
  const values = [
    ['Catalog', `v${snapshot.catalogRevision}`],
    ['Committed', versionLabel(snapshot.versionControl.committedRevision)],
    ['Active', versionLabel(snapshot.versionControl.activeRevision)],
    ['This session', versionLabel(snapshot.sessionBinding.appliedRevision)],
  ];
  return (
    <dl className="harness-management__versions">
      {values.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function PolicyGuide() {
  return (
    <dl className="harness-management__policy-guide">
      <div>
        <dt>Always applicable</dt>
        <dd>Full text at the start and after harness-aware compression.</dd>
      </div>
      <div>
        <dt>Initial ingestion only</dt>
        <dd>Full text when a new session begins.</dd>
      </div>
      <div>
        <dt>Available</dt>
        <dd>Exposed through the whitelist or blacklist.</dd>
      </div>
    </dl>
  );
}

function OptionGroup({
  label,
  inheritedLabel,
  inherited,
  options,
  selected,
  editable,
  onInheritedChange,
  onSelectedChange,
}: {
  readonly label: string;
  readonly inheritedLabel: string;
  readonly inherited: boolean;
  readonly options: readonly string[];
  readonly selected: readonly string[];
  readonly editable: boolean;
  onInheritedChange(value: boolean): void;
  onSelectedChange(value: readonly string[]): void;
}) {
  const toggle = (option: string, checked: boolean) =>
    onSelectedChange(checked ? [...selected, option] : selected.filter((item) => item !== option));
  return (
    <fieldset className="harness-management__option-group">
      <legend>{label}</legend>
      <label className="harness-management__check">
        <input
          type="checkbox"
          checked={inherited}
          disabled={!editable}
          onChange={(event) => onInheritedChange(event.target.checked)}
        />
        <span>{inheritedLabel}</span>
      </label>
      {options.map((option) => (
        <label className="harness-management__check" key={option}>
          <input
            type="checkbox"
            checked={selected.includes(option)}
            disabled={!editable}
            onChange={(event) => toggle(option, event.target.checked)}
          />
          <span>{option}</span>
        </label>
      ))}
      {options.length === 0 && <p>No restricted options are configured.</p>}
    </fieldset>
  );
}

function VersionControlCard({
  snapshot,
  commandPending,
  onCommand,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly commandPending: boolean;
  onCommand?(command: ConversationHarnessManagementCommand): void;
}) {
  const supported = snapshot.versionControl.support === 'recorded_preview' && Boolean(onCommand);
  const canCommit = supported && snapshot.workingCopy.state === 'uncommitted' && !commandPending;
  const canPush =
    supported && snapshot.workingCopy.state === 'committed_not_active' && !commandPending;
  return (
    <ManagementCard
      title="Version history"
      description="Commit records a version. Push makes that committed version active locally; it does not publish to a remote."
      wide
    >
      <div className="harness-management__lifecycle">
        <div>
          <span>Working copy</span>
          <strong>{workingStateLabel(snapshot.workingCopy.state)}</strong>
          <small>Based on v{snapshot.workingCopy.baseRevision}</small>
        </div>
        <div>
          <span>Committed</span>
          <strong>{versionLabel(snapshot.versionControl.committedRevision)}</strong>
          <small>Recorded history</small>
        </div>
        <div>
          <span>Active</span>
          <strong>{versionLabel(snapshot.versionControl.activeRevision)}</strong>
          <small>Used for new sessions</small>
        </div>
      </div>
      <p className="harness-management__support-note">{snapshot.versionControl.reason}</p>
      <div className="harness-management__actions">
        <button
          type="button"
          disabled={!canCommit}
          onClick={() =>
            onCommand?.({
              kind: 'commit',
              expectedDraftRevision: snapshot.workingCopy.draftRevision,
            })
          }
        >
          <GitCommitHorizontal size={16} aria-hidden="true" />
          Commit version
        </button>
        <button
          className="is-primary"
          type="button"
          disabled={!canPush || snapshot.versionControl.committedRevision === null}
          onClick={() => {
            if (snapshot.versionControl.committedRevision === null) return;
            onCommand?.({
              kind: 'push',
              expectedCommittedRevision: snapshot.versionControl.committedRevision,
            });
          }}
        >
          <Upload size={16} aria-hidden="true" />
          Push version
        </button>
      </div>
    </ManagementCard>
  );
}

function SessionUpdateCard({
  snapshot,
  commandPending,
  onCommand,
}: {
  readonly snapshot: ConversationHarnessManagementSnapshot;
  readonly commandPending: boolean;
  onCommand?(command: ConversationHarnessManagementCommand): void;
}) {
  const configuredPolicy = snapshot.workingCopy.configuration.updatePolicy;
  const defaultStrategy =
    configuredPolicy.status === 'configured' ? configuredPolicy.defaultStrategy : 'next_prompt';
  const [strategy, setStrategy] = useState<HarnessUpdateStrategy>(
    snapshot.sessionBinding.updateStrategy ?? defaultStrategy,
  );
  const activeRevision = snapshot.versionControl.activeRevision;
  const supported =
    snapshot.versionControl.support === 'recorded_preview' &&
    snapshot.sessionBinding.state !== 'untracked' &&
    activeRevision !== null &&
    Boolean(onCommand);
  const request = (scope: 'current_session' | 'all_relevant_sessions') => {
    if (activeRevision === null) return;
    onCommand?.({
      kind: 'request_session_update',
      expectedActiveRevision: activeRevision,
      scope,
      strategy,
    });
  };
  return (
    <ManagementCard
      title="Agent Session updates"
      description="Choose how the active version should reach this session or every relevant session."
      wide
    >
      <fieldset className="harness-management__update-choice" disabled={!supported}>
        <legend>When to update</legend>
        <label>
          <input
            type="radio"
            name={`harness-update-${snapshot.sessionId}`}
            value="next_prompt"
            checked={strategy === 'next_prompt'}
            onChange={() => setStrategy('next_prompt')}
          />
          <span>
            <strong>Wait until next prompt</strong>
            <small>Use the application send boundary before the next invocation.</small>
          </span>
        </label>
        <label>
          <input
            type="radio"
            name={`harness-update-${snapshot.sessionId}`}
            value="interrupt"
            checked={strategy === 'interrupt'}
            onChange={() => setStrategy('interrupt')}
          />
          <span>
            <strong>Interrupt now</strong>
            <small>Requires a supported application/runtime interrupt path.</small>
          </span>
        </label>
      </fieldset>
      {configuredPolicy.status === 'configured' ? (
        <ul className="harness-management__update-behavior" aria-label="Planned update behavior">
          <li>Avoid re-prefixing skill or tool guidance already present.</li>
          <li>Append changed guidance and tell the agent when removed items no longer apply.</li>
          <li>Prompt reconstruction remains deferred.</li>
        </ul>
      ) : (
        <p className="harness-management__support-note">{configuredPolicy.reason}</p>
      )}
      <div className="harness-management__actions">
        <button
          className="is-primary"
          type="button"
          disabled={!supported || commandPending}
          onClick={() => request('current_session')}
        >
          Apply updated harness
        </button>
        <button
          type="button"
          disabled={!supported || commandPending}
          onClick={() => request('all_relevant_sessions')}
        >
          Apply to all relevant sessions
          {snapshot.sessionBinding.relevantSessionCount !== null
            ? ` (${snapshot.sessionBinding.relevantSessionCount})`
            : ''}
        </button>
      </div>
    </ManagementCard>
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

function workingStateTone(
  state: ConversationHarnessManagementSnapshot['workingCopy']['state'],
): 'positive' | 'caution' | 'neutral' {
  if (state === 'uncommitted') return 'caution';
  if (state === 'committed_not_active') return 'neutral';
  return 'positive';
}

function workingStateLabel(
  state: ConversationHarnessManagementSnapshot['workingCopy']['state'],
): string {
  if (state === 'uncommitted') return 'Uncommitted changes';
  if (state === 'committed_not_active') return 'Committed, not active';
  return 'Up to date';
}

function versionLabel(revision: number | null): string {
  return revision === null ? 'Not connected' : `v${revision}`;
}

function humanize(value: string): string {
  return value.replaceAll('_', ' ');
}
