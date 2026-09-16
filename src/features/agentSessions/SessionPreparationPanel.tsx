import { useEffect, useState } from 'react';
import { Check, Circle, LoaderCircle, Minus, X } from 'lucide-react';
import {
  isSessionPreparing,
  type SessionPreparationDto,
  type SessionPreparationStepDto,
} from '../../application/agentSessions/preparation';
import './sessionPreparation.css';
export function SessionPreparationPanel({
  preparation,
  preview,
  deviceName,
  onClosePreview,
  onCancel,
  onRetry,
}: {
  preparation?: SessionPreparationDto | null;
  preview?: readonly SessionPreparationStepDto[] | null;
  deviceName: string;
  onClosePreview(): void;
  onCancel(): void;
  onRetry?(): void;
}) {
  const [collapsed, setCollapsed] = useState(false);
  useEffect(() => {
    setCollapsed(false);
  }, [preparation?.invocationId, preview]);
  useEffect(() => {
    if (preparation?.phase === 'ready' && !isSessionPreparing(preparation)) setCollapsed(true);
  }, [preparation]);
  const showPreview = Boolean(preview);
  if (!preparation && !showPreview) return null;
  const steps = showPreview ? preview! : preparation!.steps;
  const phase = showPreview
    ? 'preview'
    : isSessionPreparing(preparation)
      ? 'preparing'
      : preparation!.phase;
  const title =
    phase === 'ready'
      ? `Ready on ${deviceName}`
      : phase === 'failed'
        ? 'Setup failed'
        : phase === 'canceled'
          ? 'Setup canceled'
          : phase === 'preview'
            ? 'Preparation on Send'
            : 'Preparing session';
  return (
    <aside
      className={`session-preparation-panel${collapsed ? ' session-preparation-panel--collapsed' : ''}`}
      aria-label="Session preparation"
    >
      <header>
        <strong role="status">{title}</strong>
        <button
          type="button"
          aria-label={
            showPreview
              ? 'Close preparation preview'
              : collapsed
                ? 'View steps'
                : 'Collapse preparation'
          }
          onClick={() => (showPreview ? onClosePreview() : setCollapsed(!collapsed))}
        >
          {collapsed ? 'View steps' : showPreview ? <X size={16} /> : <Minus size={16} />}
        </button>
      </header>
      {!collapsed && (
        <>
          <ol>
            {steps.map((step) => (
              <li key={step.id} className={`session-preparation-step--${step.status}`}>
                {step.status === 'completed' ? (
                  <Check size={18} aria-hidden="true" />
                ) : step.status === 'running' ? (
                  <LoaderCircle
                    size={18}
                    className="session-preparation-spinner"
                    aria-hidden="true"
                  />
                ) : (
                  <Circle size={18} aria-hidden="true" />
                )}
                <span>
                  {step.label}
                  <small className="session-preparation-sr-only"> · {step.status}</small>
                  {step.error && <small>{step.error}</small>}
                </span>
              </li>
            ))}
          </ol>
          {!showPreview && preparation?.error && <p role="alert">{preparation.error}</p>}
          {!showPreview && (phase === 'preparing' || phase === 'accepted') && (
            <footer>
              <button type="button" onClick={onCancel}>
                Cancel setup
              </button>
            </footer>
          )}
          {!showPreview && preparation?.canRetry && onRetry && (
            <footer>
              <button type="button" onClick={onRetry}>
                Retry setup
              </button>
            </footer>
          )}
        </>
      )}
    </aside>
  );
}
