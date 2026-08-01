import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';
import type { WorktreeBuildClient } from '../application/worktreeBuild';

/** The optional recorded review composition is development-only and never enters product boot. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps | null>(null);
  const [worktreeBuild, setWorktreeBuild] = useState<{
    client: WorktreeBuildClient;
    Shell: (typeof import('../features/worktreeBuild'))['WorktreeBuildShell'];
  } | null>(null);

  useEffect(() => {
    let active = true;
    const developmentRoute = new URLSearchParams(window.location.search);
    const harnessInspectorRequested = developmentRoute.has('harness-inspector');
    if (humanReviewInstance()) {
      setComposition(createProductApplicationComposition());
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
          void loadDevelopmentReviewComposition(recorded).then((value) => {
            if (active) setComposition(value);
          });
        },
      );
    } else if (
      viteDevelopmentMode() &&
      (developmentRoute.has('recorded-plan-builder') || harnessInspectorRequested)
    ) {
      void import('../dev/orchestrationSection/recordedOrchestrationClient').then(
        ({ createRecordedDevelopmentApplicationComposition }) => {
          const recorded = createRecordedDevelopmentApplicationComposition({
            initialSurface: harnessInspectorRequested ? 'harness-inspector' : 'epics',
          });
          void loadDevelopmentReviewComposition(recorded).then((value) => {
            if (active) setComposition(value);
          });
        },
      );
    } else if (viteDevelopmentMode()) {
      void loadDevelopmentReviewComposition(createProductApplicationComposition()).then((value) => {
        if (active) setComposition(value);
      });
    } else {
      setComposition(createProductApplicationComposition());
    }
    return () => {
      active = false;
    };
  }, []);

  if (!composition) return null;
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

async function loadDevelopmentReviewComposition(composition: AppProps): Promise<AppProps> {
  const [{ tauriHumanReviewLauncher }, { HumanReviewLauncherView }] = await Promise.all([
    import('../infrastructure/tauriHumanReviewLauncher'),
    import('../features/humanReviewLauncher/HumanReviewLauncherView'),
  ]);
  return {
    ...composition,
    humanReviewLauncherView: <HumanReviewLauncherView client={tauriHumanReviewLauncher} />,
    humanReviewLauncherNavigation: () => tauriHumanReviewLauncher.proofNavigation!(),
  };
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
