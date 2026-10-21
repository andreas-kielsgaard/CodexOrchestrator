import type { VoiceSettings, VoiceView } from '../../application/voiceSessions';
import { voicePromptName, type VoiceBehaviorTarget } from './voiceWorkflowPresentation';

export function VoiceBehaviorEditor({
  target,
  view,
  settings,
  busy,
  onEdit,
  onOpenProfile,
}: {
  target: VoiceBehaviorTarget;
  view: VoiceView;
  settings: VoiceSettings;
  busy: boolean;
  onEdit(change: Partial<VoiceSettings>): void;
  onOpenProfile?(id: string): void;
}) {
  const disabled = view.state.enabled || busy;
  const affected =
    target.kind === 'model'
      ? target.group === 'routing'
        ? 'Work router and new workers'
        : 'Quick receipt, Digest and Output'
      : target.kind === 'preferences'
        ? 'Work router'
        : target.key === 'tools'
          ? 'Conversation agents and delegated workers'
          : view.workflow?.nodes
              .filter((n) => n.promptKeys.includes(target.key))
              .map((n) => n.name)
              .join(', ');
  return (
    <section className="voice-panel voice-behavior-editor" aria-label="Behavior editor">
      <h2>
        {target.kind === 'model'
          ? `${target.group === 'response' ? 'Response' : 'Routing'} model group`
          : target.kind === 'preferences'
            ? 'Routing preferences'
            : voicePromptName(target.key)}
      </h2>
      <p>
        <strong>Affects:</strong> {affected}
      </p>
      <p className="voice-muted">
        {view.state.enabled ? 'Stop the connection to edit. ' : ''}
        {target.kind === 'model'
          ? 'Save, then start a new conversation to use these models. Existing sessions retain their model settings.'
          : 'Changes stay in the shared Voice draft until saved, then apply to subsequent evaluations and agent turns.'}
      </p>
      {target.kind === 'prompt' && target.key === 'tools' && (
        <div className="voice-dependency-summary">
          <h3>Tools and integrations</h3>
          <p>
            Voice supplies tools to read conversation context, delegate and report work, request
            output and publish speech. Each role has its own permissions. These instructions guide
            discovery; editing them does not register a new tool.
          </p>
          <p>
            Additional tools and integrations come from the Capability Profile:{' '}
            <strong>
              {view.profiles.find((p) => p.id === settings.profileId)?.name ?? 'Selected profile'}
            </strong>
            .
          </p>
          {onOpenProfile && settings.profileId && (
            <button onClick={() => onOpenProfile(settings.profileId)}>
              Inspect profile and integrations
            </button>
          )}
        </div>
      )}
      {target.kind === 'model' ? (
        <fieldset disabled={disabled}>
          <legend>Shared model settings</legend>
          <label>
            Model
            <input
              value={settings.models[target.group].model}
              onChange={(e) =>
                onEdit({
                  models: {
                    ...settings.models,
                    [target.group]: { ...settings.models[target.group], model: e.target.value },
                  },
                })
              }
            />
          </label>
          <label>
            Reasoning effort
            <select
              value={settings.models[target.group].reasoningMode}
              onChange={(e) =>
                onEdit({
                  models: {
                    ...settings.models,
                    [target.group]: {
                      ...settings.models[target.group],
                      reasoningMode: e.target.value,
                    },
                  },
                })
              }
            >
              {['low', 'medium', 'high', 'xhigh', 'max', 'ultra'].map((value) => (
                <option key={value}>{value}</option>
              ))}
            </select>
          </label>
          <label className="voice-check">
            <input
              type="checkbox"
              checked={settings.models[target.group].fastMode}
              onChange={(e) =>
                onEdit({
                  models: {
                    ...settings.models,
                    [target.group]: {
                      ...settings.models[target.group],
                      fastMode: e.target.checked,
                    },
                  },
                })
              }
            />
            Fast mode
          </label>
        </fieldset>
      ) : (
        <>
          <label>
            Instructions
            <textarea
              aria-label={
                target.kind === 'preferences' ? 'Routing preferences' : voicePromptName(target.key)
              }
              rows={20}
              disabled={disabled}
              value={
                target.kind === 'preferences' ? settings.instructions : settings.prompts[target.key]
              }
              onChange={(e) =>
                onEdit(
                  target.kind === 'preferences'
                    ? { instructions: e.target.value }
                    : { prompts: { ...settings.prompts, [target.key]: e.target.value } },
                )
              }
            />
          </label>
          {target.kind === 'prompt' && (
            <button
              disabled={disabled}
              onClick={() =>
                onEdit({
                  prompts: { ...settings.prompts, [target.key]: view.defaultPrompts[target.key] },
                })
              }
            >
              Restore default prompt
            </button>
          )}
        </>
      )}
    </section>
  );
}
