import type { VoicePrompts } from '../../application/voiceSessions';

export type VoiceBehaviorTarget =
  | { kind: 'prompt'; key: keyof VoicePrompts }
  | { kind: 'model'; group: 'response' | 'routing' }
  | { kind: 'preferences' };

export const voicePromptName = (key: keyof VoicePrompts) =>
  key === 'tools' ? 'Shared tool discovery' : `${key[0].toUpperCase() + key.slice(1)} prompt`;
