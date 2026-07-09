import type {
  AgentConversation,
  AgentConversationEvidence,
  AgentConversationStreamStatus,
} from '../../domain/agentConversation';
import type { OrchestrationTruthState } from '../../domain/orchestrationState';
import type {
  ActivityTimelineItem,
  ConversationMessageItem,
  OrchestrationFileItem,
  OrchestrationStageItem,
} from './types';

export const mockDraftOrchestration: OrchestrationTruthState = {
  provenance: 'local_draft',
  status: 'draft',
};

export const mockReadyToSubmitPromptState: OrchestrationTruthState = {
  provenance: 'user_input',
  status: 'ready',
};

export const mockSubmittingPromptState: OrchestrationTruthState = {
  provenance: 'local_draft',
  status: 'starting',
};

export const mockWaitingForRuntimeAcknowledgementState: OrchestrationTruthState = {
  provenance: 'backend_response',
  status: 'waiting_for_event',
};

export const mockIntegrationPendingState: OrchestrationTruthState = {
  provenance: 'unsupported',
  status: 'integration_pending',
};

export const mockRuntimeRunningEvent: OrchestrationTruthState = {
  provenance: 'runtime_event',
  status: 'running',
};

export const mockBackendFailedResponse: OrchestrationTruthState = {
  provenance: 'backend_response',
  status: 'failed',
};

export const mockRuntimeCompletedEvent: OrchestrationTruthState = {
  provenance: 'runtime_event',
  status: 'completed',
};

export const mockBackendCompletedResponse: OrchestrationTruthState = {
  provenance: 'backend_response',
  status: 'completed',
};

export const mockPreviewOnlyState: OrchestrationTruthState = {
  provenance: 'mock_fixture',
  status: 'mock_preview',
};

export const mockTruthfulStateExamples = [
  {
    note: 'Storybook demo only: local text exists, no backend command has started.',
    state: mockDraftOrchestration,
    title: 'Local draft',
  },
  {
    note: 'Storybook demo only: user input is ready to submit, with no runtime output.',
    state: mockReadyToSubmitPromptState,
    title: 'Ready to submit prompt',
  },
  {
    note: 'Storybook demo only: a local start request is pending acknowledgement.',
    state: mockSubmittingPromptState,
    title: 'Submitting prompt',
  },
  {
    note: 'Example backend acknowledgement fixture: waiting for first runtime event.',
    state: mockWaitingForRuntimeAcknowledgementState,
    title: 'Waiting for runtime acknowledgement',
  },
  {
    note: 'Storybook demo only: integration is intentionally marked unsupported.',
    state: mockIntegrationPendingState,
    title: 'Integration pending',
  },
  {
    note: 'Runtime event-shaped fixture: represents evidence format, not a live run.',
    state: mockRuntimeRunningEvent,
    title: 'Runtime running',
  },
  {
    note: 'Backend response-shaped fixture: failed state is sample evidence only.',
    state: mockBackendFailedResponse,
    title: 'Backend failed',
  },
  {
    note: 'Runtime event-shaped fixture: completion requires backend/runtime evidence.',
    state: mockRuntimeCompletedEvent,
    title: 'Runtime completed',
  },
  {
    note: 'Storybook mock preview only: never runtime-backed.',
    state: mockPreviewOnlyState,
    title: 'Mock preview',
  },
];

export const mockStageList: OrchestrationStageItem[] = [
  {
    description: 'Prompt text is saved in the browser session only.',
    evidenceLabel: 'Demo label: no generated files or thread IDs exist.',
    id: 'mock-stage-draft',
    state: mockDraftOrchestration,
    title: 'Capture local draft',
  },
  {
    description: 'User input is locally valid and ready for a future start command.',
    evidenceLabel: 'Demo label: no backend command has been invoked.',
    id: 'mock-stage-ready',
    state: mockReadyToSubmitPromptState,
    title: 'Review prompt before submit',
  },
  {
    description: 'A local request would be in flight while the UI waits for acknowledgement.',
    evidenceLabel: 'Demo label: pending, with no runtime output yet.',
    id: 'mock-stage-starting',
    isCurrent: true,
    state: mockSubmittingPromptState,
    title: 'Submit plan-builder prompt',
  },
  {
    description: 'Backend support is not wired in this slice.',
    evidenceLabel: 'Demo label: unsupported integration path.',
    id: 'mock-stage-integration-pending',
    state: mockIntegrationPendingState,
    title: 'Runtime integration',
  },
  {
    description: 'Completion can only be shown from backend or runtime evidence.',
    evidenceLabel: 'Runtime event-shaped fixture, not live data.',
    id: 'mock-stage-completed',
    state: mockRuntimeCompletedEvent,
    title: 'Completion evidence',
  },
];

export const mockLongStageList: OrchestrationStageItem[] = [
  {
    description:
      'This intentionally long draft prompt title checks wrapping without implying that a plan builder generated anything.',
    evidenceLabel:
      'Mock/demo-only evidence label with a very long folder path C:/Users/user/Documents/very-long-orchestration-proposal-name/without-runtime-output-yet.',
    id: 'mock-stage-long-name',
    isCurrent: true,
    state: mockReadyToSubmitPromptState,
    title:
      'Review a very long orchestration prompt name that should wrap cleanly in the component shell',
  },
];

export const mockConversationMessages: ConversationMessageItem[] = [
  {
    author: 'User',
    body: 'Draft an orchestration proposal from these local notes. This is Storybook demo input only.',
    id: 'mock-message-user-draft',
    role: 'user',
    sourceLabel: 'Local user input fixture',
    state: mockReadyToSubmitPromptState,
    timestampLabel: 'Demo timestamp',
  },
  {
    author: 'UI',
    body: 'Ready to submit when a future backend integration is available. No Codex thread has been created.',
    id: 'mock-message-system-ready',
    role: 'system',
    sourceLabel: 'Local UI fixture',
    state: mockIntegrationPendingState,
    timestampLabel: 'Demo timestamp',
  },
  {
    author: 'Runtime event fixture',
    body: 'runtime_event fixture received for Storybook review. This sample is not connected to a live run.',
    id: 'mock-message-runtime',
    role: 'runtime',
    sourceLabel: 'Mock runtime_event payload',
    state: mockRuntimeRunningEvent,
    timestampLabel: 'Demo timestamp',
  },
  {
    author: 'Mock assistant',
    body: 'Mock preview content is intentionally labelled and should not be treated as product truth.',
    id: 'mock-message-assistant',
    role: 'mock',
    sourceLabel: 'Mock/demo fixture',
    state: mockPreviewOnlyState,
    timestampLabel: 'Demo timestamp',
  },
];

export const mockManyConversationMessages: ConversationMessageItem[] = Array.from(
  { length: 18 },
  (_, index) => ({
    author: index % 2 === 0 ? 'Mock user' : 'Mock assistant',
    body: `Mock/demo conversation message ${index + 1}. This verifies scrolling and does not represent runtime output.`,
    id: `mock-many-message-${index + 1}`,
    role: index % 2 === 0 ? 'user' : 'mock',
    sourceLabel: 'Mock/demo fixture',
    state: index % 2 === 0 ? mockReadyToSubmitPromptState : mockPreviewOnlyState,
    timestampLabel: 'Demo timestamp',
  }),
);

export const mockUploadedFiles: OrchestrationFileItem[] = [
  {
    detailLabel: 'Local upload fixture',
    evidenceLabel: 'Demo file only; not written by a runtime.',
    id: 'mock-upload-notes',
    kind: 'uploaded',
    name: 'local-orchestration-notes.md',
    state: mockDraftOrchestration,
  },
  {
    detailLabel: 'Long local upload fixture',
    evidenceLabel: 'Demo file only; checks filename wrapping.',
    id: 'mock-upload-long',
    kind: 'uploaded',
    name: 'very-long-local-context-file-name-for-orchestration-ui-storybook-wrapping-review.md',
    state: mockReadyToSubmitPromptState,
  },
  {
    detailLabel: 'Runtime event-shaped fixture',
    evidenceLabel: 'Example evidence shape only, not a generated file from this app.',
    id: 'mock-runtime-evidence',
    kind: 'runtime_evidence',
    name: 'runtime-event-fixture.json',
    state: mockRuntimeRunningEvent,
  },
];

export const mockActivityTimeline: ActivityTimelineItem[] = [
  {
    description: 'The draft exists locally and has not created runtime output.',
    id: 'mock-activity-draft',
    sourceLabel: 'Local draft fixture',
    state: mockDraftOrchestration,
    timestampLabel: 'Demo timestamp',
    title: 'Draft captured',
  },
  {
    description: 'The UI can represent a pending local start request.',
    id: 'mock-activity-starting',
    sourceLabel: 'Local pending fixture',
    state: mockSubmittingPromptState,
    timestampLabel: 'Demo timestamp',
    title: 'Start requested locally',
  },
  {
    description:
      'A backend acknowledgement fixture is present, but runtime output has not arrived.',
    id: 'mock-activity-waiting',
    sourceLabel: 'Backend response-shaped fixture',
    state: mockWaitingForRuntimeAcknowledgementState,
    timestampLabel: 'Demo timestamp',
    title: 'Waiting for runtime acknowledgement',
  },
  {
    description: 'A runtime_event-shaped fixture confirms active work for display review only.',
    id: 'mock-activity-running',
    sourceLabel: 'Runtime event-shaped fixture',
    state: mockRuntimeRunningEvent,
    timestampLabel: 'Demo timestamp',
    title: 'Runtime running example',
  },
  {
    description: 'A backend response-shaped fixture shows a failed command state.',
    id: 'mock-activity-failed',
    sourceLabel: 'Backend response-shaped fixture',
    state: mockBackendFailedResponse,
    timestampLabel: 'Demo timestamp',
    title: 'Backend failed example',
  },
  {
    description: 'Completion is backed by evidence-shaped data in this story fixture.',
    id: 'mock-activity-completed',
    sourceLabel: 'Runtime event-shaped fixture',
    state: mockRuntimeCompletedEvent,
    timestampLabel: 'Demo timestamp',
    title: 'Runtime completed example',
  },
];

const mockAgentConversationEvidenceLabels: Record<AgentConversationStreamStatus, string> = {
  completed: 'Runtime event-shaped fixture; not a live conversation.',
  failed: 'Backend response-shaped fixture; not a live conversation.',
  idle: 'Storybook fixture: no prompt has been submitted.',
  input_ready: 'Storybook fixture: user input is local only.',
  running: 'Runtime event-shaped fixture; not a live conversation.',
  starting: 'Storybook fixture: local optimistic start state only.',
  unavailable: 'Storybook fixture: runtime route is unavailable.',
  unsupported: 'Storybook fixture: integration is unsupported.',
  waiting_for_event: 'Backend response-shaped fixture waiting for first runtime event.',
};

function mockAgentEvidence(
  state: OrchestrationTruthState,
  label: string,
): AgentConversationEvidence {
  return {
    kind:
      state.provenance === 'runtime_event'
        ? state.status === 'completed'
          ? 'completed_runtime_output'
          : 'streamed_runtime_output'
        : state.provenance === 'backend_response'
          ? state.status === 'waiting_for_event'
            ? 'backend_acknowledgement'
            : state.status === 'failed'
              ? 'failed_runtime_output'
              : 'backend_acknowledgement'
          : state.provenance === 'unsupported'
            ? 'unsupported_integration'
            : state.provenance === 'user_input'
              ? 'user_input'
              : state.provenance === 'mock_fixture'
                ? 'mock_demo_fixture'
                : 'local_optimistic_ui',
    label,
    truth: state,
  };
}

function mockAgentConversation(
  streamStatus: AgentConversationStreamStatus,
  truth: OrchestrationTruthState,
  title: string,
): AgentConversation {
  const evidence = mockAgentEvidence(truth, mockAgentConversationEvidenceLabels[streamStatus]);
  const hasRuntimeEvent = truth.provenance === 'runtime_event';
  const hasBackendResponse = truth.provenance === 'backend_response';
  const currentTurn =
    streamStatus === 'starting' ||
    streamStatus === 'waiting_for_event' ||
    streamStatus === 'running'
      ? {
          evidence,
          summary:
            streamStatus === 'waiting_for_event'
              ? 'Backend acknowledgement fixture exists; no runtime event fixture has arrived yet.'
              : streamStatus === 'running'
                ? 'Runtime event-shaped fixture confirms active work for Storybook only.'
                : 'Local optimistic fixture is waiting for backend acknowledgement.',
          title:
            streamStatus === 'waiting_for_event'
              ? 'Waiting for first runtime event'
              : streamStatus === 'running'
                ? 'Runtime output streaming'
                : 'Start requested locally',
          truth,
        }
      : undefined;

  return {
    artifacts:
      streamStatus === 'completed'
        ? [
            {
              detail: 'Fixture output artifact only.',
              evidence,
              id: `${streamStatus}-artifact`,
              kind: hasRuntimeEvent ? 'runtime_evidence' : 'backend_evidence',
              name: 'storybook-plan-output.md',
              truth,
            },
          ]
        : [],
    attachments: [
      {
        detail: 'Storybook fixture upload.',
        evidence: mockAgentEvidence(mockDraftOrchestration, 'Storybook fixture upload only.'),
        id: `${streamStatus}-attachment`,
        kind: 'uploaded',
        name: 'storybook-notes.md',
        truth: mockDraftOrchestration,
      },
    ],
    externalThreadId: hasRuntimeEvent ? `storybook-runtime-thread-${streamStatus}` : undefined,
    id: `storybook-agent-conversation-${streamStatus}`,
    input: {
      disabledReason:
        streamStatus === 'input_ready'
          ? undefined
          : 'Storybook fixture input is disabled unless the state is input-ready.',
      enabled: streamStatus === 'input_ready',
      placeholder: 'Add a Storybook fixture prompt',
    },
    mode: streamStatus === 'input_ready' ? 'interactive' : 'read_only',
    role: 'Plan Builder',
    runtime: {
      providerId: 'storybook-fixture',
      providerLabel: 'Storybook fixture provider',
      runtimeLabel: hasRuntimeEvent
        ? 'Runtime event-shaped fixture'
        : hasBackendResponse
          ? 'Backend response-shaped fixture'
          : 'No live runtime route',
      unsupportedReason:
        streamStatus === 'unsupported'
          ? 'This Storybook fixture marks the integration unsupported.'
          : undefined,
    },
    state: {
      currentTurn,
      evidence,
      latestActivity: 'Demo timestamp',
      streamStatus,
      truth,
      unavailable:
        streamStatus === 'unsupported' || streamStatus === 'unavailable'
          ? {
              detail:
                streamStatus === 'unsupported'
                  ? 'Storybook fixture only: no backend command or runtime thread exists.'
                  : 'Storybook fixture only: runtime state is unavailable.',
              evidence,
              kind: streamStatus,
              title: streamStatus === 'unsupported' ? 'Unsupported fixture' : 'Unavailable fixture',
              truth,
            }
          : undefined,
    },
    title,
    turns: [
      {
        body: 'Storybook fixture user prompt. This is local demo data only.',
        createdAt: 'Demo timestamp',
        evidence: mockAgentEvidence(mockReadyToSubmitPromptState, 'Storybook user input fixture.'),
        id: `${streamStatus}-turn-user`,
        role: 'user',
        truth: mockReadyToSubmitPromptState,
      },
      {
        body:
          streamStatus === 'running'
            ? 'Runtime event-shaped fixture output. This is not a live Codex conversation.'
            : 'Storybook fixture status message. No live runtime output is implied.',
        createdAt: 'Demo timestamp',
        evidence,
        id: `${streamStatus}-turn-status`,
        role: hasRuntimeEvent ? 'runtime' : 'system',
        truth,
      },
    ],
  };
}

export const mockIdleAgentConversation = mockAgentConversation(
  'idle',
  mockDraftOrchestration,
  'Idle fixture conversation',
);

export const mockInputReadyAgentConversation = mockAgentConversation(
  'input_ready',
  mockReadyToSubmitPromptState,
  'Input-ready fixture conversation',
);

export const mockStartingAgentConversation = mockAgentConversation(
  'starting',
  mockSubmittingPromptState,
  'Starting fixture conversation',
);

export const mockWaitingAgentConversation = mockAgentConversation(
  'waiting_for_event',
  mockWaitingForRuntimeAcknowledgementState,
  'Waiting fixture conversation',
);

export const mockRunningAgentConversation = mockAgentConversation(
  'running',
  mockRuntimeRunningEvent,
  'Running fixture conversation',
);

export const mockCompletedAgentConversation = mockAgentConversation(
  'completed',
  mockRuntimeCompletedEvent,
  'Completed fixture conversation',
);

export const mockFailedAgentConversation = mockAgentConversation(
  'failed',
  mockBackendFailedResponse,
  'Failed fixture conversation',
);

export const mockUnsupportedAgentConversation = mockAgentConversation(
  'unsupported',
  mockIntegrationPendingState,
  'Unsupported fixture conversation',
);

export const mockAgentConversationStateExamples = [
  mockIdleAgentConversation,
  mockInputReadyAgentConversation,
  mockStartingAgentConversation,
  mockWaitingAgentConversation,
  mockRunningAgentConversation,
  mockCompletedAgentConversation,
  mockFailedAgentConversation,
  mockUnsupportedAgentConversation,
];
