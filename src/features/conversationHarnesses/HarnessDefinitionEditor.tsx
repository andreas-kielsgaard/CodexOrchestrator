import { Check, ChevronDown, Search } from 'lucide-react';
import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from 'react';
import type {
  HarnessConfigurationCatalogs,
  HarnessEffectiveConfiguration,
  HarnessMcpServerExposure,
  HarnessReasoningLevel,
  HarnessSkillPolicy,
  HarnessToolPolicy,
} from '../../application/conversationHarnesses';
import { MarkdownEditor } from '../../components/MarkdownEditor';
import { AgentMarkdown } from '../agentSessions/AgentMarkdown';
import './harnessInspector.css';

interface SelectOption<T extends string = string> {
  readonly value: T;
  readonly label: string;
  readonly description?: string;
}

export type HarnessDefinitionProperty =
  | 'identityName'
  | 'identityMachineKey'
  | 'permittedAgentNames'
  | 'visualIdentity'
  | 'promptPrefixContent'
  | 'skillDiscoveryPolicy'
  | 'skillItems'
  | 'toolDiscoveryPolicy'
  | 'toolItems'
  | 'mcpServers'
  | 'runtimeModelPolicyMode'
  | 'runtimeModels'
  | 'runtimeDefaultModel'
  | 'runtimeDefaultReasoning'
  | 'runtimeSandbox'
  | 'runtimeAuthoritySummary'
  | 'hookItems';

type HarnessDefinitionSource = 'inherited' | 'overridden' | 'instance';

export interface HarnessDefinitionEditorProps {
  readonly configuration: HarnessEffectiveConfiguration;
  readonly catalogs: HarnessConfigurationCatalogs;
  readonly editable: boolean;
  readonly mcpComponents?: readonly {
    readonly serverName: string;
    readonly toolName: string;
    readonly title: string;
  }[];
  readonly modelPolicy?: HarnessEffectiveConfiguration['runtime'];
  readonly modelPolicyEditable?: boolean;
  readonly modelPolicyNote?: ReactNode;
  readonly modelPolicyBoundaryKey?: string;
  readonly modelPolicyLabelPrefix?: string;
  readonly provenance?: Partial<Record<HarnessDefinitionProperty, HarnessDefinitionSource>>;
  onResetProperty?(property: HarnessDefinitionProperty): void;
  onChange(configuration: HarnessEffectiveConfiguration): void;
  onModelPolicyChange?(runtime: HarnessEffectiveConfiguration['runtime']): void;
}

/** Session-neutral Harness definition editor shared by Harness Management and Workflow Roles. */
export function HarnessDefinitionEditor({
  configuration,
  catalogs,
  editable,
  mcpComponents = [],
  modelPolicy = configuration.runtime,
  modelPolicyEditable = editable,
  modelPolicyNote,
  modelPolicyBoundaryKey = configuration.identity.machineKey,
  modelPolicyLabelPrefix = 'Harness',
  provenance,
  onResetProperty,
  onChange,
  onModelPolicyChange,
}: HarnessDefinitionEditorProps) {
  const updateRuntime =
    onModelPolicyChange ?? ((runtime) => onChange({ ...configuration, runtime }));
  const rememberedDefault = useRef<{
    readonly model: string;
    readonly reasoning: HarnessReasoningLevel | null;
  } | null>(null);
  useEffect(() => {
    rememberedDefault.current = null;
  }, [modelPolicyBoundaryKey]);
  const modelOptions = catalogs.models.items.map((model) => ({
    value: model.id,
    label: model.label,
  }));
  const allowedModelIds = modelPolicy.models
    .filter((model) => model.allowed)
    .map((model) => model.modelId);
  const selectedDefaultModel = modelPolicy.models.find(
    (model) => model.modelId === modelPolicy.defaultModel,
  );
  const defaultReasoningLevels = modelPolicy.defaultModel
    ? (catalogs.models.items
        .find((model) => model.id === modelPolicy.defaultModel)
        ?.reasoningLevels.filter((level) =>
          selectedDefaultModel ? reasoningWithin(level, selectedDefaultModel) : false,
        ) ?? [])
    : [...new Set(catalogs.models.items.flatMap((model) => model.reasoningLevels))];
  const defaultReasoningOptions = defaultReasoningLevels.map((level) => ({
    value: level,
    label: humanize(level),
  }));
  const updateModels = (models: HarnessEffectiveConfiguration['runtime']['models']) => {
    const reconciled = reconcileProvisionalDefault(
      modelPolicy,
      models,
      catalogs.models.items,
      rememberedDefault,
    );
    updateRuntime({ ...modelPolicy, ...reconciled });
  };

  return (
    <div className="harness-definition-editor" data-testid="harness-definition-editor">
      <DefinitionSection title="Harness details">
        <div className="harness-management__field-row">
          <DefinitionField
            label="Harness name"
            source={provenance?.identityName}
            onReset={onResetProperty ? () => onResetProperty('identityName') : undefined}
          >
            <input
              aria-label="Harness name"
              value={configuration.identity.name}
              disabled={!editable}
              onChange={(event) =>
                onChange({
                  ...configuration,
                  identity: { ...configuration.identity, name: event.currentTarget.value },
                })
              }
            />
          </DefinitionField>
          <DefinitionField
            label="Machine key"
            source={provenance?.identityMachineKey}
            onReset={onResetProperty ? () => onResetProperty('identityMachineKey') : undefined}
          >
            <input
              aria-label="Harness machine key"
              value={configuration.identity.machineKey}
              disabled={!editable}
              onChange={(event) =>
                onChange({
                  ...configuration,
                  identity: { ...configuration.identity, machineKey: event.currentTarget.value },
                })
              }
            />
          </DefinitionField>
        </div>
        <SearchableMultiSelect
          label="Permitted Agent names"
          source={provenance?.permittedAgentNames}
          onReset={onResetProperty ? () => onResetProperty('permittedAgentNames') : undefined}
          options={catalogs.agentNames.items.map((name) => ({ value: name, label: name }))}
          values={configuration.identity.permittedAgentNames ?? catalogs.agentNames.items}
          editable={editable}
          unavailableReason={catalogs.agentNames.reason}
          onChange={(values) =>
            onChange({
              ...configuration,
              identity: { ...configuration.identity, permittedAgentNames: values },
            })
          }
        />
        <SearchableSingleSelect
          label="Visual identity"
          source={provenance?.visualIdentity}
          onReset={onResetProperty ? () => onResetProperty('visualIdentity') : undefined}
          options={catalogs.agentVisualIdentities.items.map((entry) => ({
            value: `${entry.identity.token}\u0000${entry.identity.accent}`,
            label: entry.label,
          }))}
          value={
            configuration.identity.visualIdentity
              ? `${configuration.identity.visualIdentity.token}\u0000${configuration.identity.visualIdentity.accent}`
              : null
          }
          editable={editable}
          clearLabel="Not configured"
          unavailableReason={catalogs.agentVisualIdentities.reason}
          onChange={(value) => {
            const [token, accent] = value?.split('\u0000') ?? [];
            onChange({
              ...configuration,
              identity: {
                ...configuration.identity,
                visualIdentity: token && accent ? { token, accent } : null,
              },
            });
          }}
        />
      </DefinitionSection>

      <DefinitionSection title="Prompt prefix">
        <DefinitionField
          label="Prompt content"
          source={provenance?.promptPrefixContent}
          onReset={onResetProperty ? () => onResetProperty('promptPrefixContent') : undefined}
        >
          {editable ? (
            <MarkdownEditor
              label="Prompt prefix"
              value={configuration.promptPrefix.content}
              editable
              onChange={(content) =>
                onChange({
                  ...configuration,
                  promptPrefix: { ...configuration.promptPrefix, content },
                })
              }
            />
          ) : (
            <AgentMarkdown className="harness-management__markdown-view">
              {configuration.promptPrefix.content}
            </AgentMarkdown>
          )}
        </DefinitionField>
      </DefinitionSection>

      <DefinitionSection title="Skills">
        <SearchableSingleSelect
          label="Available skill discovery"
          source={provenance?.skillDiscoveryPolicy}
          onReset={onResetProperty ? () => onResetProperty('skillDiscoveryPolicy') : undefined}
          options={discoveryPolicyOptions}
          value={configuration.skills.availableDiscoveryPolicy}
          editable={editable}
          onChange={(availableDiscoveryPolicy) =>
            availableDiscoveryPolicy &&
            onChange({
              ...configuration,
              skills: { ...configuration.skills, availableDiscoveryPolicy },
            })
          }
        />
        <SearchableMultiSelect
          label="Harness skills"
          source={provenance?.skillItems}
          onReset={onResetProperty ? () => onResetProperty('skillItems') : undefined}
          options={catalogs.skills.items.map((skill) => ({
            value: skill.name,
            label: skill.name,
            description: skill.description,
          }))}
          values={configuration.skills.items.map((skill) => skill.name)}
          editable={editable}
          unavailableReason={catalogs.skills.reason}
          onChange={(names) =>
            onChange({
              ...configuration,
              skills: {
                ...configuration.skills,
                items: names.map(
                  (name) =>
                    configuration.skills.items.find((skill) => skill.name === name) ?? {
                      name,
                      path: catalogs.skills.items.find((skill) => skill.name === name)?.path ?? '',
                      purpose:
                        catalogs.skills.items.find((skill) => skill.name === name)?.description ??
                        '',
                      useWhen:
                        catalogs.skills.items.find((skill) => skill.name === name)?.description ??
                        '',
                      policy: 'available' as const,
                    },
                ),
              },
            })
          }
        />
        {configuration.skills.items.map((skill) => (
          <SearchableSingleSelect<HarnessSkillPolicy>
            key={skill.name}
            label={`${skill.name} applicability`}
            options={skillPolicyOptions}
            value={skill.policy}
            editable={editable}
            onChange={(policy) =>
              policy &&
              onChange({
                ...configuration,
                skills: {
                  ...configuration.skills,
                  items: configuration.skills.items.map((candidate) =>
                    candidate.name === skill.name ? { ...candidate, policy } : candidate,
                  ),
                },
              })
            }
          />
        ))}
      </DefinitionSection>

      <DefinitionSection title="Tools and MCP exposure">
        <SearchableSingleSelect
          label="Available tool discovery"
          source={provenance?.toolDiscoveryPolicy}
          onReset={onResetProperty ? () => onResetProperty('toolDiscoveryPolicy') : undefined}
          options={discoveryPolicyOptions}
          value={configuration.tools.availableDiscoveryPolicy}
          editable={editable}
          onChange={(availableDiscoveryPolicy) =>
            availableDiscoveryPolicy &&
            onChange({
              ...configuration,
              tools: { ...configuration.tools, availableDiscoveryPolicy },
            })
          }
        />
        <SearchableMultiSelect
          label="Harness tools"
          source={provenance?.toolItems}
          onReset={onResetProperty ? () => onResetProperty('toolItems') : undefined}
          options={catalogs.tools.items.map((tool) => ({
            value: tool.name,
            label: tool.name,
            description: tool.description,
          }))}
          values={configuration.tools.items.map((tool) => tool.name)}
          editable={editable}
          unavailableReason={catalogs.tools.reason}
          onChange={(names) =>
            onChange({
              ...configuration,
              tools: {
                ...configuration.tools,
                items: names.map(
                  (name) =>
                    configuration.tools.items.find((tool) => tool.name === name) ?? {
                      name,
                      policy: 'available' as const,
                    },
                ),
              },
            })
          }
        />
        {configuration.tools.items.map((tool) => (
          <SearchableSingleSelect<HarnessToolPolicy>
            key={tool.name}
            label={`${tool.name} exposure timing`}
            options={toolPolicyOptions}
            value={tool.policy}
            editable={editable}
            onChange={(policy) =>
              policy &&
              onChange({
                ...configuration,
                tools: {
                  ...configuration.tools,
                  items: configuration.tools.items.map((candidate) =>
                    candidate.name === tool.name ? { ...candidate, policy } : candidate,
                  ),
                },
              })
            }
          />
        ))}
        <McpExposureSelector
          components={mcpComponents}
          exposures={configuration.tools.mcpServers ?? []}
          editable={editable}
          source={provenance?.mcpServers}
          onReset={onResetProperty ? () => onResetProperty('mcpServers') : undefined}
          onChange={(mcpServers) =>
            onChange({ ...configuration, tools: { ...configuration.tools, mcpServers } })
          }
        />
        <p className="harness-management__card-footer">{configuration.tools.schemaBoundary}</p>
      </DefinitionSection>

      <DefinitionSection title="Runtime">
        <h3>Models and reasoning</h3>
        <SearchableSingleSelect
          label="Model policy ownership"
          source={provenance?.runtimeModelPolicyMode}
          onReset={onResetProperty ? () => onResetProperty('runtimeModelPolicyMode') : undefined}
          options={[
            { value: 'revision_owned', label: 'Version specific' },
            { value: 'delegated_shared', label: 'Delegated shared policy' },
          ]}
          value={modelPolicy.modelPolicyMode}
          editable={editable}
          onChange={(modelPolicyMode) =>
            modelPolicyMode && updateRuntime({ ...modelPolicy, modelPolicyMode })
          }
        />
        <SearchableMultiSelect
          label="Allowed models"
          source={provenance?.runtimeModels}
          onReset={onResetProperty ? () => onResetProperty('runtimeModels') : undefined}
          options={modelOptions}
          values={allowedModelIds}
          editable={modelPolicyEditable}
          unavailableReason={catalogs.models.reason}
          onChange={(modelIds) => {
            const existing = new Map(modelPolicy.models.map((model) => [model.modelId, model]));
            const models = [
              ...modelPolicy.models.map((model) => ({
                ...model,
                allowed: modelIds.includes(model.modelId),
              })),
              ...modelIds
                .filter((modelId) => !existing.has(modelId))
                .map((modelId) => {
                  const levels =
                    catalogs.models.items.find((model) => model.id === modelId)?.reasoningLevels ??
                    [];
                  return {
                    modelId,
                    allowed: true,
                    minReasoning: levels[0] ?? ('low' as const),
                    maxReasoning: levels.at(-1) ?? ('xhigh' as const),
                  };
                }),
            ];
            updateModels(models);
          }}
        />
        {allowedModelIds.map((modelId) => {
          const model = modelPolicy.models.find((candidate) => candidate.modelId === modelId);
          const catalogModel = catalogs.models.items.find((candidate) => candidate.id === modelId);
          if (!model || !catalogModel) return null;
          const minIndex = catalogModel.reasoningLevels.indexOf(model.minReasoning);
          const maxIndex = catalogModel.reasoningLevels.indexOf(model.maxReasoning);
          return (
            <div className="harness-definition-editor__model-range" key={modelId}>
              <strong>{catalogModel.label}</strong>
              <SearchableSingleSelect<HarnessReasoningLevel>
                label={`${modelPolicyLabelPrefix} ${catalogModel.label} minimum reasoning`}
                options={catalogModel.reasoningLevels
                  .slice(0, Math.max(0, maxIndex) + 1)
                  .map((level) => ({ value: level, label: humanize(level) }))}
                value={model.minReasoning}
                editable={modelPolicyEditable}
                onChange={(minReasoning) =>
                  minReasoning &&
                  updateModels(
                    modelPolicy.models.map((candidate) =>
                      candidate.modelId === modelId ? { ...candidate, minReasoning } : candidate,
                    ),
                  )
                }
              />
              <SearchableSingleSelect<HarnessReasoningLevel>
                label={`${modelPolicyLabelPrefix} ${catalogModel.label} maximum reasoning`}
                options={catalogModel.reasoningLevels
                  .slice(Math.max(0, minIndex))
                  .map((level) => ({ value: level, label: humanize(level) }))}
                value={model.maxReasoning}
                editable={modelPolicyEditable}
                onChange={(maxReasoning) =>
                  maxReasoning &&
                  updateModels(
                    modelPolicy.models.map((candidate) =>
                      candidate.modelId === modelId ? { ...candidate, maxReasoning } : candidate,
                    ),
                  )
                }
              />
            </div>
          );
        })}
        <SearchableSingleSelect
          label={`${modelPolicyLabelPrefix} default model`}
          source={provenance?.runtimeDefaultModel}
          onReset={onResetProperty ? () => onResetProperty('runtimeDefaultModel') : undefined}
          options={modelOptions.filter((option) => allowedModelIds.includes(option.value))}
          value={modelPolicy.defaultModel}
          editable={modelPolicyEditable}
          clearLabel="Caller choice"
          unavailableReason={catalogs.models.reason}
          onChange={(defaultModel) => {
            rememberedDefault.current = null;
            updateRuntime({ ...modelPolicy, defaultModel, defaultReasoning: null });
          }}
        />
        <SearchableSingleSelect<HarnessReasoningLevel>
          label={`${modelPolicyLabelPrefix} default reasoning`}
          source={provenance?.runtimeDefaultReasoning}
          onReset={onResetProperty ? () => onResetProperty('runtimeDefaultReasoning') : undefined}
          options={defaultReasoningOptions}
          value={modelPolicy.defaultReasoning}
          editable={modelPolicyEditable && Boolean(modelPolicy.defaultModel)}
          clearLabel="Caller choice"
          unavailableReason={catalogs.models.reason}
          onChange={(defaultReasoning) => {
            rememberedDefault.current = null;
            updateRuntime({ ...modelPolicy, defaultReasoning });
          }}
        />
        {modelPolicyNote}
        <h3>Sandbox and authority</h3>
        <SearchableSingleSelect
          label="Sandbox"
          source={provenance?.runtimeSandbox}
          onReset={onResetProperty ? () => onResetProperty('runtimeSandbox') : undefined}
          options={configuration.runtime.sandboxOptions.map((value) => ({
            value,
            label: humanize(value),
          }))}
          value={configuration.runtime.sandbox}
          editable={editable}
          unavailableReason="No sandbox options are available for this Harness."
          onChange={(sandbox) =>
            sandbox &&
            onChange({
              ...configuration,
              runtime: { ...configuration.runtime, sandbox },
            })
          }
        />
        <DefinitionField label="Approval policy">
          <output aria-label="Approval policy">Never</output>
        </DefinitionField>
        <DefinitionField
          label="Authority summary"
          source={provenance?.runtimeAuthoritySummary}
          onReset={onResetProperty ? () => onResetProperty('runtimeAuthoritySummary') : undefined}
        >
          <textarea
            aria-label="Authority summary"
            value={configuration.runtime.authoritySummary}
            disabled={!editable}
            onChange={(event) =>
              onChange({
                ...configuration,
                runtime: {
                  ...configuration.runtime,
                  authoritySummary: event.currentTarget.value,
                },
              })
            }
          />
        </DefinitionField>
      </DefinitionSection>

      <DefinitionSection title="Application hooks">
        <DefinitionField
          label="Hook definitions"
          source={provenance?.hookItems}
          onReset={onResetProperty ? () => onResetProperty('hookItems') : undefined}
        >
          {configuration.hooks.length ? (
            <ul className="harness-management__hook-list">
              {configuration.hooks.map((hook) => (
                <li key={hook.name}>
                  <div>
                    <strong>{hook.name}</strong>
                    <p>{hook.detail}</p>
                  </div>
                  <span>{humanize(hook.status)}</span>
                </li>
              ))}
            </ul>
          ) : (
            <p>No hooks configured.</p>
          )}
          {editable ? (
            <p className="harness-definition-editor__unavailable">
              Hook catalog unavailable. Existing hooks can be inspected but not replaced here.
            </p>
          ) : null}
        </DefinitionField>
      </DefinitionSection>
    </div>
  );
}

function DefinitionSection({
  title,
  children,
}: {
  readonly title: string;
  readonly children: ReactNode;
}) {
  return (
    <section className="harness-management__card">
      <header className="harness-management__card-header">
        <h2>{title}</h2>
      </header>
      {children}
    </section>
  );
}

function DefinitionField({
  label,
  source,
  onReset,
  children,
}: {
  readonly label: string;
  readonly source?: HarnessDefinitionSource;
  onReset?: () => void;
  readonly children: ReactNode;
}) {
  return (
    <div className={`harness-management__field${source ? ` is-${source}` : ''}`}>
      <span className="harness-management__field-label">
        <span>{label}</span>
        {source ? <span className="workflow-field-source">{humanize(source)}</span> : null}
        {source === 'overridden' && onReset ? (
          <button type="button" onClick={onReset}>
            Use inherited value
          </button>
        ) : null}
      </span>
      {children}
    </div>
  );
}

export function SearchableSingleSelect<T extends string>({
  label,
  source,
  onReset,
  options,
  value,
  editable,
  clearLabel,
  unavailableReason,
  onChange,
}: {
  readonly label: string;
  readonly source?: HarnessDefinitionSource;
  onReset?: () => void;
  readonly options: readonly SelectOption<T>[];
  readonly value: T | null;
  readonly editable: boolean;
  readonly clearLabel?: string;
  readonly unavailableReason?: string;
  onChange(value: T | null): void;
}) {
  const id = useId();
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const selected =
    options.find((option) => option.value === value) ??
    (value ? ({ value, label: value } satisfies SelectOption<T>) : undefined);
  const filtered = options.filter((option) => fuzzy(option, query));
  if (!options.length)
    return (
      <DefinitionField label={label} source={source} onReset={onReset}>
        {selected ? <output aria-label={label}>{selected.label}</output> : null}
        <p className="harness-definition-editor__unavailable" role="status">
          {unavailableReason ?? 'Unavailable'}
        </p>
      </DefinitionField>
    );
  if (!editable)
    return (
      <DefinitionField label={label} source={source} onReset={onReset}>
        <output aria-label={label}>{selected?.label ?? clearLabel ?? 'Not selected'}</output>
      </DefinitionField>
    );
  return (
    <DefinitionField label={label} source={source} onReset={onReset}>
      <div
        className="harness-definition-selector"
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget)) {
            setOpen(false);
            setQuery('');
          }
        }}
      >
        <div className="harness-definition-selector__control">
          <Search size={14} aria-hidden="true" />
          <input
            ref={inputRef}
            role="combobox"
            aria-label={label}
            aria-expanded={open}
            aria-controls={id}
            aria-autocomplete="list"
            value={open ? query : (selected?.label ?? clearLabel ?? '')}
            onFocus={() => {
              setQuery('');
              setOpen(true);
            }}
            onClick={() => setOpen(true)}
            onChange={(event) => {
              setQuery(event.currentTarget.value);
              setOpen(true);
            }}
            onKeyDown={(event) => {
              if (event.key === 'Escape') {
                setOpen(false);
                setQuery('');
              }
              if (event.key === 'ArrowDown') {
                event.preventDefault();
                setOpen(true);
                requestAnimationFrame(() => firstOption(listRef.current)?.focus());
              }
            }}
          />
          <ChevronDown size={14} aria-hidden="true" />
        </div>
        {open ? (
          <div
            id={id}
            ref={listRef}
            role="listbox"
            aria-label={`${label} options`}
            onKeyDown={(event) =>
              handleListboxKeyDown(event, listRef.current, () => {
                inputRef.current?.focus();
                setOpen(false);
                setQuery('');
              })
            }
          >
            {clearLabel ? (
              <button
                type="button"
                role="option"
                aria-selected={value === null}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  onChange(null);
                  setOpen(false);
                }}
              >
                {clearLabel}
              </button>
            ) : null}
            {filtered.map((option) => (
              <button
                type="button"
                role="option"
                aria-selected={option.value === value}
                key={option.value}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  onChange(option.value);
                  setOpen(false);
                }}
              >
                <span>
                  <strong>{option.label}</strong>
                  {option.description ? <small>{option.description}</small> : null}
                </span>
                {option.value === value ? <Check size={14} aria-hidden="true" /> : null}
              </button>
            ))}
            {!filtered.length ? <p>No matching options.</p> : null}
          </div>
        ) : null}
      </div>
    </DefinitionField>
  );
}

export function SearchableMultiSelect<T extends string>({
  label,
  source,
  onReset,
  options,
  values,
  editable,
  unavailableReason,
  onChange,
}: {
  readonly label: string;
  readonly source?: HarnessDefinitionSource;
  onReset?: () => void;
  readonly options: readonly SelectOption<T>[];
  readonly values: readonly T[];
  readonly editable: boolean;
  readonly unavailableReason?: string;
  onChange(values: readonly T[]): void;
}) {
  const id = useId();
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const visibleOptions = useMemo(
    () => [
      ...values
        .filter((value) => !options.some((option) => option.value === value))
        .map((value): SelectOption<T> => ({ value, label: value })),
      ...options,
    ],
    [options, values],
  );
  const filtered = useMemo(
    () => visibleOptions.filter((option) => fuzzy(option, query)),
    [query, visibleOptions],
  );
  if (!options.length)
    return (
      <DefinitionField label={label} source={source} onReset={onReset}>
        {values.length ? <output aria-label={label}>{values.join(', ')}</output> : null}
        <p className="harness-definition-editor__unavailable" role="status">
          {unavailableReason ?? 'Unavailable'}
        </p>
      </DefinitionField>
    );
  if (!editable)
    return (
      <DefinitionField label={label} source={source} onReset={onReset}>
        <output aria-label={label}>
          {values
            .map((value) => options.find((item) => item.value === value)?.label ?? value)
            .join(', ') || 'None'}
        </output>
      </DefinitionField>
    );
  return (
    <DefinitionField label={label} source={source} onReset={onReset}>
      <div
        className="harness-definition-selector"
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget)) {
            setOpen(false);
            setQuery('');
          }
        }}
      >
        <div className="harness-definition-selector__control">
          <Search size={14} aria-hidden="true" />
          <input
            ref={inputRef}
            role="combobox"
            aria-label={label}
            aria-expanded={open}
            aria-controls={id}
            aria-autocomplete="list"
            value={query}
            placeholder={values.length ? `${values.length} selected` : 'Search options'}
            onFocus={() => setOpen(true)}
            onClick={() => setOpen(true)}
            onChange={(event) => setQuery(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') {
                setOpen(false);
                setQuery('');
              }
              if (event.key === 'ArrowDown') {
                event.preventDefault();
                setOpen(true);
                requestAnimationFrame(() => firstOption(listRef.current)?.focus());
              }
            }}
          />
          <ChevronDown size={14} aria-hidden="true" />
        </div>
        {open ? (
          <div
            id={id}
            ref={listRef}
            role="listbox"
            aria-label={`${label} options`}
            aria-multiselectable="true"
            onKeyDown={(event) =>
              handleListboxKeyDown(event, listRef.current, () => {
                inputRef.current?.focus();
                setOpen(false);
                setQuery('');
              })
            }
          >
            {filtered.map((option) => {
              const selected = values.includes(option.value);
              return (
                <button
                  type="button"
                  role="option"
                  aria-selected={selected}
                  key={option.value}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() =>
                    onChange(
                      selected
                        ? values.filter((value) => value !== option.value)
                        : [...values, option.value],
                    )
                  }
                >
                  <span>
                    <strong>{option.label}</strong>
                    {option.description ? <small>{option.description}</small> : null}
                  </span>
                  {selected ? <Check size={14} aria-hidden="true" /> : null}
                </button>
              );
            })}
            {!filtered.length ? <p>No matching options.</p> : null}
          </div>
        ) : null}
      </div>
    </DefinitionField>
  );
}

function McpExposureSelector({
  components,
  exposures,
  editable,
  source,
  onReset,
  onChange,
}: {
  readonly components: readonly {
    readonly serverName: string;
    readonly toolName: string;
    readonly title: string;
  }[];
  readonly exposures: readonly HarnessMcpServerExposure[];
  readonly editable: boolean;
  readonly source?: HarnessDefinitionSource;
  onReset?: () => void;
  onChange(exposures: readonly HarnessMcpServerExposure[]): void;
}) {
  const options = components.map((component) => ({
    value: `${component.serverName}\u0000${component.toolName}`,
    label: component.title,
    description: `${component.serverName} / ${component.toolName}`,
  }));
  const serverNames = [...new Set(components.map((component) => component.serverName))];
  const fullOptions = [
    ...serverNames.map((serverName) => ({
      value: `${serverName}\u0000*`,
      label: `${serverName} (entire server)`,
      description: 'Expose every tool offered by this upstream server.',
    })),
    ...options,
  ];
  const values = exposures.flatMap((server) =>
    server.access.kind === 'entire_server'
      ? [`${server.serverName}\u0000*`]
      : server.access.toolNames.map((toolName) => `${server.serverName}\u0000${toolName}`),
  );
  const selectOptions = [
    ...values
      .filter((value) => !fullOptions.some((option) => option.value === value))
      .map((value) => {
        const [serverName, toolName] = value.split('\u0000');
        return {
          value,
          label: toolName === '*' ? `${serverName} (entire server)` : `${serverName} / ${toolName}`,
          description: 'Configured value is no longer present in the current catalog.',
        };
      }),
    ...fullOptions,
  ];
  if (!components.length) {
    return (
      <DefinitionField label="Workflow MCP exposure" source={source} onReset={onReset}>
        {exposures.length ? (
          <output
            aria-label="Workflow MCP exposure"
            className="harness-definition-editor__current-value"
          >
            {exposures
              .map((server) =>
                server.access.kind === 'entire_server'
                  ? `${server.serverName}: entire server`
                  : `${server.serverName}: ${server.access.toolNames.join(', ')}`,
              )
              .join('; ')}
          </output>
        ) : null}
        <p className="harness-definition-editor__unavailable" role="status">
          Workflow MCP component catalog unavailable.
        </p>
      </DefinitionField>
    );
  }
  return (
    <SearchableMultiSelect
      label="Workflow MCP exposure"
      source={source}
      onReset={onReset}
      options={selectOptions}
      values={values}
      editable={editable}
      unavailableReason="Workflow MCP component catalog unavailable."
      onChange={(selected) => {
        const grouped = new Map<string, { entire: boolean; tools: string[] }>();
        for (const value of selected) {
          const [serverName, toolName] = value.split('\u0000');
          if (!serverName || !toolName) continue;
          const current = grouped.get(serverName) ?? { entire: false, tools: [] };
          grouped.set(serverName, {
            entire: current.entire || toolName === '*',
            tools: toolName === '*' ? current.tools : [...current.tools, toolName],
          });
        }
        onChange(
          [...grouped].map(([serverName, selection]) => ({
            serverName,
            access: selection.entire
              ? ({ kind: 'entire_server' } as const)
              : ({ kind: 'selected_tools', toolNames: selection.tools } as const),
          })),
        );
      }}
    />
  );
}

const skillPolicyOptions: readonly SelectOption<HarnessSkillPolicy>[] = [
  { value: 'always_applicable', label: 'Always applicable' },
  { value: 'initial_ingestion', label: 'Initial ingestion only' },
  { value: 'available', label: 'Available' },
];

const toolPolicyOptions: readonly SelectOption<HarnessToolPolicy>[] = [
  { value: 'every_invocation', label: 'Every invocation' },
  { value: 'initial_invocation', label: 'Initial invocation only' },
  { value: 'available', label: 'Available' },
];

const discoveryPolicyOptions = [
  { value: 'whitelist', label: 'Whitelist' },
  { value: 'blacklist', label: 'Blacklist' },
] as const;

function firstOption(list: HTMLDivElement | null): HTMLButtonElement | null {
  return list?.querySelector<HTMLButtonElement>('button[role="option"]') ?? null;
}

function handleListboxKeyDown(
  event: KeyboardEvent<HTMLDivElement>,
  list: HTMLDivElement | null,
  close: () => void,
): void {
  if (event.key === 'Escape') {
    event.preventDefault();
    close();
    return;
  }
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
  const options = [...(list?.querySelectorAll<HTMLButtonElement>('button[role="option"]') ?? [])];
  if (!options.length) return;
  event.preventDefault();
  const current = options.indexOf(document.activeElement as HTMLButtonElement);
  const next =
    event.key === 'Home'
      ? 0
      : event.key === 'End'
        ? options.length - 1
        : event.key === 'ArrowUp'
          ? current <= 0
            ? options.length - 1
            : current - 1
          : current < 0 || current === options.length - 1
            ? 0
            : current + 1;
  options[next]?.focus();
}

function fuzzy(option: SelectOption, query: string): boolean {
  const needle = query.trim().toLowerCase();
  return !needle || `${option.label} ${option.description ?? ''}`.toLowerCase().includes(needle);
}

function reconcileProvisionalDefault(
  policy: Pick<
    HarnessEffectiveConfiguration['runtime'],
    'models' | 'defaultModel' | 'defaultReasoning'
  >,
  models: HarnessEffectiveConfiguration['runtime']['models'],
  catalog: HarnessConfigurationCatalogs['models']['items'],
  remembered: {
    current: {
      readonly model: string;
      readonly reasoning: HarnessReasoningLevel | null;
    } | null;
  },
): Pick<HarnessEffectiveConfiguration['runtime'], 'models' | 'defaultModel' | 'defaultReasoning'> {
  let next = { ...policy, models };
  if (
    remembered.current &&
    isValidPolicyChoice(remembered.current.model, remembered.current.reasoning, next, catalog)
  ) {
    next = {
      ...next,
      defaultModel: remembered.current.model,
      defaultReasoning: remembered.current.reasoning,
    };
    remembered.current = null;
    return next;
  }
  if (
    !policy.defaultModel ||
    isValidPolicyChoice(policy.defaultModel, policy.defaultReasoning, next, catalog)
  )
    return next;
  remembered.current ??= {
    model: policy.defaultModel,
    reasoning: policy.defaultReasoning,
  };
  const sameModel = models.find((model) => model.modelId === policy.defaultModel && model.allowed);
  if (sameModel) {
    const levels = catalog.find((model) => model.id === sameModel.modelId)?.reasoningLevels ?? [];
    const selectedIndex = levels.indexOf(policy.defaultReasoning ?? sameModel.minReasoning);
    const minIndex = levels.indexOf(sameModel.minReasoning);
    const maxIndex = levels.indexOf(sameModel.maxReasoning);
    const fallbackIndex = Math.max(minIndex, Math.min(selectedIndex, maxIndex));
    return {
      ...next,
      defaultModel: sameModel.modelId,
      defaultReasoning: levels[fallbackIndex] ?? sameModel.minReasoning,
    };
  }
  const fallback = models.find((model) => model.allowed);
  return {
    ...next,
    defaultModel: fallback?.modelId ?? null,
    defaultReasoning: fallback?.minReasoning ?? null,
  };
}

function isValidPolicyChoice(
  modelId: string,
  reasoning: HarnessReasoningLevel | null,
  policy: Pick<
    HarnessEffectiveConfiguration['runtime'],
    'models' | 'defaultModel' | 'defaultReasoning'
  >,
  catalog: HarnessConfigurationCatalogs['models']['items'],
): boolean {
  const model = policy.models.find((candidate) => candidate.modelId === modelId);
  if (!model?.allowed) return false;
  if (reasoning === null) return true;
  const levels = catalog.find((candidate) => candidate.id === modelId)?.reasoningLevels ?? [];
  const selected = levels.indexOf(reasoning);
  return (
    selected >= levels.indexOf(model.minReasoning) && selected <= levels.indexOf(model.maxReasoning)
  );
}

function reasoningWithin(
  value: HarnessReasoningLevel,
  model: HarnessEffectiveConfiguration['runtime']['models'][number],
): boolean {
  const levels: readonly HarnessReasoningLevel[] = ['low', 'medium', 'high', 'xhigh'];
  const ordinal = levels.indexOf(value);
  return (
    ordinal >= levels.indexOf(model.minReasoning) && ordinal <= levels.indexOf(model.maxReasoning)
  );
}

function humanize(value: string): string {
  return value.replaceAll('_', ' ').replace(/^./, (character) => character.toUpperCase());
}
