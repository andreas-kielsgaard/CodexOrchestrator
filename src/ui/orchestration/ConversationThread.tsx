import type { HTMLAttributes } from 'react';
import { StatusPill } from './StatusPill';
import type { ConversationMessageItem } from './types';

export interface ConversationThreadProps extends HTMLAttributes<HTMLDivElement> {
  emptyLabel?: string;
  messages: ConversationMessageItem[];
}

const roleLabels: Record<ConversationMessageItem['role'], string> = {
  assistant: 'Assistant',
  mock: 'Mock/demo',
  runtime: 'Runtime evidence',
  system: 'System',
  user: 'User',
};

export function ConversationThread({
  className,
  emptyLabel = 'No conversation output yet.',
  messages,
  ...props
}: ConversationThreadProps) {
  const classes = ['ui-orchestration-conversation-thread', className].filter(Boolean).join(' ');

  if (messages.length === 0) {
    return <p className="ui-orchestration-empty">{emptyLabel}</p>;
  }

  return (
    <div {...props} className={classes}>
      {messages.map((message) => (
        <article data-role={message.role} key={message.id}>
          <header>
            <div>
              <strong>{message.author ?? roleLabels[message.role]}</strong>
              <small>{message.sourceLabel ?? roleLabels[message.role]}</small>
            </div>
            {message.state ? <StatusPill state={message.state} /> : null}
          </header>
          <p>{message.body}</p>
          {message.timestampLabel ? <time>{message.timestampLabel}</time> : null}
        </article>
      ))}
    </div>
  );
}
