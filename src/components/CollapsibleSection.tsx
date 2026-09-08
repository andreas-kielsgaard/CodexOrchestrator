import { ChevronDown, ChevronRight } from 'lucide-react';
import { useId, useState, type ReactNode } from 'react';
import './collapsibleSection.css';

export interface CollapsibleSectionProps {
  readonly title: string;
  readonly description?: string;
  readonly className?: string;
  readonly defaultExpanded?: boolean;
  readonly headerAccessory?: ReactNode;
  readonly action?: ReactNode;
  readonly children: ReactNode;
}

/** Reusable disclosure section with a semantic heading and native button behavior. */
export function CollapsibleSection({
  title,
  description,
  className,
  defaultExpanded = true,
  headerAccessory,
  action,
  children,
}: CollapsibleSectionProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  const contentId = useId();

  return (
    <section className={`collapsible-section${className ? ` ${className}` : ''}`}>
      <header className="collapsible-section__header">
        <div className="collapsible-section__heading-group">
          <div className="collapsible-section__heading-row">
            <h2>
              <button
                className="collapsible-section__toggle"
                type="button"
                aria-expanded={expanded}
                aria-controls={contentId}
                aria-label={`${expanded ? 'Collapse' : 'Expand'} ${title}`}
                onClick={() => setExpanded((current) => !current)}
              >
                {expanded ? (
                  <ChevronDown size={16} aria-hidden="true" />
                ) : (
                  <ChevronRight size={16} aria-hidden="true" />
                )}
                <span>{title}</span>
              </button>
            </h2>
            {headerAccessory}
          </div>
          {description && <p>{description}</p>}
        </div>
        {action}
      </header>
      <div className="collapsible-section__content" id={contentId} hidden={!expanded}>
        {children}
      </div>
    </section>
  );
}
