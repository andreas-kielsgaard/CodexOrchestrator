import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeAll, describe, expect, it, vi } from 'vitest';
import { ImportCodexSessionDialog } from './ImportCodexSessionDialog';
import type {
  AgentSessionImportClient,
  CodexImportPreview,
} from '../../application/agentSessions/importContracts';

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
});
const preview: CodexImportPreview = {
  threadId: 'source',
  title: 'Source conversation',
  sourceDirectory: 'C:/gone',
  allocateWorkspace: true,
  turnCount: 2,
  lastTurnId: 'turn-2',
  nativeHome: 'C:/codex',
  profileId: 'profile',
  capabilityProfile: 'Default',
  excerpt: 'Remembered context',
};
describe('Codex import dialog', () => {
  it('previews the allocated-folder fallback and selects the imported session without sending a prompt', async () => {
    const client: AgentSessionImportClient = {
      preview: vi.fn().mockResolvedValue(preview),
      importConversation: vi.fn().mockResolvedValue('imported'),
    };
    const onImported = vi.fn();
    render(<ImportCodexSessionDialog client={client} onImported={onImported} onClose={() => {}} />);
    fireEvent.change(screen.getByLabelText('Codex link'), {
      target: { value: 'codex://threads/source' },
    });
    fireEvent.click(screen.getByText('Preview conversation'));
    expect(await screen.findByText('Source conversation')).toBeInTheDocument();
    expect(screen.getByText(/Orchid will create a new empty working folder/)).toBeInTheDocument();
    expect(screen.getAllByRole('textbox')).toHaveLength(1);
    expect(client.importConversation).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText('Import conversation'));
    await waitFor(() => expect(onImported).toHaveBeenCalledWith('imported'));
    expect(client.importConversation).toHaveBeenCalledWith(
      expect.objectContaining({ profileId: 'profile', lastTurnId: 'turn-2' }),
    );
  });
  it('retains the request identity on retry after a persistence error', async () => {
    const importConversation = vi
      .fn()
      .mockRejectedValueOnce(new Error('Storage unavailable'))
      .mockResolvedValue('id');
    render(
      <ImportCodexSessionDialog
        client={{ preview: async () => preview, importConversation }}
        onImported={() => {}}
        onClose={() => {}}
      />,
    );
    fireEvent.change(screen.getByLabelText('Codex link'), {
      target: { value: 'codex://threads/source' },
    });
    fireEvent.click(screen.getByText('Preview conversation'));
    fireEvent.click(await screen.findByText('Import conversation'));
    expect(await screen.findByRole('alert')).toHaveTextContent('Storage unavailable');
    fireEvent.click(screen.getByText('Import conversation'));
    await waitFor(() => expect(importConversation).toHaveBeenCalledTimes(2));
    expect(importConversation.mock.calls[0][0]).toEqual(importConversation.mock.calls[1][0]);
  });
  it('reports preview errors and cancel does not import', async () => {
    const client = {
      preview: vi.fn().mockRejectedValue(new Error('Invalid Codex link')),
      importConversation: vi.fn(),
    };
    const close = vi.fn();
    render(<ImportCodexSessionDialog client={client} onImported={() => {}} onClose={close} />);
    fireEvent.change(screen.getByLabelText('Codex link'), { target: { value: 'bad link' } });
    fireEvent.click(screen.getByText('Preview conversation'));
    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid Codex link');
    fireEvent.click(screen.getByText('Cancel'));
    expect(close).toHaveBeenCalledOnce();
    expect(client.importConversation).not.toHaveBeenCalled();
  });
});
