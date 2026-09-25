import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import type { AgentRuntimeFailureDto } from '../../../application/agentSessions';
import type {
  NativeProfileApplicationConsumer,
  NativeProfileCurrentSelection,
} from '../../../infrastructure/agentProviders/codex/profiles/nativeProfileConsumer';

interface CodexHomeGuidanceContextValue {
  readonly consumer?: Pick<NativeProfileApplicationConsumer, 'currentSelection'>;
  readonly onOpenTechnicalSettings?: () => void;
}

const CodexHomeGuidanceContext = createContext<CodexHomeGuidanceContextValue>({});

export function CodexHomeGuidanceProvider({
  consumer,
  onOpenTechnicalSettings,
  children,
}: CodexHomeGuidanceContextValue & { readonly children: ReactNode }) {
  return (
    <CodexHomeGuidanceContext.Provider value={{ consumer, onOpenTechnicalSettings }}>
      {children}
    </CodexHomeGuidanceContext.Provider>
  );
}

/** Guidance for a session whose launch found no Codex home selected. */
export function CodexHomeGuidance({
  failure,
}: {
  readonly failure: AgentRuntimeFailureDto | null;
}) {
  const { consumer, onOpenTechnicalSettings } = useContext(CodexHomeGuidanceContext);
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
