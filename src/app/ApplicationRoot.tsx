import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';
import type { WorktreeBuildClient } from '../application/worktreeBuild';

/** Development tools are composed only in the launcher build, never in isolated review windows. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps | null>(null);
  const [worktreeBuild, setWorktreeBuild] = useState<{
    client: WorktreeBuildClient;
    Shell: (typeof import('../features/worktreeBuild'))['WorktreeBuildShell'];
  } | null>(null);

  useEffect(() => {
    let active = true;
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
    if (!developmentToolsEnabled()) {
      setComposition(createProductApplicationComposition());
      return () => {
        active = false;
      };
    }

    void loadDevelopmentComposition().then((developmentComposition) => {
      if (active) setComposition(developmentComposition);
    });
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

async function loadDevelopmentComposition(): Promise<AppProps> {
  const productComposition = new URLSearchParams(window.location.search).has(
    'recorded-plan-builder',
  )
    ? await import('../dev/orchestrationSection/recordedOrchestrationClient').then(
        ({ createRecordedDevelopmentApplicationComposition }) =>
          createRecordedDevelopmentApplicationComposition(),
      )
    : createProductApplicationComposition();
  const [{ tauriHumanReviewLauncher }, { HumanReviewLauncherView }] = await Promise.all([
    import('../infrastructure/tauriHumanReviewLauncher'),
    import('../features/humanReviewLauncher/HumanReviewLauncherView'),
  ]);
  return {
    ...productComposition,
    humanReviewLauncherView: <HumanReviewLauncherView client={tauriHumanReviewLauncher} />,
    humanReviewLauncherNavigation: () => tauriHumanReviewLauncher.proofNavigation!(),
  };
}

function developmentToolsEnabled(): boolean {
  const env = (
    import.meta as unknown as {
      env?: { DEV?: boolean; VITE_WORKTREE_REVIEW_LAUNCHER?: string };
    }
  ).env;
  return env?.DEV === true || env?.VITE_WORKTREE_REVIEW_LAUNCHER === 'true';
}

function humanReviewInstance(): boolean {
  const env = (import.meta as unknown as { env?: { VITE_HUMAN_REVIEW_INSTANCE?: string } }).env;
  return env?.VITE_HUMAN_REVIEW_INSTANCE === 'true';
}
