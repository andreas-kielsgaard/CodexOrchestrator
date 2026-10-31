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
  expect(screen.getByRole('button', { name: 'Continuation' })).toBeVisible();
  expect(screen.getByRole('heading', { name: 'Workflow continuation' })).toBeVisible();
  expect(screen.queryByRole('checkbox', { name: /Include Handoff/ })).toBeNull();
  expect(screen.getByText('Unavailable selections')).toBeVisible();
  await user.click(screen.getByRole('checkbox', { name: 'Include Search' }));
  await user.click(screen.getByRole('button', { name: 'Apply selection' }));
  expect(change).toHaveBeenCalledWith([continuation, missing, server]);
});

it('toggles every endpoint exposed by an MCP server as one group', async () => {
  const user = userEvent.setup();
  const change = vi.fn();
  const server = otpCatalogue.flatMap((pkg) => pkg.agentMcpServers ?? [])[0];
  const tools = server.tools.slice(0, 2);
  const values = tools.map((tool) => key(server.serverName, tool.id));
  render(
    <OtpMcpToolsPicker
      packages={otpCatalogue}
      catalog={{
        availability: 'available',
        options: tools.map((tool) => ({
          value: key(server.serverName, tool.id),
          label: tool.name,
        })),
      }}
      values={[]}
      onChange={change}
    />,
  );

  await user.click(screen.getByRole('button', { name: 'Set MCP tools' }));
  await user.click(screen.getByRole('checkbox', { name: `Toggle ${server.name}` }));
  await user.click(screen.getByRole('button', { name: 'Apply selection' }));

  expect(change).toHaveBeenCalledWith(values);
});

it('offers the artifact-backed curation tools under Job Agent', async () => {
  const user = userEvent.setup();
  const change = vi.fn();
  const pkg = otpCatalogue.find((item) => item.id === 'job_agent')!;
  const server = pkg.agentMcpServers[0];
  const tools = server.tools.filter((tool) => tool.capability === 'curation');
  render(
    <OtpMcpToolsPicker
      packages={[pkg]}
      catalog={{
        availability: 'available',
        options: tools.map((tool) => ({
          value: key(server.serverName, tool.id),
          label: tool.name,
        })),
      }}
      values={[]}
      onChange={change}
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Set MCP tools' }));
  expect(screen.getByRole('button', { name: 'Curation' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Export curation dataset' }));
  expect(screen.getByText('Results require artifact processing.')).toBeVisible();
  await user.click(screen.getByRole('checkbox', { name: 'Include Export curation dataset' }));
  await user.click(screen.getByRole('button', { name: 'Apply selection' }));
  expect(change).toHaveBeenCalledWith([key('job_agent', 'export_curation_dataset')]);
});
