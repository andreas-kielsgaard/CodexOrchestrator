import type { ContextUsage } from './sessionUsage';

export function ContextUsageIndicator({ usage }: { usage?: ContextUsage | null }) {
  const percent = usage?.capacity ? Math.min(100, (usage.used / usage.capacity) * 100) : null;
  const label = usage
    ? `Context window: ${percent === null ? '' : `${Math.round(percent)}% used · `}${usage.used.toLocaleString()} tokens${usage.capacity ? ` of ${usage.capacity.toLocaleString()}` : ' (capacity unavailable)'}`
    : 'Context window usage unavailable';
  return (
    <span className="context-usage" title={label} aria-label={label}>
      <svg width="18" height="18" viewBox="0 0 20 20" aria-hidden="true">
        <circle
          cx="10"
          cy="10"
          r="7"
          fill="none"
          stroke="currentColor"
          opacity="0.2"
          strokeWidth="3"
        />
        <circle
          cx="10"
          cy="10"
          r="7"
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
          pathLength="100"
          strokeDasharray={`${percent ?? 0} 100`}
          transform="rotate(-90 10 10)"
        />
      </svg>
      <span>{percent === null ? '—' : `${Math.round(percent)}%`}</span>
    </span>
  );
}
