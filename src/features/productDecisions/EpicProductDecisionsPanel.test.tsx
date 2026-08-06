import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { recordedEpicProductDecisionSource } from '../../dev/productDecisions/recordedEpicProductDecisionSource';
import { EpicProductDecisionsPanel } from './EpicProductDecisionsPanel';

describe('EpicProductDecisionsPanel', () => {
  it('starts each fresh/direct route collapsed and clears stale prepared review state', async () => {
    const firstSource = { ...recordedEpicProductDecisionSource };
    const { rerender } = render(
      <EpicProductDecisionsPanel epicId="epic-codex-runner-workspace" source={firstSource} />,
    );
    expect(
      (await screen.findAllByText('Intent and evidence')).every(
        (summary) => !summary.closest('details')?.hasAttribute('open'),
      ),
    ).toBe(true);
    const reviewToggle = screen.getByRole('button', { name: /Review recorded changes/ });
    expect(reviewToggle).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(reviewToggle);
    expect(reviewToggle).toHaveAttribute('aria-expanded', 'true');

    rerender(
      <EpicProductDecisionsPanel
        epicId="epic-codex-runner-workspace"
        source={{ ...recordedEpicProductDecisionSource }}
      />,
    );
    expect(await screen.findByRole('button', { name: /Review recorded changes/ })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(
      (await screen.findAllByText('Intent and evidence')).every(
        (summary) => !summary.closest('details')?.hasAttribute('open'),
      ),
    ).toBe(true);
  });

  it('offers only resolved evidence navigation and leaves unsupported evidence visibly unavailable', async () => {
    const onOpenEvidence = vi.fn();
    render(
      <EpicProductDecisionsPanel
        epicId="epic-codex-runner-workspace"
        source={recordedEpicProductDecisionSource}
        onOpenEvidence={onOpenEvidence}
      />,
    );
    fireEvent.click((await screen.findAllByText('Intent and evidence'))[0]!);
    expect(
      screen.getAllByRole('button', { name: 'Open supporting Agent Session passage' })[0],
    ).toBeVisible();
    expect(screen.getAllByText('Exact supporting evidence is unavailable.').length).toBeGreaterThan(
      0,
    );
    fireEvent.click(
      screen.getAllByRole('button', { name: 'Open supporting Agent Session passage' })[0]!,
    );
    expect(onOpenEvidence).toHaveBeenCalledWith(
      expect.objectContaining({ evidenceId: 'evidence-agent-session-layout-passage' }),
    );

    fireEvent.click(screen.getByRole('button', { name: /Review recorded changes/ }));
    const review = screen.getByRole('region', { name: 'Recorded Product Decision changes' });
    expect(within(review).getByText('Exact supporting evidence is unavailable.')).toBeVisible();
    expect(
      within(review).queryByRole('button', { name: 'Open supporting Agent Session passage' }),
    ).toBeNull();
  });
});
