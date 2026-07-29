import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from 'react';
import '../styles/resizableSplitSurface.css';

export interface ResizableSplitSurfaceProps {
  readonly axis: 'horizontal' | 'vertical';
  readonly primary: ReactNode;
  readonly secondary: ReactNode;
  readonly primaryLabel: string;
  readonly secondaryLabel: string;
  readonly initialPrimaryPercent?: number;
  readonly minimumPrimaryPixels?: number;
  readonly minimumSecondaryPixels?: number;
  readonly maximizePrimaryLabel?: string;
}

/** Reusable two-pane surface with pointer and keyboard resizing. */
export function ResizableSplitSurface({
  axis,
  primary,
  secondary,
  primaryLabel,
  secondaryLabel,
  initialPrimaryPercent = 70,
  minimumPrimaryPixels = 160,
  minimumSecondaryPixels = 120,
  maximizePrimaryLabel,
}: ResizableSplitSurfaceProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const [primaryPixels, setPrimaryPixels] = useState<number | null>(null);
  const [stackHorizontal, setStackHorizontal] = useState(
    () =>
      axis === 'horizontal' &&
      typeof window !== 'undefined' &&
      window.matchMedia?.('(max-width: 720px)').matches === true,
  );
  const effectiveAxis = axis === 'horizontal' && stackHorizontal ? 'vertical' : axis;

  useEffect(() => {
    if (axis !== 'horizontal' || typeof window === 'undefined' || !window.matchMedia) return;
    const media = window.matchMedia('(max-width: 720px)');
    const updateAxis = () => {
      setStackHorizontal(media.matches);
      setPrimaryPixels(null);
    };
    media.addEventListener('change', updateAxis);
    return () => media.removeEventListener('change', updateAxis);
  }, [axis]);

  const bounds = useCallback(() => {
    const rect = hostRef.current?.getBoundingClientRect();
    const total = effectiveAxis === 'vertical' ? rect?.height : rect?.width;
    if (!total) return null;
    const maximum = Math.max(0, total - Math.min(minimumSecondaryPixels, total));
    return {
      total,
      minimum: Math.min(minimumPrimaryPixels, maximum),
      maximum,
    };
  }, [effectiveAxis, minimumPrimaryPixels, minimumSecondaryPixels]);

  const update = useCallback(
    (requested: number) => {
      const nextBounds = bounds();
      if (!nextBounds) return;
      setPrimaryPixels(Math.min(nextBounds.maximum, Math.max(nextBounds.minimum, requested)));
    },
    [bounds],
  );

  useEffect(() => {
    const nextBounds = bounds();
    if (!nextBounds || primaryPixels !== null) return;
    update((nextBounds.total * initialPrimaryPercent) / 100);
  }, [bounds, initialPrimaryPercent, primaryPixels, update]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(() => {
      if (primaryPixels !== null) update(primaryPixels);
    });
    observer.observe(host);
    return () => observer.disconnect();
  }, [primaryPixels, update]);

  const beginPointerResize = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    const host = hostRef.current;
    if (!host) return;
    const onMove = (moveEvent: PointerEvent) => {
      const rect = host.getBoundingClientRect();
      update(
        effectiveAxis === 'vertical' ? moveEvent.clientY - rect.top : moveEvent.clientX - rect.left,
      );
    };
    const stop = () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', stop);
      window.removeEventListener('pointercancel', stop);
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', stop);
    window.addEventListener('pointercancel', stop);
  };

  const resizeWithKeyboard = (event: KeyboardEvent<HTMLDivElement>) => {
    const nextBounds = bounds();
    if (!nextBounds) return;
    const decrease = effectiveAxis === 'vertical' ? 'ArrowUp' : 'ArrowLeft';
    const increase = effectiveAxis === 'vertical' ? 'ArrowDown' : 'ArrowRight';
    if (![decrease, increase, 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    if (event.key === 'Home') return update(nextBounds.minimum);
    if (event.key === 'End') return update(nextBounds.maximum);
    update((primaryPixels ?? nextBounds.total / 2) + (event.key === decrease ? -24 : 24));
  };

  const splitStyle = {
    '--split-primary-size':
      primaryPixels === null ? `${initialPrimaryPercent}%` : `${primaryPixels}px`,
  } as React.CSSProperties;

  return (
    <div
      ref={hostRef}
      className={`resizable-split resizable-split--${axis}`}
      style={splitStyle}
      data-split-axis={axis}
      data-effective-split-axis={effectiveAxis}
    >
      <section
        className="resizable-split__pane resizable-split__pane--primary"
        aria-label={primaryLabel}
      >
        {primary}
      </section>
      <div
        className="resizable-split__separator"
        role="separator"
        aria-label={`Resize ${primaryLabel} and ${secondaryLabel}`}
        aria-orientation={effectiveAxis === 'vertical' ? 'horizontal' : 'vertical'}
        tabIndex={0}
        onPointerDown={beginPointerResize}
        onKeyDown={resizeWithKeyboard}
      >
        <span aria-hidden="true" />
        {maximizePrimaryLabel ? (
          <button
            type="button"
            onPointerDown={(event) => event.stopPropagation()}
            onClick={() => {
              const nextBounds = bounds();
              if (nextBounds) update(nextBounds.maximum);
            }}
          >
            {maximizePrimaryLabel}
          </button>
        ) : null}
      </div>
      <section
        className="resizable-split__pane resizable-split__pane--secondary"
        aria-label={secondaryLabel}
      >
        {secondary}
      </section>
    </div>
  );
}
