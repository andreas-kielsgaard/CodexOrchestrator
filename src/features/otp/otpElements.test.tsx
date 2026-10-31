import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { OtpElementDetails } from './otpElements';
import { OtpPackageDetailPage } from '../otpCatalogue/OtpPackageDetailPage';
import { otpCatalogue } from '../workflowAuthoring/testFixtures';

const pkg = otpCatalogue.find((item) => item.id === 'job_agent')!;
const server = pkg.agentMcpServers[0];
const exportTool = server.tools.find((item) => item.id === 'export_curation_dataset')!;

it('shows curation endpoints and toggles their details back to the package', async () => {
  const user = userEvent.setup();
  render(<OtpPackageDetailPage pkg={pkg} onBack={vi.fn()} />);
  expect(screen.getByRole('button', { name: 'Curation' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Export curation dataset' }));
  expect(screen.getByRole('heading', { name: 'Result handling' })).toBeVisible();
  expect(screen.getByText('Results require artifact processing.')).toBeVisible();
  expect(screen.getByText(/Bring only selected evidence and bounded results/)).toBeVisible();
  expect(screen.getByText(/Content types: application\/x-ndjson, application\/json/)).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Export curation dataset' }));
  expect(screen.queryByRole('heading', { name: 'Result handling' })).toBeNull();
  expect(screen.getByRole('heading', { name: 'Job Agent' })).toBeVisible();
});

it('does not invent behavior for unrecognized metadata or dispositions', () => {
  render(
    <OtpElementDetails
      server={server}
      endpoint={{
        ...exportTool,
        meta: {
          'example.org/future': true,
          'com.orchid/resultHandling': { disposition: 'future' },
        },
      }}
    />,
  );
  expect(screen.queryByRole('heading', { name: 'Result handling' })).toBeNull();
});

it('keeps existing source endpoint details usable without result metadata', () => {
  render(
    <OtpElementDetails
      server={server}
      endpoint={server.tools.find((item) => item.id === 'get_source_catalog')!}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Get source catalog' })).toBeVisible();
  expect(screen.queryByRole('heading', { name: 'Result handling' })).toBeNull();
});
