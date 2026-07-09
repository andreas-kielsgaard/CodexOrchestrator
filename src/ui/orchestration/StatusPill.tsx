import type { HTMLAttributes } from 'react';
import {
  getOrchestrationStatusDescription,
  getOrchestrationStatusLabel,
  type OrchestrationTruthState,
} from '../../domain/orchestrationState';
import { getOrchestrationProvenanceLabel, getOrchestrationStatusToneClass } from './labels';

export interface StatusPillProps extends HTMLAttributes<HTMLSpanElement> {
  label?: string;
  showProvenance?: boolean;
  state: OrchestrationTruthState;
}

export function StatusPill({
  className,
  label,
  showProvenance = false,
  state,
  ...props
}: StatusPillProps) {
  const statusLabel = label ?? getOrchestrationStatusLabel(state);
  const provenanceLabel = getOrchestrationProvenanceLabel(state.provenance);
  const description = getOrchestrationStatusDescription(state);
  const classes = [
    'ui-orchestration-status-pill',
    getOrchestrationStatusToneClass(state.status),
    className,
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <span
      {...props}
      aria-label={`${statusLabel}. ${provenanceLabel}. ${description}`}
      className={classes}
      title={`${provenanceLabel}: ${description}`}
    >
      <span>{statusLabel}</span>
      {showProvenance ? <small>{provenanceLabel}</small> : null}
    </span>
  );
}
