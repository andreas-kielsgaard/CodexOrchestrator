import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface AgentMarkdownProps {
  children: string;
  className?: string;
}

export function AgentMarkdown({ children, className }: AgentMarkdownProps) {
  return (
    <div className={['agent-markdown', className].filter(Boolean).join(' ')}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
          a: ({ node, ...props }) => {
            void node;
            return <a {...props} target="_blank" rel="noreferrer" />;
          },
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}
