import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { recordedPresentationAdjunct } from '../../dev/orchestrationSection/recordedPresentationAdjunct';
import { recordedEpicProductDecisionSource } from '../../dev/productDecisions/recordedEpicProductDecisionSource';
import { EpicProductDecisionsPanel } from './EpicProductDecisionsPanel';

const productDecisionStyles = readFileSync(
  'src/features/productDecisions/epicProductDecisions.css',
  'utf8',
);
const epicDetailStyles = readFileSync('src/features/orchestrations/styles/epicDetail.css', 'utf8');

describe('EpicProductDecisionsPanel', () => {
  it('keeps the explicit hierarchy simple and progressively discloses review machinery', async () => {
    const epicRunnerSession = recordedPresentationAdjunct.epic?.epicRunnerSession;
    if (!epicRunnerSession) throw new Error('Expected the recorded Epic Runner presentation.');
    render(
      <EpicProductDecisionsPanel
        epicId="epic-codex-runner-workspace"
        source={recordedEpicProductDecisionSource}
        citedAgentSessions={[epicRunnerSession]}
      />,
    );

    const current = await screen.findByRole('region', { name: 'Current decisions' });
    expect(within(current).getByRole('heading', { name: 'Contained Epic detail' })).toBeVisible();
    expect(within(current).getByText(/Expands/)).toHaveTextContent('Expands Contained Epic detail');
    expect(screen.queryByText('Changes needing human review')).not.toBeInTheDocument();
    const reviewToggle = screen.getByRole('button', { name: /Review recorded changes/ });
    expect(reviewToggle).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(reviewToggle);
    expect(screen.getByRole('heading', { name: 'Changes needing human review' })).toBeVisible();
    expect(screen.getByText('Let the full Epic page scroll')).toBeVisible();
    expect(screen.getByText(/Acceptance and rejection are not implemented/)).toBeVisible();
  });

  it('opens a read-only Agent Session popup focused on the exact typed citation', async () => {
    const epicRunnerSession = recordedPresentationAdjunct.epic?.epicRunnerSession;
    if (!epicRunnerSession) throw new Error('Expected the recorded Epic Runner presentation.');
    render(
      <EpicProductDecisionsPanel
        epicId="epic-codex-runner-workspace"
        source={recordedEpicProductDecisionSource}
        citedAgentSessions={[epicRunnerSession]}
      />,
    );

    const details = (await screen.findAllByText('Intent and evidence'))[0];
    fireEvent.click(details);
    expect(screen.getAllByRole('heading', { name: 'Evidence on record' })[0]).toBeVisible();
    expect(screen.queryByText('Derived from')).not.toBeInTheDocument();
    expect(
      screen.getByText('Origin reference: recorded-epic-runner-manual-continuation-ready'),
    ).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Open cited conversation passage' }));
    const dialog = screen.getByRole('dialog', { name: /Conversation citation/ });
    expect(
      within(dialog).getByText(
        'For Epic detail, keep the menu and outer frame fixed; scroll only a contained region whose content exceeds its bounds.',
      ),
    ).toBeVisible();
    expect(within(dialog).queryByText('Recorded development display')).not.toBeInTheDocument();
    expect(within(dialog).queryByRole('textbox')).not.toBeInTheDocument();
    await waitFor(() =>
      expect(
        dialog.querySelector(
          '[data-invocation-id="recorded-epic-runner-manual-continuation-ready-recorded-turn"]',
        ),
      ).toHaveFocus(),
    );
  });

  it('keeps the distinct Epic view and citation presentation responsive', () => {
    expect(epicDetailStyles).toMatch(
      /@media \(max-width: 720px\)[\s\S]*\.epic-product-decisions-view__menu\s*\{[\s\S]*grid-template-columns: auto minmax\(0, 1fr\);/,
    );
    expect(productDecisionStyles).toMatch(
      /@media \(max-width: 720px\)[\s\S]*\.product-decisions__sources li\s*\{[\s\S]*grid-template-columns: minmax\(0, 1fr\);/,
    );
    expect(productDecisionStyles).toMatch(
      /@media \(max-width: 720px\)[\s\S]*\.product-decisions__citation-backdrop\s*\{[\s\S]*align-items: end;/,
    );
  });
});
