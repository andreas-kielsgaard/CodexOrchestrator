import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { ApplicationRoot } from './ApplicationRoot';

describe('ApplicationRoot development compositions', () => {
  afterEach(() => {
    window.history.replaceState({}, '', '/');
  });

  it('selects the test-mode peer tab in the recorded development composition', async () => {
    window.history.replaceState({}, '', '/?agent-test-mode');

    render(<ApplicationRoot />);

    const navigation = await screen.findByRole('navigation', { name: 'Application surfaces' });
    expect(navigation).toBeVisible();
    expect(screen.getByRole('button', { name: 'Orchestration' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Agent Sessions' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Test mode' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(
      await screen.findByRole('heading', {
        name: 'Exercise the app through semantic controls',
      }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Development-only · synthetic data/)).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'live-session' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestration' }));
    expect(await screen.findByRole('main', { name: 'Orchestration' })).toBeVisible();
  });
});
