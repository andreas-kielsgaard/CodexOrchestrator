import type { AgentRuntimeEventDto } from '../../application/agentSessions/contracts';

const methods = new Set([
  'item/commandExecution/outputDelta',
  'item/agentMessage/delta',
  'item/reasoning/textDelta',
  'item/reasoning/summaryTextDelta',
  'item/reasoning/summaryPartAdded',
  'turn/diff/updated',
  'account/rateLimits/updated',
  'mcpServer/startupStatus/updated',
  'skills/changed',
  'thread/status/changed',
]);
export function isDisplayTelemetry(event: AgentRuntimeEventDto) {
  if (event.normalized) return false;
  // Wire telemetry has no normalized record; filtering is confined to this adapter.
  // eslint-disable-next-line no-restricted-syntax
  const raw = event.rawPayload;
  return (
    !!raw &&
    typeof raw === 'object' &&
    'method' in raw &&
    typeof raw.method === 'string' &&
    methods.has(raw.method)
  );
}
