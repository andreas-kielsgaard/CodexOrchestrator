import { useEffect, useRef, useState, type ComponentType } from 'react';
import type { WorkflowRecipeSummaryDto } from '../../application/workflowAuthoring';
import type {
  RepoBranchWorktreeTargetSelectorProps,
  ResolvedRepoBranchWorktreeTarget,
} from '../../application/worktreeTargets';
import type { WorkflowInstanceClient } from '../../application/workflowInstances';
import '../workflows/workflowInstanceCreationDialog.css';

export function RecipeInstanceCreationDialog({
  recipes,
  TargetSelector,
  onSubmit,
  onClose,
}: {
  readonly recipes: readonly WorkflowRecipeSummaryDto[];
  readonly TargetSelector: ComponentType<RepoBranchWorktreeTargetSelectorProps>;
  onSubmit(input: Parameters<WorkflowInstanceClient['create']>[0]): Promise<void>;
  onClose(): void;
}) {
  const active = recipes.filter((recipe) => recipe.activeRevision !== null);
  const [recipeId, setRecipeId] = useState(active[0]?.recipeId ?? '');
  const [name, setName] = useState('');
  const [target, setTarget] = useState<ResolvedRepoBranchWorktreeTarget | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialog = useRef<HTMLFormElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    dialog.current?.querySelector<HTMLSelectElement>('select')?.focus();
    return () => previous?.focus();
  }, []);
  return (
    <div className="workflow-instance-creation-dialog__backdrop">
      <form
        className="workflow-instance-creation-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Create Workflow instance"
        ref={dialog}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && !pending) {
            event.preventDefault();
            onClose();
          }
          if (event.key !== 'Tab') return;
          const controls = [
            ...(dialog.current?.querySelectorAll<HTMLElement>(
              'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex="0"]',
            ) ?? []),
          ];
          const first = controls[0];
          const last = controls.at(-1);
          if (event.shiftKey && document.activeElement === first) {
            event.preventDefault();
            last?.focus();
          }
          if (!event.shiftKey && document.activeElement === last) {
            event.preventDefault();
            first?.focus();
          }
        }}
        onSubmit={(event) => {
          event.preventDefault();
          const recipe = active.find((item) => item.recipeId === recipeId);
          if (pending || !recipe?.activeRevision || !target || !name.trim()) return;
          setPending(true);
          setError(null);
          void onSubmit({
            recipeId,
            expectedRevision: recipe.activeRevision,
            name: name.trim(),
            target,
          }).catch((cause) => {
            setError(String(cause));
            setPending(false);
          });
        }}
      >
        <header>
          <h2>Create Workflow instance</h2>
          <button
            type="button"
            disabled={pending}
            onClick={onClose}
            aria-label="Close instance creation"
          >
            Close
          </button>
        </header>
        <p>Choose a saved Workflow and worktree. Creating the instance does not start an agent.</p>
        <label className="workflow-instance-creation-dialog__field">
          Workflow
          <select
            value={recipeId}
            onChange={(event) => setRecipeId(event.currentTarget.value)}
            disabled={pending}
          >
            {active.map((recipe) => (
              <option key={recipe.recipeId} value={recipe.recipeId}>
                {recipe.name} · v{recipe.activeRevision}
              </option>
            ))}
          </select>
        </label>
        {!active.length ? <p>Activate a Workflow before creating an instance.</p> : null}
        <label className="workflow-instance-creation-dialog__field">
          Instance name
          <input
            required
            value={name}
            disabled={pending}
            onChange={(event) => setName(event.currentTarget.value)}
          />
        </label>
        <TargetSelector value={target} onChange={setTarget} disabled={pending} />
        {error ? <p role="alert">{error}</p> : null}
        <footer>
          <button type="button" disabled={pending} onClick={onClose}>
            Cancel
          </button>
          <button type="submit" disabled={pending || !recipeId || !name.trim() || !target}>
            {pending ? 'Creating…' : 'Create instance'}
          </button>
        </footer>
      </form>
    </div>
  );
}
