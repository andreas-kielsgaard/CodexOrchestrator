import { render, screen } from '@testing-library/react';
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
  expect(await screen.findByRole('heading', { name: 'workflow' })).toBeVisible();
  expect(screen.getByText('Imported')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Refresh packages' }));
  expect(read).toHaveBeenCalledTimes(2);
  await user.click(screen.getByRole('tab', { name: 'Codex home profiles' }));
  expect(screen.getByLabelText('Manual path fallback')).toHaveValue('C:/home');
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
