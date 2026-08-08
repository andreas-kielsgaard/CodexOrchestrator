/** Neutral Sprint workspace projection. It consumes composed product read models only. */
import type {
  ProductSprintReadModelV1,
  ProductSprintRevisionViewV1,
  ProductSourcedReadValueV1,
  ReadSourceAuthorityV1,
} from './productReadModels';

type Revision = ProductSprintReadModelV1['sprintPlan']['revisions'][number];
type Concern = ProductSprintReadModelV1['concerns'][number];
type Document = ProductSprintReadModelV1['documents'][number];
type Artifact = ProductSprintReadModelV1['internalArtifacts'][number];
type AgentSessionReference = ProductSprintReadModelV1['agentSessionReferences'][number];

export interface SprintWorkspacePresentationV1 {
  readonly epicEscalationReceivers: ProductSprintReadModelV1['epicEscalationReceivers'];
  readonly sprintResultProjections?: ProductSprintReadModelV1['sprintResultProjections'];
  readonly sprint: Readonly<{
    readonly sprintId: string;
    readonly epicId: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly source: ReadSourceAuthorityV1;
    readonly lifecycle?: ProductSprintReadModelV1['lifecycle'];
    readonly planningState: ProductSprintReadModelV1['planningState'];
    readonly sprintRunnerTransition?: ProductSprintReadModelV1['sprintRunnerTransition'];
    readonly workUnitMaterializations: ProductSprintReadModelV1['workUnitMaterializations'];
  }>;
  readonly revisions: readonly Readonly<{
    readonly sprintPlanRevisionId: string;
    readonly revision: number;
    readonly summary: string;
    readonly source: ReadSourceAuthorityV1;
    readonly supersedesSprintPlanRevisionId?: string;
    readonly isCurrent: boolean;
    readonly isSelected: boolean;
    readonly workUnitScopes: Revision['workUnitScopes'];
  }>[];
  readonly activeSprintPlanRevisionId: string;
  readonly selectedSprintPlanRevisionId: string;
  readonly revisionViews: readonly ProductSprintRevisionViewV1[];
  readonly concerns: readonly Concern[];
  readonly epicRunnerObjectives: NonNullable<
    ProductSprintReadModelV1['workspacePresentation']['epicRunnerObjectives']
  >;
  readonly sprintRunnerConcerns: NonNullable<
    ProductSprintReadModelV1['workspacePresentation']['sprintRunnerConcerns']
  >;
  readonly workUnitLifecycle: NonNullable<
    ProductSprintReadModelV1['workspacePresentation']['workUnitLifecycle']
  >;
  /** Documents and internal Artifacts remain separate ownership surfaces. */
  readonly documents: readonly (Document &
    Readonly<{
      readonly displayOrder: number;
      readonly recordedAt: ProductSourcedReadValueV1<string>;
      readonly displayCategory: ProductSourcedReadValueV1<string>;
      readonly sprintPlanRevisionIds: readonly string[];
      readonly workSlicePlanningPointIds: readonly string[];
      readonly workUnitScopeIds: readonly string[];
    }>)[];
  readonly internalArtifacts: readonly Artifact[];
  readonly agentSessionReferences: readonly AgentSessionReference[];
  readonly continuation: ProductSprintReadModelV1['continuation'];
  readonly sprintContinuation?: ProductSprintReadModelV1['sprintContinuation'];
  readonly narratives?: ProductSprintReadModelV1['workspacePresentation']['narratives'];
}

/** Deterministic: grouping and display ordering come only from composed presentation metadata. */
export function projectSprintWorkspacePresentation(
  sprint: ProductSprintReadModelV1,
): SprintWorkspacePresentationV1 {
  const documentPresentationById = new Map(
    sprint.workspacePresentation.documents.map((presentation) => [
      presentation.documentRefId,
      presentation,
    ]),
  );
  return {
    epicEscalationReceivers: sprint.epicEscalationReceivers,
    sprintResultProjections: sprint.sprintResultProjections,
    sprint: {
      sprintId: sprint.sprintId,
      epicId: sprint.epicId,
      title: sprint.title,
      summary: sprint.summary,
      details: sprint.details,
      source: sprint.source,
      ...(sprint.lifecycle ? { lifecycle: sprint.lifecycle } : {}),
      planningState: sprint.planningState,
      ...(sprint.sprintRunnerTransition
        ? { sprintRunnerTransition: sprint.sprintRunnerTransition }
        : {}),
      workUnitMaterializations: (sprint.workUnitMaterializations ?? []).map((materialization) => ({
        ...materialization,
      })),
    },
    revisions: sprint.sprintPlan.revisions.map((revision) => ({ ...revision })),
    activeSprintPlanRevisionId: sprint.sprintPlan.currentSprintPlanRevisionId,
    selectedSprintPlanRevisionId: sprint.sprintPlan.selectedSprintPlanRevisionId,
    revisionViews: sprint.revisionViews.map((view) => ({
      ...view,
      workUnitScopes: view.workUnitScopes.map((scope) => ({ ...scope })),
      workSlicePlanningPointGroups: view.workSlicePlanningPointGroups
        .map((group) => ({ ...group, workUnitScopeIds: [...group.workUnitScopeIds].sort() }))
        .sort((left, right) =>
          left.workSlicePlanningPointId.localeCompare(right.workSlicePlanningPointId),
        ),
      workUnits: view.workUnits
        .map((workUnit) => ({ ...workUnit }))
        .sort((left, right) => left.workUnitScopeId.localeCompare(right.workUnitScopeId)),
      gates: view.gates
        .map((gate) => ({ ...gate }))
        .sort((left, right) => left.gateId.localeCompare(right.gateId)),
      reviews: view.reviews.map((review) => ({ ...review })),
    })),
    concerns: sprint.concerns.map((concern) => ({ ...concern })),
    epicRunnerObjectives: (sprint.workspacePresentation.epicRunnerObjectives ?? []).map(
      (objective) => ({ ...objective }),
    ),
    sprintRunnerConcerns: (sprint.workspacePresentation.sprintRunnerConcerns ?? []).map(
      (sprintRunnerConcern) => ({
        ...sprintRunnerConcern,
        graphElementRefs: sprintRunnerConcern.graphElementRefs.map((reference) => ({
          ...reference,
        })),
      }),
    ),
    workUnitLifecycle: [...(sprint.workspacePresentation.workUnitLifecycle ?? [])]
      .map((entry) => ({ ...entry }))
      .sort(
        (left, right) =>
          left.sequence - right.sequence || left.entryId.localeCompare(right.entryId),
      ),
    documents: sprint.documents
      .map((document) => {
        const presentation = documentPresentationById.get(document.documentRefId);
        if (!presentation) fail(`missing presentation for Document ${document.documentRefId}`);
        return { ...document, ...presentation };
      })
      .sort((left, right) => left.displayOrder - right.displayOrder),
    internalArtifacts: sprint.internalArtifacts.map((artifact) => ({ ...artifact })),
    agentSessionReferences: sprint.agentSessionReferences.map((reference) => ({ ...reference })),
    continuation: sprint.continuation,
    ...(sprint.sprintContinuation
      ? { sprintContinuation: sprint.sprintContinuation }
      : {}),
    ...(sprint.workspacePresentation.narratives
      ? { narratives: sprint.workspacePresentation.narratives }
      : {}),
  };
}

function fail(message: string): never {
  throw new Error(`Invalid product Sprint workspace presentation: ${message}`);
}
