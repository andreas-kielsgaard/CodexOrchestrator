import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import type { HumanReviewLauncherClient } from '../../application/humanReviewLauncher';
import { WorktreeReviewSettings } from './WorktreeReviewSettings';

describe('WorktreeReviewSettings', () => {
  it('loads disabled by default and persists the opt-in cleanup setting', async () => {
    const user = userEvent.setup();
    const settings = vi.fn(async () => ({ cleanupDetachedBuilds: false }));
    const updateSettings = vi.fn(async (value: { cleanupDetachedBuilds: boolean }) => value);
    const client = { settings, updateSettings } as unknown as HumanReviewLauncherClient;

    render(<WorktreeReviewSettings client={client} />);

    const cleanup = await screen.findByRole('switch', {
      name: /Clean builds when their worktree becomes detached/,
    });
    expect(cleanup).not.toBeChecked();
    expect(cleanup).toBeEnabled();
    await user.click(cleanup);
    await waitFor(() =>
      expect(updateSettings).toHaveBeenCalledWith({ cleanupDetachedBuilds: true }),
    );
    expect(cleanup).toBeChecked();
    expect(screen.getByRole('status')).toHaveTextContent('cleanup setting saved');
  });
});
