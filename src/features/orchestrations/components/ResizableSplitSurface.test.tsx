import { fireEvent, render, screen } from '@testing-library/react';
import { ResizableSplitSurface } from './ResizableSplitSurface';

describe('ResizableSplitSurface', () => {
  it('exposes a keyboard-resizable separator and keeps both panes mounted when maximized', () => {
    render(
      <ResizableSplitSurface
        axis="vertical"
        primary={<div>Flow content</div>}
        secondary={<div>Session content</div>}
        primaryLabel="Flow"
        secondaryLabel="Agent Session"
        maximizePrimaryLabel="Maximize flow"
      />,
    );

    const separator = screen.getByRole('separator', {
      name: 'Resize Flow and Agent Session',
    });
    expect(separator).toHaveAttribute('aria-orientation', 'horizontal');
    fireEvent.keyDown(separator, { key: 'End' });
    expect(screen.getByText('Flow content')).toBeVisible();
    expect(screen.getByText('Session content')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Maximize flow' }));
    expect(screen.getByLabelText('Agent Session')).toBeInTheDocument();
  });

  it('uses a vertical separator for side-by-side conversations', () => {
    render(
      <ResizableSplitSurface
        axis="horizontal"
        primary={<div>Planner</div>}
        secondary={<div>Worker</div>}
        primaryLabel="Planner conversation"
        secondaryLabel="Worker conversation"
      />,
    );
    expect(screen.getByRole('separator')).toHaveAttribute('aria-orientation', 'vertical');
  });

  it('switches horizontal conversations to vertical resize semantics below the breakpoint', () => {
    vi.stubGlobal('matchMedia', (query: string) => ({
      matches: query === '(max-width: 720px)',
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const { container } = render(
      <ResizableSplitSurface
        axis="horizontal"
        primary={<div>Planner</div>}
        secondary={<div>Worker</div>}
        primaryLabel="Planner conversation"
        secondaryLabel="Worker conversation"
      />,
    );

    expect(screen.getByRole('separator')).toHaveAttribute('aria-orientation', 'horizontal');
    expect(container.querySelector('.resizable-split')).toHaveAttribute(
      'data-effective-split-axis',
      'vertical',
    );

    vi.unstubAllGlobals();
  });
});
