import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';

/** The optional recorded review composition is development-only and never enters product boot. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps | null>(null);

  useEffect(() => {
    let active = true;
    const search = new URLSearchParams(window.location.search);

    if (!viteDevelopmentMode()) {
      setComposition(createProductApplicationComposition());
      return () => {
        active = false;
      };
    }

    void Promise.all([
      import('../dev/agentReview'),
      search.has('recorded-plan-builder') || search.has('agent-review')
        ? import('../dev/orchestrationSection/recordedOrchestrationClient').then(
            ({ createRecordedDevelopmentApplicationComposition }) =>
              createRecordedDevelopmentApplicationComposition(),
          )
        : Promise.resolve(createProductApplicationComposition()),
    ]).then(([{ AgentReviewLab }, baseComposition]) => {
      if (!active) return;

      setComposition({
        ...baseComposition,
        agentReviewSurface: <AgentReviewLab />,
        ...(search.has('agent-review') ? { initialSurface: 'agent-review' as const } : {}),
      });
    });

    return () => {
      active = false;
    };
  }, []);

  return composition ? <App {...composition} /> : null;
}

function viteDevelopmentMode(): boolean {
  const env = (import.meta as unknown as { env?: { DEV?: boolean } }).env;
  return env?.DEV === true;
}
