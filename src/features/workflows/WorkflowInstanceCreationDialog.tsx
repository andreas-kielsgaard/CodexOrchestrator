import { X } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentType,
  type KeyboardEvent,
} from 'react';
import type {
  RepoBranchWorktreeTargetSelectorProps,
  ResolvedRepoBranchWorktreeTarget,
} from '../../application/worktreeTargets';
import type { CreateWorkflowInstanceInput, WorkflowTypeSummary } from '../../application/workflows';
import './workflowInstanceCreationDialog.css';

export interface WorkflowInstanceCreationDialogProps {
  readonly workflowTypes: readonly WorkflowTypeSummary[];
  readonly TargetSelector: ComponentType<RepoBranchWorktreeTargetSelectorProps>;
  onSubmit(input: CreateWorkflowInstanceInput): Promise<unknown> | unknown;
  onClose(): void;
}

export function WorkflowInstanceCreationDialog({
  workflowTypes,
  TargetSelector,
  onSubmit,
  onClose,
}: WorkflowInstanceCreationDialogProps) {
  const activeWorkflowTypes = useMemo(
    () => workflowTypes.filter((workflowType) => workflowType.activeRecipeId !== null),
    [workflowTypes],
  );
  const [workflowTypeId, setWorkflowTypeId] = useState('');
  const [name, setName] = useState('');
  const [target, setTarget] = useState<ResolvedRepoBranchWorktreeTarget | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLFormElement>(null);
  const typeSelectRef = useRef<HTMLSelectElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const submittingRef = useRef(false);
  const nameValue = name.trim();
  const canSubmit = workflowTypeId !== '' && nameValue !== '' && target !== null && !pending;

  useEffect(() => {
    const previouslyFocused =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    typeSelectRef.current?.focus();
    if (document.activeElement !== typeSelectRef.current) closeButtonRef.current?.focus();
    return () => {
      if (previouslyFocused?.isConnected) previouslyFocused.focus();
    };
  }, []);

  const close = () => {
    if (!submittingRef.current) onClose();
  };

  const submit = async () => {
    if (!canSubmit || submittingRef.current || !target) return;
    submittingRef.current = true;
    setPending(true);
    setError(null);
    try {
      await onSubmit({ workflowTypeId, name: nameValue, target });
    } catch (cause) {
      submittingRef.current = false;
      setPending(false);
      setError(errorMessage(cause));
    }
  };

  return (
    <div
      className="workflow-instance-creation-dialog__backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) close();
      }}
    >
      <form
        ref={dialogRef}
        className="workflow-instance-creation-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="workflow-instance-creation-dialog-title"
        aria-describedby="workflow-instance-creation-dialog-description"
        onKeyDown={(event) => containFocus(event, dialogRef.current, close)}
        onSubmit={(event) => {
          event.preventDefault();
          void submit();
        }}
      >
        <header>
          <div>
            <p className="workflow-instance-creation-dialog__eyebrow">New Workflow instance</p>
            <h2 id="workflow-instance-creation-dialog-title">Create Workflow instance</h2>
          </div>
          <button
            ref={closeButtonRef}
            className="workflow-instance-creation-dialog__close"
            type="button"
            aria-label="Close Workflow instance creation"
            disabled={pending}
            onClick={close}
          >
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <p
          className="workflow-instance-creation-dialog__description"
          id="workflow-instance-creation-dialog-description"
        >
          Choose an activated Workflow and the repository branch where its Agent Sessions will work.
        </p>

        <label className="workflow-instance-creation-dialog__field">
          Workflow type
          <select
            ref={typeSelectRef}
            required
            value={workflowTypeId}
            disabled={pending || activeWorkflowTypes.length === 0}
            onChange={(event) => {
              setWorkflowTypeId(event.currentTarget.value);
              setError(null);
            }}
          >
            <option value="">Select an activated Workflow</option>
            {activeWorkflowTypes.map((workflowType) => (
              <option key={workflowType.id} value={workflowType.id}>
                {workflowType.name}
              </option>
            ))}
          </select>
        </label>

        {activeWorkflowTypes.length === 0 ? (
          <p className="workflow-instance-creation-dialog__note">
            Activate a Workflow type before creating an instance.
          </p>
        ) : null}

        <label className="workflow-instance-creation-dialog__field">
          Instance name
          <input
            required
            value={name}
            disabled={pending}
            placeholder="Name this Workflow"
            onChange={(event) => {
              setName(event.currentTarget.value);
              setError(null);
            }}
          />
        </label>

        <div className="workflow-instance-creation-dialog__field">
          <label htmlFor="workflow-instance-target">Repository and branch</label>
          <TargetSelector
            id="workflow-instance-target"
            value={target}
            disabled={pending}
            onChange={(selectedTarget) => {
              setTarget(selectedTarget);
              setError(null);
            }}
          />
        </div>

        {error ? (
          <p className="workflow-instance-creation-dialog__error" role="alert">
            {error}
          </p>
        ) : null}

        <footer>
          <button type="button" disabled={pending} onClick={close}>
            Cancel
          </button>
          <button
            className="workflow-instance-creation-dialog__submit"
            type="submit"
            disabled={!canSubmit}
          >
            {pending ? 'Creating…' : 'Create Workflow instance'}
          </button>
        </footer>
      </form>
    </div>
  );
}

function containFocus(
  event: KeyboardEvent<HTMLElement>,
  dialog: HTMLElement | null,
  onClose: () => void,
) {
  if (event.key === 'Escape') {
    event.preventDefault();
    event.stopPropagation();
    onClose();
    return;
  }
  if (event.key !== 'Tab' || !dialog) return;

  const focusable = [...dialog.querySelectorAll<HTMLElement>(focusableSelector)];
  if (focusable.length === 0) {
    event.preventDefault();
    dialog.focus();
    return;
  }
  const first = focusable[0];
  const last = focusable.at(-1)!;
  if (event.shiftKey && (document.activeElement === first || document.activeElement === dialog)) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

const focusableSelector = [
  'button:not([disabled])',
  'a[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
