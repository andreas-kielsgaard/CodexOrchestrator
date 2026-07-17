import { AlertOctagon, CheckCircle2, CircleDot, Clock3, Pause, Play } from 'lucide-react';
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
