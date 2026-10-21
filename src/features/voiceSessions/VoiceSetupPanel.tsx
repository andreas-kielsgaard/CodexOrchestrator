import type { VoiceSettings, VoiceView } from '../../application/voiceSessions';

export function VoiceSetupPanel({
  view,
  settings,
  busy,
  onEdit,
  onNewConversation,
  onOpenProfile,
}: {
  view: VoiceView;
  settings: VoiceSettings;
  busy: boolean;
  onEdit(change: Partial<VoiceSettings>): void;
  onNewConversation(): void;
  onOpenProfile?(id: string): void;
}) {
  const disabled = view.state.enabled || busy;
  const saved = JSON.stringify(settings) === JSON.stringify(view.state.settings);
  return (
    <section className="voice-panel voice-setup">
      <h2>Connect your phone</h2>
      <p>Connect both devices to Tailscale. Keep Orchid running and the laptop awake.</p>
      <p className="voice-muted">
        {view.state.enabled
          ? 'Stop the connection to edit these settings.'
          : 'Save any changes, then start the connection using the control above.'}
      </p>
      <label>
        Laptop Tailscale address and port
        <input
          disabled={disabled}
          value={settings.bindAddress}
          onChange={(e) => onEdit({ bindAddress: e.target.value })}
          placeholder="100.x.x.x:4818"
        />
      </label>
      <label>
        Phone endpoint
        <input readOnly value={`ws://${settings.bindAddress}/phone`} />
      </label>
      <label>
        Access token
        <input type="password" readOnly value={settings.token} />
      </label>
      <button onClick={() => void navigator.clipboard.writeText(settings.token)}>Copy token</button>
      <h3>Conversation environment</h3>
      <p>Profile and working directory apply when starting a new conversation.</p>
      <label>
        Capability Profile
        <select
          disabled={disabled}
          value={settings.profileId}
          onChange={(e) => onEdit({ profileId: e.target.value })}
        >
          <option value="">Choose a laptop profile</option>
          {view.profiles.map((profile) => (
            <option key={profile.id} value={profile.id}>
              {profile.name}
            </option>
          ))}
        </select>
      </label>
      {settings.profileId && onOpenProfile && (
        <button onClick={() => onOpenProfile(settings.profileId)}>
          Inspect profile and integrations
        </button>
      )}
      <label>
        Working directory
        <input
          disabled={disabled}
          value={settings.workingDirectory ?? ''}
          onChange={(e) => onEdit({ workingDirectory: e.target.value || null })}
          placeholder="Leave empty for an Orchid workspace"
        />
      </label>
      {view.state.conversation && (
        <>
          <button disabled={disabled || !saved || !settings.profileId} onClick={onNewConversation}>
            Start new conversation
          </button>
          <p className="voice-muted">
            Creates fresh role sessions with the saved environment and models. Previously delegated
            work continues independently.
          </p>
        </>
      )}
    </section>
  );
}
