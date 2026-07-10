import { useCallback, useLayoutEffect, useRef } from 'react';

const FOLLOW_THRESHOLD_PX = 96;

interface ScrollMetrics {
  scrollHeight: number;
  scrollTop: number;
  clientHeight: number;
}

export function isNearTranscriptBottom(
  metrics: ScrollMetrics,
  threshold = FOLLOW_THRESHOLD_PX,
): boolean {
  return metrics.scrollHeight - metrics.scrollTop - metrics.clientHeight <= threshold;
}

export function useTranscriptFollow(sessionId: string | null, revision: string) {
  const containerRef = useRef<HTMLDivElement>(null);
  const shouldFollowRef = useRef(true);
  const previousSessionRef = useRef<string | null | undefined>(undefined);

  const scrollToLatest = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;
    container.scrollTop = container.scrollHeight;
  }, []);

  const requestFollow = useCallback(() => {
    shouldFollowRef.current = true;
    scrollToLatest();
  }, [scrollToLatest]);

  const handleScroll = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;
    shouldFollowRef.current = isNearTranscriptBottom(container);
  }, []);

  useLayoutEffect(() => {
    const sessionChanged = previousSessionRef.current !== sessionId;
    if (sessionChanged) shouldFollowRef.current = true;
    previousSessionRef.current = sessionId;
    if (shouldFollowRef.current) scrollToLatest();
  }, [revision, scrollToLatest, sessionId]);

  return { containerRef, handleScroll, requestFollow };
}
