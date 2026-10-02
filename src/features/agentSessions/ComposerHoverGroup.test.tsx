import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ComposerHoverGroup } from './ComposerHoverGroup';

it('opens above its trigger without removing it, and closes immediately outside the group', () => {
  vi.useFakeTimers();
  try {
    render(
      <ComposerHoverGroup label="Target settings" summary="Target">
        <button>Choose worktree</button>
      </ComposerHoverGroup>,
    );
    const trigger = screen.getByRole('button', { name: 'Target settings' });
    expect(screen.queryByRole('button', { name: 'Choose worktree' })).not.toBeInTheDocument();
    fireEvent.mouseEnter(trigger);
    const overlay = screen.getByRole('group', { name: 'Target settings' });
    expect(overlay).toBeVisible();
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    fireEvent.mouseLeave(trigger.parentElement!);
    expect(overlay).not.toBeVisible();
    fireEvent.mouseEnter(overlay);
    expect(overlay).toBeVisible();
    fireEvent.mouseLeave(trigger.parentElement!);
    expect(overlay).not.toBeVisible();
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
  } finally {
    vi.useRealTimers();
  }
});

it('opens for keyboard access and Escape dismisses it without moving other controls', async () => {
  const user = userEvent.setup();
  render(
    <ComposerHoverGroup label="Model settings" summary="Model">
      <select aria-label="Model">
        <option>Sol</option>
      </select>
    </ComposerHoverGroup>,
  );
  await user.tab();
  expect(screen.getByRole('button', { name: 'Model settings' })).toHaveFocus();
  await user.keyboard('{Enter}');
  expect(screen.getByRole('combobox', { name: 'Model' })).toBeVisible();
  await user.tab();
  expect(screen.getByRole('combobox', { name: 'Model' })).toHaveFocus();
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('combobox', { name: 'Model' })).not.toBeInTheDocument();
});
