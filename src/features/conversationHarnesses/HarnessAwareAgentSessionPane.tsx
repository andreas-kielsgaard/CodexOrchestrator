import { SlidersHorizontal } from 'lucide-react';
import { useEffect, useState, type ReactNode } from 'react';
import type {
  ConversationHarnessInspectorRead,
  ConversationHarnessInspectorSource,
} from '../../application/conversationHarnesses';
import { ConversationHarnessInspector } from './ConversationHarnessInspector';

export interface HarnessAwareAgentSessionPaneProps {
  readonly sessionId: string;
  readonly source?: ConversationHarnessInspectorSource;
  readonly children: ReactNode;
}

/** Product-owned layer around a neutral Agent Session pane. */
export function HarnessAwareAgentSessionPane({
  sessionId,
  source,
  children,
}: HarnessAwareAgentSessionPaneProps) {
  const [mode, setMode] = useState<'conversation' | 'inspector'>('conversation');
  const [read, setRead] = useState<ConversationHarnessInspectorRead | null>(null);

  useEffect(() => {
    if (mode !== 'inspector' || !source) return;
    let active = true;
    setRead(null);
    void source.load({ sessionId }).then(
      (next) => active && setRead(next),
      () =>
        active &&
        setRead({
          kind: 'unavailable',
          reason: 'The product context could not load this harness configuration.',
        }),
    );
    return () => {
      active = false;
    };
  }, [mode, sessionId, source]);

  return (
    <div className="harness-aware-agent-session-pane">
      {mode === 'conversation' ? (
        <>
          {source && (
            <button
              className="harness-aware-agent-session-pane__inspect"
              type="button"
              onClick={() => setMode('inspector')}
            >
              <SlidersHorizontal size={15} aria-hidden="true" />
              Inspect harness
            </button>
          )}
          {children}
        </>
      ) : (
        <ConversationHarnessInspector read={read} onBack={() => setMode('conversation')} />
      )}
    </div>
  );
}
