import type { TranscriptActivity } from './transcriptProjector';
import { memo, useEffect, useState } from 'react';
import type { AgentRuntimeUsageDto } from '../../application/agentSessions';
import { AgentMarkdown } from './AgentMarkdown';
import { presentToolActivity } from './toolActivityPresentation';

interface ProcessingDisclosureProps {
  startedAt?: string | null;
  completedAt?: string | null;
  usage?: AgentRuntimeUsageDto | null;
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
  startedAt,
  completedAt,
  usage,
}: ProcessingDisclosureProps) {
  const [workingOpen, setWorkingOpen] = useState({ running, open: true });
  const isOpen = running ? !workingOpen.running || workingOpen.open : expanded;
  if (activity.length === 0 && !running) return null;

  return (
    <details className="processing-disclosure" open={isOpen}>
      <summary
        onClick={(event) => {
          event.preventDefault();
          if (running) setWorkingOpen({ running: true, open: !isOpen });
          else onToggle();
        }}
      >
        <span>{running ? 'Working' : heading}</span>
        <ElapsedTime
          startedAt={startedAt ?? activity[0]?.recordedAt}
          completedAt={completedAt}
          running={running}
        />
        <small>{activity.length ? activitySummary(activity) : 'Waiting for activity'}</small>
      </summary>
      {isOpen && (
        <>
          <div className="turn-token-estimate" aria-label="Estimated turn tokens">
            Estimated tokens · Input: {usage?.inputTokens?.toLocaleString() ?? 'unavailable'} ·
            Output: {usage?.outputTokens?.toLocaleString() ?? 'unavailable'} · Cached input:{' '}
            {usage?.cachedInputTokens?.toLocaleString() ?? 'unavailable'}
          </div>
          <ol className="turn-activity" aria-label={`Processing for invocation ${invocationId}`}>
            <ActivityGroups activity={activity} running={running} safeOnly={safeOnly} />
          </ol>
        </>
      )}
    </details>
  );
}

function ActivityGroups({
  activity,
  running,
  safeOnly,
}: {
  activity: TranscriptActivity[];
  running: boolean;
  safeOnly: boolean;
}) {
  const rows = [];
  let batch: TranscriptActivity[] = [];
  const flush = () => {
    if (batch.length)
      rows.push(<OperationGroup key={`batch:${batch[0].id}`} items={batch} safeOnly={safeOnly} />);
    batch = [];
  };
  for (const item of activity) {
    const ongoing =
      running &&
      item.kind === 'tool' &&
      item.safeDetail?.kind === 'tool' &&
      item.safeDetail.phase === 'started' &&
      !item.completedAt;
    if (item.kind === 'tool' && !ongoing) batch.push(item);
    else {
      flush();
      rows.push(
        ongoing ? (
          <ToolActivityItem key={item.id} item={item} safeOnly={safeOnly} ongoing />
        ) : (
          <ActivityItem key={item.id} item={item} safeOnly={safeOnly} />
        ),
      );
    }
  }
  flush();
  return rows;
}

function OperationGroup({ items, safeOnly }: { items: TranscriptActivity[]; safeOnly: boolean }) {
  const [open, setOpen] = useState(false);
  let actions = 0;
  let files = 0;
  for (const item of items) {
    if (item.safeDetail?.kind === 'tool' && item.safeDetail.activity === 'file_change') {
      const detail = presentToolActivity(item, safeOnly).details.find(
        (detail) => detail.label === 'Files',
      );
      files += detail ? detail.value.split('\n').length : 1;
    } else
      actions += Number(
        presentToolActivity(item, safeOnly).batchLabel?.match(/(\d+) (commands|calls)/)?.[1] ?? 1,
      );
  }
  const label = [
    actions ? `${actions} ${actions === 1 ? 'action' : 'actions'}` : null,
    files ? `${files} ${files === 1 ? 'file edit' : 'file edits'}` : null,
  ]
    .filter(Boolean)
    .join(' · ');
  return (
    <li className="turn-operation-group">
      <details open={open}>
        <summary
          onClick={(event) => {
            event.preventDefault();
            setOpen(!open);
          }}
        >
          <span>{label}</span>
          <span className="turn-operation-group__hint">
            {open ? 'Hide operations' : 'Show operations'}
          </span>
          {items.some(
            (item) =>
              item.safeDetail?.kind === 'tool' && item.safeDetail.resultClassification === 'failed',
          ) && <span className="turn-tool-event__failure">Failed</span>}
        </summary>
        {open && (
          <ol>
            {items.map((item) => (
              <ToolActivityItem key={item.id} item={item} safeOnly={safeOnly} />
            ))}
          </ol>
        )}
      </details>
    </li>
  );
}

export function ElapsedTime({
  startedAt,
  completedAt,
  running,
}: {
  startedAt?: string | null;
  completedAt?: string | null;
  running: boolean;
}) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (!running || !startedAt) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [running, startedAt]);
  if (!startedAt || (!running && !completedAt)) return null;
  const seconds = Math.max(
    0,
    Math.floor(((running ? now : Date.parse(completedAt!)) - Date.parse(startedAt)) / 1000),
  );
  if (!Number.isFinite(seconds)) return null;
  return (
    <small className="turn-elapsed" aria-label={running ? 'Elapsed time' : 'Total working time'}>
      {seconds >= 60 ? `${Math.floor(seconds / 60)}m ${seconds % 60}s` : `${seconds}s`}
    </small>
  );
}

const ActivityItem = memo(function ActivityItem({
  item,
  safeOnly,
}: {
  item: TranscriptActivity;
  safeOnly: boolean;
}) {
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
});

function ToolActivityItem({
  item,
  safeOnly,
  ongoing = false,
}: {
  item: TranscriptActivity;
  safeOnly: boolean;
  ongoing?: boolean;
}) {
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
      {ongoing && (
        <>
          <span className="turn-tool-event__running">Running</span>
          <ElapsedTime startedAt={item.recordedAt} running />
        </>
      )}
    </>
  );

  return (
    <li className={`turn-tool-event activity-tool${ongoing ? ' turn-tool-event--running' : ''}`}>
      {ongoing ? (
        <>
          <div>{content}</div>
          <dl className="turn-tool-event__details">
            {presentation.details.map((detail) => (
              <div key={detail.label}>
                <dt>{detail.label}</dt>
                <dd>{detail.code ? <pre>{detail.value}</pre> : detail.value}</dd>
              </div>
            ))}
          </dl>
        </>
      ) : presentation.details.length ? (
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
