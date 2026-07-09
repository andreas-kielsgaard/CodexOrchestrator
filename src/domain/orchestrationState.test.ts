import {
  canShowRuntimeCompletion,
  canShowRuntimeProgress,
  getOrchestrationStatusDescription,
  getOrchestrationStatusLabel,
  isMockOrUnsupported,
  transitionOrchestrationState,
  type OrchestrationTruthState,
} from './orchestrationState';

describe('orchestrationState', () => {
  const draft: OrchestrationTruthState = { status: 'draft', provenance: 'local_draft' };

  it('labels local add-flow state without claiming runtime work started', () => {
    const accepted = transitionOrchestrationState(draft, 'accept_user_input');
    const pending = transitionOrchestrationState(accepted, 'backend_unsupported');

    expect(getOrchestrationStatusLabel(accepted)).toBe('Prompt accepted locally');
    expect(getOrchestrationStatusDescription(accepted)).toContain('no runtime output exists yet');
    expect(getOrchestrationStatusLabel(pending)).toBe('Backend integration pending');
    expect(canShowRuntimeProgress(accepted)).toBe(false);
    expect(canShowRuntimeProgress(pending)).toBe(false);
    expect(isMockOrUnsupported(pending)).toBe(true);
  });

  it('does not let a local start request become running by itself', () => {
    const starting = transitionOrchestrationState(draft, 'request_backend_start');

    expect(starting).toEqual({ status: 'starting', provenance: 'local_draft' });
    expect(getOrchestrationStatusLabel(starting)).toBe('Start requested');
    expect(canShowRuntimeProgress(starting)).toBe(false);
  });

  it('shows running only when backed by backend or runtime evidence', () => {
    expect(canShowRuntimeProgress({ status: 'running', provenance: 'local_draft' })).toBe(false);
    expect(getOrchestrationStatusLabel({ status: 'running', provenance: 'local_draft' })).toBe(
      'Runtime status unconfirmed',
    );

    expect(canShowRuntimeProgress({ status: 'running', provenance: 'runtime_event' })).toBe(true);
    expect(getOrchestrationStatusLabel({ status: 'running', provenance: 'runtime_event' })).toBe(
      'Running',
    );
  });

  it('shows completed only when backed by backend or runtime evidence', () => {
    expect(canShowRuntimeCompletion({ status: 'completed', provenance: 'local_draft' })).toBe(
      false,
    );
    expect(getOrchestrationStatusLabel({ status: 'completed', provenance: 'local_draft' })).toBe(
      'Completion unverified',
    );

    expect(canShowRuntimeCompletion({ status: 'completed', provenance: 'backend_response' })).toBe(
      true,
    );
    expect(
      getOrchestrationStatusLabel({ status: 'completed', provenance: 'backend_response' }),
    ).toBe('Completed');
  });
});
