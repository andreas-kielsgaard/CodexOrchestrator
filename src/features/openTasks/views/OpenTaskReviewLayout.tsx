import {
  Activity,
  Archive,
  CheckCircle2,
  Clock3,
  Edit3,
  GitBranch,
  PauseCircle,
  ScrollText,
} from 'lucide-react';
import type { ReactNode } from 'react';
import type {
  OpenTaskAttentionValue,
  OpenTaskCardViewModel,
  OpenTaskEditDraftViewModel,
  OpenTaskExecutionValue,
  OpenTaskGroupViewId,
  OpenTaskGroupViewModel,
  OpenTaskId,
  OpenTaskReviewViewModel,
  OpenTaskRunTaskViewModel,
} from '../viewModels/taskReviewViewModel';
import { attentionOptions, executionOptions } from '../viewModels/taskOptions';
import { EditTaskForm, RunTaskForm } from './TaskForms';

const groupIcons = {
  needs_action_now: Activity,
  review_decide: CheckCircle2,
  working: GitBranch,
  waiting: Clock3,
  later: PauseCircle,
} satisfies Record<OpenTaskGroupViewId, typeof Activity>;

export interface OpenTaskReviewLayoutProps {
  review: OpenTaskReviewViewModel;
  editDraft: OpenTaskEditDraftViewModel;
  detailPanel: ReactNode;
  onEditDraftChange(draft: OpenTaskEditDraftViewModel): void;
  onSaveEdit(taskId: OpenTaskId): void;
  onCancelEdit(): void;
  onPromptChange(taskId: OpenTaskId, prompt: string): void;
  onStartRun(task: OpenTaskRunTaskViewModel): void;
  onUpdateAttention(taskId: OpenTaskId, attentionState: OpenTaskAttentionValue): void;
  onUpdateExecution(taskId: OpenTaskId, executionState: OpenTaskExecutionValue): void;
  onOpenDetail(taskId: OpenTaskId): void;
  onEditTask(task: OpenTaskCardViewModel): void;
  onArchiveTask(taskId: OpenTaskId): void;
}

export function OpenTaskReviewLayout({
  review,
  editDraft,
  detailPanel,
  onEditDraftChange,
  onSaveEdit,
  onCancelEdit,
  onPromptChange,
  onStartRun,
  onUpdateAttention,
  onUpdateExecution,
  onOpenDetail,
  onEditTask,
  onArchiveTask,
}: OpenTaskReviewLayoutProps) {
  return (
    <div className="task-review-layout">
      <section className="dashboard-grid" aria-label="Open task groups">
        {review.groups.map((group) => (
          <OpenTaskGroup
            key={group.id}
            group={group}
            editDraft={editDraft}
            onEditDraftChange={onEditDraftChange}
            onSaveEdit={onSaveEdit}
            onCancelEdit={onCancelEdit}
            onPromptChange={onPromptChange}
            onStartRun={onStartRun}
            onUpdateAttention={onUpdateAttention}
            onUpdateExecution={onUpdateExecution}
            onOpenDetail={onOpenDetail}
            onEditTask={onEditTask}
            onArchiveTask={onArchiveTask}
          />
        ))}
      </section>
      {detailPanel}
    </div>
  );
}

interface OpenTaskGroupProps
  extends Omit<OpenTaskReviewLayoutProps, 'review' | 'detailPanel'> {
  group: OpenTaskGroupViewModel;
}

function OpenTaskGroup({
  group,
  editDraft,
  onEditDraftChange,
  onSaveEdit,
  onCancelEdit,
  onPromptChange,
  onStartRun,
  onUpdateAttention,
  onUpdateExecution,
  onOpenDetail,
  onEditTask,
  onArchiveTask,
}: OpenTaskGroupProps) {
  const Icon = groupIcons[group.id];

  return (
    <article className="task-column">
      <header className="column-header">
        <div className="column-title">
          <Icon size={18} aria-hidden="true" />
          <h2>{group.title}</h2>
        </div>
        <span className="count">{group.tasks.length}</span>
      </header>

      <div className="task-list">
        {group.tasks.map((task) => (
          <OpenTaskCard
            key={task.id}
            task={task}
            editDraft={editDraft}
            onEditDraftChange={onEditDraftChange}
            onSaveEdit={onSaveEdit}
            onCancelEdit={onCancelEdit}
            onPromptChange={onPromptChange}
            onStartRun={onStartRun}
            onUpdateAttention={onUpdateAttention}
            onUpdateExecution={onUpdateExecution}
            onOpenDetail={onOpenDetail}
            onEditTask={onEditTask}
            onArchiveTask={onArchiveTask}
          />
        ))}
      </div>
    </article>
  );
}

interface OpenTaskCardProps
  extends Omit<OpenTaskReviewLayoutProps, 'review' | 'detailPanel'> {
  task: OpenTaskCardViewModel;
}

function OpenTaskCard({
  task,
  editDraft,
  onEditDraftChange,
  onSaveEdit,
  onCancelEdit,
  onPromptChange,
  onStartRun,
  onUpdateAttention,
  onUpdateExecution,
  onOpenDetail,
  onEditTask,
  onArchiveTask,
}: OpenTaskCardProps) {
  return (
    <section className={`task-card${task.selected ? ' selected' : ''}`}>
      {task.editing ? (
        <EditTaskForm
          form={editDraft}
          busy={task.busy}
          onChange={onEditDraftChange}
          onSave={() => onSaveEdit(task.id)}
          onCancel={onCancelEdit}
        />
      ) : (
        <>
          <div>
            <h3>{task.title}</h3>
            <p>{task.summary}</p>
          </div>
          <RunTaskForm
            task={task.runTask}
            prompt={task.prompt}
            runAction={task.runAction}
            busy={task.runBusy}
            onPromptChange={(prompt) => onPromptChange(task.id, prompt)}
            onStart={() => onStartRun(task.runTask)}
          />
          <div className="task-controls">
            <select
              value={task.attentionState}
              onChange={(event) =>
                onUpdateAttention(task.id, event.target.value as OpenTaskAttentionValue)
              }
              disabled={task.busy}
              aria-label={`Attention state for ${task.title}`}
            >
              {attentionOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
            <select
              value={task.executionState}
              onChange={(event) =>
                onUpdateExecution(task.id, event.target.value as OpenTaskExecutionValue)
              }
              disabled={task.busy}
              aria-label={`Execution state for ${task.title}`}
            >
              {executionOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
            <button
              className="icon-button"
              type="button"
              onClick={() => onOpenDetail(task.id)}
              disabled={task.busy}
              title="Open task detail"
              aria-label={`Open detail for ${task.title}`}
            >
              <ScrollText size={16} aria-hidden="true" />
            </button>
            <button
              className="icon-button"
              type="button"
              onClick={() => onEditTask(task)}
              disabled={task.busy}
              title="Edit task"
              aria-label={`Edit ${task.title}`}
            >
              <Edit3 size={16} aria-hidden="true" />
            </button>
            <button
              className="icon-button danger"
              type="button"
              onClick={() => onArchiveTask(task.id)}
              disabled={task.busy}
              title="Archive task"
              aria-label={`Archive ${task.title}`}
            >
              <Archive size={16} aria-hidden="true" />
            </button>
          </div>
          <footer>
            <span>{task.project}</span>
            <span>{task.priority}</span>
            <span>{task.executionState}</span>
            <span>{task.attentionState}</span>
            {task.repo && <span>{task.repo}</span>}
            {task.branch && <span>{task.branch}</span>}
            {task.worktreePath && (
              <span title={task.worktreePath}>{task.compactWorktreePath}</span>
            )}
          </footer>
        </>
      )}
    </section>
  );
}
