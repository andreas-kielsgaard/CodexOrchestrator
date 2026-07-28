import { MarkdownContent } from '../../components/MarkdownContent';

interface AgentMarkdownProps {
  children: string;
  className?: string;
}

export function AgentMarkdown({ children, className }: AgentMarkdownProps) {
  return (
    <MarkdownContent className={['agent-markdown', className].filter(Boolean).join(' ')}>
      {children}
    </MarkdownContent>
  );
}
