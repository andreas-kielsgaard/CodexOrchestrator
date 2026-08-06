import { useEffect, useState } from 'react';
import { EpicDetail } from './components/EpicDetail';
import { EpicTitleWithDescription, MovementBadge, StateBadge } from './components/EpicStatusBadges';
import type { SprintWorkspaceDetailLocation, OrchestrationSectionView } from './orchestrationModel';
import type { EmbeddedAgentSessionComposition } from '../agentSessions';
import {
  unsupportedArtifactAccessController,
  type ArtifactAccessController,
  SprintAutomaticContinuationPolicyController,
  EpicAutomaticContinuationPolicyController,
} from '../../application/orchestrations';
import './styles/orchestrationSection.css';
import type { EpicPlanningDraftSummary } from '../../application/orchestrations';
import type {
  AgentSessionProductLocation,
  AgentSessionProductOrigin,
} from '../../application/agentSessionNavigation';
import type { ContextualFileReviewResult } from '../../application/contextualFileReview';
import type { WorkUnitActivitySessionTarget } from './components/WorkUnitDetailWorkspace';

export type OrchestrationNavigationChangeIntent = 'push' | 'back';

export interface OrchestrationSectionProps {
  readonly view: OrchestrationSectionView;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly artifactAccessController?: ArtifactAccessController;
  readonly sprintAutomaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly epicAutomaticContinuationPolicyController?: EpicAutomaticContinuationPolicyController;
  readonly onPlanEpic?: () => void;
  readonly planningDrafts?: readonly EpicPlanningDraftSummary[];
  readonly onOpenPlanningDraft?: (draft: EpicPlanningDraftSummary) => void;
  readonly requestedLocation?: AgentSessionProductLocation | null;
  readonly onProductLocationChange?: (
    location: AgentSessionProductLocation | null,
    intent: OrchestrationNavigationChangeIntent,
  ) => void;
  readonly onOpenAgentSession?: (origin: AgentSessionProductOrigin) => void;
  readonly onRequestFileReview?: (
    sprintId: string,
    returnLocation?: AgentSessionProductLocation,
  ) => Promise<ContextualFileReviewResult>;
  readonly onOpenFileEvidence?: (
    target: {
      readonly reviewId: string;
      readonly changedFileId: string;
    },
    returnLocation?: AgentSessionProductLocation,
  ) => void;
  readonly onOpenWorkUnitActivitySession?: (
    target: WorkUnitActivitySessionTarget,
    origin: AgentSessionProductOrigin,
  ) => void;
  /** The application shell owns actual typed history; details retain local Back only without it. */
  readonly globalBackAvailable?: boolean;
}

export function OrchestrationSection({
  view,
  agentSessionComposition,
  artifactAccessController = unsupportedArtifactAccessController,
  sprintAutomaticContinuationPolicyController,
  epicAutomaticContinuationPolicyController,
  onPlanEpic,
  planningDrafts = [],
  onOpenPlanningDraft,
  requestedLocation,
  onProductLocationChange,
  onOpenAgentSession,
  onRequestFileReview,
  onOpenFileEvidence,
  onOpenWorkUnitActivitySession,
  globalBackAvailable = false,
}: OrchestrationSectionProps) {
  const workspace = useOrchestrationWorkspace(view, requestedLocation, onProductLocationChange);
  const selected = view.epics.find(({ id }) => id === workspace.epicId);

  if (selected) {
    return (
      <EpicDetail
        epic={selected}
        agentSessionComposition={agentSessionComposition}
        artifactAccessController={artifactAccessController}
        sprintAutomaticContinuationPolicyController={sprintAutomaticContinuationPolicyController}
        epicAutomaticContinuationPolicyController={epicAutomaticContinuationPolicyController}
        selectedSprintId={workspace.sprintId}
        selectedRevisionId={workspace.selectedRevisionId}
        detailLocation={workspace.detailLocation}
        onOpenSprint={workspace.openSprint}
        onCloseSprint={workspace.closeSprint}
        onSelectedRevisionChange={workspace.selectRevision}
        onDetailLocationChange={workspace.setDetailLocation}
        onBack={workspace.backToOverview}
        globalBackAvailable={globalBackAvailable}
        onOpenAgentSession={onOpenAgentSession}
        onRequestFileReview={onRequestFileReview}
        onOpenFileEvidence={onOpenFileEvidence}
        onOpenWorkUnitActivitySession={onOpenWorkUnitActivitySession}
      />
    );
  }

  return (
    <main className="orchestration-section" aria-label="Orchestration">
      <header className="orchestration-page-header">
        <p className="eyebrow">Orchestration</p>
        <h1>Orchestration overview</h1>
        <p>Follow coordinated work from plan through acceptance.</p>
        <button className="orchestration-page-header__plan" type="button" onClick={onPlanEpic}>
          Plan an Epic
        </button>
      </header>
      <section className="orchestration-list" aria-labelledby="orchestration-list-heading">
        <h2 className="visually-hidden" id="orchestration-list-heading">
          Your Epics
        </h2>
        <table>
          <thead>
            <tr>
              <th scope="col">Epic</th>
              <th scope="col">Current movement</th>
              <th scope="col">State</th>
            </tr>
          </thead>
          <tbody>
            {planningDrafts.map((draft) => (
              <tr key={draft.epicPlanningDraftId}>
                <td data-label="Epic">
                  <button
                    className="orchestration-list__open"
                    type="button"
                    onClick={() => onOpenPlanningDraft?.(draft)}
                  >
                    <strong>{draft.title ?? 'Untitled Epic draft'}</strong>
                    <small>Pre-initiation planning draft</small>
                  </button>
                </td>
                <td data-label="Current movement">Planning</td>
                <td data-label="State">Draft</td>
              </tr>
            ))}
            {view.epics.map((epic) => (
              <tr key={epic.id}>
                <td data-label="Epic">
                  <EpicTitleWithDescription
                    name={epic.name}
                    description={epic.goal}
                    onOpen={() => workspace.openEpic(epic.id)}
                  />
                  <small>{epic.goal}</small>
                  {epic.bootstrapTransition && (
                    <small
                      className={`orchestration-transition orchestration-transition--${epic.bootstrapTransition.kind}`}
                    >
                      {epic.bootstrapTransition.label}
                      {epic.bootstrapTransition.kind === 'blocked'
                        ? `: ${epic.bootstrapTransition.reason}`
                        : ''}
                    </small>
                  )}
                </td>
                <td data-label="Current movement">
                  <MovementBadge movement={epic.movement} />
                </td>
                <td data-label="State">
                  <StateBadge state={epic.state} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}

/** The feature tree has one owner for Orchestration, Sprint, and revision selection. */
function useOrchestrationWorkspace(
  view: OrchestrationSectionView,
  requestedLocation: AgentSessionProductLocation | null | undefined,
  onProductLocationChange?: (
    location: AgentSessionProductLocation | null,
    intent: OrchestrationNavigationChangeIntent,
  ) => void,
) {
  const [epicId, setEpicId] = useState<string | null>(null);
  const [sprintId, setSprintId] = useState<string | null>(null);
  const [selectedRevisionId, setSelectedRevisionId] = useState<string | null>(null);
  const [detailLocation, setDetailLocation] = useState<SprintWorkspaceDetailLocation>({
    kind: 'sprint',
  });
  useEffect(() => {
    if (!requestedLocation) {
      setEpicId(null);
      setSprintId(null);
      setSelectedRevisionId(null);
      setDetailLocation({ kind: 'sprint' });
      return;
    }
    if (requestedLocation.kind === 'epic_planning_draft') return;
    setEpicId(requestedLocation.epicId);
    if (requestedLocation.kind === 'epic') {
      setSprintId(null);
      setSelectedRevisionId(null);
      setDetailLocation({ kind: 'sprint' });
      return;
    }
    setSprintId(requestedLocation.sprintId);
    if (requestedLocation.kind === 'sprint') {
      setSelectedRevisionId(null);
      setDetailLocation({ kind: 'sprint' });
      return;
    }
    setSelectedRevisionId(requestedLocation.revisionId);
    if (requestedLocation.kind === 'work_slice_planning_point') {
      setDetailLocation({
        kind: 'work_slice_planning_point',
        revisionId: requestedLocation.revisionId,
        workSlicePlanningPointId: requestedLocation.workSlicePlanningPointId,
      });
      return;
    }
    setDetailLocation({
      kind: 'work_unit',
      revisionId: requestedLocation.revisionId,
      workSlicePlanningPointId: requestedLocation.workSlicePlanningPointId,
      workUnitId: requestedLocation.workUnitId,
      origin: 'work_slice_planning_point',
      ...(requestedLocation.inspectionState
        ? { inspectionState: requestedLocation.inspectionState }
        : {}),
    });
  }, [requestedLocation]);
  return {
    epicId,
    sprintId,
    selectedRevisionId,
    detailLocation,
    openEpic(id: string) {
      setEpicId(id);
      setSprintId(null);
      setSelectedRevisionId(null);
      setDetailLocation({ kind: 'sprint' });
      onProductLocationChange?.(epicProductLocation(view, id), 'push');
    },
    openSprint(id: string, revisionId: string) {
      setSprintId(id);
      setSelectedRevisionId(revisionId);
      setDetailLocation({ kind: 'sprint' });
      onProductLocationChange?.(sprintProductLocation(view, epicId, id), 'push');
    },
    closeSprint() {
      setSprintId(null);
      setSelectedRevisionId(null);
      setDetailLocation({ kind: 'sprint' });
      onProductLocationChange?.(epicProductLocation(view, epicId), 'back');
    },
    selectRevision(id: string) {
      setSelectedRevisionId(id);
    },
    setDetailLocation(location: SprintWorkspaceDetailLocation) {
      setDetailLocation(location);
      onProductLocationChange?.(
        detailProductLocation(view, epicId, sprintId, location),
        detailNavigationIntent(detailLocation, location),
      );
    },
    backToOverview() {
      setEpicId(null);
      setSprintId(null);
      setSelectedRevisionId(null);
      setDetailLocation({ kind: 'sprint' });
      onProductLocationChange?.(null, 'back');
    },
  };
}

function epicProductLocation(
  view: OrchestrationSectionView,
  epicId: string | null,
): AgentSessionProductLocation | null {
  if (!epicId) return null;
  const epic = view.epics.find(({ id }) => id === epicId);
  return epic ? { kind: 'epic', epicId: epic.id, label: epic.name } : null;
}

function sprintProductLocation(
  view: OrchestrationSectionView,
  epicId: string | null,
  sprintId: string,
): AgentSessionProductLocation | null {
  const epic = view.epics.find(({ id }) => id === epicId);
  const sprint = epic?.plan.items.find(({ id }) => id === sprintId);
  return epic && sprint ? { kind: 'sprint', epicId: epic.id, sprintId, label: sprint.name } : null;
}

function detailProductLocation(
  view: OrchestrationSectionView,
  epicId: string | null,
  sprintId: string | null,
  detailLocation: SprintWorkspaceDetailLocation,
): AgentSessionProductLocation | null {
  const sprintLocation = sprintProductLocation(view, epicId, sprintId ?? '');
  if (!sprintLocation) return null;
  if (detailLocation.kind === 'sprint') return sprintLocation;

  const epic = view.epics.find(({ id }) => id === epicId);
  const sprint = epic?.plan.items.find(({ id }) => id === sprintId);
  const revision = sprint?.workspace?.revisionViews.find(
    ({ sprintPlanRevisionId }) => sprintPlanRevisionId === detailLocation.revisionId,
  );
  const planningPoint = revision?.workSlicePlanningPointGroups.find(
    ({ workSlicePlanningPointId }) =>
      workSlicePlanningPointId === detailLocation.workSlicePlanningPointId,
  );
  if (!revision || !planningPoint || !epic || !sprint) return null;
  if (detailLocation.kind === 'work_slice_planning_point') {
    return {
      kind: 'work_slice_planning_point',
      epicId: epic.id,
      sprintId: sprint.id,
      revisionId: revision.sprintPlanRevisionId,
      workSlicePlanningPointId: planningPoint.workSlicePlanningPointId,
      label: planningPoint.title,
    };
  }
  const unit = revision.workUnits.find(
    ({ workUnitId }) => workUnitId === detailLocation.workUnitId,
  );
  if (!unit) return null;
  return {
    kind: 'work_unit',
    epicId: epic.id,
    sprintId: sprint.id,
    revisionId: revision.sprintPlanRevisionId,
    workSlicePlanningPointId: planningPoint.workSlicePlanningPointId,
    workUnitId: unit.workUnitId,
    label: unit.title,
    ...(detailLocation.inspectionState ? { inspectionState: detailLocation.inspectionState } : {}),
  };
}

function detailNavigationIntent(
  current: SprintWorkspaceDetailLocation,
  next: SprintWorkspaceDetailLocation,
): 'push' | 'back' {
  if (current.kind === 'sprint' && next.kind !== 'sprint') return 'push';
  if (current.kind === 'work_slice_planning_point' && next.kind === 'work_unit') return 'push';
  return 'back';
}
