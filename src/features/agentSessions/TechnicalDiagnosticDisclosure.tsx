import type { AgentDiagnosticDto } from '../../application/agentSessions';
import type { TranscriptActivity } from './transcriptProjector';

interface TechnicalDiagnosticDisclosureProps {
  activity: TranscriptActivity[];
  diagnostics: AgentDiagnosticDto[];
}

export function TechnicalDiagnosticDisclosure({
  activity,
  diagnostics,
}: TechnicalDiagnosticDisclosureProps) {
  if (activity.length === 0 && diagnostics.length === 0) return null;

  return (
    <details className="technical-disclosure">
      <summary>Technical details ({activity.length + diagnostics.length})</summary>
      <ul>
        {diagnostics.map((diagnostic, index) => (
          <li key={`${diagnostic.recordedAt}-${diagnostic.code}-${index}`}>
            <strong>{diagnostic.code}</strong>
            <span>{diagnostic.message}</span>
            {diagnostic.details !== null && <pre>{formatRaw(diagnostic.details)}</pre>}
          </li>
        ))}
        {activity.map((item) => (
          <li key={item.id}>
            <strong>{item.source}</strong>
            <span>{item.text}</span>
            {formatRaw(item.rawPayload) !== item.text && <pre>{formatRaw(item.rawPayload)}</pre>}
          </li>
        ))}
      </ul>
    </details>
  );
}

function formatRaw(value: unknown): string {
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}
