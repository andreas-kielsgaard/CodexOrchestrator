import { useCallback, useEffect, useMemo, useState } from 'react';
import type { NativeProfile, NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import './nativeProfileSettings.css';

export function NativeProfileSettings({ client }: { readonly client: NativeProfileClient }) {
  const [profiles, setProfiles] = useState<readonly NativeProfile[]>([]);
  const [homePath, setHomePath] = useState('');
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState('Loading durable Codex home profiles.');
  const selected = useMemo(() => profiles.find((profile) => profile.selected), [profiles]);
  const refresh = useCallback(async () => {
    try {
      setProfiles((await client.load()).profiles);
      setMessage('Showing current durable profile state.');
    } catch (error) {
      setProfiles([]);
      setMessage(error instanceof Error ? error.message : 'Profile state is unavailable.');
    }
  }, [client]);
  useEffect(() => { void refresh(); }, [refresh]);
  const run = useCallback(async (name: string, action: () => Promise<{ profiles: readonly NativeProfile[] }>) => {
    if (busy) return;
    setBusy(name); setMessage('Waiting for durable profile state.');
    try { setProfiles((await action()).profiles); setMessage('Action completed; state was refreshed from durable storage.'); }
    catch (error) { setMessage(error instanceof Error ? error.message : 'The action failed.'); }
    finally { setBusy(null); }
  }, [busy]);
  const profileAction = (name: string, action: (id: string) => Promise<{ profiles: readonly NativeProfile[] }>) =>
    selected ? () => run(name, () => action(selected.id)) : undefined;
  return (
    <main className="native-profile-settings" aria-label="Technical Codex settings">
      <header><p className="eyebrow">Technical Settings</p><h1>Codex home profiles</h1><p>Manage product-owned Codex homes and their observed setup state. Account identity and provider readiness are never inferred here.</p></header>
      <section aria-labelledby="profile-registration"><h2 id="profile-registration">Register or create a home</h2>
        <div className="native-profile-register"><label>Existing Codex home path<input value={homePath} onChange={(event) => setHomePath(event.target.value)} placeholder="C:\\Users\\you\\.codex" /></label>
          <button type="button" disabled={!homePath.trim() || busy !== null} onClick={() => void run('register', async () => { const result = await client.registerExisting(homePath.trim()); setHomePath(''); return result; })}>Register existing</button>
          <button type="button" disabled={busy !== null} onClick={() => void run('create', () => client.createDedicated())}>Create dedicated Orchestrator home</button>
        </div>
      </section>
      <button type="button" disabled={busy !== null} onClick={() => void refresh()}>Refresh durable state</button>
      {message && <p role="status">{message}</p>}
      {profiles.length === 0 ? <p>No Codex home profiles are registered.</p> : profiles.map((profile) => (
        <ProfileCard key={profile.id} profile={profile} busy={busy} onSelect={profileAction('select', client.select)} onLogin={profileAction('login', client.requestLogin)} onRefresh={profileAction('refresh', client.refreshReadiness)} onSandbox={profileAction('sandbox', client.initializeSandbox)} onCanary={profileAction('canary', client.runCanary)} onMcp={profileAction('mcp', client.probeMcp)} />
      ))}
    </main>
  );
}

function ProfileCard({ profile, busy, onSelect, onLogin, onRefresh, onSandbox, onCanary, onMcp }: { readonly profile: NativeProfile; readonly busy: string | null; readonly onSelect?: () => void; readonly onLogin?: () => void; readonly onRefresh?: () => void; readonly onSandbox?: () => void; readonly onCanary?: () => void; readonly onMcp?: () => void }) {
  const { readiness } = profile;
  return <article className="native-profile-card" aria-label={`Codex home ${profile.id}`}>
    <header><div><h2>{profile.ownership === 'application_dedicated' ? 'Dedicated Orchestrator home' : 'Existing Codex home'}</h2><code>{profile.homePath}</code></div><strong>{profile.selected ? 'Selected' : 'Available'}</strong></header>
    <dl className="native-profile-facts"><Fact label="Identity" value={profile.id} /><Fact label="Ownership" value={profile.ownership} /><Fact label="Lifecycle" value={profile.lifecycle} /><Fact label="Authentication" value={readiness.authentication} /><Fact label="Sandbox" value={readiness.sandboxInitialization} /><Fact label="WorkspaceWrite canary" value={readiness.workspaceWriteCanary} /><Fact label="MCP reporting" value={readiness.mcpReporting} /></dl>
    <div className="native-profile-actions"><button type="button" disabled={busy !== null || profile.selected} onClick={onSelect}>Select</button><button type="button" disabled={busy !== null} onClick={onRefresh}>Refresh login status</button><button type="button" disabled={busy !== null} onClick={onLogin}>Request browser login</button><button type="button" disabled={busy !== null} onClick={onSandbox}>Request sandbox initialization</button><button type="button" disabled={busy !== null} onClick={onCanary}>Run WorkspaceWrite canary</button><button type="button" disabled={busy !== null} onClick={onMcp}>Start MCP/reporting probe</button></div>
    <Attention facts={readiness.attentions} />
  </article>;
}
function Fact({ label, value }: { readonly label: string; readonly value: string }) { return <div><dt>{label}</dt><dd>{value}</dd></div>; }
function Attention({ facts }: { readonly facts: NativeProfile['readiness']['attentions'] }) { const items = Object.entries(facts).filter(([, value]) => value); return <section className="native-profile-attention" aria-label="Profile attention facts"><h3>Attention facts</h3>{items.length ? <ul>{items.map(([key, value]) => <li key={key}><strong>{key}:</strong> {value}</li>)}</ul> : <p>No attention recorded.</p>}</section>; }
