import type { AgentSessionClient } from '../application/agentSessions';
import type {
  CompletionNotificationSource,
  CompletionSoundPlayer,
} from '../application/completionNotifications';
import type { WorktreeReviewBuildActivitySource } from '../application/worktreeReview';

export function createCompletionNotificationSource(
  sessions: Pick<AgentSessionClient, 'subscribeUpdates'>,
  builds: WorktreeReviewBuildActivitySource,
): CompletionNotificationSource {
  return {
    async subscribe(listener) {
      let active = true;
      const seenInvocations = new Set<string>();
      const seenBuilds = new Set(builds.getSnapshot().unreadBuilds.map((build) => build.buildId));
      const stopBuilds = builds.subscribe(() => {
        if (!active) return;
        for (const build of builds.getSnapshot().unreadBuilds) {
          if (seenBuilds.has(build.buildId)) continue;
          seenBuilds.add(build.buildId);
          listener({ kind: 'worktree_review_build', id: build.buildId });
        }
      });
      const stopSessions = await sessions
        .subscribeUpdates((update) => {
          if (!active || update.kind !== 'invocation_terminal') return;
          if (seenInvocations.has(update.invocationId)) return;
          seenInvocations.add(update.invocationId);
          if (update.completionSound === false) return;
          listener({ kind: 'agent_turn', id: update.invocationId });
        })
        .catch(() => () => undefined);
      if (!active) {
        stopSessions();
        return () => undefined;
      }
      return () => {
        active = false;
        stopBuilds();
        stopSessions();
      };
    },
  };
}

type AudioContextConstructor = new () => AudioContext;

function audioContextConstructor(): AudioContextConstructor | undefined {
  if (typeof window === 'undefined') return undefined;
  const compatible = window as typeof window & {
    webkitAudioContext?: AudioContextConstructor;
  };
  return compatible.AudioContext ?? compatible.webkitAudioContext;
}

function tone(context: AudioContext, frequency: number, start: number, duration: number) {
  const oscillator = context.createOscillator();
  const gain = context.createGain();
  oscillator.type = 'sine';
  oscillator.frequency.setValueAtTime(frequency, start);
  gain.gain.setValueAtTime(0.0001, start);
  gain.gain.exponentialRampToValueAtTime(0.1, start + 0.015);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
  oscillator.connect(gain);
  gain.connect(context.destination);
  oscillator.start(start);
  oscillator.stop(start + duration);
}

export function createBrowserCompletionSound(): CompletionSoundPlayer {
  let context: AudioContext | undefined;
  const ensureContext = () => {
    if (context) return context;
    const Constructor = audioContextConstructor();
    if (!Constructor) return undefined;
    context = new Constructor();
    return context;
  };
  const unlock = () => {
    const current = ensureContext();
    if (current?.state === 'suspended') void current.resume().catch(() => undefined);
  };
  return {
    unlock,
    play() {
      const current = ensureContext();
      if (!current) return;
      const chime = () => {
        const start = current.currentTime + 0.01;
        tone(current, 659.25, start, 0.14);
        tone(current, 880, start + 0.11, 0.2);
      };
      if (current.state === 'suspended') {
        void current.resume().then(chime, () => undefined);
      } else {
        chime();
      }
    },
  };
}

export const browserCompletionSound = createBrowserCompletionSound();
