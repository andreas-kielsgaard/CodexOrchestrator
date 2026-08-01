import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';

/** The optional recorded review composition is development-only and never enters product boot. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps | null>(null);

  useEffect(() => {
    let active = true;
    const developmentRoute = new URLSearchParams(window.location.search);
    const harnessInspectorRequested = developmentRoute.has('harness-inspector');
    if (viteDevelopmentMode() && developmentRoute.has('file-diff-viewer')) {
      void import('../dev/fileReview/recordedFileReviewClient').then(
        ({ createRecordedFileReviewApplicationComposition }) => {
          if (active) setComposition(createRecordedFileReviewApplicationComposition());
        },
      );
    } else if (
      viteDevelopmentMode() &&
      (developmentRoute.has('recorded-plan-builder') || harnessInspectorRequested)
    ) {
      void import('../dev/orchestrationSection/recordedOrchestrationClient').then(
        ({ createRecordedDevelopmentApplicationComposition }) => {
          if (active)
            setComposition(
              createRecordedDevelopmentApplicationComposition({
                initialSurface: harnessInspectorRequested ? 'harness-inspector' : 'epics',
              }),
            );
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
