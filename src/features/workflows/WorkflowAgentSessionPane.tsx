import { ArrowLeft, ArrowUpRight, X } from 'lucide-react';
import { useMemo, useState } from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
import type { WorkflowInstanceSession } from '../../application/workflows';
import { AgentSessionWorkspace, useAgentSession } from '../agentSessions';
import { AgentSessionHeaderActionsProvider } from '../agentSessions/AgentSessionWorkspace';
import './workflowAgentSessionPane.css';

export type WorkflowAgentSessionItem = Pick<
  WorkflowInstanceSession,
  'sessionId' | 'title' | 'activity' | 'associatedAt'
>;

export interface WorkflowAgentSessionPaneProps {
  readonly node: Readonly<{
    readonly id: string;
    readonly name: string;
    readonly harnessName: string;
  }>;
  readonly sessions: readonly WorkflowAgentSessionItem[];
  /** The caller binds this client to the Workflow instance and node. */
  readonly client: AgentSessionClient;
  readonly allowEmptySession: boolean;
  readonly onReturn: () => void;
  readonly onClose: () => void;
  readonly onSessionCreated?: (sessionId: string) => void | Promise<void>;
}

/** Workflow navigation around the shared Agent Session controller and workspace. */
export function WorkflowAgentSessionPane({
  node,
  sessions,
  client,
  allowEmptySession,
  onReturn,
  onClose,
  onSessionCreated,
}: WorkflowAgentSessionPaneProps) {
  const orderedSessions = useMemo(
    () =>
      [...sessions].sort(
        (left, right) =>
          right.associatedAt.localeCompare(left.associatedAt) ||
          right.sessionId.localeCompare(left.sessionId),
      ),
    [sessions],
  );
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(
    orderedSessions[0]?.sessionId ?? null,
  );
  const [fullSession, setFullSession] = useState(false);
  const controller = useAgentSession(client, {
    selectedSessionId,
    onSessionCreated: (sessionId) => {
      setSelectedSessionId(sessionId);
      void onSessionCreated?.(sessionId);
    },
  });
  const selectedSession = orderedSessions.find(
    (session) => session.sessionId === selectedSessionId,
  );
  const showWorkspace = selectedSessionId !== null || allowEmptySession;

  if (fullSession && selectedSessionId !== null) {
    return (
      <div className="workflow-agent-session-pane workflow-agent-session-pane--full">
        <header className="workflow-agent-session-pane__toolbar">
          <button type="button" autoFocus onClick={() => setFullSession(false)}>
            <ArrowLeft size={15} aria-hidden="true" />
            Return to Session list
          </button>
          <button type="button" aria-label="Close node details" onClick={onClose}>
            <X size={16} aria-hidden="true" />
          </button>
        </header>
        <section
          className="workflow-agent-session-pane__workspace"
          aria-label="Selected Agent Session"
        >
          <AgentSessionWorkspace controller={controller} />
        </section>
      </div>
    );
  }

  return (
    <div className="workflow-agent-session-pane">
      <header className="workflow-agent-session-pane__toolbar">
        <button type="button" autoFocus onClick={onReturn}>
          <ArrowLeft size={15} aria-hidden="true" />
          Return to node
        </button>
        <div>
          <strong>{node.harnessName}</strong>
          <span>{node.name}</span>
        </div>
        <button type="button" aria-label="Close node details" onClick={onClose}>
          <X size={16} aria-hidden="true" />
        </button>
      </header>

      <div className="workflow-agent-session-pane__split">
        <nav aria-label={`${node.name} Agent Sessions`}>
          <p className="eyebrow">Agent Sessions</p>
          {allowEmptySession ? (
            <button
              type="button"
              className={selectedSessionId === null ? 'is-selected' : ''}
              aria-pressed={selectedSessionId === null}
              onClick={() => setSelectedSessionId(null)}
            >
              <span>
                <strong>New Agent Session</strong>
                <small>Start from this node</small>
              </span>
            </button>
          ) : null}
          {orderedSessions.length === 0 && !allowEmptySession ? (
            <p className="workflow-agent-session-pane__empty">
              No Agent Sessions belong to this node.
            </p>
          ) : null}
          {orderedSessions.map((session) => (
            <button
              type="button"
              key={session.sessionId}
              className={session.sessionId === selectedSessionId ? 'is-selected' : ''}
              aria-pressed={session.sessionId === selectedSessionId}
              onClick={() => setSelectedSessionId(session.sessionId)}
            >
              <span>
                <strong>{session.title}</strong>
                <small>{session.activity}</small>
              </span>
              <time dateTime={session.associatedAt}>
                {new Date(session.associatedAt).toLocaleString()}
              </time>
            </button>
          ))}
        </nav>

        <section
          className="workflow-agent-session-pane__workspace"
          aria-label="Selected Agent Session"
        >
          {showWorkspace ? (
            <AgentSessionHeaderActionsProvider
              actions={
                selectedSession ? (
                  <button
                    className="workflow-agent-session-pane__view-action"
                    type="button"
                    onClick={() => setFullSession(true)}
                  >
                    View Agent Session
                    <ArrowUpRight size={15} aria-hidden="true" />
                  </button>
                ) : null
              }
            >
              <AgentSessionWorkspace
                controller={controller}
                presentation={
                  selectedSession
                    ? undefined
                    : {
                        emptyState: {
                          heading: `Start ${node.name}`,
                          guidance:
                            'Send the first message to create an Agent Session for this Workflow node.',
                        },
                        composer: {
                          messageLabel: 'Initial message',
                          messagePlaceholder: `Message ${node.harnessName}`,
                        },
                      }
                }
              />
            </AgentSessionHeaderActionsProvider>
          ) : (
            <p className="workflow-agent-session-pane__empty">Select an Agent Session.</p>
          )}
        </section>
      </div>
    </div>
  );
}
