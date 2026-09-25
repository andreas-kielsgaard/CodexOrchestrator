import type { AgentRuntimeEventDto } from './contracts';

/**
 * Orchid's own runtime control vocabulary, mirrored from the engine's `RuntimeControlRecord`.
 * Runtime-sourced events carrying these kinds are Orchid records; everything else in a raw
 * payload is provider evidence that the UI may display but never decides on.
 */
export type RuntimeControlRecordKind =
  | 'runtime_turn_active'
  | 'runtime_request_opened'
  | 'runtime_request_unsupported'
  | 'runtime_request_response'
  | 'session_steering_pending'
  | 'session_steering_result'
  | 'runtime_working_directory_resolved'
  | 'runtime_process_exit';

const KINDS: readonly RuntimeControlRecordKind[] = [
  'runtime_turn_active',
  'runtime_request_opened',
  'runtime_request_unsupported',
  'runtime_request_response',
  'session_steering_pending',
  'session_steering_result',
  'runtime_working_directory_resolved',
  'runtime_process_exit',
];

export function runtimeControlRecordKind(
  event: Pick<AgentRuntimeEventDto, 'source' | 'rawPayload'>,
): RuntimeControlRecordKind | null {
  if (event.source !== 'runtime') return null;
  const payload = event.rawPayload;
  const kind =
    payload && typeof payload === 'object' && 'kind' in payload ? payload.kind : undefined;
  return KINDS.find((candidate) => candidate === kind) ?? null;
}
