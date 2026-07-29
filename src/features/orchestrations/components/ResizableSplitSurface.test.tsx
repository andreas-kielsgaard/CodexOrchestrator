import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach } from 'vitest';
import { ResizableSplitSurface } from './ResizableSplitSurface';

describe('ResizableSplitSurface', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('exposes dynamic separator values and a separately accessible maximize action', () => {
    const { container } = render(
      <ResizableSplitSurface
        axis="vertical"
        primary={<div>Flow content</div>}
        secondary={<div>Session content</div>}
        primaryLabel="Flow"
        secondaryLabel="Agent Session"
        maximizePrimaryLabel="Maximize flow"
      />,
    );

    const host = container.querySelector('.resizable-split') as HTMLDivElement;
    vi.spyOn(host, 'getBoundingClientRect').mockReturnValue(rect(1000, 1000));
    const separator = screen.getByRole('separator', {
      name: 'Resize Flow and Agent Session',
    });
    const maximize = screen.getByRole('button', { name: 'Maximize flow' });
    const paneIds = separator.getAttribute('aria-controls')?.split(' ') ?? [];

    expect(separator).toHaveAttribute('aria-orientation', 'horizontal');
    expect(separator).toHaveAttribute('aria-valuenow', '70');
    expect(maximize).toHaveAttribute('aria-controls', paneIds[0]);
    expect(separator).not.toContainElement(maximize);
    expect(paneIds).toHaveLength(2);
    expect(document.getElementById(paneIds[0])).toHaveAccessibleName('Flow');
    expect(document.getElementById(paneIds[1])).toHaveAccessibleName('Agent Session');

    fireEvent.keyDown(separator, { key: 'Home' });
    expect(separator).toHaveAttribute('aria-valuemin', '16');
    expect(separator).toHaveAttribute('aria-valuemax', '88');
    expect(separator).toHaveAttribute('aria-valuenow', '16');
    fireEvent.keyDown(separator, { key: 'ArrowDown' });
    expect(separator).toHaveAttribute('aria-valuenow', '18');
    expect(separator).toHaveAttribute('aria-valuetext', '18% allocated to Flow');
    fireEvent.keyDown(separator, { key: 'End' });
    expect(separator).toHaveAttribute('aria-valuenow', '88');

    fireEvent.keyDown(separator, { key: 'Home' });
    fireEvent.click(maximize);
    expect(separator).toHaveAttribute('aria-valuenow', '88');
    expect(screen.getByText('Flow content')).toBeVisible();
    expect(screen.getByText('Session content')).toBeInTheDocument();

    fireEvent.pointerDown(separator, { clientY: 0 });
    const pointerMove = new Event('pointermove');
    Object.defineProperty(pointerMove, 'clientY', { value: 500 });
    fireEvent(window, pointerMove);
    fireEvent.pointerUp(window);
    expect(separator).toHaveAttribute('aria-valuenow', '50');
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

  it('updates orientation and values when responsive conversations change axis', async () => {
    let compact = false;
    let onChange: (() => void) | undefined;
    const media = {
      get matches() {
        return compact;
      },
      media: '(max-width: 720px)',
      onchange: null,
      addEventListener: vi.fn((_type: string, listener: () => void) => {
        onChange = listener;
      }),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    };
    vi.stubGlobal(
      'matchMedia',
      vi.fn(() => media),
    );

    const { container } = render(
      <ResizableSplitSurface
        axis="horizontal"
        primary={<div>Planner</div>}
        secondary={<div>Worker</div>}
        primaryLabel="Planner conversation"
        secondaryLabel="Worker conversation"
      />,
    );
    const host = container.querySelector('.resizable-split') as HTMLDivElement;
    vi.spyOn(host, 'getBoundingClientRect').mockReturnValue(rect(1000, 800));
    const separator = screen.getByRole('separator');
    expect(separator).toHaveAttribute('aria-orientation', 'vertical');

    compact = true;
    act(() => onChange?.());
    await waitFor(() => expect(separator).toHaveAttribute('aria-orientation', 'horizontal'));
    expect(container.querySelector('.resizable-split')).toHaveAttribute(
      'data-effective-split-axis',
      'vertical',
    );
    fireEvent.keyDown(separator, { key: 'Home' });
    expect(separator).toHaveAttribute('aria-valuemin', '20');
    expect(separator).toHaveAttribute('aria-valuemax', '85');
    expect(separator).toHaveAttribute('aria-valuenow', '20');
    fireEvent.keyDown(separator, { key: 'ArrowDown' });
    expect(separator).toHaveAttribute('aria-valuenow', '23');
  });

  it('uses the wider compact breakpoint for Agent Session navigation', () => {
    const matchMedia = vi.fn(() => ({
      matches: true,
      media: '(max-width: 860px)',
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));
    vi.stubGlobal('matchMedia', matchMedia);

    const { container } = render(
      <ResizableSplitSurface
        axis="horizontal"
        primary={<div>Session tree</div>}
        secondary={<div>Selected Session</div>}
        primaryLabel="Agent Session navigation"
        secondaryLabel="Selected Agent Session"
        compactBreakpoint={860}
      />,
    );

    expect(matchMedia).toHaveBeenCalledWith('(max-width: 860px)');
    expect(container.querySelector('.resizable-split')).toHaveClass('resizable-split--compact-860');
    expect(screen.getByRole('separator')).toHaveAttribute('aria-orientation', 'horizontal');
  });
});

function rect(width: number, height: number): DOMRect {
  return {
    x: 0,
    y: 0,
    top: 0,
    right: width,
    bottom: height,
    left: 0,
    width,
    height,
    toJSON: () => ({}),
  };
}
