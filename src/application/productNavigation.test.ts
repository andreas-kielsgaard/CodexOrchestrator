import type { AgentSessionProductOrigin } from './agentSessionNavigation';
import {
  canNavigateBack,
  createProductNavigation,
  productNavigationReducer,
  restoreProductNavigation,
  sameProductNavigationDestination,
  type FileReviewProductOrigin,
  type ProductNavigationDestination,
} from './productNavigation';

describe('Product navigation history', () => {
  const overview: ProductNavigationDestination = { kind: 'orchestration', location: null };
  const fileReview: ProductNavigationDestination = {
    kind: 'file_review',
    target: { kind: 'file_evidence', reviewId: 'review-1', changedFileId: 'file-1' },
  };
  const workUnitOrigin: AgentSessionProductOrigin = {
    sessionId: 'session-handler',
    invocationId: 'invocation-handler',
    location: {
      kind: 'work_unit',
      epicId: 'epic-1',
      sprintId: 'sprint-1',
      revisionId: 'revision-1',
      workSlicePlanningPointId: 'activity-1',
      workUnitId: 'work-unit-1',
      label: 'Typed Work Unit',
      inspectionState: {
        tab: 'activity',
        activityId: 'activity-1',
        sessionId: 'session-handler',
        invocationId: 'invocation-handler',
      },
    },
  };
  const sprintFileReviewOrigin: FileReviewProductOrigin = {
    kind: 'file_review',
    launchKind: 'contextual_sprint',
    sprintId: 'sprint-1',
    returnTo: { kind: 'orchestration', location: workUnitOrigin.location },
  };
  const evidenceFileReviewOrigin: FileReviewProductOrigin = {
    kind: 'file_review',
    launchKind: 'file_evidence',
    reviewId: 'review-1',
    changedFileId: 'file-1',
    returnTo: { kind: 'orchestration', location: workUnitOrigin.location },
  };

  it('pushes and pops actual typed destinations deterministically', () => {
    let state = createProductNavigation(overview);
    expect(canNavigateBack(state)).toBe(false);

    state = productNavigationReducer(state, {
      type: 'navigate',
      intent: 'push',
      destination: fileReview,
    });
    expect(canNavigateBack(state)).toBe(true);
    expect(state.history).toEqual([{ destination: overview, intent: 'direct' }]);

    state = productNavigationReducer(state, { type: 'back' });
    expect(state.current).toEqual({ destination: overview, intent: 'restore' });
    expect(state.history).toEqual([]);
    expect(canNavigateBack(state)).toBe(false);
  });

  it('does not manufacture a self-history entry for a same-destination push', () => {
    const state = productNavigationReducer(createProductNavigation(overview), {
      type: 'navigate',
      intent: 'push',
      destination: overview,
    });
    expect(state).toEqual(createProductNavigation(overview));
  });

  it('recognizes structurally equivalent destinations and File Review targets', () => {
    const reorderedOverview: ProductNavigationDestination = {
      location: null,
      kind: 'orchestration',
    };
    const sameDestination = productNavigationReducer(createProductNavigation(overview), {
      type: 'navigate',
      intent: 'push',
      destination: reorderedOverview,
    });
    expect(sameDestination).toEqual(createProductNavigation(overview));

    const reorderedTarget = { sprintId: 'sprint-1', kind: 'contextual_sprint' } as const;
    const state = productNavigationReducer(createProductNavigation(overview), {
      type: 'open_contextual_file_review',
      target: reorderedTarget,
      origin: sprintFileReviewOrigin,
    });
    expect(state.current.destination).toEqual({ kind: 'file_review', target: reorderedTarget });
    expect(state.contextualOrigin).toBe(sprintFileReviewOrigin);

    expect(
      sameProductNavigationDestination(
        { kind: 'orchestration', location: workUnitOrigin.location },
        {
          location: {
            inspectionState: {
              invocationId: 'invocation-handler',
              sessionId: 'session-handler',
              activityId: 'activity-1',
              tab: 'activity',
            },
            label: 'Typed Work Unit',
            workUnitId: 'work-unit-1',
            workSlicePlanningPointId: 'activity-1',
            revisionId: 'revision-1',
            sprintId: 'sprint-1',
            epicId: 'epic-1',
            kind: 'work_unit',
          },
          kind: 'orchestration',
        },
      ),
    ).toBe(true);
  });

  it('keeps contextual Return separate from generic Back and preserves exact Work Unit state', () => {
    let state = createProductNavigation(overview);
    state = productNavigationReducer(state, {
      type: 'open_contextual_agent_session',
      origin: workUnitOrigin,
      focusedInvocationId: 'invocation-handler',
    });
    expect(state.current.destination).toEqual({
      kind: 'agent_sessions',
      selectedSessionId: 'session-handler',
      focusedInvocationId: 'invocation-handler',
    });
    expect(state.contextualOrigin).toBe(workUnitOrigin);
    expect(state.history).toEqual([{ destination: overview, intent: 'direct' }]);

    state = productNavigationReducer(state, {
      type: 'return_to_contextual_origin',
      origin: workUnitOrigin,
    });
    expect(state.current).toEqual({
      destination: { kind: 'orchestration', location: workUnitOrigin.location },
      intent: 'restore',
    });
    expect(state.history).toEqual([{ destination: overview, intent: 'direct' }]);
    expect(state.contextualOrigin).toBeNull();
  });

  it('keeps the contextual pointer while changing Session selection, but clears it for direct entry', () => {
    let state = productNavigationReducer(createProductNavigation(overview), {
      type: 'open_contextual_agent_session',
      origin: workUnitOrigin,
      focusedInvocationId: 'invocation-handler',
    });
    state = productNavigationReducer(state, {
      type: 'navigate',
      intent: 'replace',
      destination: {
        kind: 'agent_sessions',
        selectedSessionId: 'session-implementer',
        focusedInvocationId: null,
      },
    });
    expect(state.contextualOrigin).toBe(workUnitOrigin);

    state = productNavigationReducer(state, { type: 'enter_agent_sessions_directly' });
    expect(state.contextualOrigin).toBeNull();
    expect(state.current.destination).toEqual({
      kind: 'agent_sessions',
      selectedSessionId: 'session-implementer',
      focusedInvocationId: null,
    });
  });

  it('clears direct Agent Sessions context without adding a self-history entry', () => {
    let state = productNavigationReducer(createProductNavigation(overview), {
      type: 'open_contextual_agent_session',
      origin: workUnitOrigin,
      focusedInvocationId: 'invocation-handler',
    });
    state = productNavigationReducer(state, { type: 'enter_agent_sessions_directly' });
    expect(state.contextualOrigin).toBeNull();
    expect(state.current.destination).toEqual({
      kind: 'agent_sessions',
      selectedSessionId: 'session-handler',
      focusedInvocationId: null,
    });
    expect(state.history).toEqual([{ destination: overview, intent: 'direct' }]);
    state = productNavigationReducer(state, { type: 'back' });
    expect(state.current.destination).toEqual(overview);
  });

  it('does not restore Back or contextual return state for direct, deep, or reload initialization', () => {
    const restored = restoreProductNavigation(
      {
        current: {
          destination: { kind: 'orchestration', location: workUnitOrigin.location },
          intent: 'push',
        },
        history: [fileReview],
        contextualOrigin: workUnitOrigin,
      },
      overview,
      () => true,
    );
    expect(restored).toEqual({
      current: {
        destination: { kind: 'orchestration', location: workUnitOrigin.location },
        intent: 'restore',
      },
      history: [],
      contextualOrigin: null,
    });
  });

  it('fails closed for unsupported, stale, mismatched, and foreign return state', () => {
    const fallback = { kind: 'orchestration', location: null } as const;
    expect(
      restoreProductNavigation(
        { kind: 'agent_sessions', selectedSessionId: 'session-1', focusedInvocationId: null },
        fallback,
        () => false,
      ),
    ).toEqual(createProductNavigation(fallback, 'restore'));
    expect(
      restoreProductNavigation(
        { kind: 'route-from-transcript', name: 'Return to WU-ECS2E' },
        fallback,
        () => true,
      ),
    ).toEqual(createProductNavigation(fallback, 'restore'));

    let state = productNavigationReducer(createProductNavigation(overview), {
      type: 'open_contextual_agent_session',
      origin: workUnitOrigin,
      focusedInvocationId: 'invocation-handler',
    });
    state = productNavigationReducer(state, {
      type: 'return_to_contextual_origin',
      origin: { ...workUnitOrigin },
    });
    expect(state.contextualOrigin).toBeNull();
    expect(state.current.destination.kind).toBe('agent_sessions');
  });

  it('clears an invalid historical destination instead of inventing a Back target', () => {
    let state = productNavigationReducer(createProductNavigation(overview), {
      type: 'navigate',
      intent: 'push',
      destination: fileReview,
    });
    expect(canNavigateBack(state, () => false)).toBe(false);
    state = productNavigationReducer(state, { type: 'back' }, () => false);
    expect(state.current.destination).toBe(fileReview);
    expect(state.history).toEqual([]);
    expect(canNavigateBack(state)).toBe(false);
  });

  it('keeps direct File Review entry history-free and origin-free', () => {
    const state = createProductNavigation({ kind: 'file_review', target: { kind: 'direct' } });
    expect(state.history).toEqual([]);
    expect(state.contextualOrigin).toBeNull();
    expect(canNavigateBack(state)).toBe(false);
  });

  it('opens contextual Sprint and evidence reviews with typed Return separate from Back', () => {
    let state = createProductNavigation(overview);
    state = productNavigationReducer(
      state,
      {
        type: 'open_contextual_file_review',
        target: { kind: 'contextual_sprint', sprintId: 'sprint-1' },
        origin: sprintFileReviewOrigin,
      },
      (destination) => destination.kind !== 'file_review' || destination.target.kind !== 'direct',
    );
    expect(state.current.destination).toEqual({
      kind: 'file_review',
      target: { kind: 'contextual_sprint', sprintId: 'sprint-1' },
    });
    expect(state.contextualOrigin).toBe(sprintFileReviewOrigin);
    expect(canNavigateBack(state)).toBe(true);

    state = productNavigationReducer(state, {
      type: 'return_to_contextual_origin',
      origin: sprintFileReviewOrigin,
    });
    expect(state.current.destination).toEqual({
      kind: 'orchestration',
      location: workUnitOrigin.location,
    });
    expect(state.contextualOrigin).toBeNull();
    expect(canNavigateBack(state)).toBe(true);

    state = productNavigationReducer(state, {
      type: 'open_contextual_file_review',
      target: { kind: 'file_evidence', reviewId: 'review-1', changedFileId: 'file-1' },
      origin: evidenceFileReviewOrigin,
    });
    state = productNavigationReducer(state, { type: 'back' });
    expect(state.current.destination).toEqual({
      kind: 'orchestration',
      location: workUnitOrigin.location,
    });
    expect(state.contextualOrigin).toBeNull();
  });

  it('rejects a foreign File Review origin instead of reviving another launch', () => {
    let state = createProductNavigation({ kind: 'file_review', target: { kind: 'direct' } });
    state = productNavigationReducer(state, {
      type: 'return_to_contextual_origin',
      origin: evidenceFileReviewOrigin,
    });
    expect(state.current.destination).toEqual({ kind: 'file_review', target: { kind: 'direct' } });
    expect(state.contextualOrigin).toBeNull();
  });
});
