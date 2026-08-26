import { useEffect, useState } from 'react';
import { createProductApplicationComposition } from '../bootstrap/productApplicationComposition';
import { App, type AppProps } from './App';

/** Product composition is immediate; recorded development routes replace it only when requested. */
export function ApplicationRoot() {
  const [composition, setComposition] = useState<AppProps>(() =>
    createProductApplicationComposition(),
  );
  useEffect(() => {
    let active = true;
    const developmentRoute = new URLSearchParams(window.location.search);
    const harnessInspectorRequested = developmentRoute.has('harness-inspector');
    const workUnitReviewRequested = developmentRoute.has('recorded-work-unit-review');
    if (viteDevelopmentMode() && developmentRoute.has('file-diff-viewer')) {
      void import('../dev/fileReview/recordedFileReviewClient').then(
        ({ createRecordedFileReviewApplicationComposition }) => {
          const recorded = createRecordedFileReviewApplicationComposition(
            recordedFileReviewFixture(developmentRoute.get('file-review-fixture')),
          );
          if (active) setComposition(recorded);
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
          if (active) setComposition(recorded);
        },
      );
    }
    return () => {
      active = false;
    };
  }, []);

  return <App {...composition} />;
}

function viteDevelopmentMode(): boolean {
  const env = (import.meta as unknown as { env?: { DEV?: boolean } }).env;
  return env?.DEV === true;
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
