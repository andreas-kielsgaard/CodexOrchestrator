import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RecipeInstanceCreationDialog } from './RecipeInstanceCreationDialog';

it('opens an actionable empty modal when no active designs exist', async () => {
  const user = userEvent.setup();
  const close = vi.fn();
  const TargetSelector = () => <div>Target selector</div>;
  render(
    <RecipeInstanceCreationDialog
      recipes={[
        {
          recipeId: 'draft',
          name: 'Draft',
          draftRevision: 1,
          activeRevision: null,
          updatedAt: '2026-09-24',
        },
      ]}
      TargetSelector={TargetSelector}
      onSubmit={vi.fn()}
      onClose={close}
    />,
  );

  expect(screen.getByRole('dialog')).toHaveTextContent('No active designs found');
  expect(screen.queryByText('Target selector')).toBeNull();
  await user.click(screen.getByRole('button', { name: 'Close' }));
  expect(close).toHaveBeenCalled();
});
