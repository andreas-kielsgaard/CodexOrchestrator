import type { AgentRuntimeEventDto } from '../../application/agentSessions';

/**
 * Display text for a technical event, such as provider stderr. This is the one place the session
 * UI reads raw provider evidence, and it only shows it; nothing decides on it.
 */
export function runtimeDiagnosticText(event: AgentRuntimeEventDto): string {
  const raw = event.rawPayload;
  if (typeof raw === 'string' && raw.trim()) {
    return raw.trim();
  }
  if (raw && typeof raw === 'object') {
    const decoded = (raw as Record<string, unknown>).lossyUtf8;
    if (typeof decoded === 'string' && decoded.trim()) {
      return decoded.trim();
    }
  }
  return `${event.source} event (${event.normalized?.kind ?? 'unparsed'})`;
}
