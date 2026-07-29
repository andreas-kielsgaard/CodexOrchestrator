import { useState, type MouseEvent } from 'react';
import { EpicDetail } from './components/EpicDetail';
import { EpicDescriptionHelp, MovementSummary, StateBadge } from './components/EpicStatusBadges';
import {
  isUnavailablePresentation,
  type EpicOverviewAction,
  type EpicOverviewNavigationTarget,
  type EpicPresentation,
  type SprintWorkspaceDetailLocation,
  type OrchestrationSectionView,
} from './orchestrationModel';
import type { EmbeddedAgentSessionComposition } from '../agentSessions';
import {
  unsupportedArtifactAccessController,
  type ArtifactAccessController,
  SprintAutomaticContinuationPolicyController,
  EpicAutomaticContinuationPolicyController,
} from '../../application/orchestrations';
import './styles/orchestrationSection.css';
import type { EpicPlanningDraftSummary } from '../../application/orchestrations';

export interface OrchestrationSectionProps {
  readonly view: OrchestrationSectionView;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly artifactAccessController?: ArtifactAccessController;
  readonly sprintAutomaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly epicAutomaticContinuationPolicyController?: EpicAutomaticContinuationPolicyController;
  readonly onPlanEpic?: () => void;
  readonly planningDrafts?: readonly EpicPlanningDraftSummary[];
  readonly onOpenPlanningDraft?: (draft: EpicPlanningDraftSummary) => void;
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
}: OrchestrationSectionProps) {
  const workspace = useOrchestrationWorkspace();
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
              <th scope="col">State and next action</th>
            </tr>
          </thead>
          <tbody>
            {planningDrafts.map((draft) => (
              <PlanningDraftRow
                draft={draft}
                key={draft.epicPlanningDraftId}
                onOpen={() => onOpenPlanningDraft?.(draft)}
              />
            ))}
            {view.epics.map((epic) => (
              <EpicOverviewRow epic={epic} key={epic.id} onNavigate={workspace.openTarget} />
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}

function PlanningDraftRow({
  draft,
  onOpen,
}: {
  readonly draft: EpicPlanningDraftSummary;
  readonly onOpen: () => void;
}) {
  const title = draft.title ?? 'Untitled Epic draft';
  return (
    <tr className="orchestration-list__row" onClick={(event) => openFromRow(event, onOpen)}>
      <td data-label="Epic">
        <button
          className="orchestration-list__open"
          type="button"
          aria-label={`Open planning draft ${title}`}
          onClick={onOpen}
        >
          <strong>{title}</strong>
          <small>Pre-initiation planning draft</small>
        </button>
      </td>
      <td data-label="Current movement">
        <span className="movement-badge movement-badge--empty">Planning draft</span>
      </td>
      <td data-label="State and next action">
        <span className="epic-state">Draft</span>
      </td>
    </tr>
  );
}

function EpicOverviewRow({
  epic,
  onNavigate,
}: {
  readonly epic: EpicPresentation;
  readonly onNavigate: (target: EpicOverviewNavigationTarget) => void;
}) {
  const epicTarget: EpicOverviewNavigationTarget = { kind: 'epic', epicId: epic.id };
  const openEpic = () => onNavigate(epicTarget);
  const readyWork = Array.isArray(epic.readyWork) ? epic.readyWork : [];
  const humanInput =
    epic.humanInput && !isUnavailablePresentation(epic.humanInput) ? epic.humanInput : null;
  return (
    <tr className="orchestration-list__row" onClick={(event) => openFromRow(event, openEpic)}>
      <td data-label="Epic">
        <div className="orchestration-list__title">
          <button
            className="orchestration-list__open"
            type="button"
            aria-label={`Open ${epic.name}`}
            onClick={openEpic}
          >
            <strong>{epic.name}</strong>
          </button>
          <EpicDescriptionHelp name={epic.name} description={epic.goal} />
        </div>
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
        <MovementSummary movement={epic.movement} onNavigate={onNavigate} />
      </td>
      <td data-label="State and next action">
        <div className="epic-overview-actions">
          <StateBadge state={epic.state} />
          {readyWork.map((action) => (
            <OverviewAction action={action} key={action.actionId} onNavigate={onNavigate} />
          ))}
          {humanInput && (
            <OverviewAction
              action={humanInput}
              humanInput
              key={humanInput.actionId}
              onNavigate={onNavigate}
            />
          )}
        </div>
      </td>
    </tr>
  );
}

function OverviewAction({
  action,
  humanInput = false,
  onNavigate,
}: {
  readonly action: EpicOverviewAction;
  readonly humanInput?: boolean;
  readonly onNavigate: (target: EpicOverviewNavigationTarget) => void;
}) {
  return (
    <button
      className={`epic-overview-action${humanInput ? ' epic-overview-action--human-input' : ''}`}
      type="button"
      aria-label={humanInput ? `Human input required: ${action.label}` : action.label}
      onClick={() => onNavigate(action.target)}
    >
      {humanInput && <small>Human input</small>}
      <span>{action.label}</span>
    </button>
  );
}

function openFromRow(event: MouseEvent<HTMLTableRowElement>, open: () => void) {
  if (!(event.target instanceof Element)) return;
  if (
    event.target.closest(
      'button, a, input, select, textarea, summary, [role="button"], [role="link"], [data-row-action-exempt]',
    )
  )
    return;
  open();
}

/** The feature tree has one owner for Orchestration, Sprint, and revision selection. */
function useOrchestrationWorkspace() {
  const [epicId, setEpicId] = useState<string | null>(null);
  const [sprintId, setSprintId] = useState<string | null>(null);
  const [selectedRevisionId, setSelectedRevisionId] = useState<string | null>(null);
  const [detailLocation, setDetailLocation] = useState<SprintWorkspaceDetailLocation>({
    kind: 'sprint',
  });
  return {
    epicId,
    sprintId,
    selectedRevisionId,
    detailLocation,
    openTarget(target: EpicOverviewNavigationTarget) {
      setEpicId(target.epicId);
      if (target.kind === 'epic') {
        setSprintId(null);
        setSelectedRevisionId(null);
        setDetailLocation({ kind: 'sprint' });
        return;
      }
      setSprintId(target.sprintId);
      setSelectedRevisionId(target.revisionId);
      setDetailLocation(
        target.kind === 'sprint'
          ? { kind: 'sprint' }
          : target.kind === 'sprint_planner_activity'
            ? {
                kind: 'sprint_planner_activity_group',
                revisionId: target.revisionId,
                sprintPlannerActivityId: target.sprintPlannerActivityId,
              }
            : {
                kind: 'work_unit',
                revisionId: target.revisionId,
                sprintPlannerActivityId: target.sprintPlannerActivityId,
                workUnitId: target.workUnitId,
                origin: 'sprint_planner_activity_group',
              },
      );
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
