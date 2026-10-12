import { useState } from 'react';
import type { VoicePrompts, VoiceSettings, VoiceView } from '../../application/voiceSessions';

export function VoiceSetupPanel({
  view,
  settings,
  busy,
  onEdit,
  onSave,
}: {
  view: VoiceView;
  settings: VoiceSettings;
  busy: boolean;
  onEdit: (change: Partial<VoiceSettings>) => void;
  onSave: () => void;
}) {
  const state = view.state;
  const edit = onEdit;
  const [promptStage, setPromptStage] = useState<keyof VoicePrompts>('relevance');
  return (
    <section className="voice-panel">
      <h2>Extension setup</h2>
      <p>Connect both devices to Tailscale. Keep Orchid running and the laptop awake.</p>
      <label>
        Laptop Tailscale address and port
        <input
          disabled={state.enabled}
          value={settings.bindAddress}
          onChange={(event) => edit({ bindAddress: event.target.value })}
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
      <label>
        Capability Profile
        <select
          disabled={state.enabled}
          value={settings.profileId}
          onChange={(event) => edit({ profileId: event.target.value })}
        >
          <option value="">Choose a laptop profile</option>
          {view.profiles.map((profile) => (
            <option key={profile.id} value={profile.id}>
              {profile.name}
            </option>
          ))}
        </select>
      </label>
      <label>
        Working directory
        <input
          disabled={state.enabled}
          value={settings.workingDirectory ?? ''}
          onChange={(event) => edit({ workingDirectory: event.target.value || null })}
          placeholder="Leave empty for an Orchid workspace"
        />
      </label>
      <p className="voice-muted">
        Conversation agents receive Orchid session tools for discovery, delegation, steering, and
        runtime requests.
      </p>
      <details>
        <summary>Voice models</summary>
        {(['response', 'routing'] as const).map((role) => (
          <fieldset key={role} disabled={state.enabled}>
            <legend>
              {role === 'response'
                ? 'Immediate response, digestion and output'
                : 'Work routing and new workers'}
            </legend>
            <label>
              Model
              <input
                value={settings.models[role].model}
                onChange={(event) =>
                  edit({
                    models: {
                      ...settings.models,
                      [role]: { ...settings.models[role], model: event.target.value },
                    },
                  })
                }
              />
            </label>
            <label>
              Reasoning effort
              <select
                value={settings.models[role].reasoningMode}
                onChange={(event) =>
                  edit({
                    models: {
                      ...settings.models,
                      [role]: { ...settings.models[role], reasoningMode: event.target.value },
                    },
                  })
                }
              >
                {['low', 'medium', 'high', 'xhigh', 'max', 'ultra'].map((effort) => (
                  <option key={effort} value={effort}>
                    {effort}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <input
                type="checkbox"
                checked={settings.models[role].fastMode}
                onChange={(event) =>
                  edit({
                    models: {
                      ...settings.models,
                      [role]: { ...settings.models[role], fastMode: event.target.checked },
                    },
                  })
                }
              />
              Fast mode
            </label>
          </fieldset>
        ))}
        <p className="voice-muted">
          Save setup, then start a new conversation to use these models. Existing chats retain their
          settings.
        </p>
      </details>
      <details>
        <summary>Voice prompts</summary>
        <label>
          Stage
          <select
            value={promptStage}
            onChange={(event) => setPromptStage(event.target.value as keyof VoicePrompts)}
          >
            <option value="relevance">JEV relevance</option>
            <option value="completion">JEV input completion</option>
            <option value="continuation">JEV continuation</option>
            <option value="reaction">JEV reaction requests</option>
            <option value="summary">Input summarization</option>
            <option value="router">Routing</option>
            <option value="digest">Digest</option>
            <option value="output">Output</option>
            <option value="tools">Shared tool discovery</option>
          </select>
        </label>
        <label>
          Prompt
          <textarea
            disabled={state.enabled}
            rows={16}
            value={settings.prompts[promptStage]}
            onChange={(event) =>
              edit({ prompts: { ...settings.prompts, [promptStage]: event.target.value } })
            }
          />
        </label>
        <button
          disabled={busy || state.enabled}
          onClick={() =>
            edit({
              prompts: { ...settings.prompts, [promptStage]: view.defaultPrompts[promptStage] },
            })
          }
        >
          Reset this prompt
        </button>
        <p className="voice-muted">
          Stop the connection to edit. Save setup applies prompts to subsequent evaluations and
          agent turns.
        </p>
      </details>
      <details>
        <summary>Routing preferences</summary>
        <textarea
          disabled={state.enabled}
          rows={16}
          value={settings.instructions}
          onChange={(event) => edit({ instructions: event.target.value })}
        />
      </details>
      <button disabled={busy || state.enabled} onClick={onSave}>
        Save setup
      </button>
    </section>
  );
}
