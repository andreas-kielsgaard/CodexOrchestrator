import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type {
  ProductDecisionClient,
  ProductDecisionCurrent,
  ProductDecisionVersion,
} from '../../application/productDecisions';
import { ProductiveProductDecisionsPanel } from './ProductiveProductDecisionsPanel';

const destination = {
  kind: 'agent_session_passage' as const,
  sessionId: 'session-1',
  invocationId: 'invocation-1',
  passage: { kind: 'submitted_input' as const },
};

function version(number: number, statement = `Statement ${number}`): ProductDecisionVersion {
  return {
    versionId: `version-${number}`,
    decisionId: 'decision-1',
    epicId: 'epic-1',
    version: number,
    title: `Title ${number}`,
    statement,
    intent: `Intent ${number}`,
    acceptanceProvenance: {
      kind: 'manual_human_application',
      humanInteractionOrigin: { kind: 'human_interaction', opaqueId: `human-${number}` },
    },
    currentActionableEvidence: [
      {
        evidenceId: 'current-evidence',
        originReference: { kind: 'agent_session_completed', opaqueId: 'agent-origin' },
        destination,
      },
    ],
    historicalUnresolvedEvidence: [
      {
        evidenceId: 'historical-evidence',
        originReference: { kind: 'work_unit_approved', opaqueId: 'old-work-unit' },
        label: 'Old retained evidence',
      },
    ],
    acceptedAt: `2026-08-06T10:0${number}:00.000Z`,
  };
}

function current(currentVersion = version(1)): ProductDecisionCurrent {
  return {
    decisionId: 'decision-1',
    epicId: 'epic-1',
    currentVersion,
    applicationState: 'not_applied',
  };
}

function clientFor(
  overrides: Partial<{
    loadCurrent: ProductDecisionClient['loadCurrent'];
    loadHistory: ProductDecisionClient['loadHistory'];
    acceptVersion: ProductDecisionClient['acceptVersion'];
  }> = {},
): ProductDecisionClient {
  return {
    loadCurrent: overrides.loadCurrent ?? vi.fn().mockResolvedValue([current()]),
    loadHistory: overrides.loadHistory ?? vi.fn().mockResolvedValue([version(1)]),
    acceptVersion: overrides.acceptVersion ?? vi.fn().mockResolvedValue(version(2, 'Corrected')),
  };
}

describe('ProductiveProductDecisionsPanel', () => {
  it('keeps edits tentative until explicit acceptance and cancel discards them', async () => {
    const client = clientFor();
    render(<ProductiveProductDecisionsPanel epicId="epic-1" client={client} />);
    await screen.findByText('Title 1');

    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Tentative title' } });
    expect(client.acceptVersion).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByDisplayValue('Tentative title')).toBeNull();
    expect(screen.getByText('Title 1')).toBeVisible();
  });

  it('accepts against the displayed version, retains evidence, and updates only after success', async () => {
    const onOpenEvidence = vi.fn();
    const onPublish = vi.fn();
    const client = clientFor();
    render(
      <ProductiveProductDecisionsPanel
        epicId="epic-1"
        client={client}
        onOpenEvidence={onOpenEvidence}
        onPublish={onPublish}
      />,
    );
    await screen.findByText('Title 1');

    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByLabelText('Statement'), { target: { value: 'Corrected' } });
    fireEvent.click(screen.getByRole('button', { name: 'Accept correction' }));

    await screen.findByText('Title 2');
    expect(client.acceptVersion).toHaveBeenCalledWith(
      expect.objectContaining({
        epicId: 'epic-1',
        decisionId: 'decision-1',
        expectedCurrentVersion: 1,
        statement: 'Corrected',
        acceptanceProvenance: {
          kind: 'manual_human_application',
          humanInteractionOrigin: {
            kind: 'human_interaction',
            opaqueId: expect.any(String),
          },
        },
        currentActionableEvidence: expect.arrayContaining([
          expect.objectContaining({ evidenceId: 'current-evidence' }),
        ]),
        historicalUnresolvedEvidence: expect.arrayContaining([
          expect.objectContaining({ evidenceId: 'historical-evidence' }),
        ]),
      }),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Open supporting Agent Session passage' }));
    expect(onOpenEvidence).toHaveBeenCalledWith(destination);
    fireEvent.click(screen.getByRole('button', { name: 'Publish' }));
    expect(onPublish).toHaveBeenCalledWith(
      expect.objectContaining({ decisionId: 'decision-1', versionId: 'version-2', version: 2 }),
    );
    expect(screen.getAllByText('Not applied').length).toBeGreaterThan(0);
    expect(screen.getByText(/Historical unresolved evidence/)).toBeVisible();
  });

  it('preserves draft input after stale or idempotency conflict and requires reload', async () => {
    const loadCurrent = vi.fn().mockResolvedValue([current()]);
    const acceptVersion = vi.fn().mockRejectedValue({ code: 'revision_conflict' });
    const client = clientFor({ loadCurrent, acceptVersion });
    render(<ProductiveProductDecisionsPanel epicId="epic-1" client={client} />);
    await screen.findByText('Title 1');

    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Keep this draft' } });
    fireEvent.click(screen.getByRole('button', { name: 'Accept correction' }));
    await screen.findByText(/changed or conflicted elsewhere/);
    expect(screen.getByDisplayValue('Keep this draft')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Reload' }));
    await waitFor(() => expect(loadCurrent).toHaveBeenCalledTimes(2));
    expect(screen.queryByDisplayValue('Keep this draft')).toBeNull();
  });

  it('keeps immutable history and historical unresolved evidence non-actionable', async () => {
    const client = clientFor({
      loadHistory: vi.fn().mockResolvedValue([version(1), version(2, 'Corrected')]),
    });
    render(<ProductiveProductDecisionsPanel epicId="epic-1" client={client} />);
    const card = await screen.findByRole('article', { name: 'Title 1 current decision' });
    fireEvent.click(within(card).getByRole('button', { name: 'Version history' }));
    expect(await screen.findByText(/Version\s*2:\s*Title\s*2/)).toBeVisible();
    expect(within(card).getByText(/Retained for history only/)).toBeVisible();
    expect(within(card).queryByRole('button', { name: /historical/i })).toBeNull();
  });
});
