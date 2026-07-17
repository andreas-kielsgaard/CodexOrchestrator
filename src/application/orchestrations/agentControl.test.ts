import {
  decodeAgentControlContractsV1,
  projectContinuationEligibility,
  projectIdempotency,
  projectAgentControlOutcome,
  AGENT_CONTROL_CONTRACTS_V1,
  type AgentControlContractsV1,
} from './index';

describe('Agent Control contracts', () => {
  it('keeps a requested command distinct from acknowledgement and a resulting Orchestration Event', () => {
    const decoded = decodeAgentControlContractsV1(contracts());

    expect(projectAgentControlOutcome(decoded, 'request-sprint')).toEqual({
      requested: true,
      acknowledged: false,
      orchestrationEventRecorded: false,
    });
  });

  it('recognizes command duplicates deterministically but rejects idempotency collisions', () => {
    const duplicate = contracts();
    duplicate.commands.push({
      ...duplicate.commands[0],
      agentControlCommandId: 'request-sprint-2',
    });
    const decoded = decodeAgentControlContractsV1(duplicate);
    expect(projectIdempotency(decoded, 'request-sprint')).toEqual({
      recognized: true,
      duplicateCommandIds: ['request-sprint', 'request-sprint-2'],
    });

    const collision = contracts();
    collision.promptProvenance.push({
      promptProvenanceId: 'prompt-application',
      sourceKind: 'application_produced',
      sourceReference: 'planner-output-1',
      causalInputReferences: [],
    });
    collision.commands.push({
      ...collision.commands[0],
      agentControlCommandId: 'request-collision',
      promptProvenanceId: 'prompt-application',
    });
    expect(() => decodeAgentControlContractsV1(collision)).toThrow(
      'idempotency key cannot represent different command, target, or prompt semantics',
    );
  });

  it.each([
    ['request_next_ready_work_unit_planner', 0, 'epic'],
    ['request_next_sprint_planner', 1, 'agent_session'],
    ['request_agent_session_prompt', 2, 'sprint'],
  ] as const)(
    'rejects a wrong idempotency scope kind for %s',
    (_commandKind, index, wrongScopeKind) => {
      const invalid = contracts();
      invalid.commands[index].idempotency.scopeKind = wrongScopeKind;

      expect(() => decodeAgentControlContractsV1(invalid)).toThrow(
        'idempotency scope kind must match semantic target',
      );
    },
  );

  it.each([
    ['request_next_ready_work_unit_planner', 0, 'sprint-other'],
    ['request_next_sprint_planner', 1, 'epic-other'],
    ['request_agent_session_prompt', 2, 'agent-session-reference-other'],
  ] as const)(
    'rejects a wrong idempotency scope id for %s',
    (_commandKind, index, wrongScopeId) => {
      const invalid = contracts();
      invalid.commands[index].idempotency.scopeId = wrongScopeId;

      expect(() => decodeAgentControlContractsV1(invalid)).toThrow(
        'idempotency scope id must equal semantic target',
      );
    },
  );

  it('retains prompt source distinctions and rejects adapter leakage', () => {
    const sourceKinds = [
      'user_authored',
      'agent_session_derived',
      'application_produced',
      'repository_or_system_derived',
      'other',
    ] as const;
    const varied = contracts();
    varied.promptProvenance = sourceKinds.map((sourceKind, index) => ({
      promptProvenanceId: `prompt-${index}`,
      sourceKind,
      sourceReference: `source-${index}`,
      causalInputReferences: ['input-a'],
      ...(sourceKind === 'other' ? { otherSourceType: 'migration_note' } : {}),
    }));
    varied.commands.forEach((command, index) => {
      command.promptProvenanceId = `prompt-${index}`;
    });
    expect(
      decodeAgentControlContractsV1(varied).promptProvenance.map(({ sourceKind }) => sourceKind),
    ).toEqual(sourceKinds);

    const leaking = contracts() as Record<string, unknown>;
    leaking.providerThreadId = 'adapter-only';
    expect(() => decodeAgentControlContractsV1(leaking)).toThrow('providerThreadId is provider');
  });

  it.each([
    [true, false, false, 'eligible', undefined],
    [false, false, false, 'ineligible', undefined],
    [true, true, false, 'feedback_required', 'designed_feedback_flow'],
    [true, false, true, 'feedback_required', 'all_pending_work_blocked'],
  ] as const)(
    'projects eligible, ineligible, designed feedback, and blocked paths for both levels',
    (conditionsSatisfied, designedForFeedback, allBlocked, status, feedbackBoundary) => {
      for (const policy of [
        {
          continuationPolicyId: 'sprint',
          level: 'sprint' as const,
          sprintId: 'sprint-1',
          autoFlowEnabled: true,
        },
        {
          continuationPolicyId: 'epic',
          level: 'epic' as const,
          epicId: 'epic-1',
          autoFlowEnabled: true,
        },
      ]) {
        expect(
          projectContinuationEligibility(policy, {
            requiredConditionsSatisfied: conditionsSatisfied,
            designedForFeedback,
            allPendingDevelopmentTechnicallyBlocked: allBlocked,
          }),
        ).toEqual({ status, ...(feedbackBoundary ? { feedbackBoundary } : {}) });
      }
    },
  );

  it('requires feedback when auto-flow is off and permits no unlisted feedback boundary', () => {
    expect(
      projectContinuationEligibility(
        {
          continuationPolicyId: 'sprint',
          level: 'sprint',
          sprintId: 'sprint-1',
          autoFlowEnabled: false,
        },
        {
          requiredConditionsSatisfied: true,
          designedForFeedback: false,
          allPendingDevelopmentTechnicallyBlocked: false,
        },
      ),
    ).toEqual({ status: 'feedback_required', feedbackBoundary: 'auto_flow_off' });
  });

  it('rejects cross-level policy use and prevents either level from targeting the other continuation', () => {
    const crossLevel = contracts();
    crossLevel.commands[0].continuation!.policyId = 'policy-epic';
    crossLevel.commands[0].continuation!.eligibilityEvaluationId = 'evaluation-epic';
    crossLevel.commands[0].preconditionEvidenceReference = 'evaluation-epic';
    expect(() => decodeAgentControlContractsV1(crossLevel)).toThrow(
      'continuation request cannot use policy or eligibility from another level',
    );

    const wrongTarget = contracts();
    wrongTarget.continuationEligibilityEvaluations[0].target = {
      kind: 'next_sprint_planner',
      epicId: 'epic-1',
    } as never;
    expect(() => decodeAgentControlContractsV1(wrongTarget)).toThrow(
      'Sprint continuation target is invalid',
    );
  });

  it('binds continuation precondition evidence to the eligibility evaluation', () => {
    const invalid = contracts();
    invalid.commands[0].preconditionEvidenceReference = 'other-evidence';

    expect(() => decodeAgentControlContractsV1(invalid)).toThrow(
      'continuation precondition evidence must equal its eligibility evaluation',
    );
  });

  it('requires prompt provenance for every Agent Control command', () => {
    const invalid = contracts();
    delete (invalid.commands[2] as { promptProvenanceId?: string }).promptProvenanceId;
    expect(() => decodeAgentControlContractsV1(invalid)).toThrow('promptProvenanceId is required');

    const continuationWithoutPrompt = contracts();
    delete (continuationWithoutPrompt.commands[0] as { promptProvenanceId?: string })
      .promptProvenanceId;
    expect(() => decodeAgentControlContractsV1(continuationWithoutPrompt)).toThrow(
      'promptProvenanceId is required',
    );
  });

  it('binds a direct Agent Session prompt to its recipient session', () => {
    const invalid = contracts();
    invalid.commands[2].recipientAgentSessionRefId = 'agent-session-reference-other';

    expect(() => decodeAgentControlContractsV1(invalid)).toThrow(
      'Agent Session prompt target must equal its command recipient',
    );
  });

  it('requires resulting Orchestration Events to reference an existing command without promoting acknowledgements', () => {
    const acknowledged = contracts();
    acknowledged.results.push({
      agentControlResultId: 'result-acknowledged',
      agentControlCommandId: 'request-sprint',
      state: 'acknowledged',
      recordedAt: TIME,
    });
    expect(
      projectAgentControlOutcome(decodeAgentControlContractsV1(acknowledged), 'request-sprint'),
    ).toMatchObject({ acknowledged: true, orchestrationEventRecorded: false });

    const danglingObserved = contracts();
    danglingObserved.results.push({
      agentControlResultId: 'result-observed',
      agentControlCommandId: 'not-a-command',
      state: 'orchestration_event_recorded',
      orchestrationEventReference: 'observed-continuation-1',
      recordedAt: TIME,
    });
    expect(() => decodeAgentControlContractsV1(danglingObserved)).toThrow(
      'dangling result command',
    );
  });
});

const TIME = '2026-07-14T12:00:00.000Z';

function contracts(): Mutable<AgentControlContractsV1> {
  return {
    version: AGENT_CONTROL_CONTRACTS_V1,
    promptProvenance: [
      {
        promptProvenanceId: 'prompt-user',
        sourceKind: 'user_authored',
        sourceReference: 'feedback-1',
        causalInputReferences: [],
      },
    ],
    continuationPolicies: [
      {
        continuationPolicyId: 'policy-sprint',
        level: 'sprint',
        sprintId: 'sprint-1',
        autoFlowEnabled: true,
      },
      {
        continuationPolicyId: 'policy-epic',
        level: 'epic',
        epicId: 'epic-1',
        autoFlowEnabled: true,
      },
    ],
    continuationEligibilityEvaluations: [
      {
        continuationEligibilityEvaluationId: 'evaluation-sprint',
        continuationPolicyId: 'policy-sprint',
        level: 'sprint',
        target: { kind: 'next_ready_work_unit_planner', sprintId: 'sprint-1' },
        requiredConditionsSatisfied: true,
        designedForFeedback: false,
        allPendingDevelopmentTechnicallyBlocked: false,
        recordedAt: TIME,
        result: { status: 'eligible' },
      },
      {
        continuationEligibilityEvaluationId: 'evaluation-epic',
        continuationPolicyId: 'policy-epic',
        level: 'epic',
        target: { kind: 'next_sprint_planner', epicId: 'epic-1' },
        requiredConditionsSatisfied: true,
        designedForFeedback: false,
        allPendingDevelopmentTechnicallyBlocked: false,
        recordedAt: TIME,
        result: { status: 'eligible' },
      },
    ],
    commands: [
      {
        agentControlCommandId: 'request-sprint',
        commandKind: 'request_next_ready_work_unit_planner',
        recipientAgentSessionRefId: 'agent-session-reference-sprint-runner',
        target: { kind: 'next_ready_work_unit_planner', sprintId: 'sprint-1' },
        idempotency: { key: 'continue', scopeKind: 'sprint', scopeId: 'sprint-1' },
        initiatedBy: { sourceKind: 'application_produced', sourceReference: 'sprint-policy-loop' },
        promptProvenanceId: 'prompt-user',
        recordedAt: TIME,
        preconditionEvidenceReference: 'evaluation-sprint',
        continuation: { policyId: 'policy-sprint', eligibilityEvaluationId: 'evaluation-sprint' },
      },
      {
        agentControlCommandId: 'request-epic',
        commandKind: 'request_next_sprint_planner',
        recipientAgentSessionRefId: 'agent-session-reference-epic-runner',
        target: { kind: 'next_sprint_planner', epicId: 'epic-1' },
        idempotency: {
          key: 'continue',
          scopeKind: 'epic',
          scopeId: 'epic-1',
        },
        initiatedBy: {
          sourceKind: 'application_produced',
          sourceReference: 'epic-policy-loop',
        },
        promptProvenanceId: 'prompt-user',
        recordedAt: TIME,
        preconditionEvidenceReference: 'evaluation-epic',
        continuation: {
          policyId: 'policy-epic',
          eligibilityEvaluationId: 'evaluation-epic',
        },
      },
      {
        agentControlCommandId: 'request-prompt',
        commandKind: 'request_agent_session_prompt',
        recipientAgentSessionRefId: 'agent-session-reference-1',
        target: { kind: 'agent_session', agentSessionRefId: 'agent-session-reference-1' },
        idempotency: {
          key: 'prompt',
          scopeKind: 'agent_session',
          scopeId: 'agent-session-reference-1',
        },
        initiatedBy: { sourceKind: 'user_authored', sourceReference: 'feedback-1' },
        promptProvenanceId: 'prompt-user',
        recordedAt: TIME,
        preconditionEvidenceReference: 'session-availability-evidence-1',
      },
    ],
    results: [],
  };
}

type Mutable<T> = T extends readonly (infer Item)[]
  ? Mutable<Item>[]
  : T extends object
    ? { -readonly [Key in keyof T]: Mutable<T[Key]> }
    : T;
