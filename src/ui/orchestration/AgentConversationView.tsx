import { Send } from 'lucide-react';
import type { FormEvent, HTMLAttributes } from 'react';
import { useState } from 'react';
import {
  getAgentConversationCurrentActionLabel,
  getAgentConversationEvidenceLabel,
  getAgentConversationStatusDescription,
  getAgentConversationStatusLabel,
  type AgentConversation,
  type AgentConversationArtifact,
  type AgentConversationAttachment,
  type AgentConversationTurn,
} from '../../domain/agentConversation';
import { Button } from '../Button';
import { ConversationThread } from './ConversationThread';
import { FileList } from './FileList';
import { StatusPill } from './StatusPill';
import type { ConversationMessageItem, OrchestrationFileItem } from './types';

export interface AgentConversationViewProps extends HTMLAttributes<HTMLElement> {
  conversation: AgentConversation;
  onSubmitPrompt?: (body: string) => void;
}

export function AgentConversationView({
  className,
  conversation,
  onSubmitPrompt,
  ...props
}: AgentConversationViewProps) {
  const classes = ['ui-orchestration-agent-conversation', className].filter(Boolean).join(' ');
  const canPrompt =
    conversation.mode === 'interactive' && conversation.input?.enabled === true && onSubmitPrompt;

  return (
    <section {...props} className={classes}>
      <header className="ui-orchestration-agent-conversation__header">
        <div>
          <p className="ui-orchestration-agent-conversation__eyebrow">{conversation.role}</p>
          <h2>{conversation.title}</h2>
          <p>{conversation.runtime.providerLabel}</p>
        </div>
        <StatusPill
          label={getAgentConversationStatusLabel(conversation)}
          showProvenance
          state={conversation.state.truth}
        />
      </header>

      <AgentConversationRuntimeMeta conversation={conversation} />
      <AgentConversationCurrentTurnIndicator conversation={conversation} />

      {conversation.state.unavailable ? (
        <section className="ui-orchestration-agent-conversation__notice">
          <strong>{conversation.state.unavailable.title}</strong>
          <p>{conversation.state.unavailable.detail}</p>
          <small>{getAgentConversationEvidenceLabel(conversation.state.unavailable.evidence)}</small>
        </section>
      ) : null}

      <AgentConversationTurnList turns={conversation.turns} />

      <AgentConversationAttachmentStrip attachments={conversation.attachments} />
      <AgentConversationArtifactStrip artifacts={conversation.artifacts} />

      {conversation.mode === 'interactive' ? (
        <AgentConversationPromptForm
          disabled={!canPrompt}
          disabledReason={
            conversation.input?.disabledReason ??
            (onSubmitPrompt ? undefined : 'No prompt handler is connected for this view.')
          }
          onSubmit={canPrompt ? onSubmitPrompt : undefined}
          placeholder={conversation.input?.placeholder}
        />
      ) : null}
    </section>
  );
}

export interface AgentConversationCurrentTurnIndicatorProps extends HTMLAttributes<HTMLElement> {
  conversation: AgentConversation;
}

export function AgentConversationCurrentTurnIndicator({
  className,
  conversation,
  ...props
}: AgentConversationCurrentTurnIndicatorProps) {
  const classes = ['ui-orchestration-agent-conversation__current-turn', className]
    .filter(Boolean)
    .join(' ');
  const summary =
    conversation.state.currentTurn?.summary ?? getAgentConversationStatusDescription(conversation);
  const evidence = conversation.state.currentTurn?.evidence ?? conversation.state.evidence;

  return (
    <section {...props} className={classes}>
      <div>
        <StatusPill
          label={getAgentConversationStatusLabel(conversation)}
          state={conversation.state.truth}
        />
        <h3>{getAgentConversationCurrentActionLabel(conversation)}</h3>
        <p>{summary}</p>
        <small>{getAgentConversationEvidenceLabel(evidence)}</small>
      </div>
    </section>
  );
}

export interface AgentConversationTurnListProps extends HTMLAttributes<HTMLDivElement> {
  turns: AgentConversationTurn[];
}

export function AgentConversationTurnList({
  className,
  turns,
  ...props
}: AgentConversationTurnListProps) {
  return (
    <ConversationThread
      {...props}
      className={className}
      emptyLabel="No turns recorded for this conversation."
      messages={turns.map(toConversationMessageItem)}
    />
  );
}

export interface AgentConversationAttachmentStripProps extends HTMLAttributes<HTMLDivElement> {
  attachments: AgentConversationAttachment[];
}

export function AgentConversationAttachmentStrip({
  attachments,
  className,
  ...props
}: AgentConversationAttachmentStripProps) {
  const classes = ['ui-orchestration-agent-conversation__strip', className]
    .filter(Boolean)
    .join(' ');

  return (
    <section {...props} className={classes}>
      <h3>Files</h3>
      <FileList
        emptyLabel="No files are associated with this conversation."
        files={attachments.map(toFileItem)}
      />
    </section>
  );
}

export interface AgentConversationArtifactStripProps extends HTMLAttributes<HTMLDivElement> {
  artifacts: AgentConversationArtifact[];
}

export function AgentConversationArtifactStrip({
  artifacts,
  className,
  ...props
}: AgentConversationArtifactStripProps) {
  const classes = ['ui-orchestration-agent-conversation__strip', className]
    .filter(Boolean)
    .join(' ');

  return (
    <section {...props} className={classes}>
      <h3>Artifacts</h3>
      <FileList
        emptyLabel="No artifacts are associated with this conversation."
        files={artifacts.map(toFileItem)}
      />
    </section>
  );
}

interface AgentConversationPromptFormProps {
  disabled: boolean;
  disabledReason?: string;
  onSubmit?: (body: string) => void;
  placeholder?: string;
}

function AgentConversationPromptForm({
  disabled,
  disabledReason,
  onSubmit,
  placeholder = 'Add a prompt',
}: AgentConversationPromptFormProps) {
  const [body, setBody] = useState('');

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = body.trim();

    if (!trimmed || disabled || !onSubmit) {
      return;
    }

    onSubmit(trimmed);
    setBody('');
  };

  return (
    <form className="ui-orchestration-agent-conversation__prompt" onSubmit={handleSubmit}>
      <textarea
        disabled={disabled}
        onChange={(event) => setBody(event.currentTarget.value)}
        placeholder={placeholder}
        value={body}
      />
      <div>
        {disabledReason ? <small>{disabledReason}</small> : null}
        <Button
          disabled={disabled || body.trim().length === 0}
          trailingIcon={<Send aria-hidden="true" size={16} />}
          type="submit"
          variant="primary"
        >
          Send
        </Button>
      </div>
    </form>
  );
}

function AgentConversationRuntimeMeta({ conversation }: { conversation: AgentConversation }) {
  return (
    <dl className="ui-orchestration-agent-conversation__meta">
      <div>
        <dt>Internal conversation</dt>
        <dd>{conversation.id}</dd>
      </div>
      <div>
        <dt>External thread</dt>
        <dd>{conversation.externalThreadId ?? 'No external runtime thread id recorded.'}</dd>
      </div>
      <div>
        <dt>Runtime</dt>
        <dd>{conversation.runtime.runtimeLabel ?? 'No runtime selected.'}</dd>
      </div>
      <div>
        <dt>Latest activity</dt>
        <dd>{conversation.state.latestActivity ?? 'No activity timestamp recorded.'}</dd>
      </div>
    </dl>
  );
}

function toConversationMessageItem(turn: AgentConversationTurn): ConversationMessageItem {
  return {
    author: turn.title ?? roleLabel(turn.role),
    body: turn.body,
    id: turn.id,
    role: turn.truth.provenance === 'mock_fixture' ? 'mock' : turn.role,
    sourceLabel: getAgentConversationEvidenceLabel(turn.evidence),
    state: turn.truth,
    timestampLabel: turn.createdAt,
  };
}

function toFileItem(
  item: AgentConversationAttachment | AgentConversationArtifact,
): OrchestrationFileItem {
  return {
    detailLabel: item.detail,
    evidenceLabel: item.evidence ? getAgentConversationEvidenceLabel(item.evidence) : undefined,
    id: item.id,
    kind: item.kind === 'local_draft' ? 'draft' : item.kind,
    name: item.name,
    state: item.truth,
  };
}

function roleLabel(role: AgentConversationTurn['role']): string {
  if (role === 'runtime') {
    return 'Runtime evidence';
  }

  if (role === 'system') {
    return 'System';
  }

  if (role === 'assistant') {
    return 'Assistant';
  }

  return 'User';
}
