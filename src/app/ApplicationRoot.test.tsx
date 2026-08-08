import { act, render, screen } from '@testing-library/react';
import { vi } from 'vitest';

vi.mock('../infrastructure/tauriHumanReviewLauncher', () => new Promise(() => undefined));

import { ApplicationRoot } from './ApplicationRoot';

describe('ApplicationRoot', () => {
  it('mounts product controls before the optional development review chunk resolves', async () => {
    render(<ApplicationRoot />);
    await act(async () => undefined);

    expect(screen.getByRole('main', { name: 'Orchestration' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Plan an Epic' })).toBeVisible();
  });
});
