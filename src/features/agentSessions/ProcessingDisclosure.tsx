import type { TranscriptActivity } from './transcriptProjector';
import { AgentMarkdown } from './AgentMarkdown';

interface ProcessingDisclosureProps {
  invocationId: string;
  activity: TranscriptActivity[];
  running: boolean;
  expanded: boolean;
  onToggle(): void;
}

export function ProcessingDisclosure({
  invocationId,
  activity,
  running,
  expanded,
  onToggle,
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
        <span>{running ? 'Working' : 'Processing'}</span>
        <small>{activity.length ? `${activity.length} updates` : 'Waiting for activity'}</small>
      </summary>
      <ol aria-label={`Processing for invocation ${invocationId}`}>
        {activity.map((item) => (
          <li className={`activity-${item.kind}`} key={item.id}>
            <span>{activityLabel(item.kind)}</span>
            {item.kind === 'agent_intermediate' ? (
              <AgentMarkdown className="agent-activity-markdown">{item.text}</AgentMarkdown>
            ) : (
              <p>{item.text}</p>
            )}
            <details className="raw-event-disclosure">
              <summary>Raw event</summary>
              <pre>{formatRaw(item.rawPayload)}</pre>
            </details>
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
