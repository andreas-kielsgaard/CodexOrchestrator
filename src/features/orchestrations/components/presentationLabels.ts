import { sourceStatusLabel, type SprintPlanItemStatus } from '../orchestrationModel';

export function sprintStatusLabel(status: SprintPlanItemStatus): string {
  if (typeof status !== 'string') return `${sourceStatusLabel(status.kind)}: ${status.reason}`;
  return {
    completed: 'Completed',
    in_progress: 'In progress',
    not_started: 'Not started',
  }[status];
}
