import type { AgentSessionUpdateDto, SessionInteractionDto } from '../../application/agentSessions';
import { runtimeControlRecordKind } from '../../application/agentSessions/runtimeControlRecords';

export function pendingSessionRequests(interactions: readonly SessionInteractionDto[] = []) {
  return interactions.filter(
    (item) => item.kind === 'request' && (item.state === 'pending' || item.state === 'responding'),
  );
}

export const pendingRequestLabel = 'Waiting for you';

export function sessionSummaryChanged(update: AgentSessionUpdateDto) {
  if (update.kind !== 'event_persisted') return true;
  const kind = runtimeControlRecordKind(update.event);
  return (
    update.event.normalized?.kind === 'invocation_completed' ||
    update.event.normalized?.kind === 'processing_started' ||
    kind === 'runtime_turn_active' ||
    kind === 'runtime_request_opened' ||
    kind === 'runtime_request_response'
  );
}
