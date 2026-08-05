import { ArrowLeft, CornerUpLeft } from 'lucide-react';
import type { AgentSessionProductOrigin } from '../application/agentSessionNavigation';

export interface ProductCommandBarProps {
  readonly canGoBack: boolean;
  readonly onBack: () => void;
  readonly returnOrigin?: AgentSessionProductOrigin | null;
  readonly onReturn?: (origin: AgentSessionProductOrigin) => void;
}

/** Product navigation only; this bar deliberately does not expose arbitrary commands. */
export function ProductCommandBar({
  canGoBack,
  onBack,
  returnOrigin,
  onReturn,
}: ProductCommandBarProps) {
  return (
    <nav className="application-command-bar" aria-label="Product commands">
      <button
        className="application-command-bar__back"
        type="button"
        onClick={onBack}
        disabled={!canGoBack}
        title={canGoBack ? 'Go to the previous product destination' : 'No previous destination'}
      >
        <ArrowLeft size={15} aria-hidden="true" />
        Back
      </button>
      {returnOrigin && onReturn ? (
        <div className="application-command-bar__return">
          <span className="application-command-bar__context">
            {returnContextText(returnOrigin)}
          </span>
          <button type="button" onClick={() => onReturn(returnOrigin)}>
            <CornerUpLeft size={15} aria-hidden="true" />
            {returnActionLabel(returnOrigin)}
          </button>
        </div>
      ) : null}
    </nav>
  );
}

function returnContextText(origin: AgentSessionProductOrigin) {
  return `Opened from ${locationKindLabel(origin.location)}`;
}

function returnActionLabel(origin: AgentSessionProductOrigin) {
  return origin.location.kind === 'work_unit'
    ? 'Return to Work Unit Activity'
    : `Return to ${locationKindLabel(origin.location)}`;
}

function locationKindLabel(location: AgentSessionProductOrigin['location']) {
  return {
    epic: 'Epic',
    sprint: 'Sprint',
    work_slice_planning_point: 'Planning',
    work_unit: 'Work Unit',
    epic_planning_draft: 'Epic planning draft',
  }[location.kind];
}
