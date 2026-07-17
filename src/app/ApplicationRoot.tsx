import { useEffect, useState, type ComponentType } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';

type ApplicationRootState =
  | { readonly kind: 'app'; readonly composition: AppProps }
  | { readonly kind: 'test_mode'; readonly Component: ComponentType };

/** Optional development compositions never enter product boot. */
export function ApplicationRoot() {
  const [state, setState] = useState<ApplicationRootState | null>(null);

  useEffect(() => {
    let active = true;
    const query = new URLSearchParams(window.location.search);
    if (viteDevelopmentMode() && query.has('agent-test-mode')) {
      void import('../dev/applicationTesting/AgentTestModeRoot').then(({ AgentTestModeRoot }) => {
        if (active) setState({ kind: 'test_mode', Component: AgentTestModeRoot });
      });
    } else if (viteDevelopmentMode() && query.has('recorded-plan-builder')) {
      void import('../dev/orchestrationSection/recordedOrchestrationClient').then(
        ({ createRecordedDevelopmentApplicationComposition }) => {
          if (active)
            setState({
              kind: 'app',
              composition: createRecordedDevelopmentApplicationComposition(),
            });
        },
      );
    } else {
      setState({ kind: 'app', composition: createProductApplicationComposition() });
    }
    return () => {
      active = false;
    };
  }, []);

  if (!state) return null;
  if (state.kind === 'test_mode') return <state.Component />;
  return <App {...state.composition} />;
}

function viteDevelopmentMode(): boolean {
  const env = (import.meta as unknown as { env?: { DEV?: boolean } }).env;
  return env?.DEV === true;
}
