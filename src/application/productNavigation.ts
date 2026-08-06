import type {
  AgentSessionProductLocation,
  AgentSessionProductOrigin,
} from './agentSessionNavigation';
import type { ProductDecisionEvidenceDestination } from './productDecisions';

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
      /** An exact evidence passage; optional so ordinary Session navigation remains unchanged. */
      readonly focusedEvidence?: ProductDecisionEvidenceDestination;
    }
  | { readonly kind: 'file_review'; readonly target: FileReviewNavigationTarget }
  | { readonly kind: 'harness_inspector' }
  | { readonly kind: 'worktree_review' };

/** A File Review destination names its authoritative input; it never stores a display label or source. */
export type FileReviewNavigationTarget =
  | { readonly kind: 'direct' }
  | { readonly kind: 'contextual_sprint'; readonly sprintId: string }
  | {
      readonly kind: 'file_evidence';
      readonly reviewId: string;
      readonly changedFileId: string;
    };

export type FileReviewProductOrigin =
  | {
      readonly kind: 'file_review';
      readonly launchKind: 'contextual_sprint';
      readonly sprintId: string;
      readonly returnTo: Extract<ProductNavigationDestination, { readonly kind: 'orchestration' }>;
    }
  | {
      readonly kind: 'file_review';
      readonly launchKind: 'file_evidence';
      readonly reviewId: string;
      readonly changedFileId: string;
      readonly returnTo: Extract<ProductNavigationDestination, { readonly kind: 'orchestration' }>;
    };

export type ProductContextualOrigin = AgentSessionProductOrigin | FileReviewProductOrigin;

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
  readonly contextualOrigin: ProductContextualOrigin | null;
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
      readonly focusedEvidence?: ProductDecisionEvidenceDestination;
    }
  | {
      readonly type: 'open_contextual_file_review';
      readonly target: Exclude<FileReviewNavigationTarget, { readonly kind: 'direct' }>;
      readonly origin: FileReviewProductOrigin;
    }
  | { readonly type: 'enter_agent_sessions_directly' }
  | { readonly type: 'back' }
  | { readonly type: 'return_to_contextual_origin'; readonly origin: ProductContextualOrigin }
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
  const destination =
    candidate && isReloadSafeDestination(candidate) && supports(candidate) ? candidate : fallback;
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
        contextualOrigin: keepsContextualOrigin(state.contextualOrigin, action.destination)
          ? state.contextualOrigin
          : null,
      };
    case 'open_contextual_agent_session':
      if (
        !isAgentSessionProductOrigin(action.origin) ||
        (action.focusedEvidence !== undefined &&
          (action.focusedEvidence.sessionId !== action.origin.sessionId ||
            action.focusedEvidence.invocationId !== action.focusedInvocationId)) ||
        !supports(orchestrationDestination(action.origin))
      )
        return clearForeignOrigin(state);
      return {
        current: {
          destination: {
            kind: 'agent_sessions',
            selectedSessionId: action.origin.sessionId,
            focusedInvocationId: action.focusedInvocationId,
            ...(action.focusedEvidence ? { focusedEvidence: action.focusedEvidence } : {}),
          },
          intent: 'push',
        },
        history: [...state.history, state.current],
        contextualOrigin: action.origin,
      };
    case 'open_contextual_file_review':
      if (
        !isFileReviewProductOrigin(action.origin) ||
        !sameFileReviewLaunch(action.target, fileReviewTarget(action.origin)) ||
        !supports({ kind: 'file_review', target: action.target }) ||
        !supports(action.origin.returnTo)
      )
        return clearForeignOrigin(state);
      return {
        current: { destination: { kind: 'file_review', target: action.target }, intent: 'push' },
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
    case 'return_to_contextual_origin': {
      if (state.contextualOrigin !== action.origin) return clearForeignOrigin(state);
      if (isAgentSessionProductOrigin(action.origin)) {
        if (
          state.current.destination.kind !== 'agent_sessions' ||
          !supports(orchestrationDestination(action.origin))
        )
          return clearForeignOrigin(state);
        return {
          current: { destination: orchestrationDestination(action.origin), intent: 'restore' },
          history: state.history,
          contextualOrigin: null,
        };
      }
      if (!isFileReviewProductOrigin(action.origin)) return clearForeignOrigin(state);
      const currentDestination = state.current.destination;
      if (currentDestination.kind !== 'file_review') return clearForeignOrigin(state);
      if (currentDestination.target.kind === 'direct') return clearForeignOrigin(state);
      if (
        !sameFileReviewLaunch(currentDestination.target, fileReviewTarget(action.origin)) ||
        !supports(action.origin.returnTo)
      )
        return clearForeignOrigin(state);
      return {
        current: { destination: action.origin.returnTo, intent: 'restore' },
        history: state.history,
        contextualOrigin: null,
      };
    }
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

/** Maps an already-validated contextual origin to its typed restoration destination. */
export function contextualOriginDestination(
  origin: ProductContextualOrigin,
): Extract<ProductNavigationDestination, { readonly kind: 'orchestration' }> {
  return isAgentSessionProductOrigin(origin)
    ? { kind: 'orchestration', location: origin.location }
    : origin.returnTo;
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
        hasOnlyKeys(value, [
          'kind',
          'selectedSessionId',
          'focusedInvocationId',
          'focusedEvidence',
        ]) &&
        (value.selectedSessionId === null || isIdentifier(value.selectedSessionId)) &&
        (value.focusedInvocationId === null || isIdentifier(value.focusedInvocationId)) &&
        (value.focusedEvidence === undefined ||
          (isProductDecisionEvidenceDestination(value.focusedEvidence) &&
            value.selectedSessionId === value.focusedEvidence.sessionId &&
            value.focusedInvocationId === value.focusedEvidence.invocationId))
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

export function isFileReviewProductOrigin(value: unknown): value is FileReviewProductOrigin {
  if (
    !isRecord(value) ||
    value.kind !== 'file_review' ||
    !isProductNavigationDestination(value.returnTo)
  )
    return false;
  if (value.returnTo.kind !== 'orchestration') return false;
  if (value.launchKind === 'contextual_sprint')
    return (
      hasOnlyKeys(value, ['kind', 'launchKind', 'sprintId', 'returnTo']) &&
      isIdentifier(value.sprintId)
    );
  return (
    value.launchKind === 'file_evidence' &&
    hasOnlyKeys(value, ['kind', 'launchKind', 'reviewId', 'changedFileId', 'returnTo']) &&
    isIdentifier(value.reviewId) &&
    isIdentifier(value.changedFileId)
  );
}

function orchestrationDestination(origin: AgentSessionProductOrigin): ProductNavigationDestination {
  return { kind: 'orchestration', location: origin.location };
}

function keepsContextualOrigin(
  origin: ProductContextualOrigin | null,
  destination: ProductNavigationDestination,
): boolean {
  return (
    origin !== null && isAgentSessionProductOrigin(origin) && destination.kind === 'agent_sessions'
  );
}

export function sameProductNavigationDestination(
  left: ProductNavigationDestination,
  right: ProductNavigationDestination,
): boolean {
  if (left.kind !== right.kind) return false;
  switch (left.kind) {
    case 'orchestration':
      return right.kind === 'orchestration' && sameProductLocation(left.location, right.location);
    case 'plan_builder':
      return (
        right.kind === 'plan_builder' && left.epicPlanningDraftId === right.epicPlanningDraftId
      );
    case 'agent_sessions':
      return (
        right.kind === 'agent_sessions' &&
        left.selectedSessionId === right.selectedSessionId &&
        left.focusedInvocationId === right.focusedInvocationId &&
        sameFocusedEvidence(left.focusedEvidence, right.focusedEvidence)
      );
    case 'file_review':
      return (
        right.kind === 'file_review' && sameFileReviewNavigationTarget(left.target, right.target)
      );
    case 'harness_inspector':
    case 'worktree_review':
      return true;
  }
}

function clearForeignOrigin(state: ProductNavigationState): ProductNavigationState {
  return state.contextualOrigin ? { ...state, contextualOrigin: null } : state;
}

function isFileReviewNavigationTarget(value: unknown): value is FileReviewNavigationTarget {
  if (!isRecord(value) || typeof value.kind !== 'string') return false;
  if (value.kind === 'direct') return hasOnlyKeys(value, ['kind']);
  if (value.kind === 'contextual_sprint')
    return hasOnlyKeys(value, ['kind', 'sprintId']) && isIdentifier(value.sprintId);
  return (
    value.kind === 'file_evidence' &&
    hasOnlyKeys(value, ['kind', 'reviewId', 'changedFileId']) &&
    isIdentifier(value.reviewId) &&
    isIdentifier(value.changedFileId)
  );
}

function sameFileReviewLaunch(
  target: Exclude<FileReviewNavigationTarget, { readonly kind: 'direct' }>,
  expectedTarget: Exclude<FileReviewNavigationTarget, { readonly kind: 'direct' }>,
): boolean {
  return sameFileReviewNavigationTarget(target, expectedTarget);
}

export function sameFileReviewNavigationTarget(
  left: FileReviewNavigationTarget | undefined,
  right: FileReviewNavigationTarget,
): boolean {
  if (!left || left.kind !== right.kind) return false;
  if (left.kind === 'direct') return true;
  if (left.kind === 'contextual_sprint')
    return right.kind === 'contextual_sprint' && left.sprintId === right.sprintId;
  return (
    right.kind === 'file_evidence' &&
    left.reviewId === right.reviewId &&
    left.changedFileId === right.changedFileId
  );
}

function sameProductLocation(
  left: AgentSessionProductLocation | null,
  right: AgentSessionProductLocation | null,
): boolean {
  if (left === null || right === null) return left === right;
  if (left.kind !== right.kind || left.label !== right.label) return false;
  if (left.kind === 'epic' || left.kind === 'epic_product_decisions')
    return right.kind === left.kind && left.epicId === right.epicId;
  if (left.kind === 'sprint')
    return (
      right.kind === 'sprint' && left.epicId === right.epicId && left.sprintId === right.sprintId
    );
  if (left.kind === 'work_slice_planning_point')
    return (
      right.kind === 'work_slice_planning_point' &&
      left.epicId === right.epicId &&
      left.sprintId === right.sprintId &&
      left.revisionId === right.revisionId &&
      left.workSlicePlanningPointId === right.workSlicePlanningPointId
    );
  if (left.kind === 'work_unit')
    return (
      right.kind === 'work_unit' &&
      left.epicId === right.epicId &&
      left.sprintId === right.sprintId &&
      left.revisionId === right.revisionId &&
      left.workSlicePlanningPointId === right.workSlicePlanningPointId &&
      left.workUnitId === right.workUnitId &&
      sameWorkUnitInspectionState(left.inspectionState, right.inspectionState)
    );
  return (
    right.kind === 'epic_planning_draft' && left.epicPlanningDraftId === right.epicPlanningDraftId
  );
}

function sameWorkUnitInspectionState(
  left: Extract<AgentSessionProductLocation, { readonly kind: 'work_unit' }>['inspectionState'],
  right: Extract<AgentSessionProductLocation, { readonly kind: 'work_unit' }>['inspectionState'],
): boolean {
  return (
    left === right ||
    (left !== undefined &&
      right !== undefined &&
      left.tab === right.tab &&
      left.activityId === right.activityId &&
      left.sessionId === right.sessionId &&
      left.invocationId === right.invocationId)
  );
}

function fileReviewTarget(
  origin: FileReviewProductOrigin,
): Exclude<FileReviewNavigationTarget, { readonly kind: 'direct' }> {
  return origin.launchKind === 'contextual_sprint'
    ? { kind: 'contextual_sprint', sprintId: origin.sprintId }
    : {
        kind: 'file_evidence',
        reviewId: origin.reviewId,
        changedFileId: origin.changedFileId,
      };
}

function isAgentSessionProductLocation(value: unknown): value is AgentSessionProductLocation {
  if (!isRecord(value) || typeof value.kind !== 'string' || !isIdentifier(value.label))
    return false;
  if (value.kind === 'epic' || value.kind === 'epic_product_decisions')
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

function isProductDecisionEvidenceDestination(
  value: unknown,
): value is ProductDecisionEvidenceDestination {
  if (!isRecord(value) || !hasOnlyKeys(value, ['kind', 'sessionId', 'invocationId', 'passage']))
    return false;
  if (
    value.kind !== 'agent_session_passage' ||
    !isIdentifier(value.sessionId) ||
    !isIdentifier(value.invocationId) ||
    !isRecord(value.passage) ||
    !isIdentifier(value.passage.kind)
  )
    return false;
  if (value.passage.kind === 'submitted_input' || value.passage.kind === 'outcome')
    return hasOnlyKeys(value.passage, ['kind']);
  return (
    (value.passage.kind === 'activity' || value.passage.kind === 'final_response') &&
    hasOnlyKeys(value.passage, ['kind', 'runtimeEventId']) &&
    isIdentifier(value.passage.runtimeEventId)
  );
}

function sameFocusedEvidence(
  left: ProductDecisionEvidenceDestination | undefined,
  right: ProductDecisionEvidenceDestination | undefined,
) {
  return (
    left === right ||
    (left !== undefined &&
      right !== undefined &&
      left.kind === right.kind &&
      left.sessionId === right.sessionId &&
      left.invocationId === right.invocationId &&
      left.passage.kind === right.passage.kind &&
      ('runtimeEventId' in left.passage
        ? 'runtimeEventId' in right.passage &&
          left.passage.runtimeEventId === right.passage.runtimeEventId
        : !('runtimeEventId' in right.passage)))
  );
}

/** Evidence focus is introduced only by a live, source-resolved contextual origin. */
function isReloadSafeDestination(destination: ProductNavigationDestination): boolean {
  return destination.kind !== 'agent_sessions' || destination.focusedEvidence === undefined;
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
