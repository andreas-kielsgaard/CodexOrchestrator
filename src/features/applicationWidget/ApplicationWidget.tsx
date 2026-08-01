import { ChevronDown, ChevronUp } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import './applicationWidget.css';

export function ApplicationWidget({
  label,
  title,
  summary,
  icon,
  onOpen,
  minimized: controlledMinimized,
  onMinimizedChange,
}: {
  readonly label: string;
  readonly title: string;
  readonly summary: string;
  readonly icon: ReactNode;
  readonly onOpen: () => void;
  readonly minimized?: boolean;
  readonly onMinimizedChange?: (minimized: boolean) => void;
}) {
  const [localMinimized, setLocalMinimized] = useState(false);
  const minimized = controlledMinimized ?? localMinimized;
  const setMinimized = (next: boolean) => {
    setLocalMinimized(next);
    onMinimizedChange?.(next);
  };

  return (
    <aside
      className={`application-widget${minimized ? ' application-widget--minimized' : ''}`}
      aria-label={`${label} widget`}
      data-placement="bottom-right"
    >
      {minimized ? (
        <button
          type="button"
          className="application-widget__restore"
          onClick={() => setMinimized(false)}
          aria-label={`Restore ${label} widget`}
        >
          {icon}
          <span>{label}</span>
          <ChevronUp size={14} />
        </button>
      ) : (
        <>
          <button
            type="button"
            className="application-widget__open"
            onClick={onOpen}
            aria-label={`Open ${label} details for ${title}`}
          >
            {icon}
            <span>{label}</span>
            <strong>{title}</strong>
            <small>{summary}</small>
          </button>
          <button
            type="button"
            className="application-widget__minimize"
            onClick={() => setMinimized(true)}
            aria-label={`Minimize ${label} widget`}
          >
            <ChevronDown size={14} />
          </button>
        </>
      )}
    </aside>
  );
}
