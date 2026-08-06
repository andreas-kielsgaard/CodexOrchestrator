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

function version(
  number: number,
  statement = `Statement ${number}`,
  options: { readonly decisionId?: string; readonly epicId?: string; readonly title?: string } = {},
): ProductDecisionVersion {
  const decisionId = options.decisionId ?? 'decision-1';
  return {
    versionId:
      decisionId === 'decision-1' ? `version-${number}` : `${decisionId}-version-${number}`,
    decisionId,
    epicId: options.epicId ?? 'epic-1',
    version: number,
    title: options.title ?? `Title ${number}`,
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

function current(
  currentVersion = version(1),
  options: { readonly decisionId?: string; readonly epicId?: string } = {},
): ProductDecisionCurrent {
  return {
    decisionId: options.decisionId ?? currentVersion.decisionId,
    epicId: options.epicId ?? currentVersion.epicId,
    currentVersion,
    applicationState: 'not_applied',
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
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

  it('fails closed for superseded current and history requests after reload and context change', async () => {
    const initialCurrent = deferred<readonly ProductDecisionCurrent[]>();
    const reloadedCurrent = deferred<readonly ProductDecisionCurrent[]>();
    const oldHistory = deferred<readonly ProductDecisionVersion[]>();
    const clientOne = clientFor({
      loadCurrent: vi
        .fn()
        .mockImplementationOnce(() => initialCurrent.promise)
        .mockImplementationOnce(() => reloadedCurrent.promise),
      loadHistory: vi.fn().mockImplementation(() => oldHistory.promise),
    });
    const clientTwo = clientFor({
      loadCurrent: vi
        .fn()
        .mockResolvedValue([
          current(version(1, 'Current from Epic Two', { epicId: 'epic-2', title: 'Epic Two' })),
        ]),
    });
    const view = render(<ProductiveProductDecisionsPanel epicId="epic-1" client={clientOne} />);

    await waitFor(() => expect(clientOne.loadCurrent).toHaveBeenCalledTimes(1));
    initialCurrent.resolve([current(version(1, 'Initial Epic One', { title: 'Epic One' }))]);
    await screen.findByText('Epic One');
    const firstCard = screen.getByRole('article', { name: 'Epic One current decision' });
    fireEvent.click(within(firstCard).getByRole('button', { name: 'Version history' }));
    fireEvent.click(screen.getByRole('button', { name: 'Reload' }));
    await waitFor(() => expect(clientOne.loadCurrent).toHaveBeenCalledTimes(2));

    view.rerender(<ProductiveProductDecisionsPanel epicId="epic-2" client={clientTwo} />);
    await screen.findByText('Epic Two');
    reloadedCurrent.resolve([
      current(version(1, 'Stale Reload Result', { title: 'Stale reload' })),
    ]);
    oldHistory.resolve([version(2, 'Stale History Result', { title: 'Stale history' })]);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(screen.queryByText('Stale reload')).toBeNull();
    expect(screen.queryByText('Stale history')).toBeNull();
    expect(screen.getByText('Epic Two')).toBeVisible();
  });

  it('blocks competing actions and ignores an acceptance result after context changes', async () => {
    const acceptance = deferred<ProductDecisionVersion>();
    const secondDecision = current(
      version(1, 'Second decision', { decisionId: 'decision-2', title: 'Second decision' }),
      { decisionId: 'decision-2' },
    );
    const clientOne = clientFor({
      loadCurrent: vi.fn().mockResolvedValue([current(), secondDecision]),
      acceptVersion: vi.fn().mockImplementation(() => acceptance.promise),
    });
    const clientTwo = clientFor({
      loadCurrent: vi
        .fn()
        .mockResolvedValue([
          current(version(1, 'New context decision', { epicId: 'epic-2', title: 'New context' })),
        ]),
    });
    const onOpenEvidence = vi.fn();
    const onPublish = vi.fn();
    const view = render(
      <ProductiveProductDecisionsPanel
        epicId="epic-1"
        client={clientOne}
        onOpenEvidence={onOpenEvidence}
        onPublish={onPublish}
      />,
    );
    await screen.findByRole('article', { name: 'Second decision current decision' });

    fireEvent.click(
      within(screen.getByRole('article', { name: 'Title 1 current decision' })).getByRole(
        'button',
        { name: 'Edit' },
      ),
    );
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Old draft' } });
    fireEvent.click(screen.getByRole('button', { name: 'Accept correction' }));

    expect(screen.getByRole('button', { name: 'Reload' })).toBeDisabled();
    expect(
      screen
        .getAllByRole('button', { name: 'Edit' })
        .every((button) => (button as HTMLButtonElement).disabled),
    ).toBe(true);
    expect(
      screen
        .getAllByRole('button', { name: 'Open supporting Agent Session passage' })
        .every((button) => (button as HTMLButtonElement).disabled),
    ).toBe(true);
    expect(screen.getByRole('button', { name: 'Publish' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Publish' }));
    fireEvent.click(
      screen.getAllByRole('button', { name: 'Open supporting Agent Session passage' })[0],
    );
    expect(onPublish).not.toHaveBeenCalled();
    expect(onOpenEvidence).not.toHaveBeenCalled();

    view.rerender(<ProductiveProductDecisionsPanel epicId="epic-2" client={clientTwo} />);
    await screen.findByText('New context');
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'New draft' } });

    acceptance.resolve(version(2, 'Old accepted result'));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(screen.getByDisplayValue('New draft')).toBeVisible();
    expect(screen.queryByText('Old accepted result')).toBeNull();
    expect(screen.queryByText(/Accepted Product Decision version 2/)).toBeNull();
  });
});
