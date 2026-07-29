import { fireEvent, render, screen } from '@testing-library/react';
import { composeProductOrchestrationReadModels } from '../../../application/orchestrations';
import { presentProductOrchestrations } from '../../../app/orchestrationPresentation';
import { recordedPresentationAdjunct } from '../../../dev/orchestrationSection/recordedPresentationAdjunct';
import { recordedProductReadCompositionInput } from '../../../dev/orchestrationSection/recordedProductReadCompositionInput';
import { SprintDocumentsPanel } from './SprintDocumentsPanel';

describe('SprintDocumentsPanel', () => {
  it('invokes injected resolve, open, and copy-path operations with the selected projected document', async () => {
    const resolveForOpen = vi.fn(() =>
      Promise.resolve({
        operation: 'resolve_for_open' as const,
        status: 'observed_success' as const,
        message: 'Resolved only.',
      }),
    );
    const openWithSystemDefault = vi.fn(() =>
      Promise.resolve({
        operation: 'open_with_system_default' as const,
        status: 'unsupported' as const,
        message: 'Open is unsupported.',
      }),
    );
    const copyPath = vi.fn(() =>
      Promise.resolve({
        operation: 'copy_path' as const,
        status: 'observed_success' as const,
        message: 'Copied.',
        rawPath: 'C:/copied/only-here.md',
      }),
    );
    const onOpenFileReviewSource = vi.fn();
    const document = presentProductOrchestrations(
      composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
      recordedPresentationAdjunct,
    ).epics[0].plan.items[2].workspace!.documents[0];
    render(
      <SprintDocumentsPanel
        documents={[document]}
        artifactAccess={{ resolveForOpen, openWithSystemDefault, copyPath }}
        onOpenFileReviewSource={onOpenFileReviewSource}
      />,
    );

    const control = screen.getByRole('button', { name: new RegExp(document.title) });
    control.focus();
    expect(control).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Resolve' }));
    expect(resolveForOpen).toHaveBeenCalledWith(document);
    expect(await screen.findByText('Resolved only.')).toHaveAttribute('role', 'status');

    fireEvent.click(control);
    expect(onOpenFileReviewSource).toHaveBeenCalledWith(document.documentRefId);
    expect(openWithSystemDefault).not.toHaveBeenCalled();
    expect(screen.queryByText('C:/copied/only-here.md')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Copy path' }));
    expect(copyPath).toHaveBeenCalledWith(document);
    expect(await screen.findByText('C:/copied/only-here.md')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'View document' }));
    expect(onOpenFileReviewSource).toHaveBeenCalledTimes(2);
  });
});
