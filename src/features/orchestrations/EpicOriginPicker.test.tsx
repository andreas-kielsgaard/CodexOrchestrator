import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  EpicOriginProject,
  EpicOriginProjectClient,
} from '../../application/epicOriginProject';
import { EpicOriginPicker } from './EpicOriginPicker';

const project: EpicOriginProject = {
  name: 'Codex Orchestrator',
  path: 'C:\\Code\\Codex Orchestrator',
  gitDetected: true,
  repositoryRoot: 'C:\\Code\\Codex Orchestrator',
  branches: [
    {
      name: 'main',
      revision: '111111111111',
      relationship: 'related',
      ahead: 0,
      behind: 0,
      forkRevision: '111111111111',
      isCurrent: false,
      isBaseline: true,
    },
    {
      name: 'codex/epic-project-selection',
      revision: '222222222222',
      parentName: 'main',
      relationship: 'related',
      ahead: 4,
      behind: 0,
      forkRevision: '111111111111',
      isCurrent: true,
      isBaseline: false,
    },
  ],
};

describe('EpicOriginPicker', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('requires an explicit origin selection and persists it independently of the workflow', async () => {
    const client: EpicOriginProjectClient = {
      chooseProject: vi.fn().mockResolvedValue(project),
      inspectProject: vi.fn().mockResolvedValue(project),
    };
    const user = userEvent.setup();
    render(<EpicOriginPicker client={client} />);

    await user.click(screen.getByRole('button', { name: 'Choose project folder' }));
    expect(await screen.findByText('Git repository detected')).toBeVisible();
    expect(screen.getByText('No origin selected')).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'Select branch' }));
    const dialog = screen.getByRole('dialog', { name: 'Select origin branch' });
    expect(within(dialog).getByRole('button', { name: 'Use as origin' })).toBeDisabled();
    await user.click(
      within(dialog).getByRole('button', {
        name: /codex\/epic-project-selection 222222222222 · current checkout/,
      }),
    );
    expect(
      within(dialog).getByText('Development happens in descendant worktrees, not on this branch.'),
    ).toBeVisible();
    await user.click(within(dialog).getByRole('button', { name: 'Use as origin' }));

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByText('codex/epic-project-selection')).toBeVisible();
    expect(window.localStorage.getItem('codex-orchestrator:epic-origin-selection:v1')).toContain(
      'codex/epic-project-selection',
    );
  });

  it('restores a retained project and closes the branch dialog with Escape', async () => {
    window.localStorage.setItem(
      'codex-orchestrator:epic-origin-selection:v1',
      JSON.stringify({ projectPath: project.path, branchName: 'main' }),
    );
    const client: EpicOriginProjectClient = {
      chooseProject: vi.fn().mockResolvedValue(project),
      inspectProject: vi.fn().mockResolvedValue(project),
    };
    render(<EpicOriginPicker client={client} />);

    await waitFor(() => expect(client.inspectProject).toHaveBeenCalledWith(project.path));
    expect(await screen.findByText('main')).toBeVisible();
    const trigger = screen.getByRole('button', { name: 'Select branch' });
    await userEvent.click(trigger);
    fireEvent.keyDown(document, { key: 'Escape' });

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await waitFor(() => expect(trigger).toHaveFocus());
  });
});
