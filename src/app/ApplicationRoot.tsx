import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';

/** Development tools are composed only in the launcher build, never in isolated review windows. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps | null>(null);

  useEffect(() => {
    let active = true;
    if (!viteDevelopmentMode() || humanReviewInstance()) {
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

  return composition ? <App {...composition} /> : null;
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
