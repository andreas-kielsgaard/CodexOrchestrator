import { fireEvent, render, screen, within } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import type { WorktreeBuildClient, WorktreeProofNavigation } from '../../application/worktreeBuild';
import { WorktreeBuildShell } from './WorktreeBuildShell';

const fileReviewCss = readFileSync('src/features/fileReview/fileReview.css', 'utf8');
const worktreeBuildCss = readFileSync('src/features/worktreeBuild/worktreeBuild.css', 'utf8');
const widgetCss = readFileSync('src/features/applicationWidget/applicationWidget.css', 'utf8');

describe('WorktreeBuildShell', () => {
  it('keeps provenance visible and opens typed details plus one scoped file review', async () => {
    const client = fakeClient();
    render(
      <WorktreeBuildShell client={client}>
        <main>Orchestration surface</main>
      </WorktreeBuildShell>,
    );

    const indicator = await screen.findByRole('button', {
      name: 'Open Worktree build details for Alpha',
    });
    expect(indicator).toHaveTextContent('codex/alpha');
    expect(indicator).toHaveTextContent('Dirty');
    fireEvent.click(indicator);
    const details = await screen.findByRole('main', { name: 'Worktree build details' });
    expect(details).toHaveTextContent('abc1234');
    expect(screen.getByRole('main', { name: 'Worktree build details' })).toHaveTextContent(
      '1 ahead, 2 behind',
    );
    expect(screen.getByText(/safe output line 24/)).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Open Worktree build details for Alpha' }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Review files and changes' }));
    const review = await screen.findByRole('main', { name: 'Files and diffs' });
    const route = review.closest('.worktree-build-route');
    const inspector = within(review).getByRole('region', { name: /a-very-long-nested-file-name/ });
    const toolbar = inspector.querySelector('.file-review-toolbar');
    expect(route).not.toBeNull();
    expect(toolbar).not.toBeNull();
    expect(worktreeBuildCss).toMatch(/\.worktree-build-route\s*{[^}]*min-width:\s*0;/s);
    for (const selector of [
      'file-review-screen',
      'file-review-workspace',
      'file-review-inspector',
      'file-review-toolbar',
    ]) {
      expect(fileReviewCss).toMatch(new RegExp(`\\.${selector}\\s*\\{[^}]*min-width:\\s*0;`, 's'));
    }
    expect(fileReviewCss).toMatch(
      /\.file-review-toolbar\s*{[^}]*display:\s*flex;[^}]*justify-content:\s*space-between;/s,
    );
    expect(fileReviewCss).toMatch(/\.file-review-toolbar__modes\s*{[^}]*flex:\s*none;/s);
    expect(fileReviewCss).toMatch(
      /@media\s*\(max-width:\s*860px\)\s*{[\s\S]*?\.file-review-toolbar\s*{[^}]*flex-direction:\s*column;/,
    );
    expect(fileReviewCss).toMatch(
      /\.file-review-path strong,\s*\.file-review-path small\s*{[^}]*text-overflow:\s*ellipsis;/s,
    );
    expect(within(review).queryByLabelText('Review source')).toBeNull();
    const modes = within(review).getByRole('group', { name: 'File inspection mode' });
    const layout = within(review).getByRole('group', { name: 'Diff layout' });
    expect(modes).toBeVisible();
    expect(layout).toBeVisible();
    const reservedLayoutSlot = layout.parentElement;
    expect(reservedLayoutSlot).toHaveClass('file-review-layout-slot');
    fireEvent.click(within(modes).getByRole('button', { name: 'File' }));
    expect(within(review).queryByRole('group', { name: 'Diff layout' })).toBeNull();
    expect(reservedLayoutSlot).toBeInTheDocument();
    fireEvent.click(within(modes).getByRole('button', { name: 'Changes' }));
    expect(within(reservedLayoutSlot!).getByRole('group', { name: 'Diff layout' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Build details' }));
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    expect(screen.getByText('Orchestration surface')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Minimize Worktree build widget' }));
    expect(screen.getByRole('button', { name: 'Restore Worktree build widget' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Restore Worktree build widget' }));
    expect(widgetCss).toMatch(/\.application-widget-dock\s*{[^}]*justify-content:\s*flex-end;/s);
    expect(worktreeBuildCss).toMatch(
      /\.worktree-build-shell\s*{[^}]*grid-template-rows:\s*minmax\(0, 1fr\) auto;/s,
    );
    expect(client.markReady).toHaveBeenCalledOnce();
  });

  it('applies only enumerated non-activating widget lifecycle and detail proof state', async () => {
    const client = fakeClient();
    let navigation: WorktreeProofNavigation = {
      route: 'widget-expanded',
      sequence: '0123456789abcdef0123456789abcdef',
    };
    client.proofNavigation = vi.fn(async () => navigation);
    render(
      <WorktreeBuildShell client={client}>
        <main>Application surface</main>
      </WorktreeBuildShell>,
    );

    expect(
      await screen.findByRole('button', { name: 'Open Worktree build details for Alpha' }),
    ).toBeVisible();
    navigation = {
      route: 'widget-minimized',
      sequence: '1123456789abcdef0123456789abcdef',
    };
    expect(
      await screen.findByRole('button', { name: 'Restore Worktree build widget' }),
    ).toBeVisible();
    navigation = {
      route: 'widget-restored',
      sequence: '2123456789abcdef0123456789abcdef',
    };
    expect(
      await screen.findByRole('button', { name: 'Open Worktree build details for Alpha' }),
    ).toBeVisible();
    navigation = {
      route: 'widget-build-details',
      sequence: '3123456789abcdef0123456789abcdef',
    };
    expect(await screen.findByRole('main', { name: 'Worktree build details' })).toBeVisible();
    expect(
      screen.getByRole('button', { name: 'Open Worktree build details for Alpha' }),
    ).toBeVisible();
  });
});

function fakeClient(): WorktreeBuildClient & { markReady: ReturnType<typeof vi.fn> } {
  const context = {
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
    relatedBranches: [
      {
        name: 'codex/parent',
        ahead: 1,
        behind: 0,
        mergeBase: 'fff123456789',
        summary: '1 ahead, 0 behind this local branch',
      },
    ],
    history: [],
    comparisonBasis: 'Machine main HEAD compared with complete selected state.',
  };
  return {
    markReady: vi.fn(async () => undefined),
    proofNavigation: vi.fn(async () => null),
    context: async () => context,
    detail: async () => ({
      instanceRef: 'opaque-instance',
      name: 'Alpha',
      sourceLabel: 'codex/alpha',
      purpose: 'Human review of one selected worktree.',
      phase: 'running',
      health: 'healthy',
      stale: false,
      build: 'passed',
      compatibility: 'compatible',
      compatibilityMessage: 'Compatible.',
      orientation: 'One retained source identity and its owned review lifecycle.',
      prepareProduced: 'Prepare reserved isolated state.',
      buildProduced: 'Build produced private artifacts.',
      openProduced: 'Open established the exact usable review window.',
      currentCondition: 'Ready for human review.',
      actionRequired: false,
      actionSummary: 'No action is required.',
      reusableSummary: 'Exact private build remains reusable.',
      retention: {
        policy: 'Retained until deliberate cleanup',
        cleanup: 'Stop is process-only; cleanup is manual.',
        automatic: false,
        actionRequired: false,
      },
      artifacts: [
        {
          label: 'Private application executable',
          state: 'available',
          summary: 'Private to this instance.',
        },
      ],
      lifecycleHistory: [
        { occurredAtMs: 1_750_000_000_000, kind: 'Opened', summary: 'Window ready.' },
      ],
      operations: [
        {
          operationRef: 'fixture-operation',
          operation: 'build',
          state: 'succeeded',
          stageLabel: 'Finished',
          startedAtMs: 1_750_000_000_000,
          updatedAtMs: 1_750_000_001_000,
          output: Array.from({ length: 24 }, (_, index) => `safe output line ${index + 1}`),
          outputComplete: true,
        },
      ],
      context,
    }),
    comparison: {
      load: async () => ({
        files: [
          {
            fileId: 'file-1',
            displayPath:
              'src/features/worktreeBuild/a-very-long-nested-file-name-that-must-truncate-before-controls.ts',
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
