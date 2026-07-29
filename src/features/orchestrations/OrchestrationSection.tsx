import { useEffect, useState } from 'react';
import { EpicDetail } from './components/EpicDetail';
import { MovementBadge, StateBadge } from './components/EpicStatusBadges';
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
import type { AgentSessionProductLocation } from '../../application/agentSessionNavigation';

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
  readonly onOpenAgentSession?: (sessionId: string) => void;
  readonly onOpenFileReviewSource?: (sourceId: string) => void;
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
  onOpenAgentSession,
  onOpenFileReviewSource,
}: OrchestrationSectionProps) {
  const workspace = useOrchestrationWorkspace(requestedLocation);
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
        onOpenAgentSession={onOpenAgentSession}
        onOpenFileReviewSource={onOpenFileReviewSource}
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
                  <button
                    className="orchestration-list__open"
                    type="button"
                    aria-label={`Open ${epic.name}`}
                    onClick={() => workspace.openEpic(epic.id)}
                  >
                    <strong>{epic.name}</strong>
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
                  </button>
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
function useOrchestrationWorkspace(requestedLocation?: AgentSessionProductLocation | null) {
  const [epicId, setEpicId] = useState<string | null>(null);
  const [sprintId, setSprintId] = useState<string | null>(null);
  const [selectedRevisionId, setSelectedRevisionId] = useState<string | null>(null);
  const [detailLocation, setDetailLocation] = useState<SprintWorkspaceDetailLocation>({
    kind: 'sprint',
  });
  useEffect(() => {
    if (!requestedLocation || requestedLocation.kind === 'epic_planning_draft') return;
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
    if (requestedLocation.kind === 'sprint_planner_activity') {
      setDetailLocation({
        kind: 'sprint_planner_activity_group',
        revisionId: requestedLocation.revisionId,
        sprintPlannerActivityId: requestedLocation.sprintPlannerActivityId,
      });
      return;
    }
    setDetailLocation({
      kind: 'work_unit',
      revisionId: requestedLocation.revisionId,
      sprintPlannerActivityId: requestedLocation.sprintPlannerActivityId,
      workUnitId: requestedLocation.workUnitId,
      origin: 'sprint_planner_activity_group',
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
    },
    openSprint(id: string, revisionId: string) {
      setSprintId(id);
      setSelectedRevisionId(revisionId);
      setDetailLocation({ kind: 'sprint' });
    },
    closeSprint() {
      setSprintId(null);
      setSelectedRevisionId(null);
      setDetailLocation({ kind: 'sprint' });
    },
    selectRevision(id: string) {
      setSelectedRevisionId(id);
    },
    setDetailLocation(location: SprintWorkspaceDetailLocation) {
      setDetailLocation(location);
    },
    backToOverview() {
      setEpicId(null);
      setSprintId(null);
      setSelectedRevisionId(null);
      setDetailLocation({ kind: 'sprint' });
    },
  };
}
