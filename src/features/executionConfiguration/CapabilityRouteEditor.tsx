import { ChevronDown, ChevronRight } from 'lucide-react';
import { useEffect, useState } from 'react';
import type {
  ModelAllowanceDto,
  ProfileModelCatalogueDto,
  ProfileRoutePolicyDto,
} from '../../application/executionConfiguration';
import { CapabilityModelPickerDialog } from './CapabilityModelPickerDialog';
import { CapabilityProfileEditorMemory } from './CapabilityProfileEditorMemory';
import type { HarnessInferenceRouteOption } from './types';
import { CodexPersonalityField } from '../agentProviders/codex/CodexPersonalityField';

export const REASONING_ORDER = [
  'none',
  'minimal',
  'low',
  'medium',
  'high',
  'xhigh',
  'max',
  'ultra',
];
export const MODEL_ORDER = [
  'gpt-6-astra',
  'gpt-5.6-sol',
  'gpt-5.6-terra',
  'gpt-5.6-luna',
  'gpt-5.5',
];

export function byKnownOrder(
  values: readonly string[],
  knownOrder: readonly string[],
): readonly string[] {
  const rank = (value: string) => {
    const index = knownOrder.indexOf(value.toLowerCase());
    return index === -1 ? knownOrder.length : index;
  };
  return [...new Set(values)].sort(
    (left, right) => rank(left) - rank(right) || left.localeCompare(right),
  );
}

export interface CapabilityGroupChoice {
  readonly id: string;
  readonly label: string;
  readonly detail: string;
}

interface CapabilityRouteEditorProps {
  readonly route: ProfileRoutePolicyDto;
  readonly routeInfo?: HarnessInferenceRouteOption;
  readonly isDefault: boolean;
  readonly profileKey: string;
  readonly editorMemory: CapabilityProfileEditorMemory;
  readonly models: readonly string[];
  readonly reasoning: readonly string[];
  readonly modelCatalogue?: ProfileModelCatalogueDto;
  readonly routeCatalogueEnabled: boolean;
  readonly mcpGroups: readonly CapabilityGroupChoice[];
  readonly skillGroups: readonly CapabilityGroupChoice[];
  onChange(route: ProfileRoutePolicyDto): void;
  onSetDefault(): void;
  onRemove(): void;
}

export function CapabilityRouteEditor({
  route,
  routeInfo,
  isDefault,
  profileKey,
  editorMemory,
  models,
  reasoning,
  modelCatalogue,
  routeCatalogueEnabled,
  mcpGroups,
  skillGroups,
  onChange,
  onSetDefault,
  onRemove,
}: CapabilityRouteEditorProps) {
  const [expanded, setExpanded] = useState(false);

  useEffect(() => setExpanded(false), [profileKey, route.routeId]);

  return (
    <article className={`capability-route${isDefault ? ' is-default' : ''}`}>
      <header>
        <button
          type="button"
          className="capability-route__toggle"
          aria-expanded={expanded}
          onClick={() => setExpanded((current) => !current)}
        >
          {expanded ? (
            <ChevronDown size={16} aria-hidden="true" />
          ) : (
            <ChevronRight size={16} aria-hidden="true" />
          )}
          <span className="capability-route__summary">
            <span>{routeInfo?.deviceLabel ?? route.execution.deviceName}</span>
            <strong>{routeInfo?.harnessLabel ?? 'Configured harness'}</strong>
            <small>
              {routeInfo?.inferenceLabel ?? routeInfo?.sourceLabel ?? 'Configured inference source'}
            </small>
          </span>
        </button>
        <div className="capability-route__actions">
          {isDefault ? (
            <span className="capability-route__default">Default</span>
          ) : (
            <button type="button" className="capability-route__hover-action" onClick={onSetDefault}>
              Set as default
            </button>
          )}
          <button
            type="button"
            className="capability-route__remove"
            aria-label={`Remove ${routeInfo?.label ?? 'execution route'}`}
            onClick={onRemove}
          >
            Remove
          </button>
        </div>
      </header>
      {expanded ? (
        <>
          <p className="capability-route__detail">
            {routeInfo?.detail ?? 'The saved route is no longer in the local route catalogue.'}
          </p>
          <RouteCapabilities
            route={route}
            profileKey={profileKey}
            editorMemory={editorMemory}
            models={models}
            reasoning={reasoning}
            modelCatalogue={modelCatalogue}
            routeCatalogueEnabled={routeCatalogueEnabled}
            mcpGroups={mcpGroups}
            skillGroups={skillGroups}
            onChange={onChange}
          />
        </>
      ) : null}
    </article>
  );
}

interface RouteCapabilitiesProps {
  readonly route: ProfileRoutePolicyDto;
  readonly profileKey: string;
  readonly editorMemory: CapabilityProfileEditorMemory;
  readonly models: readonly string[];
  readonly reasoning: readonly string[];
  readonly modelCatalogue?: ProfileModelCatalogueDto;
  readonly routeCatalogueEnabled: boolean;
  readonly mcpGroups: readonly CapabilityGroupChoice[];
  readonly skillGroups: readonly CapabilityGroupChoice[];
  onChange(route: ProfileRoutePolicyDto): void;
}

function RouteCapabilities({
  route,
  profileKey,
  editorMemory,
  models,
  reasoning,
  modelCatalogue,
  routeCatalogueEnabled,
  mcpGroups,
  skillGroups,
  onChange,
}: RouteCapabilitiesProps) {
  const [addingModel, setAddingModel] = useState(false);
  const reasoningFor = (modelId: string, allowance?: ModelAllowanceDto): readonly string[] => {
    const observed = modelCatalogue?.models
      .find((model) => model.id === modelId)
      ?.reasoningModes.map((mode) => mode.id);
    return byKnownOrder(
      [
        ...(routeCatalogueEnabled ? (observed ?? []) : reasoning),
        ...(allowance ? [allowance.minimumReasoning, allowance.maximumReasoning] : []),
      ],
      REASONING_ORDER,
    );
  };
  const modelChoices = byKnownOrder(
    [...models, ...route.modelAllowances.map((entry) => entry.modelId)],
    MODEL_ORDER,
  );
  const updateAllowance = (modelId: string, update: Partial<ModelAllowanceDto>) =>
    onChange({
      ...route,
      modelAllowances: route.modelAllowances.map((allowance) =>
        allowance.modelId === modelId ? { ...allowance, ...update } : allowance,
      ),
    });
  const toggleGroup = (kind: 'mcpGroups' | 'skillGroups', id: string) => {
    const selected = route[kind];
    onChange({
      ...route,
      [kind]: selected.includes(id)
        ? selected.filter((candidate) => candidate !== id)
        : [...selected, id],
    });
  };
  const removeModel = (modelId: string) => {
    const allowance = route.modelAllowances.find((entry) => entry.modelId === modelId);
    if (allowance) editorMemory.remember(profileKey, route.routeId, allowance);
    onChange({
      ...route,
      modelAllowances: route.modelAllowances.filter((entry) => entry.modelId !== modelId),
    });
  };
  const addModel = (modelId: string) => {
    if (route.modelAllowances.some((entry) => entry.modelId === modelId)) return;
    const remembered = editorMemory.recall(profileKey, route.routeId, modelId);
    const modelReasoning = reasoningFor(modelId, remembered);
    const lowest = remembered?.minimumReasoning ?? modelReasoning[0];
    const highest = remembered?.maximumReasoning ?? modelReasoning.at(-1) ?? lowest;
    if (!lowest || !highest) return;
    onChange({
      ...route,
      modelAllowances: [
        ...route.modelAllowances,
        remembered ?? { modelId, minimumReasoning: lowest, maximumReasoning: highest },
      ],
    });
  };
  return (
    <div className="capability-route__configuration">
      <CodexPersonalityField route={route} onChange={onChange} />
      <section aria-labelledby={`${route.routeId}-models`}>
        <div className="capability-route__section-heading">
          <div>
            <h4 id={`${route.routeId}-models`}>Models and reasoning</h4>
            <p>
              Record model-specific reasoning intent for later workflow policy. These ranges do not
              restrict sessions yet.
            </p>
          </div>
          <button
            type="button"
            disabled={modelChoices.length === 0}
            onClick={() => setAddingModel(true)}
          >
            Add model
          </button>
        </div>
        {routeCatalogueEnabled ? (
          <p className="capability-route__detail">
            {modelCatalogue?.observationError
              ? `${modelCatalogue.observedAt ? `Using model options observed ${new Date(modelCatalogue.observedAt).toLocaleString()}. ` : 'No model options have been observed yet. '}Current discovery failed: ${modelCatalogue.observationError}`
              : modelCatalogue?.observedAt
                ? `Model options observed ${new Date(modelCatalogue.observedAt).toLocaleString()}; availability is checked when a session starts.`
                : 'Loading model options; you can save this route without choosing a model.'}
          </p>
        ) : null}
        {route.modelAllowances.length === 0 ? (
          <p className="capability-route__empty">
            No model preferences are recorded for this route.
          </p>
        ) : null}
        {byKnownOrder(
          route.modelAllowances.map((entry) => entry.modelId),
          MODEL_ORDER,
        ).map((modelId) => {
          const allowance = route.modelAllowances.find((entry) => entry.modelId === modelId)!;
          const modelReasoning = reasoningFor(modelId, allowance);
          const minimumIndex = Math.max(0, modelReasoning.indexOf(allowance.minimumReasoning));
          const maximumIndex = Math.max(
            minimumIndex,
            modelReasoning.indexOf(allowance.maximumReasoning),
          );
          return (
            <div className="model-allowance" key={modelId}>
              <strong>{modelId}</strong>
              <label>
                <span>Lowest reasoning</span>
                <select
                  aria-label={`${modelId} lowest reasoning`}
                  value={allowance.minimumReasoning}
                  onChange={(event) => {
                    const nextMinimum = event.currentTarget.value;
                    const nextIndex = modelReasoning.indexOf(nextMinimum);
                    updateAllowance(modelId, {
                      minimumReasoning: nextMinimum,
                      maximumReasoning:
                        nextIndex > maximumIndex ? nextMinimum : allowance.maximumReasoning,
                    });
                  }}
                >
                  {modelReasoning.map((value) => (
                    <option key={value} value={value}>
                      {value}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>Highest reasoning</span>
                <select
                  aria-label={`${modelId} highest reasoning`}
                  value={allowance.maximumReasoning}
                  onChange={(event) => {
                    const nextMaximum = event.currentTarget.value;
                    const nextIndex = modelReasoning.indexOf(nextMaximum);
                    updateAllowance(modelId, {
                      minimumReasoning:
                        nextIndex < minimumIndex ? nextMaximum : allowance.minimumReasoning,
                      maximumReasoning: nextMaximum,
                    });
                  }}
                >
                  {modelReasoning.map((value) => (
                    <option key={value} value={value}>
                      {value}
                    </option>
                  ))}
                </select>
              </label>
              <button
                type="button"
                className="model-allowance__remove"
                aria-label={`Remove ${modelId}`}
                onClick={() => removeModel(modelId)}
              >
                Remove
              </button>
            </div>
          );
        })}
      </section>

      <GroupToggles
        id={`${route.routeId}-mcp-groups`}
        title="MCP tools"
        description="Enable broad MCP groups. Orchid filters brokered tool calls again when the session starts."
        groups={mcpGroups}
        selected={route.mcpGroups}
        onToggle={(id) => toggleGroup('mcpGroups', id)}
      />
      <GroupToggles
        id={`${route.routeId}-skill-groups`}
        title="Skills"
        description="Enable skill roots. Orchid compiles the selected session skill manifest at launch."
        groups={skillGroups}
        selected={route.skillGroups}
        onToggle={(id) => toggleGroup('skillGroups', id)}
      />

      {addingModel ? (
        <CapabilityModelPickerDialog
          models={modelChoices}
          observed={models}
          selected={route.modelAllowances.map((entry) => entry.modelId)}
          reasoningByModel={Object.fromEntries(
            modelChoices.map((model) => [model, reasoningFor(model)]),
          )}
          onAdd={addModel}
          onRemove={removeModel}
          onClose={() => setAddingModel(false)}
        />
      ) : null}
    </div>
  );
}

function GroupToggles({
  id,
  title,
  description,
  groups,
  selected,
  onToggle,
}: {
  readonly id: string;
  readonly title: string;
  readonly description: string;
  readonly groups: readonly CapabilityGroupChoice[];
  readonly selected: readonly string[];
  onToggle(id: string): void;
}) {
  return (
    <section className="capability-groups" aria-labelledby={id}>
      <div>
        <h4 id={id}>{title}</h4>
        <p>{description}</p>
      </div>
      <div className="capability-groups__list">
        {groups.map((group) => (
          <label key={group.id}>
            <input
              type="checkbox"
              checked={selected.includes(group.id)}
              onChange={() => onToggle(group.id)}
            />
            <span>
              <strong>{group.label}</strong>
              <small>{group.detail}</small>
            </span>
          </label>
        ))}
      </div>
    </section>
  );
}
