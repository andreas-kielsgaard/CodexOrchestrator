import {
  canClaimAgentConversationActiveWork,
  canClaimAgentConversationCompleted,
  canClaimAgentConversationRunning,
  getAgentConversationStatusDescription,
  getAgentConversationStatusLabel,
  type AgentConversation,
  type AgentConversationEvidence,
  type AgentConversationStreamStatus,
} from './agentConversation';
import type { OrchestrationTruthState } from './orchestrationState';

describe('agentConversation', () => {
  const localDraftTruth: OrchestrationTruthState = {
    provenance: 'local_draft',
    status: 'draft',
  };

  const runtimeRunningTruth: OrchestrationTruthState = {
    provenance: 'runtime_event',
    status: 'running',
  };

  const backendCompletedTruth: OrchestrationTruthState = {
    provenance: 'backend_response',
    status: 'completed',
  };

  function evidence(
    truth: OrchestrationTruthState,
    label = 'Test fixture evidence',
  ): AgentConversationEvidence {
    return {
      kind: truth.provenance === 'runtime_event' ? 'streamed_runtime_output' : 'mock_demo_fixture',
      label,
      truth,
    };
  }

  function conversation(
    streamStatus: AgentConversationStreamStatus,
    truth: OrchestrationTruthState,
  ): AgentConversation {
    return {
      artifacts: [],
      attachments: [],
      id: 'test-conversation',
      mode: 'read_only',
      role: 'Plan Builder',
      runtime: {
        providerId: 'storybook-fixture',
        providerLabel: 'Test fixture provider',
        runtimeLabel: 'Test fixture runtime',
      },
      state: {
        evidence: evidence(truth),
        streamStatus,
        truth,
      },
      title: 'Fixture conversation',
      turns: [],
    };
  }

  it('does not claim active work from local-only running-shaped state', () => {
    const localRunning = conversation('running', {
      provenance: 'local_draft',
      status: 'running',
    });

    expect(canClaimAgentConversationRunning(localRunning)).toBe(false);
    expect(canClaimAgentConversationActiveWork(localRunning)).toBe(false);
    expect(getAgentConversationStatusLabel(localRunning)).toBe('Runtime status unconfirmed');
    expect(getAgentConversationStatusDescription(localRunning)).toContain(
      'backend or runtime evidence',
    );
  });

  it('claims active work only when running is backed by runtime evidence', () => {
    const runtimeRunning = conversation('running', runtimeRunningTruth);

    expect(canClaimAgentConversationRunning(runtimeRunning)).toBe(true);
    expect(canClaimAgentConversationActiveWork(runtimeRunning)).toBe(true);
    expect(getAgentConversationStatusLabel(runtimeRunning)).toBe('Running');
  });

  it('does not claim completion from mock or local provenance', () => {
    const localCompleted = conversation('completed', {
      provenance: 'mock_fixture',
      status: 'completed',
    });

    expect(canClaimAgentConversationCompleted(localCompleted)).toBe(false);
    expect(getAgentConversationStatusLabel(localCompleted)).toBe('Completion unverified');
    expect(getAgentConversationStatusDescription(localCompleted)).toContain(
      'backend or runtime evidence',
    );
  });

  it('claims completion when completion is backed by backend evidence', () => {
    const backendCompleted = conversation('completed', backendCompletedTruth);

    expect(canClaimAgentConversationCompleted(backendCompleted)).toBe(true);
    expect(getAgentConversationStatusLabel(backendCompleted)).toBe('Completed');
  });

  it('keeps waiting-for-event distinct from running', () => {
    const waiting = conversation('waiting_for_event', {
      provenance: 'backend_response',
      status: 'waiting_for_event',
    });

    expect(canClaimAgentConversationRunning(waiting)).toBe(false);
    expect(canClaimAgentConversationActiveWork(waiting)).toBe(false);
    expect(getAgentConversationStatusLabel(waiting)).toBe('Waiting for first runtime event');
    expect(getAgentConversationStatusDescription(waiting)).toContain('no runtime event');
  });

  it('labels input-ready as local user input without runtime claims', () => {
    const inputReady = conversation('input_ready', {
      provenance: 'user_input',
      status: 'ready',
    });

    expect(canClaimAgentConversationRunning(inputReady)).toBe(false);
    expect(getAgentConversationStatusLabel(inputReady)).toBe('Input ready');
    expect(getAgentConversationStatusDescription(inputReady)).toContain('ready locally');
  });

  it('keeps idle fixture state non-active', () => {
    const idle = conversation('idle', localDraftTruth);

    expect(canClaimAgentConversationActiveWork(idle)).toBe(false);
    expect(getAgentConversationStatusLabel(idle)).toBe('Not started');
  });
});

