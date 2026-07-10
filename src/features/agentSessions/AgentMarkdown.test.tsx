import { render, screen, within } from '@testing-library/react';
import { AgentMarkdown } from './AgentMarkdown';

describe('AgentMarkdown', () => {
  it('renders common agent response Markdown and GFM structures', () => {
    const { container } = render(
      <AgentMarkdown>{`# Result

- first
- second

\`\`\`ts
const answer = 42;
\`\`\`

| Name | State |
| --- | --- |
| Session | ready |

[Source](https://example.com)
`}</AgentMarkdown>,
    );

    expect(screen.getByRole('heading', { name: 'Result' })).toBeInTheDocument();
    expect(within(screen.getByRole('list')).getAllByRole('listitem')).toHaveLength(2);
    expect(screen.getByText('const answer = 42;')).toBeInTheDocument();
    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Source' })).toHaveAttribute('rel', 'noreferrer');
    expect(container.querySelector('pre code')).toBeInTheDocument();
  });

  it('does not interpret raw HTML from agent output', () => {
    const { container } = render(
      <AgentMarkdown>{'<script>alert("unsafe")</script><strong>raw</strong>'}</AgentMarkdown>,
    );

    expect(container.querySelector('script')).not.toBeInTheDocument();
    expect(container.querySelector('strong')).not.toBeInTheDocument();
  });
});
