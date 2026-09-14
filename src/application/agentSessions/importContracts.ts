export interface CodexImportPreview {
  threadId: string;
  title: string;
  sourceDirectory: string | null;
  allocateWorkspace: boolean;
  turnCount: number;
  lastTurnId: string;
  nativeHome: string;
  profileId: string;
  capabilityProfile: string;
  excerpt: string;
}
export interface CodexImportCommand {
  requestId: string;
  link: string;
  profileId: string;
  lastTurnId: string;
}
export interface AgentSessionImportClient {
  preview(link: string): Promise<CodexImportPreview>;
  importConversation(command: CodexImportCommand): Promise<string>;
}
