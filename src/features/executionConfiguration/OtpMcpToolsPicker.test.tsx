import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { OtpMcpToolsPicker } from './OtpMcpToolsPicker';
import { otpCatalogue } from '../workflowAuthoring/testFixtures';
import { mcpToolCatalogValue as key } from './types';

it('groups by OTP while preserving server tools and unavailable saved choices', async () => {
  const user = userEvent.setup();
  const change = vi.fn();
  const continuation = key('workflow', 'trigger_workflow_continuation');
  const server = key('external', 'search');
  const missing = key('workflow', 'handoff_to_agent');
  render(
    <OtpMcpToolsPicker
      packages={otpCatalogue}
      catalog={{
        availability: 'available',
        options: [
          { value: continuation, label: 'Continuation' },
          { value: server, label: 'Search' },
        ],
      }}
      values={[continuation, missing]}
      onChange={change}
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Set MCP tools' }));
  expect(screen.getByRole('button', { name: 'Workflow continuation' })).toBeVisible();
  expect(screen.queryByRole('checkbox', { name: /Include Handoff/ })).toBeNull();
  expect(screen.getByText('Unavailable selections')).toBeVisible();
  await user.click(screen.getByRole('button', { name: /MCP server: external/ }));
  await user.click(screen.getByRole('checkbox', { name: 'Include Search' }));
  await user.click(screen.getByRole('button', { name: 'Apply selection' }));
  expect(change).toHaveBeenCalledWith([continuation, missing, server]);
});
