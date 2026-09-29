import type { TranscriptActivity } from './transcriptProjector';
import { AgentMarkdown } from './AgentMarkdown';
import { presentToolActivity } from './toolActivityPresentation';

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
  heading = 'Worked',
}: ProcessingDisclosureProps) {
  if (activity.length === 0 && !running) return null;

  return (
    <details className="processing-disclosure" open={expanded}>
      <summary
        onClick={(event) => {
          event.preventDefault();
          onToggle();
        }}
      >
        <span>{running ? 'Working' : heading}</span>
        <small>{activity.length ? activitySummary(activity) : 'Waiting for activity'}</small>
      </summary>
      <ol className="turn-activity" aria-label={`Processing for invocation ${invocationId}`}>
        {activity.map((item) => (
          <ActivityItem item={item} safeOnly={safeOnly} key={item.id} />
        ))}
      </ol>
    </details>
  );
}

function ActivityItem({ item, safeOnly }: { item: TranscriptActivity; safeOnly: boolean }) {
  if (item.kind === 'agent_intermediate' || item.kind === 'processing') {
    return (
      <li className={`turn-message-event activity-${item.kind}`}>
        <AgentMarkdown className="agent-activity-markdown">{item.text}</AgentMarkdown>
      </li>
    );
  }

  if (item.kind === 'tool') return <ToolActivityItem item={item} safeOnly={safeOnly} />;

  return (
    <li className={`turn-supporting-event activity-${item.kind}`}>
      <span>{item.kind === 'usage' ? 'Usage' : 'Technical detail'}</span>
      <p>{item.text}</p>
      {safeOnly && item.safeDetail && (
        <p className="recorded-step-detail">{formatSafeDetail(item.safeDetail)}</p>
      )}
    </li>
  );
}

function ToolActivityItem({ item, safeOnly }: { item: TranscriptActivity; safeOnly: boolean }) {
  const presentation = presentToolActivity(item, safeOnly);
  const content = (
    <>
      <span className="turn-tool-event__icon" aria-hidden="true">
        {presentation.icon}
      </span>
      <span className="turn-tool-event__label">{presentation.label}</span>
      {presentation.batchLabel && (
        <span className="turn-tool-event__batch">{presentation.batchLabel}</span>
      )}
      {presentation.failed && <span className="turn-tool-event__failure">Failed</span>}
    </>
  );

  return (
    <li className="turn-tool-event activity-tool">
      {presentation.details.length ? (
        <details>
          <summary>{content}</summary>
          <dl className="turn-tool-event__details">
            {presentation.details.map((detail) => (
              <div key={`${detail.label}:${detail.value}`}>
                <dt>{detail.label}</dt>
                <dd>{detail.code ? <pre>{detail.value}</pre> : detail.value}</dd>
              </div>
            ))}
          </dl>
        </details>
      ) : (
        <div>{content}</div>
      )}
    </li>
  );
}

function activitySummary(activity: TranscriptActivity[]): string {
  const tools = activity.filter((item) => item.kind === 'tool').length;
  const messages = activity.filter(
    (item) => item.kind === 'agent_intermediate' || item.kind === 'processing',
  ).length;
  const parts = [
    tools ? `${tools} ${tools === 1 ? 'action' : 'actions'}` : null,
    messages ? `${messages} ${messages === 1 ? 'update' : 'updates'}` : null,
  ].filter((part): part is string => Boolean(part));
  return parts.length ? parts.join(' · ') : `${activity.length} updates`;
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

  return 'Tool activity detail unavailable';
}
