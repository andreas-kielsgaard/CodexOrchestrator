import type { SessionFolderTarget } from './organization';
import type { SessionExecutionTargetDto } from '../executionTargets/contracts';
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
  readonly executionTarget?: SessionExecutionTargetDto;
}
export interface AgentSessionProfileClient {
  loadCurrentProfile?(sessionId: string): Promise<PinnedAgentSessionProfileDto>;
  sendPreparedMessage?(
    input: import('./preparation').SendPreparedAgentSessionMessageInput,
  ): Promise<{ sessionId: string; invocationId: string }>;
  loadPreparation?(
    sessionId: string,
  ): Promise<import('./preparation').SessionPreparationDto | null>;
  cancelPreparation?(invocationId: string): Promise<void>;
  retryPreparation?(invocationId: string): Promise<void>;
  requestTargetTransition?(
    input: import('./preparation').RequestSessionTargetTransitionInput,
  ): Promise<import('./preparation').SessionTargetTransitionDto>;
  loadTargetTransition?(
    sessionId: string,
  ): Promise<import('./preparation').SessionTargetTransitionDto | null>;
  startTargetTransition?(
    sessionId: string,
  ): Promise<import('./preparation').SessionTargetTransitionDto>;
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
