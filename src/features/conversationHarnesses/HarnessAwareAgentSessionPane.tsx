import { SlidersHorizontal } from 'lucide-react';
import { useEffect, useState, type ReactNode } from 'react';
import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementRead,
  ConversationHarnessManagementSource,
} from '../../application/conversationHarnesses';
import { ConversationHarnessManagement } from './ConversationHarnessInspector';
import './harnessInspector.css';

export interface HarnessAwareAgentSessionPaneProps {
  readonly sessionId: string;
  readonly source?: ConversationHarnessManagementSource;
  readonly children: ReactNode;
}

/** Product-owned layer around a neutral Agent Session pane. */
export function HarnessAwareAgentSessionPane({
  sessionId,
  source,
  children,
}: HarnessAwareAgentSessionPaneProps) {
  const [mode, setMode] = useState<'conversation' | 'management'>('conversation');
  const [boundRead, setBoundRead] = useState<ConversationHarnessManagementRead | null>(null);
  const [read, setRead] = useState<ConversationHarnessManagementRead | null>(null);
  const [commandPending, setCommandPending] = useState(false);
  const [commandError, setCommandError] = useState<string | null>(null);

  useEffect(() => {
    setMode('conversation');
    setRead(null);
    setBoundRead(null);
    setCommandError(null);
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
    if (mode !== 'management' || !source) return;
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
    setMode('management');
  };

  const runCommand = async (command: ConversationHarnessManagementCommand) => {
    if (!source?.dispatch || (commandPending && command.kind !== 'save_working_copy')) return;
    const tracksPending = command.kind !== 'save_working_copy';
    if (tracksPending) setCommandPending(true);
    setCommandError(null);
    try {
      const next = await source.dispatch({ sessionId, command });
      if (next.kind !== 'available') {
        setCommandError(next.reason);
        return;
      }
      setRead(next);
      setBoundRead(next);
    } catch {
      setCommandError('The harness change could not be recorded.');
    } finally {
      if (tracksPending) setCommandPending(false);
    }
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
              Manage harness
            </button>
          )}
          {children}
        </>
      ) : (
        <ConversationHarnessManagement
          read={read}
          commandPending={commandPending}
          commandError={commandError}
          onBack={() => setMode('conversation')}
          onCommand={source?.dispatch ? (command) => void runCommand(command) : undefined}
        />
      )}
    </div>
  );
}
