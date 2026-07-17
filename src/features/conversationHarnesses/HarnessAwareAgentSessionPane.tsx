import { SlidersHorizontal } from 'lucide-react';
import { useEffect, useState, type ReactNode } from 'react';
import type {
  ConversationHarnessInspectorRead,
  ConversationHarnessInspectorSource,
} from '../../application/conversationHarnesses';
import { ConversationHarnessInspector } from './ConversationHarnessInspector';
import './harnessInspector.css';

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
  const [boundRead, setBoundRead] = useState<ConversationHarnessInspectorRead | null>(null);
  const [read, setRead] = useState<ConversationHarnessInspectorRead | null>(null);

  useEffect(() => {
    setMode('conversation');
    setRead(null);
    setBoundRead(null);
    if (!source) return;
    let active = true;
    void source.load({ sessionId }).then(
      (next) => active && setBoundRead(next),
      () =>
        active &&
        setBoundRead({
          kind: 'unavailable',
          reason: 'The product context could not load this harness configuration.',
        }),
    );
    return () => {
      active = false;
    };
  }, [sessionId, source]);

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

  const openInspector = () => {
    if (!source || boundRead?.kind !== 'available') return;
    setMode('inspector');
  };

  return (
    <div className="harness-aware-agent-session-pane">
      {mode === 'conversation' ? (
        <>
          {boundRead?.kind === 'available' && (
            <button
              className="harness-aware-agent-session-pane__inspect"
              type="button"
              onClick={openInspector}
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
