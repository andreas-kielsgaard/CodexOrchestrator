import { Check, LoaderCircle } from 'lucide-react';
import type { SessionQueueEntryViewDto } from '../../application/agentSessions/sessionQueue';
import { queueEntryTitle, targetActionLabel } from './sessionQueueLabels';
import './sessionDeviceActivity.css';

export function SessionDeviceActivityList({
  activities,
  onRunNow,
}: {
  readonly activities: readonly SessionQueueEntryViewDto[];
  readonly onRunNow?: (entryId: string) => Promise<void>;
}) {
  if (!activities.length) return null;
  return (
    <aside className="session-device-activity" aria-label="Device activity">
      <strong>Device activity</strong>
      <ol>
        {activities.map((item) => {
          const operation = item.targetOperation!;
          const current =
            operation.actions.find((action) => action.status === 'running') ??
            operation.actions.find((action) => action.status === 'pending') ??
            operation.actions.at(-1);
          const completed = operation.actions.filter(
            (action) => action.status === 'completed',
          ).length;
          return (
            <li key={item.entry.id}>
              {operation.phase !== 'pending' && (
                <LoaderCircle
                  className="session-device-activity__spinner"
                  size={15}
                  aria-hidden="true"
                />
              )}
              <span>
                <b>{queueEntryTitle(item)}</b>
                {current && <small>{targetActionLabel(current.kind)}</small>}
                {operation.phase === 'pending' && (
                  <>
                    <small>Waiting for the next message</small>
                    {operation.projectedPlan.steps.map((step) => (
                      <small key={step.kind}>{targetActionLabel(step.kind)}</small>
                    ))}
                    {onRunNow && (
                      <button onClick={() => void onRunNow(item.entry.id)}>Run now</button>
                    )}
                  </>
                )}
              </span>
              <span
                className="session-device-activity__count"
                aria-label={`${completed} of ${operation.actions.length} actions complete`}
              >
                <Check size={11} />
                {completed}/{operation.actions.length}
              </span>
            </li>
          );
        })}
      </ol>
    </aside>
  );
}
