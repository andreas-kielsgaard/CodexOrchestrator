export type AgentSessionProductLocation =
  | {
      readonly kind: 'epic';
      readonly epicId: string;
      readonly label: string;
    }
  | {
      /** Epic-local Product Decisions view; recorded and productive data remain separate. */
      readonly kind: 'epic_product_decisions';
      readonly epicId: string;
      readonly label: string;
    }
  | {
      readonly kind: 'sprint';
      readonly epicId: string;
      readonly sprintId: string;
      readonly label: string;
    }
  | {
      readonly kind: 'work_slice_planning_point';
      readonly epicId: string;
      readonly sprintId: string;
      readonly revisionId: string;
      readonly workSlicePlanningPointId: string;
      readonly label: string;
    }
  | {
      readonly kind: 'work_unit';
      readonly epicId: string;
      readonly sprintId: string;
      readonly revisionId: string;
      readonly workSlicePlanningPointId: string;
      readonly workUnitId: string;
      readonly label: string;
      readonly inspectionState?: Readonly<{
        readonly tab: 'activity' | 'evidence';
        readonly activityId: string;
        readonly sessionId: string;
        readonly invocationId: string;
      }>;
    }
  | {
      readonly kind: 'epic_planning_draft';
      readonly epicPlanningDraftId: string;
      readonly label: string;
    };

/** An immutable application-owned return destination for a standalone Session visit. */
export interface AgentSessionProductOrigin {
  readonly sessionId: string;
  readonly location: AgentSessionProductLocation;
  /** Present only when the opening product control has an exact durable invocation pointer. */
  readonly invocationId?: string;
}
