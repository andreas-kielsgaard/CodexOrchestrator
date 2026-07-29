import { AlertOctagon, Check } from 'lucide-react';
import type { SprintPlanItemPresentation } from '../orchestrationModel';
import { sprintStatusLabel } from './presentationLabels';
import '../styles/sprintPlan.css';

export interface SprintPlanProps {
  readonly items: readonly SprintPlanItemPresentation[];
  readonly onOpen: (item: SprintPlanItemPresentation, opener: HTMLButtonElement) => void;
}

export function SprintPlan({ items, onOpen }: SprintPlanProps) {
  return (
    <section className="epic-plan" aria-label="Epic plan">
      <ol>
        {items.map((item, index) => (
          <SprintPlanItem
            key={item.id}
            item={item}
            position={index + 1}
            onOpen={(opener) => onOpen(item, opener)}
          />
        ))}
      </ol>
    </section>
  );
}

function SprintPlanItem({
  item,
  position,
  onOpen,
}: {
  readonly item: SprintPlanItemPresentation;
  readonly position: number;
  readonly onOpen: (opener: HTMLButtonElement) => void;
}) {
  const viewable = item.status !== 'not_started' || Boolean(item.workspace);
  const statusClass = typeof item.status === 'string' ? item.status : item.status.kind;
  const content = (
    <>
      <span className="sprint-plan-item__marker" aria-hidden="true">
        {item.status === 'completed' ? <Check size={18} /> : position}
      </span>
      <span className="sprint-plan-item__body">
        <span className="sprint-plan-item__meta">
          <span>{sprintStatusLabel(item.status)}</span>
          <code>{item.id}</code>
        </span>
        <strong>{item.name}</strong>
        <small>{item.purpose}</small>
        {item.agentSession && (
          <span className="sprint-session-reference">
            Agent Session · {item.agentSession.title}
          </span>
        )}
        {item.blocker && (
          <span className="sprint-blocker" role="status">
            <AlertOctagon size={16} aria-hidden="true" />
            <span>
              <strong>Blocked: {item.blocker.summary}</strong>
              <small>{item.blocker.needs}</small>
            </span>
          </span>
        )}
      </span>
    </>
  );

  return (
    <li className={`sprint-plan-item sprint-plan-item--${statusClass}`}>
      {viewable ? (
        <button
          type="button"
          data-sprint-id={item.id}
          onClick={(event) => onOpen(event.currentTarget)}
          aria-label={`${item.status === 'not_started' ? 'View proposed Plan' : 'Open Sprint'}: ${item.name}`}
        >
          {content}
        </button>
      ) : (
        <div aria-label={`${item.name}, not started`}>{content}</div>
      )}
    </li>
  );
}
