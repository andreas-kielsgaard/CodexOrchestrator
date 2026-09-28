import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WorkflowNavigation } from './WorkflowNavigation';

it('keeps instance creation available when designs exist but none are active', async () => {
  const user = userEvent.setup();
  const create = vi.fn();
  render(
    <WorkflowNavigation
      summaries={[
        {
          recipeId: 'draft',
          name: 'Draft',
          draftRevision: 1,
          activeRevision: null,
          updatedAt: '2026-09-24',
        },
      ]}
      instances={[]}
      selectedRecipeId="draft"
      selectedInstanceId={null}
      busy={false}
      canCreateInstance
      onTabChange={vi.fn()}
      onCreateDesign={vi.fn()}
      onCreateInstance={create}
      onOpenRecipe={vi.fn()}
      onOpenInstance={vi.fn()}
    />,
  );
  expect(screen.queryByRole('button', { name: 'Reload workflows' })).toBeNull();
  await user.click(screen.getByRole('tab', { name: 'Instances' }));
  const button = screen.getByRole('button', { name: 'Create instance' });
  expect(button).toBeEnabled();
  await user.click(button);
  expect(create).toHaveBeenCalled();
});
