export type OrchestrationStatus =
  | 'draft'
  | 'ready'
  | 'starting'
  | 'waiting_for_event'
  | 'running'
  | 'blocked'
  | 'failed'
  | 'completed'
  | 'integration_pending'
  | 'mock_preview';

export type OrchestrationProvenance =
  | 'user_input'
  | 'local_draft'
  | 'persisted_snapshot'
  | 'backend_response'
  | 'runtime_event'
  | 'mock_fixture'
  | 'unsupported';

export interface OrchestrationTruthState {
  status: OrchestrationStatus;
  provenance: OrchestrationProvenance;
}

export type OrchestrationTransitionEvent =
  | 'accept_user_input'
  | 'save_local_draft'
  | 'request_backend_start'
  | 'backend_acknowledged'
  | 'runtime_event_received'
  | 'backend_unsupported'
  | 'command_failed'
  | 'complete_from_backend'
  | 'show_mock_preview';

const runtimeProvenance = new Set<OrchestrationProvenance>(['backend_response', 'runtime_event']);

export function getOrchestrationStatusLabel(state: OrchestrationTruthState): string {
  if (state.status === 'mock_preview') {
    return 'Mock preview';
  }

  if (state.status === 'integration_pending') {
    return 'Backend integration pending';
  }

  if (state.status === 'running' && canShowRuntimeProgress(state)) {
    return 'Running';
  }

  if (state.status === 'running') {
    return 'Runtime status unconfirmed';
  }

  if (state.status === 'completed' && canShowRuntimeCompletion(state)) {
    return 'Completed';
  }

  if (state.status === 'completed') {
    return 'Completion unverified';
  }

  if (state.status === 'waiting_for_event') {
    return 'Waiting for runtime acknowledgement';
  }

  if (state.status === 'starting') {
    return 'Start requested';
  }

  if (state.status === 'ready' && state.provenance === 'user_input') {
    return 'Prompt accepted locally';
  }

  if (state.status === 'ready') {
    return 'Ready to start plan builder';
  }

  if (state.status === 'draft' && state.provenance === 'local_draft') {
    return 'Draft held in this session';
  }

  if (state.status === 'draft') {
    return 'Not started';
  }

  if (state.status === 'blocked' && state.provenance === 'unsupported') {
    return 'Unsupported';
  }

  if (state.status === 'blocked') {
    return 'Blocked';
  }

  return 'Failed';
}

export function getOrchestrationStatusDescription(state: OrchestrationTruthState): string {
  if (state.status === 'mock_preview') {
    return 'This is demo data only and is not backed by a runtime run.';
  }

  if (state.status === 'integration_pending') {
    return 'The UI has a local draft, but no backend command or Codex thread has started.';
  }

  if (state.status === 'running' && canShowRuntimeProgress(state)) {
    return 'A backend response or runtime event confirms work is active.';
  }

  if (state.status === 'running') {
    return 'Running cannot be shown until backend or runtime evidence arrives.';
  }

  if (state.status === 'completed' && canShowRuntimeCompletion(state)) {
    return 'A backend response or runtime event confirms this action completed.';
  }

  if (state.status === 'completed') {
    return 'Completion cannot be shown without backend or runtime evidence.';
  }

  if (state.status === 'waiting_for_event') {
    return 'The app has a backend acknowledgement and is waiting for the first runtime event.';
  }

  if (state.status === 'starting') {
    return 'A start request was made and the app is waiting for acknowledgement.';
  }

  if (state.status === 'ready' && state.provenance === 'user_input') {
    return 'The prompt is captured in local UI state; no runtime output exists yet.';
  }

  if (state.status === 'ready') {
    return 'Enough local input exists to request the next action.';
  }

  if (state.status === 'draft') {
    return 'This information exists locally and has not started runtime work.';
  }

  if (state.status === 'blocked' && state.provenance === 'unsupported') {
    return 'This step cannot continue until backend support exists.';
  }

  if (state.status === 'blocked') {
    return 'The flow needs user input, configuration, or a supported backend capability.';
  }

  return 'The last command or load action failed.';
}

export function canShowRuntimeProgress(state: OrchestrationTruthState): boolean {
  return state.status === 'running' && runtimeProvenance.has(state.provenance);
}

export function canShowRuntimeCompletion(state: OrchestrationTruthState): boolean {
  return state.status === 'completed' && runtimeProvenance.has(state.provenance);
}

export function isMockOrUnsupported(state: OrchestrationTruthState): boolean {
  return (
    state.status === 'mock_preview' ||
    state.status === 'integration_pending' ||
    state.provenance === 'mock_fixture' ||
    state.provenance === 'unsupported'
  );
}

export function transitionOrchestrationState(
  _current: OrchestrationTruthState,
  event: OrchestrationTransitionEvent,
): OrchestrationTruthState {
  if (event === 'accept_user_input') {
    return { status: 'ready', provenance: 'user_input' };
  }

  if (event === 'save_local_draft') {
    return { status: 'draft', provenance: 'local_draft' };
  }

  if (event === 'request_backend_start') {
    return { status: 'starting', provenance: 'local_draft' };
  }

  if (event === 'backend_acknowledged') {
    return { status: 'waiting_for_event', provenance: 'backend_response' };
  }

  if (event === 'runtime_event_received') {
    return { status: 'running', provenance: 'runtime_event' };
  }

  if (event === 'backend_unsupported') {
    return { status: 'integration_pending', provenance: 'unsupported' };
  }

  if (event === 'command_failed') {
    return { status: 'failed', provenance: 'backend_response' };
  }

  if (event === 'complete_from_backend') {
    return { status: 'completed', provenance: 'backend_response' };
  }

  return { status: 'mock_preview', provenance: 'mock_fixture' };
}
