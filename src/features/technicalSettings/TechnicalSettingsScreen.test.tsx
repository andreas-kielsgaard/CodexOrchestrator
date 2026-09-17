import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import { TechnicalSettingsScreen } from './TechnicalSettingsScreen';
import { OtpConfigurationPanel } from './OtpConfigurationPanel';
import { otpCatalogue } from '../workflowAuthoring/testFixtures';

it('shows the imported catalogue and preserves home input across accessible tabs', async () => {
  const user = userEvent.setup();
  const client = {
    load: async () => ({ profiles: [] }),
    discoverHomes: async () => [],
  } as unknown as NativeProfileClient;
  const read = vi.fn(async () => otpCatalogue);
  render(<TechnicalSettingsScreen client={client} readOtpCatalogue={read} />);
  await user.type(screen.getByLabelText('Manual path fallback'), 'C:/home');
  await user.click(screen.getByRole('tab', { name: 'OTP configuration' }));
  const workflowCard = (await screen.findByRole('heading', { name: 'Workflow' })).closest(
    'article',
  );
  expect(workflowCard).not.toBeNull();
  expect(screen.getAllByText('Imported')).toHaveLength(2);
  await user.click(
    within(workflowCard as HTMLElement).getByRole('button', { name: 'View details' }),
  );
  expect(screen.getByRole('heading', { name: 'Workflow' })).toBeVisible();
  const elements = screen.getByRole('navigation', { name: 'Workflow offered elements' });
  const continuation = within(elements).getByRole('button', { name: 'Workflow continuation' });
  expect(elements).toHaveClass('otp-catalogue__element-list');
  await user.click(continuation);
  expect(continuation).toHaveAttribute('aria-pressed', 'true');
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(screen.getByText('Expected behavior')).toBeVisible();
  expect(screen.getByText('Declared results')).toBeVisible();
  await user.click(continuation);
  expect(continuation).toHaveAttribute('aria-pressed', 'false');
  expect(screen.getByText('Imported Orchestration Tool Package')).toBeVisible();
  await user.click(screen.getByRole('button', { name: '← OTP configuration' }));
  await user.click(screen.getByRole('button', { name: 'Refresh packages' }));
  expect(read).toHaveBeenCalledTimes(2);
  await user.click(screen.getByRole('tab', { name: 'Codex home profiles' }));
  expect(screen.getByLabelText('Manual path fallback')).toHaveValue('C:/home');
});

it('shows Job Agent capability groups and endpoint documentation', async () => {
  const user = userEvent.setup();
  const view = render(<OtpConfigurationPanel readCatalogue={async () => otpCatalogue} />);
  const jobAgentCard = (await screen.findByRole('heading', { name: 'Job Agent' })).closest(
    'article',
  );
  expect(jobAgentCard).not.toBeNull();
  await user.click(
    within(jobAgentCard as HTMLElement).getByRole('button', { name: 'View details' }),
  );
  const elements = screen.getByRole('navigation', { name: 'Job Agent offered elements' });
  const sourceDiscovery = within(elements).getByRole('button', { name: 'Source discovery' });
  await user.click(sourceDiscovery);
  expect(sourceDiscovery).toHaveAttribute('aria-pressed', 'true');
  expect(screen.getByRole('heading', { name: 'Source discovery' })).toBeVisible();
  await user.click(sourceDiscovery);
  expect(sourceDiscovery).toHaveAttribute('aria-pressed', 'false');
  expect(screen.getByText('Imported Orchestration Tool Package')).toBeVisible();
  const sourceCatalog = within(elements).getByRole('button', { name: 'Get source catalog' });
  await user.click(sourceCatalog);
  expect(sourceCatalog).toHaveAttribute('aria-pressed', 'true');
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(screen.getByText('Successful response')).toBeVisible();
  expect(screen.getByText('Required authorization')).toBeVisible();
  expect(screen.getByText(/configured agent sessions/i)).toBeVisible();
  view.unmount();
});
it('distinguishes no imported package from a failed read', async () => {
  const read = vi.fn(async () => []);
  const view = render(<OtpConfigurationPanel readCatalogue={read} />);
  expect(await screen.findByText('No OTP packages are imported.')).toBeVisible();
  view.rerender(
    <OtpConfigurationPanel
      readCatalogue={async () => {
        throw new Error('Unavailable registry');
      }}
    />,
  );
  expect(await screen.findByRole('alert')).toHaveTextContent('Unavailable registry');
  expect(screen.queryByText('Imported')).toBeNull();
});
