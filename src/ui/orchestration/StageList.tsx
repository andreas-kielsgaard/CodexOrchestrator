import type { HTMLAttributes } from 'react';
import { getOrchestrationStatusDescription } from '../../domain/orchestrationState';
import { StatusPill } from './StatusPill';
import type { OrchestrationStageItem } from './types';

export interface StageListProps extends HTMLAttributes<HTMLOListElement> {
  ariaLabel?: string;
  emptyLabel?: string;
  stages: OrchestrationStageItem[];
}

export function StageList({
  ariaLabel = 'Orchestration stages',
  className,
  emptyLabel = 'No orchestration stages to show yet.',
  stages,
  ...props
}: StageListProps) {
  const classes = ['ui-orchestration-stage-list', className].filter(Boolean).join(' ');

  if (stages.length === 0) {
    return <p className="ui-orchestration-empty">{emptyLabel}</p>;
  }

  return (
    <ol {...props} aria-label={ariaLabel} className={classes}>
      {stages.map((stage, index) => (
        <li data-current={stage.isCurrent || undefined} key={stage.id}>
          <span className="ui-orchestration-stage-list__index">{index + 1}</span>
          <div className="ui-orchestration-stage-list__content">
            <header>
              <strong>{stage.title}</strong>
              <StatusPill state={stage.state} />
            </header>
            <p>{stage.description ?? getOrchestrationStatusDescription(stage.state)}</p>
            {stage.evidenceLabel ? <small>{stage.evidenceLabel}</small> : null}
          </div>
        </li>
      ))}
    </ol>
  );
}
