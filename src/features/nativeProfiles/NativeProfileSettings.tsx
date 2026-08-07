import { useCallback, useEffect, useRef, useState } from 'react';
import type { NativeProfile, NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import './nativeProfileSettings.css';

export function NativeProfileSettings({ client }: { readonly client: NativeProfileClient }) {
  const [profiles, setProfiles] = useState<readonly NativeProfile[]>([]);
  const [homePath, setHomePath] = useState('');
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState('Loading durable Codex home profiles.');
  const requestVersion = useRef(0);
  const refresh = useCallback(async () => {
    const version = ++requestVersion.current;
    try {
      const result = await client.load();
      if (version !== requestVersion.current) return;
      setProfiles(result.profiles);
      setMessage('Showing current durable profile state.');
    } catch (error) {
      if (version !== requestVersion.current) return;
      setProfiles([]);
      setMessage(error instanceof Error ? error.message : 'Profile state is unavailable.');
    }
  }, [client]);
  useEffect(() => { void refresh(); }, [refresh]);
  const run = useCallback(async (name: string, action: () => Promise<{ profiles: readonly NativeProfile[] }>) => {
    if (busy) return;
    const version = ++requestVersion.current;
    const label = actionLabel(name);
    setBusy(name); setMessage('Waiting for durable profile state.');
    try {
      const result = await action();
      if (version !== requestVersion.current) return;
      setProfiles(result.profiles);
      setMessage(actionResultMessage(name, label));
    } catch (error) {
      if (version !== requestVersion.current) return;
      setMessage(`${label} failed: ${error instanceof Error ? error.message : 'the request was rejected.'}`);
    }
    finally { if (version === requestVersion.current) setBusy(null); }
  }, [busy]);
  return (
    <main className="native-profile-settings" aria-label="Technical Codex settings" tabIndex={0}>
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
        <ProfileCard key={profile.id} profile={profile} busy={busy}
          onSelect={() => void run('select', () => client.select(profile.id))}
          onLogin={() => void run('login', () => client.requestLogin(profile.id))}
          onRefresh={() => void run('refresh', () => client.refreshReadiness(profile.id))}
          onSelectExecutionMode={(mode) => void run('mode', () => client.selectExecutionMode(profile.id, mode))}
          onAuthorizeDanger={() => void run('authorizeDanger', () => client.authorizeDangerFullAccess(profile.id))}
          onRevokeDanger={() => void run('revokeDanger', () => client.revokeDangerFullAccess(profile.id))}
          onSandbox={() => void run('sandbox', () => client.initializeSandbox(profile.id))}
          onConfirmSandbox={() => void run('confirmSandbox', () => client.confirmSandboxInitialization(profile.id))}
          onVerifyPreprovisionedSandbox={() => void run('verifyPreprovisionedSandbox', () => client.verifyPreprovisionedSandbox(profile.id))}
          onConfirmPreprovisionedSandboxAdoption={() => void run('confirmPreprovisionedSandboxAdoption', () => client.confirmPreprovisionedSandboxAdoption(profile.id))}
          onCanary={() => void run('canary', () => client.runCanary(profile.id))}
          onMcp={() => void run('mcp', () => client.probeMcp(profile.id))} />
      ))}
    </main>
  );
}

function actionLabel(name: string): string {
  return ({ select: 'Profile selection', mode: 'Execution mode selection', authorizeDanger: 'Danger Full Access authorization', revokeDanger: 'Danger Full Access revocation', login: 'Browser login request', refresh: 'Login status refresh', sandbox: 'Sandbox initialization request', confirmSandbox: 'Sandbox/UAC confirmation', verifyPreprovisionedSandbox: 'External sandbox verification', confirmPreprovisionedSandboxAdoption: 'External sandbox adoption confirmation', canary: 'WorkspaceWrite canary', mcp: 'MCP/reporting probe' } as Record<string, string>)[name] ?? 'Profile action';
}
function actionResultMessage(name: string, label: string): string {
  if (name === 'login') return 'Browser login request returned; review the durable login-attempt state. Browser handoff and authentication are separate facts.';
  if (name === 'sandbox') return 'Sandbox initialization request returned; review the durable setup-attempt facts. Launch acceptance, terminal outcome, UAC confirmation, and sandbox initialization are separate facts.';
  if (name === 'verifyPreprovisionedSandbox') return 'External sandbox verification returned; it records only the selected profile’s current safe configuration observation and does not claim a product setup request or UAC interaction.';
  if (name === 'confirmPreprovisionedSandboxAdoption') return 'External sandbox adoption confirmation returned; it does not claim that this product created the setup or observed UAC.';
  if (name === 'canary') return 'Workspace Write canary request returned; review the durable setup-attempt facts. Request return, launch acceptance, terminal outcome, and canary readiness are separate facts.';
  return `${label} completed; state was refreshed from durable storage.`;
}

function ProfileCard({ profile, busy, onSelect, onLogin, onRefresh, onSelectExecutionMode, onAuthorizeDanger, onRevokeDanger, onSandbox, onConfirmSandbox, onVerifyPreprovisionedSandbox, onConfirmPreprovisionedSandboxAdoption, onCanary, onMcp }: { readonly profile: NativeProfile; readonly busy: string | null; readonly onSelect?: () => void; readonly onLogin?: () => void; readonly onRefresh?: () => void; readonly onSelectExecutionMode?: (mode: NativeProfile['execution']['selectedMode']) => void; readonly onAuthorizeDanger?: () => void; readonly onRevokeDanger?: () => void; readonly onSandbox?: () => void; readonly onConfirmSandbox?: () => void; readonly onVerifyPreprovisionedSandbox?: () => void; readonly onConfirmPreprovisionedSandboxAdoption?: () => void; readonly onCanary?: () => void; readonly onMcp?: () => void }) {
  const { readiness } = profile;
  return <article className="native-profile-card" aria-label={`Codex home ${profile.id}`}>
    <header><div><h2>{profile.ownership === 'application_dedicated' ? 'Dedicated Orchestrator home' : 'Existing Codex home'}</h2><code>{profile.homePath}</code></div><strong>{profile.selected ? 'Selected' : 'Available'}</strong></header>
    <dl className="native-profile-facts"><Fact label="Identity" value={profile.id} /><Fact label="Ownership" value={profile.ownership} /><Fact label="Lifecycle" value={profile.lifecycle} /><Fact label="Authentication" value={readiness.authentication} /><Fact label="Sandbox" value={readiness.sandboxInitialization} /><Fact label="Workspace Write canary" value={readiness.workspaceWriteCanary} /><Fact label="Full-access canary" value={readiness.dangerFullAccessCanary} /><Fact label="MCP reporting" value={readiness.mcpReporting} /></dl>
    <LoginAttempt attempt={profile.loginAttempt} />
    <SetupAttempt attempt={profile.setupAttempt} />
    <SandboxAdoption adoption={profile.sandboxAdoption} confirmation={profile.sandboxAdoptionConfirmation} />
    <ExecutionSettings profile={profile} busy={busy} onSelectExecutionMode={onSelectExecutionMode} onAuthorizeDanger={onAuthorizeDanger} onRevokeDanger={onRevokeDanger} />
    <div className="native-profile-actions"><button type="button" disabled={busy !== null || profile.selected} onClick={onSelect}>Select</button><button type="button" disabled={busy !== null} onClick={onRefresh}>Refresh login status</button><button type="button" disabled={busy !== null} onClick={onLogin}>Request browser login</button><button type="button" disabled={busy !== null} onClick={onSandbox}>Request sandbox initialization</button><button type="button" disabled={busy !== null} onClick={onConfirmSandbox}>Confirm sandbox/UAC completion</button><button type="button" disabled={busy !== null} onClick={onVerifyPreprovisionedSandbox}>Verify already-provisioned sandbox</button><button type="button" disabled={busy !== null || profile.sandboxAdoption.disposition !== 'verified' || profile.sandboxAdoptionConfirmation.disposition === 'confirmed'} onClick={onConfirmPreprovisionedSandboxAdoption}>Confirm external sandbox adoption</button><button type="button" disabled={busy !== null} onClick={onCanary}>Run WorkspaceWrite canary</button><button type="button" disabled={busy !== null} onClick={onMcp}>Start MCP/reporting probe</button></div>
    <Attention facts={readiness.attentions} />
  </article>;
}
function LoginAttempt({ attempt }: { readonly attempt: NativeProfile['loginAttempt'] }) { return <section className="native-profile-login-attempt" aria-label="Browser login attempt"><h3>Browser login attempt</h3><p><strong>{loginDisposition(attempt.disposition)}</strong></p><dl><Fact label="Browser handoff" value={attempt.browserHandoff} /><Fact label="Requested" value={attempt.requestedAt ?? 'not recorded'} /><Fact label="Launch accepted" value={attempt.launchAcceptedAt ?? 'not observed'} /><Fact label="Terminal disposition" value={attempt.settledAt ?? 'not settled'} /></dl><p>Process activity, browser handoff, and authentication are separate facts.</p></section>; }
function loginDisposition(value: NativeProfile['loginAttempt']['disposition']) { return ({ not_requested: 'No login attempt recorded.', pending: 'Login process launch was accepted; browser handoff is unobserved.', launch_failed: 'Login process was not launched.', terminal_succeeded: 'Login process ended successfully; browser handoff is unobserved.', terminal_failed: 'Login process ended unsuccessfully.', cancelled: 'Login attempt was cancelled.', recovered_unobserved: 'Login attempt was recovered without an owned process to observe.' } as Record<NativeProfile['loginAttempt']['disposition'], string>)[value]; }
function SetupAttempt({ attempt }: { readonly attempt: NativeProfile['setupAttempt'] }) { return <section className="native-profile-login-attempt" aria-label="Native setup attempt"><h3>Native setup attempt</h3><p><strong>{setupDisposition(attempt.disposition)}</strong></p><dl><Fact label="Phase" value={attempt.phase} /><Fact label="Executable" value={attempt.executable ?? 'not recorded'} /><Fact label="Version" value={attempt.version ?? 'not recorded'} /><Fact label="Workspace sandbox capability" value={attempt.workspaceSandboxSupported === null ? 'not recorded' : String(attempt.workspaceSandboxSupported)} /><Fact label="Correlation" value={attempt.correlationId ?? 'not recorded'} /><Fact label="Requested" value={attempt.requestedAt ?? 'not recorded'} /><Fact label="Launch accepted" value={attempt.launchAcceptedAt ?? 'not observed'} /><Fact label="Deadline" value={attempt.deadlineAt ?? 'not recorded'} /><Fact label="Settled" value={attempt.settledAt ?? 'not settled'} /><Fact label="Terminal classification" value={attempt.terminalClassification} /><Fact label="Terminal exit code" value={attempt.terminalExitCode === null ? 'not observed' : String(attempt.terminalExitCode)} /></dl><p>Request return, child launch, terminal outcome, UAC confirmation, sandbox initialization, and canary readiness are separate facts.</p></section>; }
function SandboxAdoption({ adoption, confirmation }: { readonly adoption: NativeProfile['sandboxAdoption']; readonly confirmation: NativeProfile['sandboxAdoptionConfirmation'] }) { return <section className="native-profile-login-attempt" aria-label="External sandbox adoption"><h3>External sandbox adoption</h3><p><strong>External observation: {adoption.disposition.replace('_', ' ')}</strong></p><dl><Fact label="Executable" value={adoption.executable ?? 'not recorded'} /><Fact label="Version" value={adoption.version ?? 'not recorded'} /><Fact label="Workspace command capability" value={adoption.workspaceSandboxSupported === null ? 'not recorded' : String(adoption.workspaceSandboxSupported)} /><Fact label="Windows setup capability" value={adoption.windowsSandboxSetupSupported === null ? 'not recorded' : String(adoption.windowsSandboxSetupSupported)} /><Fact label="Elevated mode observed" value={adoption.elevatedModeObserved === null ? 'not observed' : String(adoption.elevatedModeObserved)} /><Fact label="Observation correlation" value={adoption.correlationId ?? 'not recorded'} /><Fact label="Observed" value={adoption.observedAt ?? 'not recorded'} /><Fact label="Product adoption confirmation" value={confirmation.disposition.replace('_', ' ')} /><Fact label="Confirmation correlation" value={confirmation.correlationId ?? 'not recorded'} /><Fact label="Confirmed" value={confirmation.confirmedAt ?? 'not recorded'} /></dl><p>External configuration observation and explicit product adoption are separate receipts. Neither means this product requested setup, accepted a child launch, observed UAC, or completed a canary.</p></section>; }
function setupDisposition(value: NativeProfile['setupAttempt']['disposition']) { return ({ not_requested: 'No native setup attempt recorded.', pending: 'Native setup process launch was accepted; terminal outcome is not yet observed.', launch_failed: 'Native setup process was not launched.', terminal_succeeded: 'Native setup process ended successfully; UAC confirmation and readiness remain separate.', terminal_failed: 'Native setup process ended unsuccessfully.', timed_out: 'Native setup attempt timed out before a terminal outcome was observed.', cancelled: 'Native setup attempt was cancelled.', recovered_unobserved: 'Native setup attempt was recovered without an owned process to observe.', legacy_unclassified_failed: 'Native setup was durably recorded as failed under the old schema; launch acceptance, terminal category, and exit code were not recorded.', policy_unsupported: 'The installed Codex CLI cannot establish the required application-owned Workspace Write and disabled-network policy without opaque state. No setup process was launched.' } as Record<NativeProfile['setupAttempt']['disposition'], string>)[value]; }
function ExecutionSettings({ profile, busy, onSelectExecutionMode, onAuthorizeDanger, onRevokeDanger }: { readonly profile: NativeProfile; readonly busy: string | null; readonly onSelectExecutionMode?: (mode: NativeProfile['execution']['selectedMode']) => void; readonly onAuthorizeDanger?: () => void; readonly onRevokeDanger?: () => void }) {
  const { selectedMode, dangerFullAccessAuthorized } = profile.execution;
  return <section className="native-profile-execution" aria-labelledby={`execution-${profile.id}`}>
    <h3 id={`execution-${profile.id}`}>Execution mode</h3>
    <p>Workspace Write is the safer default. Danger Full Access is a separate mode and does not authorize itself.</p>
    <fieldset disabled={busy !== null}>
      <legend>Selected mode</legend>
      <label><input type="radio" name={`execution-mode-${profile.id}`} value="workspace_write" checked={selectedMode === 'workspace_write'} onChange={() => onSelectExecutionMode?.('workspace_write')} /> Workspace Write <span>Commands are restricted to the assigned worktree.</span></label>
      <label><input type="radio" name={`execution-mode-${profile.id}`} value="danger_full_access" checked={selectedMode === 'danger_full_access'} onChange={() => onSelectExecutionMode?.('danger_full_access')} /> Danger Full Access <span>Requires separate authorization.</span></label>
    </fieldset>
    <p className="native-profile-danger-warning"><strong>Danger Full Access warning.</strong> This permits commands to read and write outside the assigned worktree and can affect the full machine under the OS rights of the user launching Codex. It does not grant administrator rights or suppress Windows UAC.</p>
    <p className="native-profile-authorization">Authorization: <strong>{dangerFullAccessAuthorized ? 'authorized for this profile' : 'not authorized'}</strong></p>
    {dangerFullAccessAuthorized ? <button type="button" disabled={busy !== null} onClick={onRevokeDanger}>Revoke Danger Full Access authorization</button> : <button type="button" disabled={busy !== null || selectedMode !== 'danger_full_access'} onClick={onAuthorizeDanger}>Authorize Danger Full Access for this profile</button>}
  </section>;
}
function Fact({ label, value }: { readonly label: string; readonly value: string }) { return <div><dt>{label}</dt><dd>{value}</dd></div>; }
function Attention({ facts }: { readonly facts: NativeProfile['readiness']['attentions'] }) { const items = Object.entries(facts).filter(([, value]) => value); return <section className="native-profile-attention" aria-label="Profile attention facts"><h3>Attention facts</h3>{items.length ? <ul>{items.map(([key, value]) => <li key={key}><strong>{key}:</strong> {value}</li>)}</ul> : <p>No attention recorded.</p>}</section>; }
