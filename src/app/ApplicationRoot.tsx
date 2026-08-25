import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';
import type { WorktreeBuildClient } from '../application/worktreeBuild';

/** Product boot. Recorded fixtures may replace product data only in development routes. */
export function ApplicationRoot() {
  const [worktreeReviewInstance] = useState(humanReviewInstance);
  const [composition, setComposition] = useState<AppProps>(() =>
    createProductApplicationComposition({ includeWorktreeReview: !worktreeReviewInstance }),
  );
  const [worktreeBuild, setWorktreeBuild] = useState<{
    client: WorktreeBuildClient;
    Shell: (typeof import('../features/worktreeBuild'))['WorktreeBuildShell'];
  } | null>(null);

  useEffect(() => {
    let active = true;
    const developmentRoute = new URLSearchParams(window.location.search);
    const harnessInspectorRequested = developmentRoute.has('harness-inspector');
    const workUnitReviewRequested = developmentRoute.has('recorded-work-unit-review');
    if (worktreeReviewInstance) {
      void Promise.all([
        import('../infrastructure/tauriWorktreeBuild'),
        import('../features/worktreeBuild'),
      ]).then(([{ tauriWorktreeBuild }, { WorktreeBuildShell }]) => {
        if (active) setWorktreeBuild({ client: tauriWorktreeBuild, Shell: WorktreeBuildShell });
      });
      return () => {
        active = false;
      };
    }
    if (viteDevelopmentMode() && developmentRoute.has('file-diff-viewer')) {
      void import('../dev/fileReview/recordedFileReviewClient').then(
        ({ createRecordedFileReviewApplicationComposition }) => {
          const recorded = createRecordedFileReviewApplicationComposition(
            recordedFileReviewFixture(developmentRoute.get('file-review-fixture')),
          );
          if (active) {
            setComposition({
              ...recorded,
              worktreeReviewClient: composition.worktreeReviewClient,
            });
          }
        },
      );
    } else if (
      viteDevelopmentMode() &&
      (developmentRoute.has('recorded-plan-builder') ||
        harnessInspectorRequested ||
        workUnitReviewRequested)
    ) {
      void import('../dev/orchestrationSection/recordedOrchestrationClient').then(
        ({ createRecordedDevelopmentApplicationComposition }) => {
          const recorded = createRecordedDevelopmentApplicationComposition({
            initialSurface: harnessInspectorRequested ? 'harness-inspector' : 'epics',
            includeWorkUnitReview: workUnitReviewRequested,
          });
          if (active) {
            setComposition({
              ...recorded,
              worktreeReviewClient: composition.worktreeReviewClient,
            });
          }
        },
      );
    }
    return () => {
      active = false;
    };
  }, [composition.worktreeReviewClient, worktreeReviewInstance]);

  if (worktreeBuild) {
    const { client, Shell } = worktreeBuild;
    return (
      <Shell client={client}>
        <App {...composition} />
      </Shell>
    );
  }
  return <App {...composition} />;
}

function viteDevelopmentMode(): boolean {
  const env = (import.meta as unknown as { env?: { DEV?: boolean } }).env;
  return env?.DEV === true;
}

function humanReviewInstance(): boolean {
  const env = (import.meta as unknown as { env?: { VITE_HUMAN_REVIEW_INSTANCE?: string } }).env;
  return env?.VITE_HUMAN_REVIEW_INSTANCE === 'true';
}

function recordedFileReviewFixture(value: string | null) {
  if (
    value === 'staged' ||
    value === 'commit-range' ||
    value === 'generated' ||
    value === 'application-owned'
  )
    return value;
  return 'working-tree' as const;
}
