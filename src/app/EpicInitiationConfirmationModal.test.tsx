import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import {
  EpicInitiationConfirmationError,
  type EpicInitiationConfirmationClient,
  type EpicInitiationConfirmationEvent,
} from '../application/orchestrations';
import { EpicInitiationConfirmationModal } from './EpicInitiationConfirmationModal';
import { useEpicInitiationConfirmation } from './useEpicInitiationConfirmation';

const buttonRequest = {
  requestId: 'button-request',
  source: { kind: 'button' as const },
  epicPlanningDraftId: 'draft-button',
  state: 'requested' as const,
};
const agentRequest = {
  requestId: 'agent-request',
  source: {
    kind: 'agent' as const,
    agentSessionId: 'session-agent',
    agentInvocationId: 'invocation-agent',
  },
  epicPlanningDraftId: 'draft-agent',
  state: 'requested' as const,
};

function fixture() {
  let listener: ((event: EpicInitiationConfirmationEvent) => void) | undefined;
  let malformed: (() => void) | undefined;
  const resolve = vi.fn(async (requestId: string, decision: 'confirmed' | 'rejected') => {
    if (decision === 'rejected') throw new EpicInitiationConfirmationError('rejected');
    return {
      requestId,
      state: 'projected' as const,
      initiation: {
        initiationId: 'initiation-1',
        epicId: 'epic-1',
        proposalRevisionId: 'revision-1',
        materialSnapshotHash: 'hash',
        idempotentReplay: false,
      },
    };
  });
  const client: EpicInitiationConfirmationClient = {
    request: vi.fn().mockResolvedValue(buttonRequest),
    resolve,
    subscribe: vi.fn(async (next, invalid) => {
      listener = next;
      malformed = invalid;
      return vi.fn();
    }),
    describe: vi.fn(async (request) => ({
      title: request.epicPlanningDraftId === 'draft-button' ? 'Button Epic' : 'Agent Epic',
      sprintTitles: ['Scope Sprint'],
    })),
  };
  return {
    client,
    resolve,
    emit: (request: typeof buttonRequest | typeof agentRequest) =>
      listener?.({ request, state: 'requested' }),
    malformed: () => malformed?.(),
  };
}

function Harness({
  client,
  onProjected = vi.fn(),
}: {
  client: EpicInitiationConfirmationClient;
  onProjected?: () => Promise<void> | void;
}) {
  const confirmation = useEpicInitiationConfirmation(client, async () => {
    await onProjected();
  });
  const [, renderAgain] = useState(0);
  return (
    <>
      <button
        type="button"
        onClick={() =>
          void confirmation.requestButton({
            epicPlanningDraftId: 'draft-button',
            expectedRevisionToken: 'token',
            idempotencyKey: 'key',
          })
        }
      >
        Button request
      </button>
      <button type="button" onClick={() => renderAgain((value) => value + 1)}>
        Outside
      </button>
      {confirmation.receiptError && <p role="alert">{confirmation.receiptError}</p>}
      <EpicInitiationConfirmationModal confirmation={confirmation} />
    </>
  );
}

describe('shared Epic initiation confirmation modal', () => {
  it('uses the same modal for button and agent requests and serializes distinct requests', async () => {
    const f = fixture();
    render(<Harness client={f.client} />);
    fireEvent.click(screen.getByRole('button', { name: 'Button request' }));
    expect(await screen.findByRole('dialog', { name: 'Initiate this Epic?' })).toHaveTextContent(
      'Button Epic',
    );
    await act(async () => {
      f.emit(buttonRequest);
      f.emit(agentRequest);
    });
    expect(screen.getByText('1 other confirmation request waiting.')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(await screen.findByRole('dialog', { name: 'Initiate this Epic?' })).toHaveTextContent(
      'Agent Epic',
    );
    expect(f.resolve).toHaveBeenCalledWith('button-request', 'rejected');
  });

  it('confirms explicitly, disables repeat resolution, and reports projected completion', async () => {
    let release!: () => void;
    const f = fixture();
    const projected = vi.fn();
    f.resolve.mockImplementationOnce(async (requestId) => {
      await new Promise<void>((resolve) => {
        release = resolve;
      });
      return {
        requestId,
        state: 'projected',
        initiation: {
          initiationId: 'i',
          epicId: 'e',
          proposalRevisionId: 'r',
          materialSnapshotHash: 'h',
          idempotentReplay: false,
        },
      };
    });
    render(<Harness client={f.client} onProjected={projected} />);
    fireEvent.click(screen.getByRole('button', { name: 'Button request' }));
    const confirm = await screen.findByRole('button', { name: 'Confirm initiation' });
    fireEvent.click(confirm);
    expect(screen.getByRole('button', { name: 'Resolving…' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Resolving…' }));
    expect(f.resolve).toHaveBeenCalledOnce();
    await act(async () => release());
    await waitFor(() => expect(projected).toHaveBeenCalledOnce());
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('confirms an agent request through the shared resolution path', async () => {
    const f = fixture();
    render(<Harness client={f.client} />);
    await act(async () => f.emit(agentRequest));
    expect(await screen.findByRole('dialog', { name: 'Initiate this Epic?' })).toHaveTextContent(
      'Agent Epic',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Confirm initiation' }));
    await waitFor(() => expect(f.resolve).toHaveBeenCalledWith('agent-request', 'confirmed'));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('keeps confirmation successful when the later application refresh fails', async () => {
    const f = fixture();
    const refresh = vi.fn().mockRejectedValue(new Error('refresh unavailable'));
    render(<Harness client={f.client} onProjected={refresh} />);
    fireEvent.click(screen.getByRole('button', { name: 'Button request' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Confirm initiation' }));
    expect(
      await screen.findByText(/initiation was confirmed.*could not be refreshed/i),
    ).toBeVisible();
    expect(f.resolve).toHaveBeenCalledOnce();
    expect(refresh).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByText(/confirmation.*failed/i)).toBeNull();
  });

  it('supports Escape, traps focus, restores focus, and rejects malformed events', async () => {
    const f = fixture();
    render(<Harness client={f.client} />);
    const outside = screen.getByRole('button', { name: 'Outside' });
    outside.focus();
    fireEvent.click(screen.getByRole('button', { name: 'Button request' }));
    const dialog = await screen.findByRole('dialog');
    const confirm = screen.getByRole('button', { name: 'Confirm initiation' });
    const cancel = screen.getByRole('button', { name: 'Cancel' });
    expect(confirm).toHaveFocus();
    fireEvent.keyDown(dialog, { key: 'Tab' });
    expect(cancel).toHaveFocus();
    fireEvent.keyDown(dialog, { key: 'Tab', shiftKey: true });
    expect(confirm).toHaveFocus();
    fireEvent.keyDown(dialog, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(outside).toHaveFocus();
    await act(async () => f.malformed());
    expect(screen.getByRole('alert')).toHaveTextContent('invalid confirmation event');
  });
});
