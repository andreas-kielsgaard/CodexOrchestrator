export interface VoiceSettings {
  bindAddress: string;
  token: string;
  profileId: string;
  workingDirectory: string | null;
  instructions: string;
}
export interface VoiceView {
  profiles: { id: string; name: string }[];
  state: {
    settings: VoiceSettings;
    enabled: boolean;
    phoneConnected: boolean;
    speechConfigured: boolean;
    error: string | null;
    conversation: {
      id: string;
      mainSessionId: string;
      delegations: { sessionId: string; task: string; invocationId: string | null }[];
      pendingTopics: { id: string; title: string; material: string }[];
      utterances: { id: string; kind: string; text: string; createdAt: string }[];
      inputs: { id: string; text: string; createdAt: string }[];
      playback: { utteranceId: string; playedMs: number; state: string } | null;
    } | null;
  };
}
export interface VoiceSessionClient {
  read(): Promise<VoiceView>;
  configure(settings: VoiceSettings): Promise<VoiceView>;
  start(fresh: boolean): Promise<VoiceView>;
  stop(): Promise<VoiceView>;
  sendText(text: string): Promise<unknown>;
}
