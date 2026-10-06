import { invoke } from '@tauri-apps/api/core';
import type { VoiceSessionClient } from '../../application/voiceSessions';
export const tauriVoiceSessionClient: VoiceSessionClient = {
  read: () => invoke('voice_session_view'),
  configure: (settings) => invoke('configure_voice_session', { settings }),
  start: (fresh) => invoke('start_voice_session', { fresh }),
  stop: () => invoke('stop_voice_session'),
  sendText: (text) => invoke('send_voice_session_text', { text }),
};
