import { ArrowLeft } from 'lucide-react';
import { useEffect, useRef, type ReactNode } from 'react';
import { ResizableSplitSurface } from './ResizableSplitSurface';
import '../styles/detailWorkspace.css';

export interface DetailWorkspaceProps {
  readonly ariaLabel: string;
  readonly controlsLabel: string;
  readonly contextLabel: string;
  readonly backLabel: string;
  readonly onBack: () => void;
  readonly focusBackOnMount?: boolean;
  readonly hotbarContext?: ReactNode;
  readonly hotbarNavigation?: ReactNode;
  readonly control: ReactNode;
  readonly context: ReactNode;
  readonly primary: ReactNode;
  readonly agentSession?: ReactNode;
}

/** Shared contained composition for Epic and Sprint detail surfaces. */
export function DetailWorkspace({
  ariaLabel,
  controlsLabel,
  contextLabel,
  backLabel,
  onBack,
  focusBackOnMount = false,
  hotbarContext,
  hotbarNavigation,
  control,
  context,
  primary,
  agentSession,
}: DetailWorkspaceProps) {
  const backButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (focusBackOnMount) backButtonRef.current?.focus();
  }, [focusBackOnMount]);

  return (
    <main
      className={`detail-workspace${agentSession ? '' : ' detail-workspace--single-panel'}`}
      aria-label={ariaLabel}
      data-viewport-contained="true"
    >
      <div className="detail-workspace__hotbar" aria-label={controlsLabel}>
        <button
          ref={backButtonRef}
          className="detail-workspace__back"
          type="button"
          onClick={onBack}
        >
          <ArrowLeft size={16} aria-hidden="true" />
          {backLabel}
        </button>
        {hotbarContext && <div className="detail-workspace__hotbar-context">{hotbarContext}</div>}
        {hotbarNavigation && (
          <nav className="detail-workspace__hotbar-navigation">{hotbarNavigation}</nav>
        )}
        <div className="detail-workspace__control">{control}</div>
      </div>

      <div className="detail-workspace__layout">
        <aside className="detail-workspace__context-rail" aria-label={contextLabel}>
          {context}
        </aside>
        <div className="detail-workspace__main-column">
          {agentSession ? (
            <ResizableSplitSurface
              axis="vertical"
              primary={<div className="detail-workspace__primary">{primary}</div>}
              secondary={<div className="detail-workspace__agent-session">{agentSession}</div>}
              primaryLabel="Detail flow"
              secondaryLabel="Agent Session"
              initialPrimaryPercent={82}
              minimumPrimaryPixels={220}
              minimumSecondaryPixels={44}
              maximizePrimaryLabel="Maximize flow"
            />
          ) : (
            <div className="detail-workspace__primary">{primary}</div>
          )}
        </div>
      </div>
    </main>
  );
}
