import { useCallback, useReducer, useRef } from 'react';
import {
  canNavigateBack,
  canNavigateForward,
  createProductNavigation,
  productNavigationReducer,
  type ProductNavigationAction,
  type ProductNavigationDestination,
  type ProductNavigationDestinationSupport,
} from '../application/productNavigation';

export type ApplicationSurface =
  | 'epics'
  | 'workflows'
  | 'capability-profiles'
  | 'agent-sessions'
  | 'harness-inspector'
  | 'file-review'
  | 'worktree-review'
  | 'native-settings'
  | 'product-decision-publish';

export function surfaceForDestination(
  destination: ProductNavigationDestination,
): ApplicationSurface {
  switch (destination.kind) {
    case 'orchestration':
    case 'plan_builder':
      return 'epics';
    case 'workflow':
      return 'workflows';
    case 'capability_profiles':
      return 'capability-profiles';
    case 'agent_sessions':
      return 'agent-sessions';
    case 'harness_inspector':
      return 'harness-inspector';
    case 'file_review':
      return 'file-review';
    case 'worktree_review':
      return 'worktree-review';
    case 'technical_settings':
      return 'native-settings';
    case 'product_decision_publish':
      return 'product-decision-publish';
  }
}

/** One product-wide controller for typed location, reversible history, and stale async work. */
export function useProductNavigation(
  initialDestination: ProductNavigationDestination,
  supports: ProductNavigationDestinationSupport,
) {
  const epoch = useRef(0);
  const [state, reduce] = useReducer(
    (
      current: ReturnType<typeof createProductNavigation>,
      action: ProductNavigationAction,
    ) => productNavigationReducer(current, action, supports),
    createProductNavigation(initialDestination),
  );
  const dispatch = useCallback((action: ProductNavigationAction) => {
    epoch.current += 1;
    reduce(action);
  }, []);
  const back = useCallback(() => dispatch({ type: 'back' }), [dispatch]);
  const forward = useCallback(() => dispatch({ type: 'forward' }), [dispatch]);
  const invalidate = useCallback(() => {
    epoch.current += 1;
  }, []);

  return {
    state,
    dispatch,
    epoch,
    surface: surfaceForDestination(state.current.destination),
    canGoBack: canNavigateBack(state, supports),
    canGoForward: canNavigateForward(state, supports),
    back,
    forward,
    invalidate,
  };
}
