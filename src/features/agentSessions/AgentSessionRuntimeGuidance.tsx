import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import type { AgentRuntimeFailureDto } from '../../application/agentSessions';
import type {
  NativeProfileApplicationConsumer,
  NativeProfileCurrentSelection,
} from '../../infrastructure/nativeProfiles/nativeProfileConsumer';

interface AgentSessionRuntimeGuidanceContextValue {
  readonly consumer?: Pick<NativeProfileApplicationConsumer, 'currentSelection'>;
  readonly onOpenTechnicalSettings?: () => void;
}

const AgentSessionRuntimeGuidanceContext = createContext<AgentSessionRuntimeGuidanceContextValue>(
  {},
);

export function AgentSessionRuntimeGuidanceProvider({
  consumer,
  onOpenTechnicalSettings,
  children,
}: AgentSessionRuntimeGuidanceContextValue & { readonly children: ReactNode }) {
  return (
    <AgentSessionRuntimeGuidanceContext.Provider value={{ consumer, onOpenTechnicalSettings }}>
      {children}
    </AgentSessionRuntimeGuidanceContext.Provider>
  );
}

export function AgentSessionRuntimeGuidance({
  failure,
}: {
  readonly failure: AgentRuntimeFailureDto | null;
}) {
  const { consumer, onOpenTechnicalSettings } = useContext(AgentSessionRuntimeGuidanceContext);
  const [selection, setSelection] = useState<
    NativeProfileCurrentSelection | { readonly kind: 'loading' } | { readonly kind: 'unavailable' }
  >({ kind: 'loading' });
  const missingHome =
    failure?.code === 'runtime_preflight_failed' &&
    failure.message === 'No native Codex home is selected';

  useEffect(() => {
    let current = true;
    if (!missingHome || !consumer) {
      setSelection(consumer ? { kind: 'loading' } : { kind: 'unavailable' });
      return () => {
        current = false;
      };
    }
    setSelection({ kind: 'loading' });
    void consumer.currentSelection().then(
      (next) => current && setSelection(next),
      () => current && setSelection({ kind: 'unavailable' }),
    );
    return () => {
      current = false;
    };
  }, [consumer, missingHome]);

  if (!missingHome) return null;
  return (
    <aside className="agent-runtime-guidance" aria-label="Codex home guidance">
      {selection.kind === 'none' ? (
        <>
          <p>No Codex home is selected in Technical Settings.</p>
          {onOpenTechnicalSettings ? (
            <button type="button" onClick={onOpenTechnicalSettings}>
              Open Technical Settings
            </button>
          ) : null}
        </>
      ) : selection.kind === 'selected' ? (
        <p>
          Technical Settings currently selects <code>{selection.codexHome}</code>. This invocation
          recorded its failure before that current selection was observed.
        </p>
      ) : selection.kind === 'loading' ? (
        <p>Checking the current Technical Settings selection…</p>
      ) : (
        <p>The current Technical Settings selection could not be read.</p>
      )}
    </aside>
  );
}
