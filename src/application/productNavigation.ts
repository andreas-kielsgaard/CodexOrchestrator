import type {
  AgentSessionProductLocation,
  AgentSessionProductOrigin,
} from './agentSessionNavigation';

/** Product-owned destinations only; this is deliberately not a command vocabulary. */
export type ProductNavigationDestination =
  | {
      readonly kind: 'orchestration';
      readonly location: AgentSessionProductLocation | null;
    }
  | { readonly kind: 'plan_builder'; readonly epicPlanningDraftId: string | null }
  | {
      readonly kind: 'agent_sessions';
      readonly selectedSessionId: string | null;
      readonly focusedInvocationId: string | null;
    }
  | { readonly kind: 'file_review'; readonly target: FileReviewNavigationTarget }
  | { readonly kind: 'harness_inspector' }
  | { readonly kind: 'worktree_review' };

/** A File Review destination names its authoritative input; it never stores a display label or source. */
export type FileReviewNavigationTarget =
  | { readonly kind: 'contextual_sprint'; readonly sprintId: string }
  | {
      readonly kind: 'file_evidence';
      readonly reviewId: string;
      readonly changedFileId: string;
    };

export type ProductNavigationIntent = 'direct' | 'push' | 'replace' | 'restore';

export interface ProductNavigationEntry {
  readonly destination: ProductNavigationDestination;
  readonly intent: ProductNavigationIntent;
}

export interface ProductNavigationState {
  readonly current: ProductNavigationEntry;
  /** Only destinations entered through a product push are candidates for generic Back. */
  readonly history: readonly ProductNavigationEntry[];
  /** This single contextual pointer is independent of generic history. */
  readonly contextualOrigin: AgentSessionProductOrigin | null;
}

export type ProductNavigationAction =
  | {
      readonly type: 'navigate';
      readonly intent: Extract<ProductNavigationIntent, 'push' | 'replace'>;
      readonly destination: ProductNavigationDestination;
    }
  | {
      readonly type: 'open_contextual_agent_session';
      readonly origin: AgentSessionProductOrigin;
      readonly focusedInvocationId: string | null;
    }
  | { readonly type: 'enter_agent_sessions_directly' }
  | { readonly type: 'back' }
  | { readonly type: 'return_to_contextual_origin'; readonly origin: AgentSessionProductOrigin }
  | { readonly type: 'clear_contextual_origin' };

export type ProductNavigationDestinationSupport = (
  destination: ProductNavigationDestination,
) => boolean;

/** Initializes direct or deep entry. Neither contextual return nor Back state is rehydrated. */
export function createProductNavigation(
  destination: ProductNavigationDestination,
  intent: Extract<ProductNavigationIntent, 'direct' | 'restore'> = 'direct',
): ProductNavigationState {
  return {
    current: { destination, intent },
    history: [],
    contextualOrigin: null,
  };
}

/**
 * Reload input is untrusted. Only one supported current destination survives; prior entries and
 * contextual origin are intentionally never restored.
 */
export function restoreProductNavigation(
  value: unknown,
  fallback: ProductNavigationDestination,
  supports: ProductNavigationDestinationSupport,
): ProductNavigationState {
  const candidate = restoredDestination(value);
  const destination = candidate && supports(candidate) ? candidate : fallback;
  return createProductNavigation(destination, 'restore');
}

export function productNavigationReducer(
  state: ProductNavigationState,
  action: ProductNavigationAction,
  supports: ProductNavigationDestinationSupport = () => true,
): ProductNavigationState {
  switch (action.type) {
    case 'navigate':
      if (!supports(action.destination)) return clearForeignOrigin(state);
      if (
        action.intent === 'push' &&
        sameProductNavigationDestination(state.current.destination, action.destination)
      )
        return state;
      return {
        current: { destination: action.destination, intent: action.intent },
        history: action.intent === 'push' ? [...state.history, state.current] : state.history,
        contextualOrigin: keepsContextualOrigin(action.destination) ? state.contextualOrigin : null,
      };
    case 'open_contextual_agent_session':
      if (
        !isAgentSessionProductOrigin(action.origin) ||
        !supports(orchestrationDestination(action.origin))
      )
        return clearForeignOrigin(state);
      return {
        current: {
          destination: {
            kind: 'agent_sessions',
            selectedSessionId: action.origin.sessionId,
            focusedInvocationId: action.focusedInvocationId,
          },
          intent: 'push',
        },
        history: [...state.history, state.current],
        contextualOrigin: action.origin,
      };
    case 'enter_agent_sessions_directly': {
      const destination: ProductNavigationDestination = {
        kind: 'agent_sessions',
        selectedSessionId:
          state.current.destination.kind === 'agent_sessions'
            ? state.current.destination.selectedSessionId
          : null,
        focusedInvocationId: null,
      };
      if (state.current.destination.kind === 'agent_sessions')
        return {
          current: { destination, intent: 'replace' },
          history: state.history,
          contextualOrigin: null,
        };
      return {
        current: { destination, intent: 'push' },
        history: [...state.history, state.current],
        contextualOrigin: null,
      };
    }
    case 'back': {
      const previous = state.history.at(-1);
      if (!previous) return clearForeignOrigin(state);
      if (!supports(previous.destination))
        return {
          ...state,
          history: state.history.slice(0, -1),
          contextualOrigin: null,
        };
      return {
        current: { destination: previous.destination, intent: 'restore' },
        history: state.history.slice(0, -1),
        contextualOrigin: null,
      };
    }
    case 'return_to_contextual_origin':
      if (
        state.contextualOrigin !== action.origin ||
        !isAgentSessionProductOrigin(action.origin) ||
        !supports(orchestrationDestination(action.origin))
      )
        return clearForeignOrigin(state);
      return {
        current: { destination: orchestrationDestination(action.origin), intent: 'restore' },
        history: state.history,
        contextualOrigin: null,
      };
    case 'clear_contextual_origin':
      return { ...state, contextualOrigin: null };
  }
}

export function canNavigateBack(
  state: ProductNavigationState,
  supports: ProductNavigationDestinationSupport = () => true,
): boolean {
  const previous = state.history.at(-1);
  return previous !== undefined && supports(previous.destination);
}

export function isProductNavigationDestination(
  value: unknown,
): value is ProductNavigationDestination {
  if (!isRecord(value) || typeof value.kind !== 'string') return false;
  switch (value.kind) {
    case 'orchestration':
      return (
        hasOnlyKeys(value, ['kind', 'location']) &&
        (value.location === null || isAgentSessionProductLocation(value.location))
      );
    case 'plan_builder':
      return (
        hasOnlyKeys(value, ['kind', 'epicPlanningDraftId']) &&
        (value.epicPlanningDraftId === null || isIdentifier(value.epicPlanningDraftId))
      );
    case 'agent_sessions':
      return (
        hasOnlyKeys(value, ['kind', 'selectedSessionId', 'focusedInvocationId']) &&
        (value.selectedSessionId === null || isIdentifier(value.selectedSessionId)) &&
        (value.focusedInvocationId === null || isIdentifier(value.focusedInvocationId))
      );
    case 'file_review':
      return hasOnlyKeys(value, ['kind', 'target']) && isFileReviewNavigationTarget(value.target);
    case 'harness_inspector':
    case 'worktree_review':
      return hasOnlyKeys(value, ['kind']);
    default:
      return false;
  }
}

export function isAgentSessionProductOrigin(value: unknown): value is AgentSessionProductOrigin {
  return (
    isRecord(value) &&
    hasOnlyKeys(value, ['sessionId', 'location', 'invocationId']) &&
    isIdentifier(value.sessionId) &&
    isAgentSessionProductLocation(value.location) &&
    (value.invocationId === undefined || isIdentifier(value.invocationId))
  );
}

function orchestrationDestination(origin: AgentSessionProductOrigin): ProductNavigationDestination {
  return { kind: 'orchestration', location: origin.location };
}

function keepsContextualOrigin(destination: ProductNavigationDestination): boolean {
  return destination.kind === 'agent_sessions';
}

function sameProductNavigationDestination(
  left: ProductNavigationDestination,
  right: ProductNavigationDestination,
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function clearForeignOrigin(state: ProductNavigationState): ProductNavigationState {
  return state.contextualOrigin ? { ...state, contextualOrigin: null } : state;
}

function isFileReviewNavigationTarget(value: unknown): value is FileReviewNavigationTarget {
  if (!isRecord(value) || typeof value.kind !== 'string') return false;
  if (value.kind === 'contextual_sprint')
    return hasOnlyKeys(value, ['kind', 'sprintId']) && isIdentifier(value.sprintId);
  return (
    value.kind === 'file_evidence' &&
    hasOnlyKeys(value, ['kind', 'reviewId', 'changedFileId']) &&
    isIdentifier(value.reviewId) &&
    isIdentifier(value.changedFileId)
  );
}

function isAgentSessionProductLocation(value: unknown): value is AgentSessionProductLocation {
  if (!isRecord(value) || typeof value.kind !== 'string' || !isIdentifier(value.label))
    return false;
  if (value.kind === 'epic')
    return hasOnlyKeys(value, ['kind', 'epicId', 'label']) && isIdentifier(value.epicId);
  if (value.kind === 'sprint')
    return (
      hasOnlyKeys(value, ['kind', 'epicId', 'sprintId', 'label']) &&
      isIdentifier(value.epicId) &&
      isIdentifier(value.sprintId)
    );
  if (value.kind === 'work_slice_planning_point')
    return (
      hasOnlyKeys(value, [
        'kind',
        'epicId',
        'sprintId',
        'revisionId',
        'workSlicePlanningPointId',
        'label',
      ]) &&
      isIdentifier(value.epicId) &&
      isIdentifier(value.sprintId) &&
      isIdentifier(value.revisionId) &&
      isIdentifier(value.workSlicePlanningPointId)
    );
  if (value.kind === 'work_unit')
    return (
      hasOnlyKeys(value, [
        'kind',
        'epicId',
        'sprintId',
        'revisionId',
        'workSlicePlanningPointId',
        'workUnitId',
        'label',
        'inspectionState',
      ]) &&
      isIdentifier(value.epicId) &&
      isIdentifier(value.sprintId) &&
      isIdentifier(value.revisionId) &&
      isIdentifier(value.workSlicePlanningPointId) &&
      isIdentifier(value.workUnitId) &&
      (value.inspectionState === undefined || isWorkUnitInspectionState(value.inspectionState))
    );
  return (
    value.kind === 'epic_planning_draft' &&
    hasOnlyKeys(value, ['kind', 'epicPlanningDraftId', 'label']) &&
    isIdentifier(value.epicPlanningDraftId)
  );
}

function isWorkUnitInspectionState(value: unknown): boolean {
  return (
    isRecord(value) &&
    hasOnlyKeys(value, ['tab', 'activityId', 'sessionId', 'invocationId']) &&
    (value.tab === 'activity' || value.tab === 'evidence') &&
    isIdentifier(value.activityId) &&
    isIdentifier(value.sessionId) &&
    isIdentifier(value.invocationId)
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function restoredDestination(value: unknown): ProductNavigationDestination | null {
  if (isProductNavigationDestination(value)) return value;
  if (!isRecord(value) || !isRecord(value.current)) return null;
  return isProductNavigationDestination(value.current.destination)
    ? value.current.destination
    : null;
}

function hasOnlyKeys(value: Record<string, unknown>, allowed: readonly string[]): boolean {
  return Object.keys(value).every((key) => allowed.includes(key));
}

function isIdentifier(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}
