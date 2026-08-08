import type { TranscriptActivity } from './transcriptProjector';
import { AgentMarkdown } from './AgentMarkdown';

interface ProcessingDisclosureProps {
  invocationId: string;
  activity: TranscriptActivity[];
  running: boolean;
  expanded: boolean;
  onToggle(): void;
  safeOnly?: boolean;
  heading?: string;
}

export function ProcessingDisclosure({
  invocationId,
  activity,
  running,
  expanded,
  onToggle,
  safeOnly = false,
  heading = 'Processing',
}: ProcessingDisclosureProps) {
  if (activity.length === 0 && !running) return null;
  const isOpen = running || expanded;

  return (
    <details className="processing-disclosure" open={isOpen}>
      <summary
        onClick={(event) => {
          event.preventDefault();
          if (!running) onToggle();
        }}
      >
        <span>{running ? 'Working' : heading}</span>
        <small>{activity.length ? `${activity.length} updates` : 'Waiting for activity'}</small>
      </summary>
      <ol aria-label={`Processing for invocation ${invocationId}`}>
        {activity.map((item) => (
          <li className={`activity-${item.kind}`} key={item.id}>
            <span>{activityLabel(item.kind)}</span>
            {safeOnly && item.kind === 'tool' ? (
              <p>{formatSafeToolDetail(item.safeDetail)}</p>
            ) : item.kind === 'agent_intermediate' ? (
              <AgentMarkdown className="agent-activity-markdown">{item.text}</AgentMarkdown>
            ) : (
              <p>{item.text}</p>
            )}
            {safeOnly ? (
              item.kind !== 'tool' &&
              item.safeDetail && (
                <p className="recorded-step-detail">{formatSafeDetail(item.safeDetail)}</p>
              )
            ) : (
              <details className="raw-event-disclosure">
                <summary>Raw event</summary>
                <pre>{formatRaw(item.rawPayload)}</pre>
              </details>
            )}
          </li>
        ))}
      </ol>
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

function activityLabel(kind: TranscriptActivity['kind']): string {
  switch (kind) {
    case 'agent_intermediate':
      return 'Agent update';
    case 'tool':
      return 'Tool';
    case 'usage':
      return 'Usage';
    case 'processing':
      return 'Processing';
    case 'technical':
      return 'Technical';
  }
}

function formatSafeDetail(detail: NonNullable<TranscriptActivity['safeDetail']>): string {
  if (detail.kind === 'usage') {
    const parts = [
      detail.inputTokens === null ? null : `${detail.inputTokens} input`,
      detail.cachedInputTokens === null ? null : `${detail.cachedInputTokens} cached`,
      detail.outputTokens === null ? null : `${detail.outputTokens} output`,
    ].filter((part): part is string => part !== null);
    return parts.length ? `Usage detail: ${parts.join(', ')}` : 'Usage detail unavailable';
  }

  return formatSafeToolDetail(detail);
}

function formatSafeToolDetail(detail: TranscriptActivity['safeDetail']): string {
  if (!detail || detail.kind !== 'tool') return 'Tool activity detail unavailable';
  const label = [detail.server, detail.tool].filter(Boolean).join(' / ');
  return [label || 'Tool identity unavailable', detail.phase, detail.resultClassification].join(
    ' · ',
  );
}
