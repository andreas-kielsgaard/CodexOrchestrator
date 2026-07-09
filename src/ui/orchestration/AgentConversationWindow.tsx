import { ExternalLink } from 'lucide-react';
import type { HTMLAttributes } from 'react';
import {
  canClaimAgentConversationActiveWork,
  getAgentConversationLatestSummary,
  getAgentConversationStatusDescription,
  getAgentConversationStatusLabel,
  type AgentConversation,
} from '../../domain/agentConversation';
import { Button } from '../Button';
import { StatusPill } from './StatusPill';

export interface AgentConversationWindowProps extends HTMLAttributes<HTMLElement> {
  conversation: AgentConversation;
  onOpen?: () => void;
  openLabel?: string;
}

export function AgentConversationWindow({
  className,
  conversation,
  onOpen,
  openLabel = 'Open',
  ...props
}: AgentConversationWindowProps) {
  const classes = ['ui-orchestration-agent-window', className].filter(Boolean).join(' ');
  const activeWork = canClaimAgentConversationActiveWork(conversation);

  return (
    <article {...props} className={classes}>
      <header>
        <div>
          <p className="ui-orchestration-agent-window__eyebrow">{conversation.role}</p>
          <h3>{conversation.title}</h3>
        </div>
        <StatusPill
          label={getAgentConversationStatusLabel(conversation)}
          state={conversation.state.truth}
        />
      </header>
      <p>{getAgentConversationLatestSummary(conversation)}</p>
      <dl>
        <div>
          <dt>Active work</dt>
          <dd>{activeWork ? 'Runtime evidence confirms active work.' : 'No active runtime work.'}</dd>
        </div>
        <div>
          <dt>Last updated</dt>
          <dd>{conversation.state.latestActivity ?? 'No activity timestamp recorded.'}</dd>
        </div>
        <div>
          <dt>External thread</dt>
          <dd>{conversation.externalThreadId ?? 'No external runtime thread id recorded.'}</dd>
        </div>
      </dl>
      <footer>
        <small>{getAgentConversationStatusDescription(conversation)}</small>
        {onOpen ? (
          <Button
            onClick={onOpen}
            trailingIcon={<ExternalLink aria-hidden="true" size={16} />}
            variant="secondary"
          >
            {openLabel}
          </Button>
        ) : (
          <small>No conversation route is connected for this window.</small>
        )}
      </footer>
    </article>
  );
}

