import { useEffect, useRef, useState } from 'react';
import type {
  SprintPlanItemPresentation,
  SprintWorkspaceDetailLocation,
  EpicPresentation,
} from '../orchestrationModel';
import { ContinuationControl } from './ContinuationControl';
import { DetailWorkspace } from './DetailWorkspace';
import { SprintDetailDialog } from './SprintDetailDialog';
import { SprintPlan } from './SprintPlan';
import { SprintWorkspace } from './SprintWorkspace';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import { sprintStatusLabel } from './presentationLabels';
import '../styles/epicDetail.css';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type {
  ArtifactAccessController,
  SprintAutomaticContinuationPolicyController,
  EpicAutomaticContinuationPolicyController,
} from '../../../application/orchestrations';

export interface EpicDetailProps {
  readonly epic: EpicPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly artifactAccessController: ArtifactAccessController;
  readonly sprintAutomaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly epicAutomaticContinuationPolicyController?: EpicAutomaticContinuationPolicyController;
  readonly selectedSprintId: string | null;
  readonly selectedRevisionId: string | null;
  readonly detailLocation: SprintWorkspaceDetailLocation;
  readonly onOpenSprint: (sprintId: string, revisionId: string) => void;
  readonly onCloseSprint: () => void;
  readonly onSelectedRevisionChange: (revisionId: string) => void;
  readonly onDetailLocationChange: (location: SprintWorkspaceDetailLocation) => void;
  readonly onBack: () => void;
  readonly onOpenFileReviewSource?: (sourceId: string) => void;
}

export function EpicDetail({
  epic,
  agentSessionComposition,
  artifactAccessController,
  sprintAutomaticContinuationPolicyController,
  epicAutomaticContinuationPolicyController,
  selectedSprintId,
  selectedRevisionId,
  detailLocation,
  onOpenSprint,
  onCloseSprint,
  onSelectedRevisionChange,
  onDetailLocationChange,
  onBack,
  onOpenFileReviewSource,
}: EpicDetailProps) {
  const restoreSprintIdRef = useRef<string | null>(null);
  const [selectedSprintOpener, setSelectedSprintOpener] = useState<{
    readonly sprint: SprintPlanItemPresentation;
    readonly opener: HTMLButtonElement;
  } | null>(null);
  const selectedSprint = epic.plan.items.find(({ id }) => id === selectedSprintId);
  const activeSprint =
    epic.plan.items.find(({ status }) => status === 'in_progress') ??
    [...epic.plan.items].reverse().find(({ status }) => status === 'completed');

  useEffect(() => {
    if (selectedSprint || !restoreSprintIdRef.current) return;
    const sprintId = restoreSprintIdRef.current;
    restoreSprintIdRef.current = null;
    Array.from(document.querySelectorAll<HTMLButtonElement>('[data-sprint-id]'))
      .find((button) => button.dataset.sprintId === sprintId)
      ?.focus();
  }, [selectedSprint]);

  if (selectedSprint?.workspace) {
    return (
      <SprintWorkspace
        workspace={selectedSprint.workspace}
        epicObjective={epic.goal}
        adjunct={selectedSprint.workspaceAdjunct}
        artifactAccessController={artifactAccessController}
        agentSessionComposition={agentSessionComposition}
        automaticContinuationPolicyController={sprintAutomaticContinuationPolicyController}
        selectedRevisionId={
          selectedRevisionId ?? selectedSprint.workspace.selectedSprintPlanRevisionId
        }
        onSelectedRevisionChange={onSelectedRevisionChange}
        detailLocation={detailLocation}
        onDetailLocationChange={onDetailLocationChange}
        onOpenFileReviewSource={onOpenFileReviewSource}
        onBack={() => {
          restoreSprintIdRef.current = selectedSprint.id;
          onCloseSprint();
        }}
      />
    );
  }

  return (
    <>
      <DetailWorkspace
        ariaLabel="Epic detail"
        controlsLabel="Epic controls"
        contextLabel="Epic context"
        backLabel="Back to Epics"
        onBack={onBack}
        control={
          epic.continuation ? (
            <ContinuationControl
              continuation={epic.continuation}
              controller={epicAutomaticContinuationPolicyController}
            />
          ) : null
        }
        context={
          <>
            <div className="orchestration-context-rail__epic">
              <p className="eyebrow">Epic</p>
              <h1>{epic.name}</h1>
            </div>
            {activeSprint && (
              <div className="orchestration-context-rail__sprint">
                <p className="eyebrow">Active Sprint</p>
                <strong>{activeSprint.name}</strong>
                <span
                  className={`sprint-context-status sprint-context-status--${
                    typeof activeSprint.status === 'string'
                      ? activeSprint.status
                      : activeSprint.status.kind
                  }`}
                >
                  {sprintStatusLabel(activeSprint.status)}
                </span>
              </div>
            )}
          </>
        }
        primary={
          <SprintPlan
            items={epic.plan.items}
            onOpen={(sprint, opener) => {
              setSelectedSprintOpener({ sprint, opener });
              onOpenSprint(sprint.id, sprint.workspace?.selectedSprintPlanRevisionId ?? '');
            }}
          />
        }
        agentSession={
          epic.epicRunnerSession ? (
            <SharedAgentSessionPanel
              ariaLabel="Epic Runner Agent Session"
              conversationAriaLabel="Epic Runner Agent Session conversation"
              session={epic.epicRunnerSession}
              composition={agentSessionComposition}
            />
          ) : undefined
        }
      />

      {selectedSprint && !selectedSprint.workspace && selectedSprintOpener && (
        <SprintDetailDialog
          sprint={selectedSprint}
          restoreFocusTo={selectedSprintOpener.opener}
          onClose={() => onCloseSprint()}
        />
      )}
    </>
  );
}
