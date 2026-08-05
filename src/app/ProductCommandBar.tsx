import { ArrowLeft, CornerUpLeft } from 'lucide-react';
import type { AgentSessionProductOrigin } from '../application/agentSessionNavigation';
import {
  isAgentSessionProductOrigin,
  type FileReviewProductOrigin,
  type ProductContextualOrigin,
} from '../application/productNavigation';

export interface ProductCommandBarProps {
  readonly canGoBack: boolean;
  readonly onBack: () => void;
  readonly returnOrigin?: ProductContextualOrigin | null;
  readonly onReturn?: (origin: ProductContextualOrigin) => void;
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

function returnContextText(origin: ProductContextualOrigin) {
  return isAgentSessionProductOrigin(origin)
    ? `Opened from ${locationKindLabel(origin.location)}`
    : `Opened from ${fileReviewLocationLabel(origin)}`;
}

function returnActionLabel(origin: ProductContextualOrigin) {
  if (!isAgentSessionProductOrigin(origin)) return `Return to ${fileReviewLocationLabel(origin)}`;
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

function fileReviewLocationLabel(origin: FileReviewProductOrigin) {
  const location = origin.returnTo.location;
  if (location?.kind === 'work_unit' && location.inspectionState?.tab === 'evidence')
    return 'Work Unit Evidence';
  if (location?.kind === 'work_unit') return 'Work Unit Activity';
  if (location?.kind === 'work_slice_planning_point') return 'Planning Point';
  if (location?.kind === 'sprint') return 'Sprint';
  if (location?.kind === 'epic') return 'Epic';
  return 'Orchestration';
}
