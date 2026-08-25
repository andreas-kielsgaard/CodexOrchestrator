import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import type { WorktreeReviewClient } from '../../application/worktreeReview';
import { WorktreeReviewScreen } from './WorktreeReviewScreen';

describe('WorktreeReviewScreen', () => {
  it('presents repository readiness instead of hiding the product capability', async () => {
    const client = clientWith({
      readiness: async () => ({
        status: 'needsRepository',
        message: 'Choose the repository to review.',
      }),
    });

    render(<WorktreeReviewScreen client={client} />);

    expect(await screen.findByText('Choose a repository')).toBeVisible();
    expect(screen.getByText('Choose the repository to review.')).toBeVisible();
    expect(screen.getByRole('textbox', { name: 'Repository folder' })).toBeEnabled();
  });

  it('submits an explicit repository path and enters the launcher when it is ready', async () => {
    const selectRepository = vi.fn(async (repositoryRoot: string) => ({
      status: 'ready' as const,
      message: 'Ready.',
      repositoryRoot,
    }));
    const client = clientWith({
      readiness: async () => ({ status: 'needsRepository', message: 'Choose a repository.' }),
      selectRepository,
    });
    const user = userEvent.setup();
    render(<WorktreeReviewScreen client={client} />);

    const input = await screen.findByRole('textbox', { name: 'Repository folder' });
    await user.type(input, 'C:\\code\\product');
    await user.click(screen.getByRole('button', { name: 'Use repository' }));

    await waitFor(() => expect(selectRepository).toHaveBeenCalledWith('C:\\code\\product'));
    expect(await screen.findByRole('main', { name: 'Worktree review launcher' })).toBeVisible();
  });

  it('presents structured command errors without exposing their transport shape', async () => {
    const client = clientWith({
      readiness: async () => ({ status: 'needsRepository', message: 'Choose a repository.' }),
      selectRepository: async () =>
        Promise.reject({
          code: 'worktree_review_unavailable',
          message: 'That repository is unavailable.',
        }),
    });
    const user = userEvent.setup();
    render(<WorktreeReviewScreen client={client} />);

    await user.type(await screen.findByRole('textbox', { name: 'Repository folder' }), 'C:\\gone');
    await user.click(screen.getByRole('button', { name: 'Use repository' }));

    expect(await screen.findByRole('status')).toHaveTextContent('That repository is unavailable.');
  });
});

function clientWith(overrides: Partial<WorktreeReviewClient> = {}): WorktreeReviewClient {
  return {
    readiness: async () => ({ status: 'ready', message: 'Ready.' }),
    selectRepository: async (repositoryRoot) => ({
      status: 'ready',
      message: 'Ready.',
      repositoryRoot,
    }),
    listSources: async () => [],
    listRepositoryHistory: async () => [],
    listInstances: async () => [],
    listProgress: async () => [],
    ...overrides,
  } as WorktreeReviewClient;
}
