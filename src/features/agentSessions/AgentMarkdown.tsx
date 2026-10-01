import { MarkdownContent } from '../../components/MarkdownContent';
import { memo } from 'react';

interface AgentMarkdownProps {
  children: string;
  className?: string;
}

export const AgentMarkdown = memo(function AgentMarkdown({
  children,
  className,
}: AgentMarkdownProps) {
  return (
    <MarkdownContent className={['agent-markdown', className].filter(Boolean).join(' ')}>
      {children}
    </MarkdownContent>
  );
});
