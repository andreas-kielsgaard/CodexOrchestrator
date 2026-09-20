import { Plus, RefreshCw, Trash2 } from 'lucide-react';
import type { OtpCatalogueReader, OtpPackageDto } from '../../application/otp';
import type { ExecutionTargetClient } from '../../application/executionTargets/contracts';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { DraftWorkspace } from '../../components/draftWorkspace';
import { useDraftCloseWarning } from '../../components/useDraftCloseWarning';
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
  RuntimeProfileSnapshotDto,
} from '../../application/executionConfiguration';
import { CapabilityProfileEditor } from './CapabilityProfileEditor';
import { NativeCapabilityInventory } from './NativeCapabilityInventory';
import { runtimeProfileViewModel } from './presentation';
import type { CapabilityProfileDraft, HarnessInferenceRouteOption } from './types';
import './mountedExecutionConfiguration.css';

export interface ExecutionConfigurationScreenProps {
  readonly client: ExecutionConfigurationClient;
  /** Design-time package descriptions group selectable MCP tools without probing them. */
  readonly readOtpCatalogue?: OtpCatalogueReader;
  readonly workspace?: DraftWorkspace<CapabilityProfileDraft>;
  /** Native profiles are projected into non-secret local harness/source routes. */
  readonly nativeProfileClient?: NativeProfileClient;
  readonly executionTargetClient?: ExecutionTargetClient;
}

const EMPTY_RUNTIME: RuntimeProfileSnapshotDto = {
  contractVersion: 1,
  profileRef: 'unavailable',
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
  return {
    capabilityProfileId: profile.capabilityProfileId,
    name: profile.name,
    revision: profile.revision,
    allowedCapabilities: profile.allowedCapabilities,
    defaults: profile.defaults,
    execution: profile.execution,
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
    ...(execution ? { execution } : {}),
  };
}

function localHarnessRoutes(
  query: Awaited<ReturnType<NativeProfileClient['load']>> | null,
): readonly HarnessInferenceRouteOption[] {
  return (query?.profiles ?? [])
    .filter((profile) => profile.lifecycle === 'active')
    .map((profile) => ({
      id: `local-codex:${profile.id}`,
      selected: profile.selected,
      label: profile.selected ? 'This device · selected Codex CLI' : 'This device · Codex CLI',
      sourceLabel: 'OpenAI via Codex CLI',
      detail: `${profile.homePath} · the account is configured in this Codex home`,
      execution: {
        deviceId: 'local',
        deviceName: 'This device',
        provider: 'codex' as const,
        configurationRef: profile.id,
        connection: { kind: 'local' as const },
      },
    }));
}

export function ExecutionConfigurationScreen({
  client,
  readOtpCatalogue,
  workspace: providedWorkspace,
  nativeProfileClient,
  executionTargetClient,
}: ExecutionConfigurationScreenProps) {
  const localWorkspace = useMemo(() => new DraftWorkspace<CapabilityProfileDraft>(), []);
  const workspace = providedWorkspace ?? localWorkspace;
  const [defaultProfileId, setDefaultProfileId] = useState<string | null>(null);
  const [runtime, setRuntime] = useState<RuntimeProfileSnapshotDto>(EMPTY_RUNTIME);
  const [profiles, setProfiles] = useState<readonly CapabilityProfileDto[]>([]);
  const [otpPackages, setOtpPackages] = useState<readonly OtpPackageDto[]>([]);
  const [routes, setRoutes] = useState<readonly HarnessInferenceRouteOption[]>([]);
  const [draft, setDraft] = useState<CapabilityProfileDraft>(() => newDraft(EMPTY_RUNTIME));
  const [selectedId, setSelectedId] = useState<string | null>(workspace.selectedKey);
  const selectedRef = useRef(selectedId);
  const loadRouteRuntime = useCallback(
    async (execution: CapabilityProfileDraft['execution']) => {
      if (!execution || !executionTargetClient) return;
      try {
        const observed = await executionTargetClient.loadRuntime(execution);
        setRuntime(observed.runtimeProfile);
      } catch (caught) {
        setError(`The selected harness could not be observed: ${errorMessage(caught)}`);
      }
    },
    [executionTargetClient],
  );
  const editDraft = (next: CapabilityProfileDraft) => {
    workspace.edit(selectedRef.current ?? '$new', next);
    setDraft(next);
    if (JSON.stringify(next.execution) !== JSON.stringify(draft.execution))
      void loadRouteRuntime(next.execution);
  };
  useDraftCloseWarning(() => workspace.dirty());
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextRuntime, nextProfiles, nextDefault, nextOtpPackages, nativeProfiles] =
        await Promise.all([
        client.loadSelectedRuntimeProfile(),
        client.listCapabilityProfiles(),
        client.loadDefaultCapabilityProfile?.() ?? Promise.resolve(null),
        // The catalogue is local design-time metadata. A read failure must not
        // prevent a capability profile from being viewed or edited.
        readOtpCatalogue?.().catch(() => []) ?? Promise.resolve([]),
        nativeProfileClient?.load().catch(() => null) ?? Promise.resolve(null),
      ]);
      const nextRoutes = localHarnessRoutes(nativeProfiles);
      const defaultRoute =
        nextRoutes.find((route) => route.label.includes('selected')) ?? nextRoutes[0];
      const hasNewDraft = selectedRef.current === null && workspace.read('$new') !== undefined;
      const selected = hasNewDraft
        ? undefined
        : (nextProfiles.find((profile) => profile.capabilityProfileId === selectedRef.current) ??
          nextProfiles[0]);
      const nextDraft = workspace.load(
        selected?.capabilityProfileId ?? '$new',
        selected ? draftFromProfile(selected) : newDraft(nextRuntime, defaultRoute?.execution),
      );
      setRuntime(nextRuntime);
      setDefaultProfileId(nextDefault);
      setProfiles(nextProfiles);
      setOtpPackages(nextOtpPackages);
      setRoutes(nextRoutes);
      setSelectedId(selected?.capabilityProfileId ?? null);
      selectedRef.current = selected?.capabilityProfileId ?? null;
      workspace.selectedKey = selectedRef.current;
      setDraft(nextDraft);
      await loadRouteRuntime(nextDraft.execution);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, [client, workspace, readOtpCatalogue, nativeProfileClient, loadRouteRuntime]);

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
  const selectedRouteExecution =
    draft.execution ?? routes.find((route) => route.selected)?.execution;

  const selectProfile = (profile: CapabilityProfileDto) => {
    selectedRef.current = profile.capabilityProfileId;
    workspace.selectedKey = profile.capabilityProfileId;
    setSelectedId(profile.capabilityProfileId);
    const next = workspace.load(profile.capabilityProfileId, draftFromProfile(profile));
    setDraft(next);
    void loadRouteRuntime(next.execution);
    setError(null);
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
              capabilityProfileId: next.capabilityProfileId.trim(),
              name: next.name.trim(),
              allowedCapabilities: next.allowedCapabilities,
              defaults: next.defaults,
              ...(next.execution ? { execution: next.execution } : {}),
            })
          : await client.updateCapabilityProfile({
              capabilityProfileId: next.capabilityProfileId,
              name: next.name.trim(),
              allowedCapabilities: next.allowedCapabilities,
              defaults: next.defaults,
              ...(next.execution ? { execution: next.execution } : {}),
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
        workspace.load(saved.capabilityProfileId, draftFromProfile(saved));
        workspace.edit(saved.capabilityProfileId, working);
        workspace.discard(key);
      }
      if ((selectedRef.current ?? '$new') === key) {
        selectedRef.current = saved.capabilityProfileId;
        workspace.selectedKey = saved.capabilityProfileId;
        setSelectedId(saved.capabilityProfileId);
        setDraft(working);
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
      workspace.discard(selectedId);
      const nextProfiles = await client.listCapabilityProfiles();
      setProfiles(nextProfiles);
      const next = nextProfiles[0];
      setSelectedId(next?.capabilityProfileId ?? null);
      selectedRef.current = next?.capabilityProfileId ?? null;
      workspace.selectedKey = selectedRef.current;
      setDraft(
        workspace.load(
          selectedRef.current ?? '$new',
          next
            ? draftFromProfile(next)
            : newDraft(runtime, routes.find((route) => route.label.includes('selected'))?.execution),
        ),
      );
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
            setDraft(
              workspace.load(
                '$new',
                newDraft(
                  runtime,
                  routes.find((route) => route.label.includes('selected'))?.execution,
                ),
              ),
            );
            setError(null);
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
            <p>A selected worktree supplies its own Capability Profile.</p>
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
              <span>{profile.capabilityProfileId}</span>
              <small>Revision {profile.revision}</small>
            </button>
          ))}
          {!loading && profiles.length === 0 ? <p>No Capability Profiles yet.</p> : null}
        </nav>
      </aside>
      <section className="execution-configuration-screen__workspace">
        <NativeCapabilityInventory
          key={JSON.stringify(selectedRouteExecution)}
          client={client}
          loadInventory={
            executionTargetClient && selectedRouteExecution
              ? async () =>
                  (await executionTargetClient.loadRuntime(selectedRouteExecution)).nativeInventory
              : undefined
          }
        />
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
            <CapabilityProfileEditor
              profile={draft}
              runtime={runtimeView}
              routes={routes}
              saving={saving}
              onChange={editDraft}
              onSave={(next) => void save(next)}
            />
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
