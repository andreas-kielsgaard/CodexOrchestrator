export async function waitForChange({
  baseline,
  condition,
  observe,
  pollMs,
  stableObservations,
  timeoutMs,
  signal,
  isCancelled = async () => false,
  now = () => Date.now(),
  sleep = defaultSleep,
}) {
  validateBaseline(baseline, condition);

  const startedAtMs = now();
  const deadline = startedAtMs + timeoutMs;
  let lastObservation = null;
  let stableCandidate = null;
  let stableCount = 0;
  let observationCount = 0;

  while (true) {
    if (signal?.aborted || (await isCancelled())) {
      return outcome('cancelled', startedAtMs, now(), {
        observationCount,
        stableCount,
        lastObservation,
      });
    }

    const remaining = deadline - now();
    if (remaining <= 0) {
      return outcome('timed_out', startedAtMs, now(), {
        observationCount,
        stableCount,
        lastObservation,
      });
    }

    try {
      await sleep(Math.min(pollMs, remaining), signal);
    } catch (error) {
      if (signal?.aborted || error?.name === 'AbortError') {
        return outcome('cancelled', startedAtMs, now(), {
          observationCount,
          stableCount,
          lastObservation,
        });
      }
      throw error;
    }

    if (signal?.aborted || (await isCancelled())) continue;
    if (now() >= deadline) {
      return outcome('timed_out', startedAtMs, now(), {
        observationCount,
        stableCount,
        lastObservation,
      });
    }

    lastObservation = await observe({
      signal,
      timeoutMs: Math.max(1, deadline - now()),
    });
    observationCount += 1;
    if (now() >= deadline) {
      return outcome('timed_out', startedAtMs, now(), {
        observationCount,
        stableCount,
        lastObservation,
      });
    }
    const candidate = changedCandidate(baseline, lastObservation, condition);
    if (!candidate) {
      stableCandidate = null;
      stableCount = 0;
    } else {
      const key = `${candidate.kind}:${candidate.value}`;
      if (key === stableCandidate) {
        stableCount += 1;
      } else {
        stableCandidate = key;
        stableCount = 1;
      }
      if (stableCount >= stableObservations) {
        return outcome('completed', startedAtMs, now(), {
          observationCount,
          stableCount,
          lastObservation,
          trigger: candidate,
        });
      }
    }
  }
}

export function observationSignals(snapshot) {
  return {
    visual: snapshot?.application?.screenshot?.value?.sha256 ?? null,
    durable: snapshot?.application?.durableState?.value?.fingerprint ?? null,
  };
}

export function changedCandidate(baseline, observation, condition) {
  const durableChanged =
    baseline.durable !== null &&
    observation.durable !== null &&
    baseline.durable !== observation.durable;
  const visualChanged =
    baseline.visual !== null &&
    observation.visual !== null &&
    baseline.visual !== observation.visual;

  if ((condition === 'durable' || condition === 'either') && durableChanged) {
    return { kind: 'durable', value: observation.durable };
  }
  if ((condition === 'visual' || condition === 'either') && visualChanged) {
    return { kind: 'visual', value: observation.visual };
  }
  return null;
}

function validateBaseline(baseline, condition) {
  if (!['visual', 'durable', 'either'].includes(condition)) {
    throw new Error('--condition must be visual, durable, or either.');
  }
  if (condition === 'visual' && baseline.visual === null) {
    throw new Error('The baseline has no observed window-render hash for a visual wait.');
  }
  if (condition === 'durable' && baseline.durable === null) {
    throw new Error('The baseline has no observed durable-state fingerprint for a durable wait.');
  }
  if (condition === 'either' && baseline.visual === null && baseline.durable === null) {
    throw new Error(
      'The baseline has neither an observed render hash nor durable-state fingerprint.',
    );
  }
}

function outcome(status, startedAtMs, finishedAtMs, details) {
  return {
    status,
    startedAt: new Date(startedAtMs).toISOString(),
    finishedAt: new Date(finishedAtMs).toISOString(),
    elapsedMs: Math.max(0, finishedAtMs - startedAtMs),
    ...details,
  };
}

function defaultSleep(milliseconds, signal) {
  return delay(milliseconds, undefined, { signal });
}
import { setTimeout as delay } from 'node:timers/promises';
