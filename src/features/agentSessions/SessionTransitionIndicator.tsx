import { transitionLabel, type SessionTransition } from './sessionHistoryProjector';
import './sessionTransitions.css';
export function SessionTransitionIndicator({
  transition,
  onOpen,
}: {
  transition: SessionTransition;
  onOpen(transition: SessionTransition): void;
}) {
  return (
    <div className={`session-transition session-transition--${transition.kind}`}>
      <button type="button" onClick={() => onOpen(transition)}>
        {transitionLabel(transition)}
      </button>
    </div>
  );
}
