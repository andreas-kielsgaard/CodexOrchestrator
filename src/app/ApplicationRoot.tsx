import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';

/** Optional recorded review compositions are development-only and never enter product boot. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps | null>(null);

  useEffect(() => {
    let active = true;
    const parameters = new URLSearchParams(window.location.search);
    if (viteDevelopmentMode() && parameters.has('file-diff-viewer')) {
      void import('../dev/fileReview/recordedFileReviewClient').then(
        ({ createRecordedFileReviewApplicationComposition }) => {
          if (active) setComposition(createRecordedFileReviewApplicationComposition());
        },
      );
    } else if (viteDevelopmentMode() && parameters.has('recorded-plan-builder')) {
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
