import type { SessionFolderTarget } from './organization';
import type {
  DirectUserInvocationResolutionDto,
  SessionCreationResolutionDto,
  SandboxModeDto,
} from '../executionConfiguration';

export interface PinnedAgentSessionProfileDto {
  readonly sessionId: string;
  readonly creationResolution: SessionCreationResolutionDto;
}

/** Message-local choices; they do not mutate the pinned Session Profile. */
export interface SendDirectUserAgentSessionMessageInput {
  readonly sessionId: string;
  readonly submittedText: string;
  readonly model: string | null;
  readonly reasoningMode: string | null;
  readonly sandboxMode?: SandboxModeDto | null;
}

export interface SendDirectUserAgentSessionMessageResultDto {
  readonly sessionId: string;
  readonly invocationId: string;
  readonly invocationResolution: DirectUserInvocationResolutionDto;
}

export interface StartDirectUserAgentSessionInput extends Omit<
  SendDirectUserAgentSessionMessageInput,
  'sessionId'
> {
  readonly title: string | null;
  readonly workingDirectory: string | null;
  readonly folderTarget?: SessionFolderTarget | null;
}
export interface AgentSessionProfileClient {
  loadQuickFeatures?(
    input: import('./quickFeatures').LoadAgentSessionQuickFeaturesInput,
  ): Promise<import('./quickFeatures').AgentSessionQuickFeatures>;
  startDirectUserSession(
    input: StartDirectUserAgentSessionInput,
  ): Promise<SendDirectUserAgentSessionMessageResultDto>;
  loadPinnedProfile(sessionId: string): Promise<PinnedAgentSessionProfileDto>;
  sendDirectUserMessage(
    input: SendDirectUserAgentSessionMessageInput,
  ): Promise<SendDirectUserAgentSessionMessageResultDto>;
}
