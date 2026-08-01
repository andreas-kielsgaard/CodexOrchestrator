import { AlertOctagon, CheckCircle2, CircleDot, Clock3, Pause, Play } from 'lucide-react';
import { useId, useState } from 'react';
import type { EpicPresentation, EpicState, EpicStatePresentation } from '../orchestrationModel';
import { movementLabel, sourceStatusLabel } from '../orchestrationModel';

export function MovementBadge({ movement }: { readonly movement: EpicPresentation['movement'] }) {
  return (
    <span className="movement-badge">
      <Clock3 size={16} aria-hidden="true" />
      {movementLabel(movement)}
    </span>
  );
}

export function EpicTitleWithDescription({ name, description, onOpen }: { readonly name: string; readonly description: string; readonly onOpen: () => void }) {
  const tooltipId = useId();
  const [open, setOpen] = useState(false);
  return <span className="epic-title-help" onMouseEnter={() => setOpen(true)} onMouseLeave={() => setOpen(false)}>
    <button className="orchestration-list__open" type="button" aria-label={`Open ${name}`} aria-describedby={open ? tooltipId : undefined} onFocus={() => setOpen(true)} onBlur={() => setOpen(false)} onClick={onOpen}><strong>{name}</strong></button>
    {open ? <span className="epic-title-help__tooltip" id={tooltipId} role="tooltip">{description}</span> : null}
  </span>;
}

export function StateBadge({ state }: { readonly state: EpicStatePresentation }) {
  if (typeof state !== 'string') {
    return (
      <span className="epic-state">{`${sourceStatusLabel(state.kind)}: ${state.reason}`}</span>
    );
  }
  const Icon = {
    running: Play,
    ready_to_continue: CircleDot,
    paused: Pause,
    blocked: AlertOctagon,
    completed: CheckCircle2,
  }[state];
  return (
    <span className={`epic-state epic-state--${state}`}>
      <Icon size={15} aria-hidden="true" />
      {stateLabel(state)}
    </span>
  );
}

function stateLabel(state: EpicState): string {
  return {
    running: 'Running',
    ready_to_continue: 'Ready to continue',
    paused: 'Paused',
    blocked: 'Blocked',
    completed: 'Completed',
  }[state];
}
