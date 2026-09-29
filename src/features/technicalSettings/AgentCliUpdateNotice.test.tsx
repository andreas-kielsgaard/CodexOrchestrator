import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { AgentCliUpdateClient } from '../../application/agentCliUpdates';
import { AgentCliUpdateNotice } from './AgentCliUpdateNotice';

describe('AgentCliUpdateNotice', () => {
  it('shows background progress and package-provided update notes', async () => {
    const client: AgentCliUpdateClient = {
      overview: vi.fn(async () => ({
        providers: [
          {
            provider: 'codex',
            label: 'Codex CLI',
            phase: 'updated' as const,
            channel: 'Stable',
            previousVersion: 'codex-cli 1.0.0',
            installedVersion: 'codex-cli 1.1.0',
            checkedAt: '2026-09-29T12:00:00Z',
            summary: 'Codex CLI updated from codex-cli 1.0.0 to codex-cli 1.1.0.',
            notes: ['Release notes', '- Faster startup'],
          },
        ],
      })),
    };

    render(<AgentCliUpdateNotice client={client} />);

    expect(await screen.findByText('Updated')).toBeVisible();
    expect(screen.getByText(/updated from codex-cli 1\.0\.0/i)).toBeVisible();
    fireEvent.click(screen.getByText('Update notes'));
    expect(screen.getByText(/Faster startup/)).toBeVisible();
  });
});
