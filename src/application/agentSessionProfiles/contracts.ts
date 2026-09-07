import type {
  DirectUserInvocationResolutionDto,
  SessionCreationResolutionDto,
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
}

export interface SendDirectUserAgentSessionMessageResultDto {
  readonly sessionId: string;
  readonly invocationId: string;
  readonly invocationResolution: DirectUserInvocationResolutionDto;
}

export interface AgentSessionProfileClient {
  startDirectUserSession(
    input: Omit<SendDirectUserAgentSessionMessageInput, 'sessionId'> & {
      readonly title: string | null;
      readonly workingDirectory: string | null;
    },
  ): Promise<SendDirectUserAgentSessionMessageResultDto>;
  loadPinnedProfile(sessionId: string): Promise<PinnedAgentSessionProfileDto>;
  sendDirectUserMessage(
    input: SendDirectUserAgentSessionMessageInput,
  ): Promise<SendDirectUserAgentSessionMessageResultDto>;
}
