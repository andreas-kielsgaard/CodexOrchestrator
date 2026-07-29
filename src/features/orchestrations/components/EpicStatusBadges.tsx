import { useId, useRef, useState } from 'react';
import {
  AlertOctagon,
  CheckCircle2,
  ChevronDown,
  Clock3,
  LoaderCircle,
  Pause,
  Play,
  Search,
} from 'lucide-react';
import type {
  EpicMovementPresentation,
  EpicOverviewNavigationTarget,
  EpicState,
  EpicStatePresentation,
} from '../orchestrationModel';
import { movementLabel, sourceStatusLabel } from '../orchestrationModel';

export function EpicTitleWithDescription({
  name,
  description,
  onOpen,
}: {
  readonly name: string;
  readonly description: string;
  readonly onOpen: () => void;
}) {
  const tooltipId = useId();
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const open = hovered || focused;
  return (
    <span
      className="epic-title-help"
      data-row-action-exempt
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <button
        className="orchestration-list__open"
        type="button"
        aria-label={`Open ${name}`}
        aria-describedby={open ? tooltipId : undefined}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onClick={onOpen}
      >
        <strong>{name}</strong>
      </button>
      {open && (
        <span className="epic-title-help__tooltip" id={tooltipId} role="tooltip">
          {description}
        </span>
      )}
    </span>
  );
}

export function MovementSummary({
  movement,
  onNavigate,
}: {
  readonly movement: EpicMovementPresentation;
  readonly onNavigate: (target: EpicOverviewNavigationTarget) => void;
}) {
  const [open, setOpen] = useState(false);
  const popoverId = useId();
  const triggerRef = useRef<HTMLButtonElement>(null);

  if (movement.kind !== 'available' || movement.items.length === 0) {
    return (
      <span className="movement-badge movement-badge--empty">
        <Clock3 size={16} aria-hidden="true" />
        {movementLabel(movement)}
      </span>
    );
  }

  return (
    <div
      className="movement-summary"
      onKeyDown={(event) => {
        if (event.key !== 'Escape') return;
        setOpen(false);
        triggerRef.current?.focus();
      }}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
      }}
    >
      <button
        className="movement-badge movement-badge--interactive"
        type="button"
        aria-expanded={open}
        aria-controls={popoverId}
        aria-haspopup="dialog"
        onClick={() => setOpen((current) => !current)}
        ref={triggerRef}
      >
        <Clock3 size={16} aria-hidden="true" />
        {movementLabel(movement)}
        <ChevronDown size={14} aria-hidden="true" />
      </button>
      {open && (
        <div
          className="movement-popover"
          id={popoverId}
          role="dialog"
          aria-label="Current movement details"
        >
          <ul>
            {movement.items.map((item) => {
              const Icon = item.state === 'processing' ? LoaderCircle : Search;
              return (
                <li key={item.movementItemId}>
                  <button
                    type="button"
                    onClick={() => {
                      setOpen(false);
                      onNavigate(item.target);
                    }}
                  >
                    <Icon size={15} aria-hidden="true" />
                    <span>{item.label}</span>
                    <small>{item.state === 'processing' ? 'Processing' : 'Reviewing'}</small>
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
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
    paused: 'Paused',
    blocked: 'Blocked',
    completed: 'Completed',
  }[state];
}
