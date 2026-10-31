import type { VoiceSettings, VoiceView } from '../../application/voiceSessions';

export function VoiceSetupPanel({
  view,
  settings,
  busy,
  onEdit,
}: {
  view: VoiceView;
  settings: VoiceSettings;
  busy: boolean;
  onEdit(change: Partial<VoiceSettings>): void;
  onNewConversation?(): void;
  onOpenProfile?(id: string): void;
}) {
  const disabled = view.state.enabled || busy;
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
    </section>
  );
}
