export interface VoicePrompts {
  relevance: string;
  completion: string;
  continuation: string;
  summary: string;
  router: string;
  digest: string;
  output: string;
  tools: string;
}
export interface VoiceSettings {
  bindAddress: string;
  token: string;
  profileId: string;
  workingDirectory: string | null;
  instructions: string;
  prompts: VoicePrompts;
}
export interface VoiceView {
  diagnostics?: {
    updatedAt: string;
    phoneConnected: boolean;
    capture: { recording: boolean; requestedMicrophone: string; routedMicrophone: string; external: boolean; silenced: boolean; level: number; framesRead: number; segmentsSent: number; error: string | null; updatedAt: string } | null;
    audio: { received: number; transcribed: number; lastTranscript: string | null; lastReceivedAt: string | null; lastTranscribedAt: string | null; error: string | null };
    pendingAudio: number;
    input: { title: string; detail: string; last: { text: string; relevance: string | null; completion: string | null; filterPending: boolean; createdAt: string } | null };
    stages: { role: string; sessionId: string; status: string | null; queued: number; error: string | null }[];
    error: string | null;
  };
  defaultPrompts: VoicePrompts;
  profiles: { id: string; name: string }[];
  state: {
    settings: VoiceSettings;
    enabled: boolean;
    phoneConnected: boolean;
    speechConfigured: boolean;
    error: string | null;
    speaking?: boolean;
    pendingAudio?: number;
    conversation: {
      id: string;
      mainSessionId: string;
      pipeline?: { filter: string; summary: string; digest: string; output: string } | null;
      inputBuffer?: { blocks: { id: string; text: string }[]; complete: boolean };
      delegations: { sessionId: string; task: string; invocationId: string | null }[];
      pendingTopics: { id: string; title: string; material: string }[];
      utterances: { id: string; kind: string; text: string; createdAt: string }[];
      inputs: { id: string; text: string; createdAt: string; relevance?: string | null; completion?: string | null; filterPending?: boolean }[];
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
