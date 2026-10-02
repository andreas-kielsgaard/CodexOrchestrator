import { memo, useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { skillReferenceParts, useSkillReferences } from './SkillReferences';

interface AgentMarkdownProps {
  children: string;
  className?: string;
}

export const AgentMarkdown = memo(function AgentMarkdown({
  children,
  className,
}: AgentMarkdownProps) {
  const references = useSkillReferences();
  const referencePlugin = useMemo(
    () => () => (tree: MarkdownNode) => {
      const visit = (node: MarkdownNode) => {
        if (['code', 'inlineCode', 'link', 'linkReference', 'html'].includes(node.type)) return;
        node.children = node.children?.flatMap((child) => {
          if (child.type !== 'text') {
            visit(child);
            return [child];
          }
          return skillReferenceParts(child.value ?? '', references.skills).map((part) =>
            part.skill
              ? {
                  type: 'link',
                  url: `#orchid-skill-${encodeURIComponent(part.skill.id)}`,
                  children: [{ type: 'text', value: part.text }],
                }
              : { type: 'text', value: part.text },
          );
        });
      };
      visit(tree);
    },
    [references.skills],
  );
  return (
    <div className={['agent-markdown', className].filter(Boolean).join(' ')}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm, referencePlugin]}
        skipHtml
        components={{
          a: ({ node, href, children, ...props }) => {
            void node;
            const skill = href?.startsWith('#orchid-skill-')
              ? references.skills.find(
                  (skill) => `#orchid-skill-${encodeURIComponent(skill.id)}` === href,
                )
              : undefined;
            return skill ? (
              <button
                type="button"
                className="skill-reference"
                onClick={() => references.open(skill)}
              >
                {children}
              </button>
            ) : (
              <a {...props} href={href} target="_blank" rel="noreferrer">
                {children}
              </a>
            );
          },
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
});

interface MarkdownNode {
  type: string;
  value?: string;
  url?: string;
  children?: MarkdownNode[];
}
