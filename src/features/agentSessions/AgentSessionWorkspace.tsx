import { Check, ClipboardCopy } from 'lucide-react';
import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { ConversationViewport } from './ConversationViewport';
import { AgentIdentityBadge } from '../../components/AgentIdentityBadge';
import {
  browserAgentSessionClipboard,
  formatAgentSessionContext,
  type AgentSessionClipboard,
} from './sessionClipboard';
import type { AgentSessionWorkspaceController } from './useAgentSessionController';

export interface AgentSessionPresentation {
  readonly showHeader?: boolean;
  readonly ariaLabel?: string;
  readonly emptyState?: Readonly<{ heading: string; guidance: string }>;
  readonly composer?: Readonly<{
    readonly messageLabel?: string;
    readonly messagePlaceholder?: string;
    /** Hidden by default; a context with a real routing need must explicitly opt in. */
    readonly showWorkingDirectory?: boolean;
    readonly keyboardHint?: 'tooltip' | 'hidden';
  }>;
}

export interface AgentSessionWorkspaceProps {
  controller: AgentSessionWorkspaceController;
  readonly presentation?: AgentSessionPresentation;
  readonly clipboard?: AgentSessionClipboard;
}

const AgentSessionHeaderActionsContext = createContext<ReactNode>(null);

export function AgentSessionHeaderActionsProvider({
  actions,
  children,
}: {
  readonly actions: ReactNode;
  readonly children: ReactNode;
}) {
  return (
    <AgentSessionHeaderActionsContext.Provider value={actions}>
      {children}
    </AgentSessionHeaderActionsContext.Provider>
  );
}

export function AgentSessionWorkspace({
  controller,
  presentation = {},
  clipboard = browserAgentSessionClipboard,
}: AgentSessionWorkspaceProps) {
  const contextualHeaderActions = useContext(AgentSessionHeaderActionsContext);
  const active = Boolean(controller.transcript?.activeInvocationId);
  const identity = controller.details?.session.agentIdentity ?? null;
  const title = identity
    ? `${identity.name}: ${identity.harnessRole}`
    : (controller.details?.session.title ?? 'New Agent Session');
  const showHeader = presentation.showHeader ?? true;
  const [copyState, setCopyState] = useState<'idle' | 'copying' | 'copied' | 'failed'>('idle');
  useEffect(() => {
    if (copyState !== 'copied') return;
    const timeout = window.setTimeout(() => setCopyState('idle'), 2000);
    return () => window.clearTimeout(timeout);
  }, [copyState]);
  const copySession = async () => {
    if (!controller.details || !controller.transcript || copyState === 'copying') return;
    setCopyState('copying');
    try {
      await clipboard.writeText(
        formatAgentSessionContext(controller.details, controller.transcript),
      );
      setCopyState('copied');
    } catch {
      setCopyState('failed');
    }
  };
  const copyAction = (
    <div className="agent-session-copy-action">
      <button
        className="agent-session-copy-button"
        type="button"
        disabled={!controller.details || copyState === 'copying'}
        onClick={() => void copySession()}
      >
        {copyState === 'copied' ? (
          <Check size={15} aria-hidden="true" />
        ) : (
          <ClipboardCopy size={15} aria-hidden="true" />
        )}
        {copyState === 'copying'
          ? 'Copying…'
          : copyState === 'copied'
            ? 'Copied'
            : 'Copy entire session'}
      </button>
      <span className="agent-session-copy-feedback" role="status" aria-live="polite">
        {copyState === 'failed' ? 'Session could not be copied.' : ''}
      </span>
    </div>
  );
  return (
    <section
      className={`agent-session-workspace${showHeader ? '' : ' agent-session-workspace--header-hidden'}`}
      aria-label={presentation.ariaLabel ?? title}
    >
      {showHeader && (
        <header className="agent-session-header">
          <div className="agent-session-header__identity">
            <div className="agent-session-header__title-row">
              {identity && <AgentIdentityBadge identity={identity} compact />}
              <h2>{title}</h2>
            </div>
            {presentation.composer?.showWorkingDirectory &&
              controller.details?.session.workingDirectory && (
                <p
                  className="session-working-directory"
                  title={controller.details.session.workingDirectory}
                >
                  {controller.details.session.workingDirectory}
                </p>
              )}
          </div>
          <div className="agent-session-header__actions">
            {contextualHeaderActions}
            {copyAction}
            {active && (
              <span className="working-status" role="status">
                Working
              </span>
            )}
          </div>
        </header>
      )}
      {!showHeader && (
        <div className="agent-session-utility-bar">
          {contextualHeaderActions}
          {copyAction}
        </div>
      )}
      <ConversationViewport
        agentIdentity={identity}
        segments={
          controller.transcript
            ? [{ id: controller.transcript.sessionId, transcript: controller.transcript }]
            : []
        }
        loading={controller.loading}
        expandedProcessing={controller.expandedProcessing}
        onToggleProcessing={controller.toggleProcessing}
        error={controller.error}
        onClearError={controller.clearError}
        emptyState={presentation.emptyState}
        composerPresentation={presentation.composer}
        composerTarget={{
          sessionId: controller.selectedSessionId,
          draft: controller.draft,
          workingDirectory: controller.workingDirectory,
          sending: controller.sending,
          active,
          canceling: controller.canceling,
          setDraft: controller.setDraft,
          setWorkingDirectory: controller.setWorkingDirectory,
          send: controller.send,
          cancel: controller.cancel,
        }}
      />
    </section>
  );
}
