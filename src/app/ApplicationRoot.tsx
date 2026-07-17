import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';

/** Optional development compositions never enter product boot. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps | null>(null);

  useEffect(() => {
    let active = true;
    const query = new URLSearchParams(window.location.search);
    if (viteDevelopmentMode() && query.has('agent-test-mode')) {
      void import('../dev/applicationTesting/agentTestModeDevelopmentComposition').then(
        ({ createAgentTestModeDevelopmentApplicationComposition }) => {
          if (active) setComposition(createAgentTestModeDevelopmentApplicationComposition());
        },
      );
    } else if (viteDevelopmentMode() && query.has('recorded-plan-builder')) {
      void import('../dev/orchestrationSection/recordedOrchestrationClient').then(
        ({ createRecordedDevelopmentApplicationComposition }) => {
          if (active) setComposition(createRecordedDevelopmentApplicationComposition());
        },
      );
    } else {
      setComposition(createProductApplicationComposition());
    }
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
