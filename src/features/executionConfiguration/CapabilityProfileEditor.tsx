import { useMemo, useState } from 'react';
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
import { executionRouteKey } from '../../application/executionTargets/contracts';
import { CapabilityProfileDialog } from './CapabilityProfileDialog';
import { CapabilityProfileEditorMemory } from './CapabilityProfileEditorMemory';
import {
  byKnownOrder,
  CapabilityRouteEditor,
  MODEL_ORDER,
  REASONING_ORDER,
} from './CapabilityRouteEditor';
import './executionConfiguration.css';

export interface CapabilityProfileEditorProps {
  readonly profile: CapabilityProfileDraft;
  readonly profileKey?: string;
  readonly editorMemory?: CapabilityProfileEditorMemory;
  readonly runtime: RuntimeProfileViewModel;
  readonly routes?: readonly HarnessInferenceRouteOption[];
  readonly modelCatalogues?: Readonly<Record<string, ProfileModelCatalogueDto>>;
  readonly validationErrors?: readonly string[];
  readonly saving?: boolean;
  onChange(profile: CapabilityProfileDraft): void;
  onSave?(profile: CapabilityProfileDraft): void;
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
  if (!allowance.minimumReasoning || !allowance.maximumReasoning) return [];
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
            id: 'native-mcps',
            label: 'Provider configuration MCP tools',
            detail: 'MCP servers exposed by the selected provider configuration.',
          },
        ]
      : [
          {
            id: 'native-skills',
            label: 'Provider-discovered skills',
            detail: 'Skills registered by the selected provider configuration.',
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
  profileKey,
  editorMemory,
  runtime,
  routes = [],
  modelCatalogues,
  validationErrors = [],
  saving = false,
  onChange,
  onSave,
}: CapabilityProfileEditorProps) {
  const [localEditorMemory] = useState(() => new CapabilityProfileEditorMemory());
  const memory = editorMemory ?? localEditorMemory;
  const memoryKey = profileKey ?? profile.capabilityProfileId ?? '$new';
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
              <CapabilityRouteEditor
                key={route.routeId}
                route={route}
                routeInfo={routeInfo}
                isDefault={isDefault}
                profileKey={memoryKey}
                editorMemory={memory}
                models={
                  modelCatalogues
                    ? byKnownOrder(
                        (modelCatalogues[executionRouteKey(route.execution)]?.models ?? []).map(
                          (model) => model.id,
                        ),
                        MODEL_ORDER,
                      )
                    : modelOptions
                }
                reasoning={reasoningOptions}
                modelCatalogue={modelCatalogues?.[executionRouteKey(route.execution)]}
                routeCatalogueEnabled={modelCatalogues !== undefined}
                mcpGroups={mcpGroups}
                skillGroups={skillGroups}
                onChange={updateRoute}
                onSetDefault={() => changeRoutes(profile.routePolicies, route.routeId)}
                onRemove={() => removeRoute(route.routeId)}
              />
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
    <CapabilityProfileDialog title="Add execution route" onClose={onClose}>
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
    </CapabilityProfileDialog>
  );
}
