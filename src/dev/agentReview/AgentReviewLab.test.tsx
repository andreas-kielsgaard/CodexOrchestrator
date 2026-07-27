import { render, screen, within } from '@testing-library/react';
import { AgentReviewLab } from './AgentReviewLab';

describe('AgentReviewLab', () => {
  it('keeps the proven CDP attachment distinct from the uninvoked MCP route', () => {
    render(<AgentReviewLab />);

    const lane = screen
      .getByRole('heading', { name: 'Windows Tauri / WebView2 attachment' })
      .closest('article');
    expect(lane).not.toBeNull();
    expect(within(lane!).getByText('Verified attachment')).toBeVisible();
    expect(
      within(lane!).getByText(/Playwright CDP was proven; Chrome DevTools MCP was not invoked\./),
    ).toBeVisible();
  });

  it('reports the accepted native IPC proof without expanding it to mocking or production', () => {
    render(<AgentReviewLab />);

    const lane = screen.getByRole('heading', { name: 'Native Tauri E2E' }).closest('article');
    expect(lane).not.toBeNull();
    expect(within(lane!).getByText('Verified native IPC')).toBeVisible();
    expect(
      within(lane!).getByText(/Command mocking, cross-platform behavior, visual fidelity/),
    ).toBeVisible();
    expect(within(lane!).getByText('accepted')).toBeVisible();
  });

  it('warns against production authority and links only to the recorded browser scenario', () => {
    render(<AgentReviewLab />);

    const warning = screen.getByRole('note', { name: 'Production warning' });
    expect(within(warning).getByText('Development evidence only')).toBeVisible();
    expect(warning).toHaveTextContent(
      'It grants no production, native, or orchestration authority.',
    );
    expect(screen.getByRole('link', { name: 'Open recorded Plan Builder' })).toHaveAttribute(
      'href',
      '?recorded-plan-builder',
    );
    expect(screen.queryByRole('button', { name: /run|attach/i })).not.toBeInTheDocument();
  });

  it('describes the worktree-runtime handoff without claiming integration', () => {
    render(<AgentReviewLab />);

    expect(
      screen.getByRole('heading', { name: 'Owned instance → bounded evidence' }),
    ).toBeVisible();
    expect(screen.getByText('Interface defined · application integration unproven')).toBeVisible();
    expect(
      screen.getByText(
        /This branch defines but does not yet integrate the application-to-worktree-runtime/,
      ),
    ).toBeVisible();
    expect(screen.getByText(/semantic references, not process or driver authority/)).toBeVisible();
  });
});
