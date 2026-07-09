import { Check, LoaderCircle, Play, X } from 'lucide-react';
import type {
  OpenTaskEditDraftViewModel,
  OpenTaskPriorityValue,
  OpenTaskRunFeedbackViewModel,
  OpenTaskRunTaskViewModel,
  OpenTaskAttentionValue,
  OpenTaskExecutionValue,
} from '../viewModels/taskReviewViewModel';
import { attentionOptions, executionOptions, priorityOptions } from '../viewModels/taskOptions';

export interface RunTaskFormProps {
  task: OpenTaskRunTaskViewModel;
  prompt: string;
  runAction?: OpenTaskRunFeedbackViewModel;
  busy: boolean;
  onPromptChange(prompt: string): void;
  onStart(): void;
}

export function RunTaskForm({
  task,
  prompt,
  runAction,
  busy,
  onPromptChange,
  onStart,
}: RunTaskFormProps) {
  const hasWorktree = Boolean(task.worktreePath);
  const canStart = hasWorktree && prompt.trim().length > 0 && !busy;

  return (
    <div className="run-controls">
      <div className="run-command">
        <textarea
          value={prompt}
          onChange={(event) => onPromptChange(event.target.value)}
          disabled={!hasWorktree || busy}
          placeholder={hasWorktree ? 'Codex prompt' : 'Worktree required'}
          aria-label={`Codex prompt for ${task.title}`}
          rows={2}
        />
        <button
          className="icon-button run-button"
          type="button"
          onClick={onStart}
          disabled={!canStart}
          title={hasWorktree ? 'Start Codex run' : 'Task needs a worktree'}
          aria-label={`Start Codex run for ${task.title}`}
        >
          {busy ? (
            <LoaderCircle size={16} aria-hidden="true" />
          ) : (
            <Play size={16} aria-hidden="true" />
          )}
        </button>
      </div>
      {hasWorktree ? (
        runAction && (
          <p className={`run-feedback ${runAction.status}`} role="status">
            {runAction.message}
          </p>
        )
      ) : (
        <p className="run-feedback unavailable" role="status">
          No worktree linked
        </p>
      )}
    </div>
  );
}

export interface EditTaskFormProps {
  form: OpenTaskEditDraftViewModel;
  busy: boolean;
  onChange(form: OpenTaskEditDraftViewModel): void;
  onSave(): void;
  onCancel(): void;
}

export function EditTaskForm({ form, busy, onChange, onSave, onCancel }: EditTaskFormProps) {
  return (
    <div className="edit-task-form">
      <input
        value={form.title}
        onChange={(event) => onChange({ ...form, title: event.target.value })}
        disabled={busy}
        aria-label="Edit task title"
      />
      <textarea
        value={form.summary}
        onChange={(event) => onChange({ ...form, summary: event.target.value })}
        disabled={busy}
        aria-label="Edit task summary"
        rows={3}
      />
      <div className="edit-grid">
        <select
          value={form.attentionState}
          onChange={(event) =>
            onChange({ ...form, attentionState: event.target.value as OpenTaskAttentionValue })
          }
          disabled={busy}
          aria-label="Edit attention state"
        >
          {attentionOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <select
          value={form.executionState}
          onChange={(event) =>
            onChange({ ...form, executionState: event.target.value as OpenTaskExecutionValue })
          }
          disabled={busy}
          aria-label="Edit execution state"
        >
          {executionOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <select
          value={form.priority}
          onChange={(event) =>
            onChange({ ...form, priority: event.target.value as OpenTaskPriorityValue })
          }
          disabled={busy}
          aria-label="Edit priority"
        >
          {priorityOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
      <div className="edit-actions">
        <button
          className="icon-button"
          type="button"
          onClick={onSave}
          disabled={busy || !form.title.trim() || !form.summary.trim()}
          title="Save task"
          aria-label="Save task"
        >
          <Check size={16} aria-hidden="true" />
        </button>
        <button
          className="icon-button"
          type="button"
          onClick={onCancel}
          disabled={busy}
          title="Cancel edit"
          aria-label="Cancel edit"
        >
          <X size={16} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
