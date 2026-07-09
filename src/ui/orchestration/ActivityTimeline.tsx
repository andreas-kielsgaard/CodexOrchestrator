import type { HTMLAttributes } from 'react';
import { StatusPill } from './StatusPill';
import type { ActivityTimelineItem } from './types';

export interface ActivityTimelineProps extends HTMLAttributes<HTMLOListElement> {
  emptyLabel?: string;
  events: ActivityTimelineItem[];
}

export function ActivityTimeline({
  className,
  emptyLabel = 'No activity has been recorded yet.',
  events,
  ...props
}: ActivityTimelineProps) {
  const classes = ['ui-orchestration-activity-timeline', className].filter(Boolean).join(' ');

  if (events.length === 0) {
    return <p className="ui-orchestration-empty">{emptyLabel}</p>;
  }

  return (
    <ol {...props} className={classes}>
      {events.map((event) => (
        <li key={event.id}>
          <span aria-hidden="true" className="ui-orchestration-activity-timeline__dot" />
          <div>
            <header>
              <strong>{event.title}</strong>
              <StatusPill state={event.state} />
            </header>
            {event.description ? <p>{event.description}</p> : null}
            <footer>
              <small>{event.sourceLabel}</small>
              {event.timestampLabel ? <time>{event.timestampLabel}</time> : null}
            </footer>
          </div>
        </li>
      ))}
    </ol>
  );
}
