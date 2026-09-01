import { Plus, RefreshCw, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
  RuntimeProfileSnapshotDto,
} from '../../application/executionConfiguration';
import { CapabilityProfileEditor } from './CapabilityProfileEditor';
import { runtimeProfileViewModel } from './presentation';
import type { CapabilityProfileDraft } from './types';
import './mountedExecutionConfiguration.css';

export interface ExecutionConfigurationScreenProps {
  readonly client: ExecutionConfigurationClient;
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
  };
}

function newDraft(runtime: RuntimeProfileSnapshotDto): CapabilityProfileDraft {
  return {
    capabilityProfileId: '',
    name: '',
    revision: null,
    allowedCapabilities: runtime.exposure,
  };
}

export function ExecutionConfigurationScreen({ client }: ExecutionConfigurationScreenProps) {
  const [runtime, setRuntime] = useState<RuntimeProfileSnapshotDto>(EMPTY_RUNTIME);
  const [profiles, setProfiles] = useState<readonly CapabilityProfileDto[]>([]);
  const [draft, setDraft] = useState<CapabilityProfileDraft>(() => newDraft(EMPTY_RUNTIME));
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextRuntime, nextProfiles] = await Promise.all([
        client.loadSelectedRuntimeProfile(),
        client.listCapabilityProfiles(),
      ]);
      setRuntime(nextRuntime);
      setProfiles(nextProfiles);
      const selected =
        nextProfiles.find((profile) => profile.capabilityProfileId === selectedId) ??
        nextProfiles[0];
      setSelectedId(selected?.capabilityProfileId ?? null);
      setDraft(selected ? draftFromProfile(selected) : newDraft(nextRuntime));
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, [client, selectedId]);

  useEffect(() => {
    void load();
    // Selection changes are handled locally; reloads explicitly retain the current ID.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client]);

  const runtimeView = useMemo(() => runtimeProfileViewModel(runtime), [runtime]);

  const selectProfile = (profile: CapabilityProfileDto) => {
    setSelectedId(profile.capabilityProfileId);
    setDraft(draftFromProfile(profile));
    setError(null);
  };

  const save = async (next: CapabilityProfileDraft) => {
    setSaving(true);
    setError(null);
    try {
      const saved =
        next.revision === null
          ? await client.createCapabilityProfile({
              capabilityProfileId: next.capabilityProfileId.trim(),
              name: next.name.trim(),
              allowedCapabilities: next.allowedCapabilities,
            })
          : await client.updateCapabilityProfile({
              capabilityProfileId: next.capabilityProfileId,
              name: next.name.trim(),
              allowedCapabilities: next.allowedCapabilities,
            });
      const nextProfiles = await client.listCapabilityProfiles();
      setProfiles(nextProfiles);
      setSelectedId(saved.capabilityProfileId);
      setDraft(draftFromProfile(saved));
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
      const nextProfiles = await client.listCapabilityProfiles();
      setProfiles(nextProfiles);
      const next = nextProfiles[0];
      setSelectedId(next?.capabilityProfileId ?? null);
      setDraft(next ? draftFromProfile(next) : newDraft(runtime));
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
            setSelectedId(null);
            setDraft(newDraft(runtime));
            setError(null);
          }}
        >
          <Plus size={16} aria-hidden="true" />
          New profile
        </button>
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
              saving={saving}
              onChange={setDraft}
              onSave={(next) => void save(next)}
            />
            {selectedId ? (
              <button
                className="execution-configuration-screen__delete"
                type="button"
                disabled={saving}
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
