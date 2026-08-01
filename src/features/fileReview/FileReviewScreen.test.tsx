import { fireEvent, render, screen, within } from '@testing-library/react';
import { createRecordedFileReviewSource } from '../../dev/fileReview/recordedFileReviewClient';
import { FileReviewScreen } from './FileReviewScreen';

describe('FileReviewScreen', () => {
  it('navigates changed files, expands context, and switches unified and split layouts', async () => {
    const { container } = render(
      <FileReviewScreen source={createRecordedFileReviewSource('working-tree')} />,
    );

    expect(await screen.findByText('5 changed files')).toBeVisible();
    expect(screen.getByText('+12')).toBeVisible();
    expect(screen.getByText('−4')).toBeVisible();
    expect(screen.queryByText(/The viewer presents/)).toBeNull();
    expect(screen.queryByRole('button', { name: /edit|save|stage|discard|write/i })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Show 3 unchanged lines above' }));
    expect(screen.getByText(/The viewer presents/)).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Split' }));
    expect(screen.getByRole('button', { name: 'Split' })).toHaveAttribute('aria-pressed', 'true');
    expect(container.querySelector('.file-review-split')).not.toBeNull();

    fireEvent.click(
      screen.getByRole('button', {
        name: 'Review src/features/fileReview/FileReviewScreen.tsx',
      }),
    );
    expect(screen.getByText('@@ -0,0 +1,7 @@')).toBeVisible();
  });

  it('reuses safe Markdown rendering and names binary and unsupported states', async () => {
    const { container } = render(
      <FileReviewScreen source={createRecordedFileReviewSource('working-tree')} />,
    );

    await screen.findByText('5 changed files');
    fireEvent.click(screen.getByRole('button', { name: 'File' }));

    expect(
      screen.getByRole('heading', { name: 'A review surface inside the product' }),
    ).toBeVisible();
    expect(screen.getByRole('table')).toBeVisible();
    expect(container.querySelector('button.agent-markdown')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Untrusted HTML action' })).toBeNull();

    fireEvent.click(
      screen.getByRole('button', {
        name: 'Review docs/review/file-review-walkthrough.mp4',
      }),
    );
    expect(screen.getByRole('heading', { name: 'Binary preview unavailable' })).toBeVisible();

    fireEvent.click(
      screen.getByRole('button', {
        name: 'Review docs/review/layout-study.sketch',
      }),
    );
    expect(screen.getByRole('heading', { name: 'File type not supported' })).toBeVisible();
  });

  it('renders one scoped review without origin or storage provenance controls', async () => {
    render(<FileReviewScreen source={createRecordedFileReviewSource('application-owned')} />);

    expect(await screen.findByText('1 changed files')).toBeVisible();
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(screen.queryByText('Application-owned')).toBeNull();
    expect(screen.queryByText('Recorded application-owned review material')).toBeNull();
    expect(screen.queryByText(/C:\\|C:\//)).toBeNull();
  });

  it('names an empty scoped review instead of remaining in loading', async () => {
    render(
      <FileReviewScreen
        source={{
          load: async () => ({ files: [] }),
        }}
      />,
    );

    expect(await screen.findByRole('heading', { name: 'No changed files' })).toBeVisible();
    expect(screen.getByText('No files are available for this review.')).toBeVisible();
  });

  it('keeps Unified/Split left and Changes/File right with a stable non-focusable slot', async () => {
    render(<FileReviewScreen source={createRecordedFileReviewSource('working-tree')} />);

    await screen.findByText('5 changed files');
    const inspectionModes = screen.getByRole('group', { name: 'File inspection mode' });
    const layoutSlot = screen.getByTestId('diff-layout-slot');
    expect(layoutSlot.nextElementSibling).toBe(inspectionModes);
    expect(screen.getByRole('group', { name: 'Diff layout' })).toBe(layoutSlot.firstElementChild);
    expect(layoutSlot).toHaveStyle({ width: '152px' });

    fireEvent.click(screen.getByRole('button', { name: 'File' }));

    expect(screen.getByRole('group', { name: 'File inspection mode' })).toBe(inspectionModes);
    expect(layoutSlot.nextElementSibling).toBe(inspectionModes);
    expect(layoutSlot).toHaveStyle({ width: '152px' });
    expect(screen.queryByRole('group', { name: 'Diff layout' })).toBeNull();
    expect(within(layoutSlot).queryAllByRole('button')).toHaveLength(0);
  });
});
