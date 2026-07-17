import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';

/** The optional recorded review composition is development-only and never enters product boot. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps | null>(null);

  useEffect(() => {
    let active = true;
    if (!viteDevelopmentMode()) {
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
  const [{ createDevelopmentWorktreeRuntimeSource }, { WorktreeRuntimeExplorationView }] =
    await Promise.all([
      import('../dev/worktreeRuntime/createDevelopmentWorktreeRuntimeSource'),
      import('../features/worktreeRuntime/WorktreeRuntimeExplorationView'),
    ]);
  return {
    ...productComposition,
    worktreeRuntimeExplorationView: (
      <WorktreeRuntimeExplorationView source={createDevelopmentWorktreeRuntimeSource()} />
    ),
  };
}

function viteDevelopmentMode(): boolean {
  const env = (import.meta as unknown as { env?: { DEV?: boolean } }).env;
  return env?.DEV === true;
}
