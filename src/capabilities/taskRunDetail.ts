import type {
  TaskRunDetailSnapshot,
} from '../application/queries/taskRunDetailClient';
import type { EntityId } from '../domain/model';

export type {
  TaskRunDetailArtifactGroups,
  TaskRunDetailRun,
  TaskRunDetailSnapshot,
  TaskRunDetailTaskAnchor,
  TaskRunDetailValidationRun,
} from '../application/queries/taskRunDetailClient';

export interface TaskRunDetailCapability {
  loadTaskRunDetail(taskId: EntityId): Promise<TaskRunDetailSnapshot>;
}
