import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
interface Settings {
  deviceId: string;
  serverUrl: string | null;
  tokenConfigured: boolean;
  sessions: 'retained' | 'cache';
  knownSessions: number;
  completeSessions: number;
}
export function DeviceSynchronizationSettings() {
  const [settings, setSettings] = useState<Settings>();
  const [url, setUrl] = useState('');
  const [token, setToken] = useState('');
  const [retention, setRetention] = useState<'retained' | 'cache'>('retained');
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState(false);
  const load = async () => {
    const value = await invoke<Settings>('load_device_sync_settings');
    setSettings(value);
    setUrl(value.serverUrl ?? '');
    setRetention(value.sessions);
  };
  useEffect(() => {
    void load().catch((e) => setMessage(String(e)));
  }, []);
  const save = async () => {
    setBusy(true);
    try {
      await invoke('save_device_sync_settings', {
        url: url || null,
        token: token || null,
        sessions: retention,
      });
      setToken('');
      await load();
      setMessage('Saved. Restart Orchid to apply the server connection. Local work can continue.');
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section aria-labelledby="device-synchronization-title">
      <h2 id="device-synchronization-title">Synchronization server</h2>
      <p>Optional. This device keeps executing work when the server is unavailable.</p>
      {settings && (
        <p>
          Device: {settings.deviceId}. Full copied histories: {settings.completeSessions} of{' '}
          {settings.knownSessions}.
        </p>
      )}
      <label>
        Server address{' '}
        <input
          value={url}
          placeholder="https://orchid.example"
          onChange={(e) => setUrl(e.target.value)}
        />
      </label>
      <label>
        Service token{' '}
        <input
          type="password"
          autoComplete="off"
          value={token}
          placeholder={
            settings?.tokenConfigured ? 'Saved token; leave blank to keep' : 'Service token'
          }
          onChange={(e) => setToken(e.target.value)}
        />
      </label>
      <label>
        Conversations from other devices{' '}
        <select
          value={retention}
          onChange={(e) => setRetention(e.target.value as 'retained' | 'cache')}
        >
          <option value="retained">Retain complete copies</option>
          <option value="cache">Cache when opened</option>
        </select>
      </label>
      <button type="button" disabled={busy} onClick={() => void save()}>
        Save synchronization settings
      </button>
      {settings?.sessions === 'cache' && (
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            void invoke('evict_cached_session_copies')
              .then(load)
              .catch((e) => setMessage(String(e)));
          }}
        >
          Clear cached conversation copies
        </button>
      )}
      {message && <p role="status">{message}</p>}
    </section>
  );
}
