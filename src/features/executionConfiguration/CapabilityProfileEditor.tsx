import { useMemo, useState, type ReactNode } from 'react';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { ValidationSummary } from '../../components/ValidationSummary';
import type {
  ModelAllowanceDto,
  ProfileModelCatalogueDto,
  ProfileRoutePolicyDto,
  RuntimeSelectionsDto,
} from '../../application/executionConfiguration';
import type {
  CapabilityProfileDraft,
  HarnessInferenceRouteOption,
  RuntimeProfileViewModel,
} from './types';
import './executionConfiguration.css';

export interface CapabilityProfileEditorProps {
  readonly profile: CapabilityProfileDraft;
  readonly runtime: RuntimeProfileViewModel;
  readonly routes?: readonly HarnessInferenceRouteOption[];
  readonly modelCatalogues?: Readonly<Record<string, ProfileModelCatalogueDto>>;
  readonly validationErrors?: readonly string[];
  readonly saving?: boolean;
  onChange(profile: CapabilityProfileDraft): void;
  onSave?(profile: CapabilityProfileDraft): void;
}

const REASONING_ORDER = ['none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'];
const MODEL_ORDER = ['gpt-6-astra', 'gpt-5.6-sol', 'gpt-5.6-terra', 'gpt-5.6-luna', 'gpt-5.5'];

function byKnownOrder(values: readonly string[], knownOrder: readonly string[]): readonly string[] {
  const rank = (value: string) => {
    const index = knownOrder.indexOf(value.toLowerCase());
    return index === -1 ? knownOrder.length : index;
  };
  return [...new Set(values)].sort(
    (left, right) => rank(left) - rank(right) || left.localeCompare(right),
  );
}

function sameExecution(
  left: ProfileRoutePolicyDto['execution'],
  right: ProfileRoutePolicyDto['execution'],
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function rangeValues(
  allowance: ModelAllowanceDto,
  reasoning: readonly string[],
): readonly string[] {
  const first = reasoning.indexOf(allowance.minimumReasoning);
  const last = reasoning.indexOf(allowance.maximumReasoning);
  return first < 0 || last < first ? [] : reasoning.slice(first, last + 1);
}

function synchronizeLegacyContract(
  profile: CapabilityProfileDraft,
  routePolicies: readonly ProfileRoutePolicyDto[],
  defaultRouteId: string | null,
): CapabilityProfileDraft {
  const selected = routePolicies.find((route) => route.routeId === defaultRouteId);
  const allowances = routePolicies.flatMap((route) => route.modelAllowances);
  const models = byKnownOrder(
    allowances.map((allowance) => allowance.modelId),
    MODEL_ORDER,
  );
  const reasoningModes = byKnownOrder(
    allowances.flatMap((allowance) => rangeValues(allowance, REASONING_ORDER)),
    REASONING_ORDER,
  );
  return {
    ...profile,
    routePolicies,
    defaultRouteId,
    execution: selected?.execution ?? profile.execution,
    defaults: selected?.defaults ?? profile.defaults ?? emptyDefaults(),
    allowedCapabilities: { ...profile.allowedCapabilities, models, reasoningModes },
  };
}

function emptyDefaults(): RuntimeSelectionsDto {
  // Sandboxing is deliberately not a Capability Profile choice in this interim slice.
  return { model: null, reasoningMode: null, sandboxMode: null };
}

function groupChoices(runtime: RuntimeProfileViewModel, kind: 'mcp' | 'skill') {
  const otp = runtime.catalogs.otpPackages ?? [];
  const root =
    kind === 'mcp'
      ? [
          {
            id: 'codex-profile-mcps',
            label: 'Codex profile MCP tools',
            detail: 'MCP servers configured by this Codex profile.',
          },
        ]
      : [
          {
            id: 'codex-profile-skills',
            label: 'Skills discovered by Codex',
            detail: 'Skills registered by the selected Codex profile.',
          },
          {
            id: 'orchid-skills',
            label: 'Orchid skills',
            detail: 'Product-owned skills delivered by Orchid.',
          },
        ];
  return [
    ...root,
    ...otp
      .filter((packageInfo) =>
        kind === 'mcp'
          ? packageInfo.tools.some((tool) => tool.entrypoint.kind === 'mcp') ||
            packageInfo.agentMcpServers.length > 0
          : (packageInfo.skillRoots?.length ?? 0) > 0,
      )
      .map((packageInfo) => ({
        id: `otp:${packageInfo.id}:${kind === 'mcp' ? 'mcps' : 'skills'}`,
        label: `${packageInfo.name} ${kind === 'mcp' ? 'MCP tools' : 'skills'}`,
        detail: `Provided by the ${packageInfo.name} OTP package.`,
      })),
  ];
}

/** Controlled editor for reusable Device -> Harness -> Inference capability routes. */
export function CapabilityProfileEditor({
  profile,
  runtime,
  routes = [],
  modelCatalogues,
  validationErrors = [],
  saving = false,
  onChange,
  onSave,
}: CapabilityProfileEditorProps) {
  const [addingRoute, setAddingRoute] = useState(false);
  const existing = profile.revision !== null;
  const modelOptions = useMemo(
    () =>
      byKnownOrder(
        runtime.catalogs.models.options.map((option) => option.value),
        MODEL_ORDER,
      ),
    [runtime.catalogs.models.options],
  );
  const reasoningOptions = useMemo(
    () =>
      byKnownOrder(
        runtime.catalogs.reasoningModes.options.map((option) => option.value),
        REASONING_ORDER,
      ),
    [runtime.catalogs.reasoningModes.options],
  );
  const mcpGroups = useMemo(() => groupChoices(runtime, 'mcp'), [runtime]);
  const skillGroups = useMemo(() => groupChoices(runtime, 'skill'), [runtime]);

  const changeRoutes = (
    routePolicies: readonly ProfileRoutePolicyDto[],
    defaultRouteId: string | null = profile.defaultRouteId,
  ) => onChange(synchronizeLegacyContract(profile, routePolicies, defaultRouteId));

  const addRoute = (route: HarnessInferenceRouteOption) => {
    const policy: ProfileRoutePolicyDto = {
      routeId: `route-${Date.now()}-${profile.routePolicies.length + 1}`,
      execution: route.execution,
      modelAllowances: [],
      mcpGroups: [],
      skillGroups: [],
      defaults: emptyDefaults(),
    };
    const next = [...profile.routePolicies, policy];
    changeRoutes(next, profile.defaultRouteId ?? policy.routeId);
    setAddingRoute(false);
  };

  const updateRoute = (nextRoute: ProfileRoutePolicyDto) =>
    changeRoutes(
      profile.routePolicies.map((route) =>
        route.routeId === nextRoute.routeId ? nextRoute : route,
      ),
    );

  const removeRoute = (routeId: string) => {
    const next = profile.routePolicies.filter((route) => route.routeId !== routeId);
    changeRoutes(
      next,
      profile.defaultRouteId === routeId ? (next[0]?.routeId ?? null) : profile.defaultRouteId,
    );
  };

  return (
    <div className="execution-configuration" data-testid="capability-profile-editor">
      <header className="execution-configuration__header">
        <div>
          <span>Capability profile</span>
          <h1>{existing ? profile.name : 'New capability profile'}</h1>
          <p>
            Choose the device, harness, inference source and capability groups available to a
            session. Devices are configured separately in Technical Settings.
          </p>
        </div>
        {existing ? (
          <span className="execution-configuration__badge">Revision {profile.revision}</span>
        ) : null}
      </header>

      <ValidationSummary errors={validationErrors} />

      <CollapsibleSection
        title="Profile details"
        description="A name for this reusable session capability policy. Orchid creates its internal ID when you save."
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
      </CollapsibleSection>

      <CollapsibleSection
        title="Execution routes"
        description="Each route connects one configured device, harness and inference source. The default route is used unless a future node narrows the choice."
        className="execution-configuration__section"
      >
        <div className="capability-routes">
          {profile.routePolicies.map((route) => {
            const routeInfo = routes.find((candidate) =>
              sameExecution(candidate.execution, route.execution),
            );
            const isDefault = route.routeId === profile.defaultRouteId;
            return (
              <article
                className={`capability-route${isDefault ? ' is-default' : ''}`}
                key={route.routeId}
              >
                <header>
                  <div>
                    <span>{routeInfo?.deviceLabel ?? route.execution.deviceName}</span>
                    <h3>{routeInfo?.harnessLabel ?? 'Configured harness'}</h3>
                    <p>
                      {routeInfo?.inferenceLabel ??
                        routeInfo?.sourceLabel ??
                        'Configured inference source'}
                    </p>
                  </div>
                  <div className="capability-route__actions">
                    {isDefault ? (
                      <span className="capability-route__default">Default</span>
                    ) : (
                      <button
                        type="button"
                        className="capability-route__hover-action"
                        onClick={() => changeRoutes(profile.routePolicies, route.routeId)}
                      >
                        Set as default
                      </button>
                    )}
                    <button
                      type="button"
                      className="capability-route__remove"
                      aria-label={`Remove ${routeInfo?.label ?? 'execution route'}`}
                      onClick={() => removeRoute(route.routeId)}
                    >
                      Remove
                    </button>
                  </div>
                </header>
                <p className="capability-route__detail">
                  {routeInfo?.detail ??
                    'The saved route is no longer in the local route catalogue.'}
                </p>
                <RouteCapabilities
                  route={route}
                  models={
                    modelCatalogues
                      ? byKnownOrder(
                          (modelCatalogues[route.execution.configurationRef]?.models ?? []).map(
                            (model) => model.id,
                          ),
                          MODEL_ORDER,
                        )
                      : modelOptions
                  }
                  reasoning={reasoningOptions}
                  modelCatalogue={modelCatalogues?.[route.execution.configurationRef]}
                  routeCatalogueEnabled={modelCatalogues !== undefined}
                  mcpGroups={mcpGroups}
                  skillGroups={skillGroups}
                  onChange={updateRoute}
                />
              </article>
            );
          })}
          {profile.routePolicies.length === 0 ? (
            <p className="execution-configuration__route-empty">
              Add a route to choose where sessions using this profile run.
            </p>
          ) : null}
          <button
            type="button"
            className="capability-routes__add"
            onClick={() => setAddingRoute(true)}
          >
            Add execution route
          </button>
        </div>
      </CollapsibleSection>

      {onSave ? (
        <footer className="execution-configuration__actions">
          <button
            type="button"
            className="is-primary"
            disabled={
              saving ||
              validationErrors.length > 0 ||
              !profile.name.trim() ||
              profile.routePolicies.length === 0
            }
            onClick={() => onSave(profile)}
          >
            {saving ? 'Saving…' : existing ? 'Save new revision' : 'Create profile'}
          </button>
        </footer>
      ) : null}

      {addingRoute ? (
        <AddRouteDialog routes={routes} onAdd={addRoute} onClose={() => setAddingRoute(false)} />
      ) : null}
    </div>
  );
}

interface RouteCapabilitiesProps {
  readonly route: ProfileRoutePolicyDto;
  readonly models: readonly string[];
  readonly reasoning: readonly string[];
  readonly modelCatalogue?: ProfileModelCatalogueDto;
  readonly routeCatalogueEnabled: boolean;
  readonly mcpGroups: readonly GroupChoice[];
  readonly skillGroups: readonly GroupChoice[];
  onChange(route: ProfileRoutePolicyDto): void;
}

interface GroupChoice {
  readonly id: string;
  readonly label: string;
  readonly detail: string;
}

function RouteCapabilities({
  route,
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
  const availableModels = models.filter(
    (model) => !route.modelAllowances.some((entry) => entry.modelId === model),
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
  return (
    <div className="capability-route__configuration">
      <section aria-labelledby={`${route.routeId}-models`}>
        <div className="capability-route__section-heading">
          <div>
            <h4 id={`${route.routeId}-models`}>Models and reasoning</h4>
            <p>Record model-specific reasoning intent for later workflow policy. These ranges do not restrict sessions yet.</p>
          </div>
          <button
            type="button"
            disabled={availableModels.length === 0}
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
          <p className="capability-route__empty">No model preferences are recorded for this route.</p>
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
                onClick={() =>
                  onChange({
                    ...route,
                    modelAllowances: route.modelAllowances.filter(
                      (entry) => entry.modelId !== modelId,
                    ),
                  })
                }
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
        <AddModelDialog
          models={availableModels}
          reasoningByModel={Object.fromEntries(
            availableModels.map((model) => [model, reasoningFor(model)]),
          )}
          onAdd={(modelId) => {
            const modelReasoning = reasoningFor(modelId);
            const lowest = modelReasoning[0];
            const highest = modelReasoning.at(-1) ?? lowest;
            if (lowest && highest) {
              onChange({
                ...route,
                modelAllowances: [
                  ...route.modelAllowances,
                  { modelId, minimumReasoning: lowest, maximumReasoning: highest },
                ],
              });
            }
            setAddingModel(false);
          }}
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
  id: string;
  title: string;
  description: string;
  groups: readonly GroupChoice[];
  selected: readonly string[];
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

function AddModelDialog({
  models,
  reasoningByModel,
  onAdd,
  onClose,
}: {
  models: readonly string[];
  reasoningByModel: Readonly<Record<string, readonly string[]>>;
  onAdd(model: string): void;
  onClose(): void;
}) {
  return (
    <Dialog title="Add model" onClose={onClose}>
      <p>
        Choose from the latest observed options for this route. Current availability is checked when
        a session starts.
      </p>
      {models.length === 0 ? <p>No additional models are available.</p> : null}
      <ul className="capability-dialog__list">
        {models.map((model) => (
          <li key={model}>
            <span>{model}</span>
            <button
              type="button"
              onClick={() => onAdd(model)}
              disabled={!reasoningByModel[model]?.length}
            >
              Add
            </button>
          </li>
        ))}
      </ul>
    </Dialog>
  );
}

function AddRouteDialog({
  routes,
  onAdd,
  onClose,
}: {
  routes: readonly HarnessInferenceRouteOption[];
  onAdd(route: HarnessInferenceRouteOption): void;
  onClose(): void;
}) {
  const [routeId, setRouteId] = useState(routes[0]?.id ?? '');
  const route = routes.find((candidate) => candidate.id === routeId);
  const devices = [
    ...new Set(routes.map((candidate) => candidate.deviceLabel ?? candidate.execution.deviceName)),
  ];
  const selectedDevice = route?.deviceLabel ?? route?.execution.deviceName ?? devices[0] ?? '';
  const matchingDeviceRoutes = routes.filter(
    (candidate) => (candidate.deviceLabel ?? candidate.execution.deviceName) === selectedDevice,
  );
  return (
    <Dialog title="Add execution route" onClose={onClose}>
      <p>Choose the configured device, harness and inference source for this profile.</p>
      {routes.length === 0 ? (
        <p>
          No active device routes are available yet. Configure a device connection in Technical
          Settings first.
        </p>
      ) : (
        <>
          <label className="execution-configuration__field">
            <span>Device</span>
            <select
              aria-label="Device"
              value={selectedDevice}
              onChange={(event) =>
                setRouteId(
                  routes.find(
                    (candidate) =>
                      (candidate.deviceLabel ?? candidate.execution.deviceName) ===
                      event.currentTarget.value,
                  )?.id ?? '',
                )
              }
            >
              {devices.map((device) => (
                <option key={device}>{device}</option>
              ))}
            </select>
          </label>
          <label className="execution-configuration__field">
            <span>Harness</span>
            <select
              aria-label="Harness"
              value={routeId}
              onChange={(event) => setRouteId(event.currentTarget.value)}
            >
              {matchingDeviceRoutes.map((candidate) => (
                <option key={candidate.id} value={candidate.id}>
                  {candidate.harnessLabel ?? candidate.label}
                </option>
              ))}
            </select>
          </label>
          <label className="execution-configuration__field">
            <span>Inference source</span>
            <output>{route?.inferenceLabel ?? route?.sourceLabel ?? 'Choose a harness'}</output>
          </label>
          <footer className="capability-dialog__actions">
            <button type="button" onClick={onClose}>
              Cancel
            </button>
            <button
              type="button"
              className="is-primary"
              disabled={!route}
              onClick={() => route && onAdd(route)}
            >
              Add route
            </button>
          </footer>
        </>
      )}
    </Dialog>
  );
}

function Dialog({
  title,
  children,
  onClose,
}: {
  title: string;
  children: ReactNode;
  onClose(): void;
}) {
  return (
    <div className="capability-dialog__backdrop" role="presentation">
      <section className="capability-dialog" role="dialog" aria-modal="true" aria-label={title}>
        <header>
          <h2>{title}</h2>
          <button type="button" aria-label={`Close ${title}`} onClick={onClose}>
            ×
          </button>
        </header>
        {children}
      </section>
    </div>
  );
}
