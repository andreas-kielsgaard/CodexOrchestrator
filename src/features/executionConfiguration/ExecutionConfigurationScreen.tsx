import { Plus, RefreshCw, Trash2 } from 'lucide-react';
import type { OtpCatalogueReader, OtpPackageDto } from '../../application/otp';
import type { NativeProfileClient } from '../../infrastructure/agentProviders/codex/profiles/nativeProfileClient';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { DraftWorkspace } from '../../components/draftWorkspace';
import { localCodexRoutes } from '../../application/agentProviders/codex/localRoutes';
import {
  executionRouteKey,
  executionRouteRef,
  type ExecutionRouteRefDto,
} from '../../application/executionTargets/contracts';
import { useDraftCloseWarning } from '../../components/useDraftCloseWarning';
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
  ProfileModelCatalogueDto,
  ProfileRoutePolicyDto,
  RuntimeProfileSnapshotDto,
} from '../../application/executionConfiguration';
import { CapabilityProfileEditor } from './CapabilityProfileEditor';
import { CapabilityProfileEditorMemory } from './CapabilityProfileEditorMemory';
import { runtimeProfileViewModel } from './presentation';
import type { CapabilityProfileDraft, HarnessInferenceRouteOption } from './types';
import './mountedExecutionConfiguration.css';

export interface ExecutionConfigurationScreenProps {
  readonly client: ExecutionConfigurationClient;
  /** Design-time package descriptions group selectable MCP tools without probing them. */
  readonly readOtpCatalogue?: OtpCatalogueReader;
  readonly workspace?: DraftWorkspace<CapabilityProfileDraft>;
  readonly editorMemory?: CapabilityProfileEditorMemory;
  /** Native profiles are projected into non-secret local harness/source routes. */
  readonly nativeProfileClient?: NativeProfileClient;
  readonly selection?: { readonly profileId: string | null; readonly newProfile: boolean };
  readonly onSelectionChange?: (selection: {
    readonly profileId: string | null;
    readonly newProfile: boolean;
  }) => void;
}

const EMPTY_RUNTIME: RuntimeProfileSnapshotDto = {
  contractVersion: 1,
  configuration: { provider: 'unavailable', configurationId: 'unavailable' },
  exposure: {
    models: [],
    reasoningModes: [],
    sandboxModes: [],
    mcpTools: {},
    skills: [],
  },
  locked: { model: null, reasoningMode: null, sandboxMode: null },
};

function draftFromProfile(profile: CapabilityProfileDto): CapabilityProfileDraft {
  const legacyRoute: ProfileRoutePolicyDto | undefined = profile.execution
    ? {
        routeId: `${profile.capabilityProfileId}:legacy`,
        execution: profile.execution,
        modelAllowances: profile.allowedCapabilities.models.map((modelId) => ({
          modelId,
          minimumReasoning: profile.allowedCapabilities.reasoningModes[0] ?? 'none',
          maximumReasoning:
            profile.allowedCapabilities.reasoningModes.at(-1) ??
            profile.allowedCapabilities.reasoningModes[0] ??
            'none',
        })),
        mcpGroups: [],
        skillGroups: [],
        defaults: profile.defaults ?? { model: null, reasoningMode: null, sandboxMode: null },
      }
    : undefined;
  const routePolicies = profile.routePolicies?.length
    ? profile.routePolicies
    : legacyRoute
      ? [legacyRoute]
      : [];
  return {
    capabilityProfileId: profile.capabilityProfileId,
    name: profile.name,
    revision: profile.revision,
    allowedCapabilities: profile.allowedCapabilities,
    defaults: profile.defaults,
    execution: profile.execution,
    routePolicies,
    defaultRouteId: profile.defaultRouteId ?? routePolicies[0]?.routeId ?? null,
  };
}

function newDraft(
  runtime: RuntimeProfileSnapshotDto,
  execution?: CapabilityProfileDraft['execution'],
): CapabilityProfileDraft {
  return {
    capabilityProfileId: '',
    name: '',
    revision: null,
    allowedCapabilities: runtime.exposure,
    routePolicies: [],
    defaultRouteId: null,
    ...(execution ? { execution } : {}),
  };
}

export function ExecutionConfigurationScreen({
  client,
  readOtpCatalogue,
  workspace: providedWorkspace,
  editorMemory: providedEditorMemory,
  nativeProfileClient,
  selection,
  onSelectionChange,
}: ExecutionConfigurationScreenProps) {
  const localWorkspace = useMemo(() => new DraftWorkspace<CapabilityProfileDraft>(), []);
  const localEditorMemory = useMemo(() => new CapabilityProfileEditorMemory(), []);
  const workspace = providedWorkspace ?? localWorkspace;
  const editorMemory = providedEditorMemory ?? localEditorMemory;
  const [defaultProfileId, setDefaultProfileId] = useState<string | null>(null);
  const [runtime, setRuntime] = useState<RuntimeProfileSnapshotDto>(EMPTY_RUNTIME);
  const [profiles, setProfiles] = useState<readonly CapabilityProfileDto[]>([]);
  const [otpPackages, setOtpPackages] = useState<readonly OtpPackageDto[]>([]);
  const [routes, setRoutes] = useState<readonly HarnessInferenceRouteOption[]>([]);
  const [modelCatalogues, setModelCatalogues] = useState<
    Readonly<Record<string, ProfileModelCatalogueDto>>
  >({});
  const [draft, setDraft] = useState<CapabilityProfileDraft>(() => newDraft(EMPTY_RUNTIME));
  const [selectedId, setSelectedId] = useState<string | null>(
    selection?.profileId ?? workspace.selectedKey,
  );
  const selectedRef = useRef(selectedId);
  const loadModelCatalogue = useCallback(
    async (execution: ExecutionRouteRefDto) => {
      if (!client.loadProfileModelCatalogue) return;
      const route = executionRouteRef(execution);
      const key = executionRouteKey(route);
      try {
        const catalogue = await client.loadProfileModelCatalogue(route);
        setModelCatalogues((current) => ({ ...current, [key]: catalogue }));
      } catch (cause) {
        setModelCatalogues((current) => ({
          ...current,
          [key]: {
            route,
            observedAt: null,
            models: [],
            observationError: errorMessage(cause),
          },
        }));
      }
    },
    [client],
  );
  const editDraft = (next: CapabilityProfileDraft) => {
    workspace.edit(selectedRef.current ?? '$new', next);
    setDraft(next);
    for (const route of next.routePolicies) {
      if (
        !draft.routePolicies.some(
          (previous) =>
            executionRouteKey(previous.execution) === executionRouteKey(route.execution),
        )
      )
        void loadModelCatalogue(route.execution);
    }
  };
  useDraftCloseWarning(() => workspace.dirty());
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextProfiles, nextDefault, nextOtpPackages, nativeProfiles] = await Promise.all([
        client.listCapabilityProfiles(),
        client.loadDefaultCapabilityProfile?.().catch(() => null) ?? Promise.resolve(null),
        // The catalogue is local design-time metadata. A read failure must not
        // prevent a capability profile from being viewed or edited. The selected
        // native runtime remains an available local route below.
        readOtpCatalogue?.().catch(() => []) ?? Promise.resolve([]),
        nativeProfileClient?.load().catch(() => null) ?? Promise.resolve(null),
      ]);
      const nextRoutes = localCodexRoutes(nativeProfiles?.profiles ?? []);
      const hasNewDraft = selectedRef.current === null && workspace.read('$new') !== undefined;
      const selected = hasNewDraft
        ? undefined
        : nextProfiles.find((profile) => profile.capabilityProfileId === selectedRef.current);
      const nextDraft = selected
        ? workspace.load(selected.capabilityProfileId, draftFromProfile(selected))
        : hasNewDraft
          ? workspace.load('$new', newDraft(EMPTY_RUNTIME))
          : newDraft(EMPTY_RUNTIME);
      setDefaultProfileId(nextDefault);
      setProfiles(nextProfiles);
      setOtpPackages(nextOtpPackages);
      setRoutes(nextRoutes);
      setSelectedId(selected?.capabilityProfileId ?? null);
      selectedRef.current = selected?.capabilityProfileId ?? null;
      workspace.selectedKey = selectedRef.current;
      setDraft(nextDraft);
      const routes = new Map(
        nextProfiles.flatMap((profile) =>
          (profile.routePolicies ?? []).map(
            (route) => [executionRouteKey(route.execution), route.execution] as const,
          ),
        ),
      );
      for (const execution of routes.values()) {
        void loadModelCatalogue(execution);
      }
      void client.loadSelectedRuntimeProfile().then(setRuntime, () => setRuntime(EMPTY_RUNTIME));
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, [client, workspace, readOtpCatalogue, nativeProfileClient, loadModelCatalogue]);

  useEffect(() => {
    void load();
    // Selection changes are handled locally; reloads explicitly retain the current ID.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client]);

  const runtimeView = useMemo(
    () =>
      (() => {
        const view = runtimeProfileViewModel(runtime);
        return { ...view, catalogs: { ...view.catalogs, otpPackages } };
      })(),
    [runtime, otpPackages],
  );
  const selectProfile = (profile: CapabilityProfileDto) => {
    selectedRef.current = profile.capabilityProfileId;
    workspace.selectedKey = profile.capabilityProfileId;
    setSelectedId(profile.capabilityProfileId);
    const next = workspace.load(profile.capabilityProfileId, draftFromProfile(profile));
    setDraft(next);
    for (const route of next.routePolicies) void loadModelCatalogue(route.execution);
    setError(null);
    onSelectionChange?.({ profileId: profile.capabilityProfileId, newProfile: false });
  };

  const save = async (next: CapabilityProfileDraft) => {
    if (saving) return;
    const key = selectedRef.current ?? '$new';
    setSaving(true);
    setError(null);
    try {
      const saved =
        next.revision === null
          ? await client.createCapabilityProfile({
              name: next.name.trim(),
              allowedCapabilities: next.allowedCapabilities,
              defaults: next.defaults,
              ...(next.execution ? { execution: next.execution } : {}),
              routePolicies: next.routePolicies,
              defaultRouteId: next.defaultRouteId,
            })
          : await client.updateCapabilityProfile({
              capabilityProfileId: next.capabilityProfileId,
              name: next.name.trim(),
              allowedCapabilities: next.allowedCapabilities,
              defaults: next.defaults,
              ...(next.execution ? { execution: next.execution } : {}),
              routePolicies: next.routePolicies,
              defaultRouteId: next.defaultRouteId,
            });
      const working = workspace.acceptSave(
        key,
        next,
        draftFromProfile(saved),
        (current, baseline) => ({
          ...current,
          capabilityProfileId: baseline.capabilityProfileId,
          revision: baseline.revision,
        }),
      );
      if (key !== saved.capabilityProfileId) {
        editorMemory.rekey(key, saved.capabilityProfileId);
        workspace.load(saved.capabilityProfileId, draftFromProfile(saved));
        workspace.edit(saved.capabilityProfileId, working);
        workspace.discard(key);
      }
      if ((selectedRef.current ?? '$new') === key) {
        selectedRef.current = saved.capabilityProfileId;
        workspace.selectedKey = saved.capabilityProfileId;
        setSelectedId(saved.capabilityProfileId);
        setDraft(working);
        onSelectionChange?.({ profileId: saved.capabilityProfileId, newProfile: false });
      }
      setProfiles(await client.listCapabilityProfiles());
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!selectedId || !window.confirm(`Delete Capability Profile “${draft.name}”?`)) return;
    setSaving(true);
    setError(null);
    try {
      await client.deleteCapabilityProfile(selectedId);
      editorMemory.clear(selectedId);
      workspace.discard(selectedId);
      const nextProfiles = await client.listCapabilityProfiles();
      setProfiles(nextProfiles);
      setSelectedId(null);
      selectedRef.current = null;
      workspace.selectedKey = null;
      setDraft(newDraft(runtime));
      onSelectionChange?.({ profileId: null, newProfile: false });
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setSaving(false);
    }
  };

  return (
    <main className="execution-configuration-screen">
      <aside className="execution-configuration-screen__catalog" aria-label="Capability Profiles">
        <header>
          <div>
            <p>Technical capability</p>
            <h1>Capability Profiles</h1>
          </div>
          <button type="button" aria-label="Reload Capability Profiles" onClick={() => void load()}>
            <RefreshCw size={16} aria-hidden="true" />
          </button>
        </header>
        <button
          className="execution-configuration-screen__new"
          type="button"
          onClick={() => {
            selectedRef.current = null;
            workspace.selectedKey = null;
            setSelectedId(null);
            setDraft(workspace.load('$new', newDraft(runtime)));
            setError(null);
            onSelectionChange?.({ profileId: null, newProfile: true });
          }}
        >
          <Plus size={16} aria-hidden="true" />
          New profile
        </button>
        {client.setDefaultCapabilityProfile && (
          <label className="default-capability-profile">
            <span>Default for new Agent Sessions</span>
            <select
              aria-label="Default Capability Profile"
              value={defaultProfileId ?? ''}
              disabled={loading || saving}
              onChange={(event) => {
                const id = event.target.value;
                if (!id) return;
                setSaving(true);
                void client.setDefaultCapabilityProfile!(id)
                  .then(() => setDefaultProfileId(id))
                  .catch((cause) => setError(errorMessage(cause)))
                  .finally(() => setSaving(false));
              }}
            >
              <option value="" disabled>
                Choose a default profile
              </option>
              {profiles.map((profile) => (
                <option key={profile.capabilityProfileId} value={profile.capabilityProfileId}>
                  {profile.name}
                </option>
              ))}
            </select>
            {!defaultProfileId && <p>Choose a default for sessions without a target worktree.</p>}
          </label>
        )}
        <nav aria-label="Saved Capability Profiles">
          {profiles.map((profile) => (
            <button
              type="button"
              className={selectedId === profile.capabilityProfileId ? 'is-selected' : undefined}
              key={profile.capabilityProfileId}
              onClick={() => selectProfile(profile)}
            >
              <strong>{profile.name}</strong>
              <small>Revision {profile.revision}</small>
            </button>
          ))}
          {!loading && profiles.length === 0 ? <p>No Capability Profiles yet.</p> : null}
        </nav>
      </aside>
      <section className="execution-configuration-screen__workspace">
        {selectedId && selectedId === defaultProfileId && (
          <p>Choose another default before deleting this profile.</p>
        )}
        {error ? (
          <div className="execution-configuration-screen__error" role="alert">
            {error}
          </div>
        ) : null}
        {loading ? <p className="execution-configuration-screen__loading">Loading…</p> : null}
        {!loading ? (
          <>
            {selectedId || workspace.read('$new') ? (
              <CapabilityProfileEditor
                profile={draft}
                profileKey={selectedId ?? '$new'}
                editorMemory={editorMemory}
                runtime={runtimeView}
                routes={routes}
                modelCatalogues={client.loadProfileModelCatalogue ? modelCatalogues : undefined}
                saving={saving}
                onChange={editDraft}
                onSave={(next) => void save(next)}
              />
            ) : (
              <div className="execution-configuration-screen__empty-state">
                <h2>Choose a Capability Profile</h2>
                <p>
                  Select a saved profile, or create one to configure its routes and capabilities.
                </p>
              </div>
            )}
            {selectedId ? (
              <button
                className="execution-configuration-screen__delete"
                type="button"
                disabled={saving || selectedId === defaultProfileId}
                onClick={() => void remove()}
              >
                <Trash2 size={15} aria-hidden="true" />
                Delete profile
              </button>
            ) : null}
          </>
        ) : null}
      </section>
    </main>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
