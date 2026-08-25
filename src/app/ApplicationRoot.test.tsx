import { act, render, screen } from '@testing-library/react';

import { ApplicationRoot } from './ApplicationRoot';

describe('ApplicationRoot', () => {
  it('mounts Worktree Review as a normal product capability', async () => {
    render(<ApplicationRoot />);
    await act(async () => undefined);

    expect(screen.getByRole('main', { name: 'Orchestration' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Plan an Epic' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Worktree Review' })).toBeVisible();
  });
});
