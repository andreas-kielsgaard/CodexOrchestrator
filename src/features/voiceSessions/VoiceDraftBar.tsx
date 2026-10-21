import type { VoiceSettings } from '../../application/voiceSessions';

export function VoiceDraftBar({
  settings,
  saved,
  locked,
  onSave,
  onDiscard,
}: {
  settings: VoiceSettings;
  saved: VoiceSettings;
  locked: boolean;
  onSave(): void;
  onDiscard(): void;
}) {
  const changes: string[] = [];
  for (const key of Object.keys(settings.prompts) as (keyof VoiceSettings['prompts'])[]) {
    if (settings.prompts[key] !== saved.prompts[key])
      changes.push(
        `${key === 'tools' ? 'Shared tool discovery' : key[0].toUpperCase() + key.slice(1)} prompt`,
      );
  }
  for (const group of ['response', 'routing'] as const) {
    if (JSON.stringify(settings.models[group]) !== JSON.stringify(saved.models[group]))
      changes.push(`${group === 'response' ? 'Response' : 'Routing'} models · new conversations`);
  }
  if ((settings.workflow?.directReactions ?? true) !== (saved.workflow?.directReactions ?? true))
    changes.push(
      `Direct reaction route: ${settings.workflow?.directReactions ? 'enabled' : 'disabled'}`,
    );
  if (
    JSON.stringify(settings.workflow?.notificationSounds ?? {}) !==
    JSON.stringify(saved.workflow?.notificationSounds ?? {})
  )
    changes.push('Turn notification sounds');
  for (const [key, name] of [
    ['bindAddress', 'Connection address'],
    ['profileId', 'Capability profile'],
    ['workingDirectory', 'Working directory'],
    ['instructions', 'Routing preferences'],
    ['token', 'Access token'],
  ] as const) {
    if (settings[key] !== saved[key]) changes.push(name);
  }
  return (
    <section
      className={`voice-draft-bar${changes.length ? ' is-dirty' : ''}`}
      aria-label="Voice settings draft"
    >
      {changes.length ? (
        <details>
          <summary>
            Review {changes.length} unsaved {changes.length === 1 ? 'change' : 'changes'}
          </summary>
          <ul>
            {changes.map((change) => (
              <li key={change}>{change}</li>
            ))}
          </ul>
          <p>
            Save applies all changes across Voice. Prompts affect subsequent turns; models, profile
            and working directory affect new conversations.
          </p>
        </details>
      ) : (
        <span>Saved configuration</span>
      )}
      {!!changes.length && (
        <div>
          <button disabled={locked} onClick={onDiscard}>
            Discard changes
          </button>
          <button disabled={locked} onClick={onSave}>
            Save all changes
          </button>
        </div>
      )}
    </section>
  );
}
