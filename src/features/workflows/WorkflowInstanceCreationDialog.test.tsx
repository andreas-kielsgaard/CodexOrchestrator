import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { vi } from 'vitest';
import type {
  RepoBranchWorktreeTargetSelectorProps,
  ResolvedRepoBranchWorktreeTarget,
} from '../../application/worktreeTargets';
import type { WorkflowTypeSummary } from '../../application/workflows';
import { WorkflowInstanceCreationDialog } from './WorkflowInstanceCreationDialog';

describe('WorkflowInstanceCreationDialog', () => {
  it('offers only activated Workflow types and requires the name and target', () => {
    const onSubmit = vi.fn();
    render(
      <WorkflowInstanceCreationDialog
        workflowTypes={[activeWorkflowType, inactiveWorkflowType]}
        TargetSelector={TestTargetSelector}
        onSubmit={onSubmit}
        onClose={() => undefined}
      />,
    );

    const typeSelect = screen.getByLabelText('Workflow type');
    expect(typeSelect).toHaveFocus();
    expect(within(typeSelect).getByRole('option', { name: 'Code review' })).toBeVisible();
    expect(within(typeSelect).queryByRole('option', { name: 'Draft only' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Create Workflow instance' })).toBeDisabled();

    fireEvent.change(typeSelect, { target: { value: activeWorkflowType.id } });
    fireEvent.change(screen.getByLabelText('Instance name'), {
      target: { value: '   ' },
    });
    fireEvent.click(screen.getByLabelText('Repository and branch'));
    expect(screen.getByRole('button', { name: 'Create Workflow instance' })).toBeDisabled();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('forwards the exact injected target and trimmed name once while creation is pending', async () => {
    const pending = deferred<void>();
    const onSubmit = vi.fn(() => pending.promise);
    render(
      <WorkflowInstanceCreationDialog
        workflowTypes={[activeWorkflowType, inactiveWorkflowType]}
        TargetSelector={TestTargetSelector}
        onSubmit={onSubmit}
        onClose={() => undefined}
      />,
    );

    fireEvent.change(screen.getByLabelText('Workflow type'), {
      target: { value: activeWorkflowType.id },
    });
    fireEvent.change(screen.getByLabelText('Instance name'), {
      target: { value: '  Review checkout  ' },
    });
    fireEvent.click(screen.getByLabelText('Repository and branch'));
    fireEvent.click(screen.getByRole('button', { name: 'Create Workflow instance' }));
    fireEvent.submit(screen.getByRole('dialog'));

    expect(onSubmit).toHaveBeenCalledOnce();
    expect(onSubmit).toHaveBeenCalledWith({
      workflowTypeId: activeWorkflowType.id,
      name: 'Review checkout',
      target,
    });
    expect(screen.getByRole('button', { name: 'Creating…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled();

    await act(async () => pending.resolve());
  });

  it('surfaces an ordinary creation error and allows another submission', async () => {
    const onSubmit = vi
      .fn<(input: unknown) => Promise<void>>()
      .mockRejectedValueOnce(new Error('Instance could not be recorded.'))
      .mockResolvedValueOnce();
    render(
      <WorkflowInstanceCreationDialog
        workflowTypes={[activeWorkflowType]}
        TargetSelector={TestTargetSelector}
        onSubmit={onSubmit}
        onClose={() => undefined}
      />,
    );

    completeForm();
    fireEvent.click(screen.getByRole('button', { name: 'Create Workflow instance' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Instance could not be recorded.');
    const submit = screen.getByRole('button', { name: 'Create Workflow instance' });
    expect(submit).toBeEnabled();
    fireEvent.click(submit);
    expect(onSubmit).toHaveBeenCalledTimes(2);
  });

  it('supports Escape and backdrop dismissal without treating dialog interaction as backdrop input', () => {
    const onClose = vi.fn();
    const { unmount } = render(
      <WorkflowInstanceCreationDialog
        workflowTypes={[activeWorkflowType]}
        TargetSelector={TestTargetSelector}
        onSubmit={() => undefined}
        onClose={onClose}
      />,
    );

    const dialog = screen.getByRole('dialog');
    fireEvent.mouseDown(dialog);
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledOnce();

    fireEvent.mouseDown(document.querySelector('.workflow-instance-creation-dialog__backdrop')!);
    expect(onClose).toHaveBeenCalledTimes(2);
    unmount();
  });

  it('explains when no activated Workflow type is available', () => {
    render(
      <WorkflowInstanceCreationDialog
        workflowTypes={[inactiveWorkflowType]}
        TargetSelector={TestTargetSelector}
        onSubmit={() => undefined}
        onClose={() => undefined}
      />,
    );

    expect(screen.getByText('Activate a Workflow type before creating an instance.')).toBeVisible();
    expect(screen.getByLabelText('Workflow type')).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Close Workflow instance creation' })).toHaveFocus();
  });
});

function TestTargetSelector({
  id,
  value,
  disabled,
  onChange,
}: RepoBranchWorktreeTargetSelectorProps) {
  return (
    <button type="button" id={id} disabled={disabled} onClick={() => onChange(target)}>
      {value ? 'Review repo · feature/workflow' : 'Use review repo and feature branch'}
    </button>
  );
}

function completeForm() {
  fireEvent.change(screen.getByLabelText('Workflow type'), {
    target: { value: activeWorkflowType.id },
  });
  fireEvent.change(screen.getByLabelText('Instance name'), {
    target: { value: 'Review checkout' },
  });
  fireEvent.click(screen.getByLabelText('Repository and branch'));
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

const activeWorkflowType: WorkflowTypeSummary = {
  id: 'workflow-code-review',
  name: 'Code review',
  activeRecipeId: 'recipe-1',
  editedElementCount: 0,
  createdAt: '2026-08-25T09:00:00.000Z',
  updatedAt: '2026-08-25T10:00:00.000Z',
};

const inactiveWorkflowType: WorkflowTypeSummary = {
  id: 'workflow-draft',
  name: 'Draft only',
  activeRecipeId: null,
  editedElementCount: 2,
  createdAt: '2026-08-25T09:00:00.000Z',
  updatedAt: '2026-08-25T10:00:00.000Z',
};

const target: ResolvedRepoBranchWorktreeTarget = {
  repository: {
    id: 'repo-1',
    name: 'Review repo',
    rootPath: 'C:\\repos\\review',
  },
  branch: { id: 'branch-1', name: 'feature/workflow' },
  worktree: { id: 'worktree-1', path: 'C:\\worktrees\\review' },
};
