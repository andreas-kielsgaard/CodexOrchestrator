import { useEffect, useRef, useState } from 'react';
import type { VoiceSessionClient, VoiceSettings, VoiceView } from '../../application/voiceSessions';
import type { JevQueryClient } from '../../application/agentSessions/jevQueries';
import { VoiceSetupPanel } from './VoiceSetupPanel';
import {
  WorkflowDesignEditor,
  type WorkflowDesignEditorProps,
} from '../workflowAuthoring/WorkflowDesignEditor';
import { VoiceActivityPanel } from './VoiceActivityPanel';
import { VoiceDraftBar } from './VoiceDraftBar';
import './voiceSessions.css';
import './voiceWorkflow.css';
import { JevQueryInspector } from '../agentProviders/jev/JevQueryInspector';
import { voiceQuestionSubject } from './voiceJevQueryPresentation';

export function VoiceSessionsScreen({
  client,
  onOpenSession,
  onOpenProfile,
  workflowEditor,
  queryClient,
  active = true,
}: {
  client: VoiceSessionClient;
  onOpenSession(id: string, invocationId?: string): void;
  onOpenProfile?(id: string): void;
  queryClient?: JevQueryClient;
  workflowEditor?: WorkflowDesignEditorProps;
  active?: boolean;
}) {
  const [view, setView] = useState<VoiceView>();
  const [settings, setSettings] = useState<VoiceSettings>();
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState<'workflow' | 'activity' | 'connection' | 'evaluations'>(
    'workflow',
  );
  const [instanceId, setInstanceId] = useState<string>();
  const lastSavedSettings = useRef<VoiceSettings | undefined>(undefined);
  useEffect(() => {
    if (!active) return;
    let alive = true;
    const read = () =>
      client
        .read()
        .then((result) => {
          if (!alive) return;
          setView(result);
          const previous = lastSavedSettings.current;
          lastSavedSettings.current = result.state.settings;
          setSettings((current) =>
            !current || JSON.stringify(current) === JSON.stringify(previous)
              ? result.state.settings
              : current,
          );
        })
        .catch((error: unknown) => {
          if (alive) setError(String(error));
        });
    void read();
    const timer = setInterval(() => void read(), 2000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [client, active]);
  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    setError('');
    try {
      await action();
      setView(await client.read());
      return true;
    } catch (error) {
      setError(String(error));
      return false;
    } finally {
      setBusy(false);
    }
  };
  if (!view || !settings)
    return (
      <main className="voice-screen">
        <h1>Voice workflow</h1>
        <p>{error || 'Loading conversation…'}</p>
      </main>
    );
  const state = view.state;
  const saved = JSON.stringify(settings) === JSON.stringify(state.settings);
  const edit = (change: Partial<VoiceSettings>) =>
    setSettings((current) => (current ? { ...current, ...change } : current));
  const saveSettings = async () => {
    const submitted = settings;
    const result = await client.configure(submitted);
    setSettings((current) => (current === submitted ? result.state.settings : current));
  };
  return (
    <main className="voice-screen">
      <header className="voice-page-header">
        <div>
          <h1>Voice workflow</h1>
          <p>See how speech becomes actions and responses.</p>
        </div>
        <div className="voice-connection-actions">
          <span className={`voice-status ${state.phoneConnected ? 'connected' : ''}`}>
            {state.phoneConnected
              ? 'Phone connected'
              : state.enabled
                ? 'Waiting for phone'
                : 'Connection stopped'}
          </span>
          {state.enabled ? (
            <button disabled={busy} onClick={() => void run(() => client.stop())}>
              Stop connection
            </button>
          ) : (
            <button
              disabled={busy || !(instanceId || state.conversation?.workflowInstanceId) || !saved}
              onClick={() => void run(() => client.start(false, instanceId))}
            >
              Start connection
            </button>
          )}
        </div>
      </header>
      {(error || state.error) && (
        <p role="alert" className="voice-error">
          {error || state.error}
        </p>
      )}
      <label>
        Voice instance
        <select
          disabled={busy || state.enabled}
          value={instanceId ?? state.conversation?.workflowInstanceId ?? ''}
          onChange={(e) => setInstanceId(e.target.value)}
        >
          <option value="">Create or choose an instance</option>
          {view.instances?.map((instance) => (
            <option key={instance.id} value={instance.id}>
              {instance.name}
            </option>
          ))}
        </select>
      </label>
      <p>
        Activate your design and create an instance with a worktree below. Design edits apply to new
        instances; reconnecting resumes the selected instance.
      </p>
      {!saved && !state.enabled && (
        <p className="voice-muted">Save or discard your draft before starting the connection.</p>
      )}
      <div className="voice-navigation">
        <nav className="voice-tabs" aria-label="Voice views">
          {(['workflow', 'activity', 'evaluations', 'connection'] as const).map((item) => (
            <button key={item} aria-pressed={tab === item} onClick={() => setTab(item)}>
              {item[0].toUpperCase() + item.slice(1)}
            </button>
          ))}
        </nav>
        {tab === 'connection' && (
          <VoiceDraftBar
            settings={settings}
            saved={state.settings}
            locked={busy || state.enabled}
            onSave={() => void run(saveSettings)}
            onDiscard={() => setSettings(state.settings)}
          />
        )}
      </div>
      <div hidden={tab !== 'workflow'}>
        {workflowEditor ? (
          <WorkflowDesignEditor
            {...workflowEditor}
            recipeId={view.recipeId}
            onOpenInstance={setInstanceId}
          />
        ) : (
          <p>Workflow editing is unavailable.</p>
        )}
      </div>
      <div hidden={tab !== 'activity'}>
        <VoiceActivityPanel
          view={view}
          busy={busy}
          onOpenSession={onOpenSession}
          onSend={(text) => run(() => client.sendText(text))}
          onShowRoutes={() => {
            setTab('workflow');
          }}
        />
      </div>
      {queryClient &&
        Object.entries(state.conversation?.evaluationSessions ?? {}).map(([nodeId, sessionId]) => (
          <div key={nodeId} hidden={tab !== 'evaluations'}>
            <h2>{nodeId}</h2>
            <JevQueryInspector
              client={queryClient}
              sessionId={sessionId}
              active={active && tab === 'evaluations'}
              applications={state.conversation?.jevApplications}
              questionSubject={voiceQuestionSubject}
            />
          </div>
        ))}
      <div hidden={tab !== 'connection'}>
        <VoiceSetupPanel
          view={view}
          settings={settings}
          busy={busy}
          onEdit={edit}
          onOpenProfile={onOpenProfile}
        />
      </div>
    </main>
  );
}
