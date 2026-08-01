import { fireEvent, render, screen } from '@testing-library/react';
import { recordedEpicProductDecisionSource } from '../../dev/productDecisions/recordedEpicProductDecisionSource';
import { EpicProductDecisionsPanel } from './EpicProductDecisionsPanel';

describe('EpicProductDecisionsPanel', () => {
  it('shows current policy and keeps a contrary recorded candidate in human review', async () => {
    render(
      <EpicProductDecisionsPanel
        epicId="epic-codex-runner-workspace"
        source={recordedEpicProductDecisionSource}
      />,
    );

    expect(await screen.findByRole('heading', { name: 'Stable workspace' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Evidence on record' })).toBeVisible();
    expect(screen.queryByText('Derived from')).not.toBeInTheDocument();
    expect(screen.getByText('Origin reference: layout-review-outcome-2026-07-29')).toBeVisible();
    expect(screen.getByText('Codebase review requested')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: /Review conflicts/ }));
    expect(screen.getByRole('heading', { name: 'Human judgment required' })).toBeVisible();
    expect(screen.getByText('Let the full Epic page scroll')).toBeVisible();
    expect(screen.getByText(/Acceptance and rejection are not implemented/)).toBeVisible();
  });
});
