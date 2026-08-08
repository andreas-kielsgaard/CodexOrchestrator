import { fireEvent, render, screen } from '@testing-library/react';
import { IntegrationSettlementReviewHarness } from './IntegrationSettlementReviewHarness';

describe('IntegrationSettlementReviewHarness', () => {
  it('keeps only agent turns in Session stream and nests application and MCP records in activity', () => {
    const { container } = render(<IntegrationSettlementReviewHarness />);

    const session = screen.getByLabelText('Merged Agent Session passages');
    expect(session.textContent).toContain('Ada Lovelace');
    expect(session.textContent).toContain('Grace Hopper');
    expect(session.textContent).not.toContain('Codex Orchestrator');
    expect(session.textContent).not.toContain('read_work_unit_evidence');

    expect(container.querySelectorAll('.integration-activity > ol > li > button')).toHaveLength(3);
    expect(container.querySelectorAll('.integration-activity__application-stage')).toHaveLength(3);
    expect(
      container.querySelector('.integration-activity__application-chain')?.textContent,
    ).toContain('Application evidence');

    const evidenceRecords = screen.getByLabelText('Application evidence application records');
    expect(evidenceRecords.textContent).toContain('Codex Orchestrator');
    expect(evidenceRecords.textContent).toContain('read_work_unit_evidence');

    fireEvent.click(screen.getByRole('button', { name: /read_work_unit_evidence MCP/ }));
    expect(
      screen.getByText(/Called read_work_unit_evidence with the exact Work Unit/),
    ).toBeTruthy();
  });

  it('expands a read-only full turn inside the Session stream with steps and no composer', () => {
    render(<IntegrationSettlementReviewHarness />);

    const session = screen.getByLabelText('Merged Agent Session passages');
    const adaTurn = screen.getByRole('button', { name: /Ada Lovelace Worker/ });
    fireEvent.click(adaTurn);

    expect(session.textContent).toContain('Grace Hopper');
    expect(screen.getByLabelText('Ada Lovelace full turn output')).toBeTruthy();
    expect(screen.getByText('Read-only session turn')).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Full output' })).toBeTruthy();
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(screen.queryByRole('button', { name: /Back to session stream/ })).toBeNull();

    const step = screen.getByRole('button', { name: /Confirmed the bounded scope/ });
    expect(step.getAttribute('aria-expanded')).toBe('false');
    fireEvent.click(step);
    expect(step.getAttribute('aria-expanded')).toBe('true');
    expect(screen.getByText(/Matched the requested Work Unit/)).toBeTruthy();

    fireEvent.click(adaTurn);
    expect(screen.queryByLabelText('Ada Lovelace full turn output')).toBeNull();
  });

  it('opens Evidence with no detail and reveals file or test detail only after selection', () => {
    render(<IntegrationSettlementReviewHarness />);

    fireEvent.click(screen.getByRole('tab', { name: /Evidence 2/ }));

    expect(screen.getByRole('heading', { name: 'Work Unit evidence' })).toBeTruthy();
    expect(screen.getByText('Select evidence to inspect')).toBeTruthy();
    expect(screen.queryByRole('heading', { name: 'Accepted file change' })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: /src\/integration\/apply.ts/ }));
    expect(screen.getByRole('heading', { name: 'Accepted file change' })).toBeTruthy();
    expect(screen.getByLabelText('File comparison').textContent).toContain('recordSettlement');

    fireEvent.click(screen.getByRole('button', { name: /Focused integration test/ }));
    expect(screen.getByRole('heading', { name: 'Integration settlement validation' })).toBeTruthy();
    expect(screen.getByText('npm test -- --run src/integration/apply.test.ts')).toBeTruthy();
  });

  it('shows evidence-to-activity linkage on hover', () => {
    render(<IntegrationSettlementReviewHarness />);

    const implementerActivity = screen.getByRole('button', { name: /Implementer claim Claimed/ });
    fireEvent.click(screen.getByRole('tab', { name: /Evidence 2/ }));
    const fileEvidence = screen.getByRole('button', { name: /src\/integration\/apply.ts/ });

    fireEvent.mouseEnter(fileEvidence);
    expect(fileEvidence.textContent).toContain('Linked to Implementer claim');
    expect(implementerActivity.closest('li')?.classList.contains('is-related')).toBe(true);

    fireEvent.mouseLeave(fileEvidence);
    expect(implementerActivity.closest('li')?.classList.contains('is-related')).toBe(false);
  });

  it('keeps dated processing metadata and unavailable evidence explicit', () => {
    const { container } = render(<IntegrationSettlementReviewHarness />);

    expect(container.querySelector('.integration-session__turns time')).toBeNull();
    expect(screen.getAllByText('5 Aug 2026, 10:02:37 UTC').length).toBeGreaterThan(0);
    expect(screen.getAllByText('1 min 22 sec').length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('tab', { name: /Evidence 2/ }));
    expect(
      (
        screen.getByRole('button', {
          name: /Integration manifest Evidence unavailable/,
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
  });
});
