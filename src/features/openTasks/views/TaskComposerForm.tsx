import { Plus } from 'lucide-react';
import type { FormEvent } from 'react';
import type {
  TaskComposerFormViewModel,
  TaskComposerProjectOption,
  TaskComposerWorktreeOption,
} from '../viewModels/taskFormViewModel';
import { attentionOptions, type AttentionOptionValue } from '../viewModels/taskOptions';

export interface TaskComposerFormProps {
  form: TaskComposerFormViewModel;
  projects: TaskComposerProjectOption[];
  worktrees: TaskComposerWorktreeOption[];
  busy: boolean;
  canCreate: boolean;
  onSubmit(event: FormEvent<HTMLFormElement>): void;
  onSelectProject(projectId: string): void;
  onSelectWorktree(worktreeId: string): void;
  onChange(patch: Partial<TaskComposerFormViewModel>): void;
}

export function TaskComposerForm({
  form,
  projects,
  worktrees,
  busy,
  canCreate,
  onSubmit,
  onSelectProject,
  onSelectWorktree,
  onChange,
}: TaskComposerFormProps) {
  return (
    <form className="task-composer" onSubmit={onSubmit} aria-label="Create open task">
      <select
        value={form.projectId}
        onChange={(event) => onSelectProject(event.target.value)}
        disabled={projects.length === 0 || busy}
        aria-label="Project"
      >
        {projects.length === 0 ? (
          <option value="">No persisted projects</option>
        ) : (
          projects.map((project) => (
            <option key={project.id} value={project.id}>
              {project.label}
            </option>
          ))
        )}
      </select>
      <select
        value={form.worktreeId}
        onChange={(event) => {
          onSelectWorktree(event.target.value);
        }}
        disabled={worktrees.length === 0 || busy}
        aria-label="Worktree"
      >
        {worktrees.length === 0 ? (
          <option value="">No registered worktrees</option>
        ) : (
          <>
            <option value="">No worktree</option>
            {worktrees.map((worktree) => (
              <option key={worktree.id} value={worktree.id}>
                {worktree.label}
              </option>
            ))}
          </>
        )}
      </select>
      <input
        value={form.title}
        onChange={(event) => onChange({ title: event.target.value })}
        disabled={!canCreate}
        placeholder="Task title"
        aria-label="Task title"
      />
      <input
        value={form.summary}
        onChange={(event) => onChange({ summary: event.target.value })}
        disabled={!canCreate}
        placeholder="Summary"
        aria-label="Task summary"
      />
      <select
        value={form.attentionState}
        onChange={(event) =>
          onChange({
            attentionState: event.target.value as AttentionOptionValue,
          })
        }
        disabled={!canCreate}
        aria-label="Attention state"
      >
        {attentionOptions.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      <button
        className="primary-action"
        type="submit"
        disabled={!canCreate || !form.title.trim() || !form.summary.trim()}
      >
        <Plus size={17} aria-hidden="true" />
        Create
      </button>
    </form>
  );
}
