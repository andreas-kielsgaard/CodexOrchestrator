import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { ApplicationRoot } from './ApplicationRoot';

describe('ApplicationRoot development compositions', () => {
  afterEach(() => {
    window.history.replaceState({}, '', '/');
  });

  it('loads the explicit agent test-mode composition in development', async () => {
    window.history.replaceState({}, '', '/?agent-test-mode');

    render(<ApplicationRoot />);

    expect(
      await screen.findByRole('heading', {
        name: 'Exercise the app through semantic controls',
      }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Development-only · synthetic data/)).toBeInTheDocument();
  });
});
