import { fireEvent, render, screen, within } from '@testing-library/react';
import type { WorktreeBuildClient } from '../../application/worktreeBuild';
import { WorktreeBuildShell } from './WorktreeBuildShell';

describe('WorktreeBuildShell', () => {
  it('keeps provenance visible and opens typed details plus one scoped file review', async () => {
    const client = fakeClient();
    render(
      <WorktreeBuildShell client={client}>
        <main>Orchestration surface</main>
      </WorktreeBuildShell>,
    );

    const indicator = await screen.findByRole('button', {
      name: 'Open Worktree details for Alpha',
    });
    expect(indicator).toHaveTextContent('codex/alpha');
    expect(indicator).toHaveTextContent('Dirty');
    fireEvent.click(indicator);
    expect(screen.getByRole('main', { name: 'Worktree details' })).toHaveTextContent('abc1234');
    expect(screen.getByRole('main', { name: 'Worktree details' })).toHaveTextContent(
      '1 ahead, 2 behind',
    );
    expect(screen.getByRole('button', { name: 'Open Worktree details for Alpha' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Review files and changes' }));
    const review = await screen.findByRole('main', { name: 'Files and diffs' });
    expect(within(review).queryByLabelText('Review source')).toBeNull();
    expect(within(review).getByText('Committed divergence + Uncommitted change')).toBeVisible();
    const modes = within(review).getByRole('group', { name: 'File inspection mode' });
    const layout = within(review).getByRole('group', { name: 'Diff layout' });
    expect(modes.compareDocumentPosition(layout) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Worktree details' }));
    fireEvent.click(screen.getByRole('button', { name: 'Application' }));
    expect(screen.getByText('Orchestration surface')).toBeVisible();
    expect(client.markReady).toHaveBeenCalledOnce();
  });

  it('accepts only enumerated non-activating proof navigation state', async () => {
    const client = fakeClient();
    client.proofNavigation = vi.fn(async () => ({
      route: 'worktree-details' as const,
      sequence: '0123456789abcdef0123456789abcdef',
    }));
    render(
      <WorktreeBuildShell client={client}>
        <main>Application surface</main>
      </WorktreeBuildShell>,
    );

    expect(await screen.findByRole('main', { name: 'Worktree details' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Open Worktree details for Alpha' })).toBeVisible();
  });
});

function fakeClient(): WorktreeBuildClient & { markReady: ReturnType<typeof vi.fn> } {
  return {
    markReady: vi.fn(async () => undefined),
    proofNavigation: vi.fn(async () => null),
    context: async () => ({
      name: 'Alpha',
      branch: 'codex/alpha',
      detached: false,
      head: {
        id: 'abc123456789',
        abbreviatedId: 'abc1234',
        message: 'Feature',
        committedAt: '2026-07-29T10:00:00Z',
      },
      dirty: { dirty: true, staged: 1, unstaged: 1, untracked: 1 },
      main: {
        branch: 'main',
        detached: false,
        head: {
          id: 'def123456789',
          abbreviatedId: 'def1234',
          message: 'Main',
          committedAt: '2026-07-28T10:00:00Z',
        },
        dirty: { dirty: false, staged: 0, unstaged: 0, untracked: 0 },
      },
      relationship: {
        ahead: 1,
        behind: 2,
        mergeBase: 'fff123456789',
        summary: '1 ahead, 2 behind machine main HEAD',
      },
      history: [],
      comparisonBasis: 'Machine main HEAD compared with complete selected state.',
    }),
    comparison: {
      load: async () => ({
        files: [
          {
            fileId: 'file-1',
            displayPath: 'src/example.ts',
            changeKind: 'modified',
            additions: 1,
            deletions: 1,
            provenance: ['committed-divergence', 'uncommitted'],
            content: { kind: 'text', text: 'new\n' },
            hunks: [
              {
                hunkId: 'hunk-1',
                header: '@@ -1,1 +1,1 @@',
                lines: [
                  { kind: 'deletion', oldLineNumber: 1, text: 'old' },
                  { kind: 'addition', newLineNumber: 1, text: 'new' },
                ],
              },
            ],
          },
        ],
      }),
    },
  };
}
