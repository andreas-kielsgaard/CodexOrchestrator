import { ArrowRight } from 'lucide-react';
import type { HTMLAttributes } from 'react';
import { Button } from '../Button';
import {
  getOrchestrationStatusDescription,
  getOrchestrationStatusLabel,
  isMockOrUnsupported,
  type OrchestrationTruthState,
} from '../../domain/orchestrationState';
import { getOrchestrationProvenanceLabel } from './labels';
import { StatusPill } from './StatusPill';

export interface CurrentActionProps extends HTMLAttributes<HTMLElement> {
  actionLabel?: string;
  busy?: boolean;
  description?: string;
  onAction?: () => void;
  state: OrchestrationTruthState;
  title?: string;
}

export function CurrentAction({
  actionLabel,
  busy = false,
  className,
  description,
  onAction,
  state,
  title,
  ...props
}: CurrentActionProps) {
  const classes = ['ui-orchestration-current-action', className].filter(Boolean).join(' ');
  const resolvedTitle = title ?? getOrchestrationStatusLabel(state);
  const resolvedDescription = description ?? getOrchestrationStatusDescription(state);
  const provenanceLabel = getOrchestrationProvenanceLabel(state.provenance);
  const actionDisabled = !onAction || isMockOrUnsupported(state);

  return (
    <section {...props} className={classes}>
      <div>
        <StatusPill showProvenance state={state} />
        <h3>{resolvedTitle}</h3>
        <p>{resolvedDescription}</p>
        <small>{provenanceLabel}</small>
      </div>
      {actionLabel ? (
        <Button
          busy={busy}
          disabled={actionDisabled}
          onClick={onAction}
          trailingIcon={<ArrowRight aria-hidden="true" size={16} />}
          variant="primary"
        >
          {actionLabel}
        </Button>
      ) : null}
    </section>
  );
}
