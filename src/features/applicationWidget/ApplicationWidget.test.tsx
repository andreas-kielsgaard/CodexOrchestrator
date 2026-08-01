import { fireEvent, render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { ApplicationWidget } from './ApplicationWidget';

const css = readFileSync('src/features/applicationWidget/applicationWidget.css', 'utf8');

describe('ApplicationWidget', () => {
  it('provides a reusable bottom-right, non-overlapping dock widget with minimize and restore', () => {
    const open = vi.fn();
    render(
      <div className="application-widget-dock">
        <ApplicationWidget
          label="Worktree build"
          title="Alpha"
          summary="codex/alpha · Clean"
          icon={<span aria-hidden="true">W</span>}
          onOpen={open}
        />
      </div>,
    );

    const widget = screen.getByRole('complementary', { name: 'Worktree build widget' });
    expect(widget).toHaveAttribute('data-placement', 'bottom-right');
    fireEvent.click(screen.getByRole('button', { name: 'Open Worktree build details for Alpha' }));
    expect(open).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole('button', { name: 'Minimize Worktree build widget' }));
    expect(screen.getByRole('button', { name: 'Restore Worktree build widget' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Restore Worktree build widget' }));
    expect(screen.getByText('Alpha')).toBeVisible();

    expect(css).toMatch(/\.application-widget-dock\s*{[^}]*justify-content:\s*flex-end;/s);
    expect(css).toMatch(/\.application-widget-dock\s*{[^}]*min-height:\s*48px;/s);
    expect(css).toMatch(/\.application-widget\s*{[^}]*width:\s*min\(360px, 100%\);/s);
  });
});
