import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ModalDialog } from '../../../../components/ModalDialog';
import type {
  DiscoveredNativeCodexHome,
  NativeProfile,
  NativeProfileClient,
} from '../../../../infrastructure/agentProviders/codex/profiles/nativeProfileClient';
import './codexProfiles.css';

type Operation = { readonly kind: 'health' | 'login'; readonly profile: NativeProfile } | null;

export function CodexProfilesScreen({
  client,
  selectedProfileId,
  onSelectedProfileChange,
}: {
  readonly client: NativeProfileClient;
  readonly selectedProfileId?: string | null;
  readonly onSelectedProfileChange?: (profileId: string | null) => void;
}) {
  const [profiles, setProfiles] = useState<readonly NativeProfile[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(selectedProfileId ?? null);
  const selectedChangeRef = useRef(onSelectedProfileChange);
  selectedChangeRef.current = onSelectedProfileChange;
  const select = (profileId: string | null) => {
    setSelectedId(profileId);
    onSelectedProfileChange?.(profileId);
  };
  const [message, setMessage] = useState<string | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [operation, setOperation] = useState<Operation>(null);
  const load = useCallback(async () => {
    try {
      const query = await client.load();
      setProfiles(query.profiles);
      setSelectedId((current) => {
        const preferred = selectedProfileId ?? current;
        const next = preferred && query.profiles.some((profile) => profile.id === preferred)
          ? preferred
          : query.profiles.find((profile) => profile.selected)?.id ?? query.profiles[0]?.id ?? null;
        selectedChangeRef.current?.(next);
        return next;
      });
    } catch (cause) {
      setMessage(cause instanceof Error ? cause.message : 'Codex profiles are unavailable.');
    }
  }, [client, selectedProfileId]);
  useEffect(() => {
    if (selectedProfileId !== undefined && selectedProfileId !== selectedId) {
      setSelectedId(selectedProfileId);
    }
  }, [selectedId, selectedProfileId]);
  useEffect(() => { void load(); }, [load]);
  const selected = useMemo(
    () => profiles.find((profile) => profile.id === selectedId) ?? null,
    [profiles, selectedId],
  );
  const makeDefault = async (profile: NativeProfile) => {
    try {
      const result = await client.select(profile.id);
      setProfiles(result.profiles);
      select(profile.id);
      setMessage(`${displayPath(profile.homePath)} is now the default Codex profile.`);
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : 'The operation was rejected.';
      setMessage(detail);
    }
  };
  return (
    <main className="codex-profiles" aria-label="Codex profiles" tabIndex={0}>
      <header className="codex-profiles__header">
        <div>
          <p className="eyebrow">Technical Settings</p>
          <h1>Codex profiles</h1>
          <p>Choose the local Codex configuration folders Orchid can use on this device.</p>
        </div>
        <button type="button" className="is-primary" onClick={() => setAddOpen(true)}>
          Add Codex profile
        </button>
      </header>
      {message ? <p className="codex-profiles__message" role="status">{message}</p> : null}
      <div className="codex-profiles__layout">
        <CodexProfileList
          profiles={profiles}
          selectedId={selectedId}
          onSelect={(id) => select(id)}
          onMakeDefault={(profile) => void makeDefault(profile)}
        />
        {selected ? (
          <CodexProfileDetail
            key={selected.id}
            client={client}
            profile={selected}
            onProfiles={setProfiles}
            onCheckHealth={() => setOperation({ kind: 'health', profile: selected })}
            onVerifyLogin={() => setOperation({ kind: 'login', profile: selected })}
          />
        ) : (
          <section className="codex-profiles__empty" aria-label="No selected Codex profile">
            <h2>No Codex profile registered</h2>
            <p>Add a profile to connect a local Codex CLI configuration.</p>
          </section>
        )}
      </div>
      {addOpen ? <AddCodexProfileDialog client={client} onClose={() => setAddOpen(false)} onAdded={(result) => {
        setProfiles(result);
        select(result.at(-1)?.id ?? selectedId);
        setAddOpen(false);
      }} /> : null}
      {operation?.kind === 'health' ? <ProfileHealthDialog client={client} profile={operation.profile} onProfiles={setProfiles} onClose={() => setOperation(null)} /> : null}
      {operation?.kind === 'login' ? <ProfileLoginDialog client={client} profile={operation.profile} onProfiles={setProfiles} onClose={() => setOperation(null)} /> : null}
    </main>
  );
}

function CodexProfileList({ profiles, selectedId, onSelect, onMakeDefault }: {
  readonly profiles: readonly NativeProfile[];
  readonly selectedId: string | null;
  readonly onSelect: (id: string) => void;
  readonly onMakeDefault: (profile: NativeProfile) => void;
}) {
  return <aside className="codex-profile-list" aria-label="Registered Codex profiles">
    <header><h2>Registered profiles</h2><span>{profiles.length}</span></header>
    {profiles.length === 0 ? <p>No profiles yet.</p> : <ul>
      {profiles.map((profile) => <li key={profile.id}>
        <button
          type="button"
          className={profile.id === selectedId ? 'is-selected' : ''}
          aria-current={profile.id === selectedId ? 'true' : undefined}
          onClick={() => onSelect(profile.id)}
        >
          <span className="codex-profile-list__path">{displayPath(profile.homePath)}</span>
          <span className="codex-profile-list__meta">{profile.ownership === 'application_dedicated' ? 'Orchid-managed' : 'Existing folder'}</span>
        </button>
        <span className="codex-profile-list__default">
          {profile.selected ? 'Default' : <button type="button" onClick={() => onMakeDefault(profile)}>Make default</button>}
        </span>
      </li>)}
    </ul>}
  </aside>;
}

function CodexProfileDetail({ client, profile, onProfiles, onCheckHealth, onVerifyLogin }: {
  readonly client: NativeProfileClient;
  readonly profile: NativeProfile;
  readonly onProfiles: (profiles: readonly NativeProfile[]) => void;
  readonly onCheckHealth: () => void;
  readonly onVerifyLogin: () => void;
}) {
  const [pathMenuOpen, setPathMenuOpen] = useState(false);
  const [tools, setTools] = useState<Awaited<ReturnType<NonNullable<NativeProfileClient['loadHarnessTools']>>> | null>(null);
  const [toolsError, setToolsError] = useState<string | null>(null);
  const [loadingTools, setLoadingTools] = useState(false);
  const [skills, setSkills] = useState<Awaited<ReturnType<NativeProfileClient['loadSkills']>> | null>(null);
  const [skillsError, setSkillsError] = useState<string | null>(null);
  const [loadingSkills, setLoadingSkills] = useState(false);
  const [savingPersonality, setSavingPersonality] = useState(false);
  const [personalityError, setPersonalityError] = useState<string | null>(null);
  const loadSkills = useCallback(async () => {
    setLoadingSkills(true); setSkillsError(null);
    try { setSkills(await client.loadSkills(profile.id)); }
    catch (cause) { setSkillsError(cause instanceof Error ? cause.message : 'Skill discovery is unavailable.'); }
    finally { setLoadingSkills(false); }
  }, [client, profile.id]);
  useEffect(() => { void loadSkills(); }, [loadSkills]);
  const loadTools = async () => {
    setLoadingTools(true); setToolsError(null);
    try { setTools(await client.loadHarnessTools(profile.id)); }
    catch (cause) { setToolsError(cause instanceof Error ? cause.message : 'Tool inventory is unavailable.'); }
    finally { setLoadingTools(false); }
  };
  return <section className="codex-profile-detail" aria-label="Selected Codex profile">
    <header>
      <p className="eyebrow">Codex profile</p>
      <h2>{profile.ownership === 'application_dedicated' ? 'Orchid-managed profile' : 'Existing Codex profile'}</h2>
    </header>
    <section className="codex-profile-detail__section">
      <h3>Folder</h3>
      <div className="codex-profile-path">
        <code>{displayPath(profile.homePath)}</code>
        <div>
          <button type="button" aria-label="Folder options" aria-expanded={pathMenuOpen} onClick={() => setPathMenuOpen((value) => !value)}>⋯</button>
          {pathMenuOpen ? <div className="codex-profile-path__menu" role="menu">
            <button type="button" role="menuitem" onClick={() => void client.openInExplorer(profile.id)}>Open in Explorer</button>
          </div> : null}
        </div>
      </div>
      <p>{profile.lifecycle === 'active' ? 'Folder is available.' : 'This folder needs attention before Orchid can use it.'}</p>
    </section>
    <section className="codex-profile-detail__section codex-profile-detail__actions">
      <div><h3>Health</h3><p>Checks the folder, local Codex CLI, and current login state without changing setup.</p></div>
      <button type="button" onClick={onCheckHealth} disabled={profile.lifecycle !== 'active'}>Check health</button>
    </section>
    <section className="codex-profile-detail__section codex-profile-detail__actions">
      <div><h3>Login</h3><p>{loginSummary(profile)}</p></div>
      <button type="button" onClick={onVerifyLogin} disabled={profile.lifecycle !== 'active' || profile.readiness.authentication === 'authenticated'}>Verify login</button>
    </section>
    <section className="codex-profile-detail__section codex-profile-detail__actions">
      <div>
        <h3>Codex personality</h3>
        <p>The default Codex response style for execution routes using this profile.</p>
      </div>
      <label>
        <span className="sr-only">Codex personality</span>
        <select
          value={profile.personality ?? 'inherit'}
          disabled={savingPersonality || profile.lifecycle !== 'active'}
          onChange={(event) => {
            const value = event.currentTarget.value;
            setSavingPersonality(true);
            setPersonalityError(null);
            const update = client.setPersonality?.(
              profile.id,
              value === 'inherit' ? null : value as 'none' | 'friendly' | 'pragmatic',
            );
            if (!update) {
              setPersonalityError('Codex personality configuration is unavailable.');
              setSavingPersonality(false);
              return;
            }
            void update.then((result) => onProfiles(result.profiles)).catch((cause: unknown) => {
              setPersonalityError(cause instanceof Error ? cause.message : 'Could not update the Codex personality.');
            }).finally(() => setSavingPersonality(false));
          }}
        >
          <option value="inherit">Use Codex configuration default</option>
          <option value="none">None</option>
          <option value="friendly">Friendly</option>
          <option value="pragmatic">Pragmatic</option>
        </select>
      </label>
      {personalityError ? <p role="alert">{personalityError}</p> : null}
    </section>
    <section className="codex-profile-detail__section">
      <div className="codex-profile-detail__actions">
        <div><h3>Codex skills{skills ? ` · ${skills.skills.filter((skill) => skill.enabled).length}` : ''}</h3><p>Skills this Codex profile discovers. A project working folder may add more.</p></div>
        <button type="button" onClick={() => void loadSkills()} disabled={loadingSkills}>{loadingSkills ? 'Discovering…' : 'Refresh skills'}</button>
      </div>
      {skillsError ? <p role="alert">Could not discover skills: {skillsError}</p> : null}
      {skills?.limitations.map((item, index) => <p role="status" key={`${item}:${index}`}>{item}</p>)}
      {skills && skills.skills.length === 0 ? <p>No Codex skills were reported for this profile.</p> : null}
      {skills ? <ul className="codex-profile-tools">{skills.skills.map((skill) => <li key={skill.path}><strong>{skill.name}</strong><span>{skill.scope} · {skill.enabled ? 'Enabled' : 'Disabled'} · {displayPath(skill.path)}</span>{skill.description ? <span>{skill.description}</span> : null}</li>)}</ul> : null}
    </section>
    <section className="codex-profile-detail__section">
      <h3>Harness-provided tools</h3>
      <p>Shows only capabilities reported by this profile's current Codex runtime.</p>
      <button type="button" onClick={() => void loadTools()} disabled={loadingTools}>{loadingTools ? 'Reading tools…' : 'Read provided tools'}</button>
      {toolsError ? <p role="alert">{toolsError}</p> : null}
      {tools ? <>
        {tools.limitations.map((item) => <p key={item}>{item}</p>)}
        <ul className="codex-profile-tools">{tools.entries.filter((item) => item.kind !== 'skill').map((item, index) => <li key={`${item.kind}:${item.name}:${index}`}><strong>{item.name}</strong><span>{item.kind.replaceAll('_', ' ')} · {item.origin} · {item.state}</span></li>)}</ul>
        {tools.entries.every((item) => item.kind === 'skill') ? <p>No discoverable tools were reported.</p> : null}
      </> : null}
    </section>
  </section>;
}

function AddCodexProfileDialog({ client, onClose, onAdded }: {
  readonly client: NativeProfileClient;
  readonly onClose: () => void;
  readonly onAdded: (profiles: readonly NativeProfile[]) => void;
}) {
  const [address, setAddress] = useState('');
  const [createNew, setCreateNew] = useState(false);
  const [discovering, setDiscovering] = useState(false);
  const [discovered, setDiscovered] = useState<readonly DiscoveredNativeCodexHome[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const discover = async () => {
    setBusy(true); setError(null);
    try { setDiscovered(await client.discoverHomes()); setDiscovering(true); }
    catch (cause) { setError(cause instanceof Error ? cause.message : 'Discovery failed.'); }
    finally { setBusy(false); }
  };
  const save = async () => {
    setBusy(true); setError(null);
    try {
      const result = createNew ? await client.createDedicated() : await client.registerExisting(address.trim());
      onAdded(result.profiles);
    } catch (cause) { setError(cause instanceof Error ? cause.message : 'Could not add the Codex profile.'); }
    finally { setBusy(false); }
  };
  return <ModalDialog labelledBy="add-codex-profile-title" onClose={onClose} className="codex-profile-dialog">
    <header><h2 id="add-codex-profile-title">Add Codex profile</h2><p>Register an existing Codex home folder or create an isolated one for Orchid.</p></header>
    <label className="codex-profile-dialog__toggle"><input type="checkbox" checked={createNew} onChange={(event) => setCreateNew(event.currentTarget.checked)} /> Create new Codex home folder</label>
    <label className="codex-profile-dialog__field"><span>Codex home folder</span><input value={address} disabled={createNew} placeholder="C:\\Users\\you\\.codex" onChange={(event) => setAddress(event.currentTarget.value)} /></label>
    <div className="codex-profile-dialog__actions"><button type="button" onClick={() => void discover()} disabled={busy || createNew}>Discover existing home folders</button><button type="button" className="is-primary" disabled={busy || (!createNew && !address.trim())} onClick={() => void save()}>{busy ? 'Adding…' : createNew ? 'Create Codex profile' : 'Add Codex profile'}</button><button type="button" onClick={onClose} disabled={busy}>Cancel</button></div>
    {error ? <p role="alert">{error}</p> : null}
    {discovering ? <DiscoveryDialog homes={discovered} onClose={() => setDiscovering(false)} onPick={(path) => { setAddress(path); setDiscovering(false); }} /> : null}
  </ModalDialog>;
}

function DiscoveryDialog({ homes, onClose, onPick }: { readonly homes: readonly DiscoveredNativeCodexHome[]; readonly onClose: () => void; readonly onPick: (path: string) => void }) {
  return <ModalDialog labelledBy="discover-codex-home-title" onClose={onClose} className="codex-profile-dialog">
    <header><h2 id="discover-codex-home-title">Discovered Codex home folders</h2><p>Choose an existing folder to use as the address.</p></header>
    {homes.length ? <ul className="codex-profile-discovery">{homes.map((home) => <li key={home.homePath}><div><strong>{displayPath(home.homePath)}</strong><span>{home.registeredProfileId ? 'Already registered' : home.source.replaceAll('_', ' ')}</span></div><button type="button" disabled={home.registeredProfileId !== null} onClick={() => onPick(home.homePath)}>Use this folder</button></li>)}</ul> : <p>No existing Codex home folders were found.</p>}
    <footer><button type="button" onClick={onClose}>Back</button></footer>
  </ModalDialog>;
}

function ProfileHealthDialog({ client, profile, onProfiles, onClose }: { readonly client: NativeProfileClient; readonly profile: NativeProfile; readonly onProfiles: (profiles: readonly NativeProfile[]) => void; readonly onClose: () => void }) {
  const [phase, setPhase] = useState<'ready' | 'running' | 'passed' | 'failed'>('ready');
  const [error, setError] = useState<string | null>(null);
  const run = async () => {
    setPhase('running'); setError(null);
    try { const result = await client.refreshReadiness(profile.id); onProfiles(result.profiles); setPhase('passed'); }
    catch (cause) { setError(cause instanceof Error ? cause.message : 'Health check failed.'); setPhase('failed'); }
  };
  return <ModalDialog labelledBy="profile-health-title" onClose={onClose} dismissible={phase !== 'running'} className="codex-profile-dialog">
    <header><h2 id="profile-health-title">Check profile health</h2><p>{displayPath(profile.homePath)}</p></header>
    <OperationSteps state={phase} items={['Confirming folder continuity', 'Resolving the local Codex CLI', 'Checking current login status']} />
    {error ? <p role="alert">{error}</p> : null}
    <footer>{phase === 'ready' ? <button type="button" className="is-primary" onClick={() => void run()}>Run check</button> : null}{phase !== 'running' ? <button type="button" onClick={onClose}>{phase === 'passed' ? 'Done' : 'Close'}</button> : null}</footer>
  </ModalDialog>;
}

function ProfileLoginDialog({ client, profile, onProfiles, onClose }: { readonly client: NativeProfileClient; readonly profile: NativeProfile; readonly onProfiles: (profiles: readonly NativeProfile[]) => void; readonly onClose: () => void }) {
  const [phase, setPhase] = useState<'ready' | 'checking' | 'authenticated' | 'unauthenticated' | 'requesting' | 'failed'>('ready');
  const [error, setError] = useState<string | null>(null);
  const verify = async () => {
    setPhase('checking'); setError(null);
    try { const result = await client.refreshReadiness(profile.id); onProfiles(result.profiles); const current = result.profiles.find((item) => item.id === profile.id); setPhase(current?.readiness.authentication === 'authenticated' ? 'authenticated' : 'unauthenticated'); }
    catch (cause) { setError(cause instanceof Error ? cause.message : 'Login verification failed.'); setPhase('failed'); }
  };
  const request = async () => {
    setPhase('requesting'); setError(null);
    try { const result = await client.requestLogin(profile.id); onProfiles(result.profiles); setPhase('unauthenticated'); }
    catch (cause) { setError(cause instanceof Error ? cause.message : 'Could not request browser login.'); setPhase('failed'); }
  };
  return <ModalDialog labelledBy="profile-login-title" onClose={onClose} dismissible={phase !== 'checking' && phase !== 'requesting'} className="codex-profile-dialog">
    <header><h2 id="profile-login-title">Verify login</h2><p>{displayPath(profile.homePath)}</p></header>
    <OperationSteps state={phase === 'checking' || phase === 'requesting' ? 'running' : phase === 'authenticated' ? 'passed' : phase === 'failed' ? 'failed' : 'ready'} items={['Resolving the local Codex CLI', 'Checking current sign-in status']} />
    {phase === 'unauthenticated' ? <p>Codex is not signed in for this profile. You can request the normal browser login flow.</p> : null}
    {phase === 'authenticated' ? <p>Codex reports that this profile is signed in.</p> : null}
    {error ? <p role="alert">{error}</p> : null}
    <footer>{phase === 'ready' ? <button type="button" className="is-primary" onClick={() => void verify()}>Verify login</button> : null}{phase === 'unauthenticated' ? <button type="button" className="is-primary" onClick={() => void request()}>Request browser login</button> : null}{phase !== 'checking' && phase !== 'requesting' ? <button type="button" onClick={onClose}>{phase === 'authenticated' ? 'Done' : 'Close'}</button> : null}</footer>
  </ModalDialog>;
}

function OperationSteps({ state, items }: { readonly state: 'ready' | 'running' | 'passed' | 'failed'; readonly items: readonly string[] }) {
  return <ol className="codex-profile-operation-steps">{items.map((item, index) => <li key={item} data-state={state === 'ready' ? 'pending' : state === 'running' && index === items.length - 1 ? 'running' : state === 'failed' && index === items.length - 1 ? 'failed' : 'passed'}>{item}</li>)}</ol>;
}

function loginSummary(profile: NativeProfile): string {
  if (profile.readiness.authentication === 'authenticated') return 'Codex is signed in for this profile.';
  if (profile.readiness.authentication === 'unauthenticated') return 'Codex is not signed in for this profile.';
  return 'Login has not been verified yet.';
}

export function displayPath(path: string): string {
  return path.replace(/^\\\\\?\\/, '');
}
