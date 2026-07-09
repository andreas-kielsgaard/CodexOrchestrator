import type { OrchestrationProvenance, OrchestrationStatus } from '../../domain/orchestrationState';

const provenanceLabels: Record<OrchestrationProvenance, string> = {
  backend_response: 'Backend response evidence',
  local_draft: 'Local draft only',
  mock_fixture: 'Mock/demo fixture',
  persisted_snapshot: 'Persisted snapshot',
  runtime_event: 'Runtime event evidence',
  unsupported: 'Unsupported integration',
  user_input: 'User input held locally',
};

const statusToneClasses: Record<OrchestrationStatus, string> = {
  blocked: 'ui-orchestration-status-pill--danger',
  completed: 'ui-orchestration-status-pill--success',
  draft: 'ui-orchestration-status-pill--neutral',
  failed: 'ui-orchestration-status-pill--danger',
  integration_pending: 'ui-orchestration-status-pill--warning',
  mock_preview: 'ui-orchestration-status-pill--mock',
  ready: 'ui-orchestration-status-pill--warning',
  running: 'ui-orchestration-status-pill--info',
  starting: 'ui-orchestration-status-pill--warning',
  waiting_for_event: 'ui-orchestration-status-pill--warning',
};

export function getOrchestrationProvenanceLabel(provenance: OrchestrationProvenance): string {
  return provenanceLabels[provenance];
}

export function getOrchestrationStatusToneClass(status: OrchestrationStatus): string {
  return statusToneClasses[status];
}
