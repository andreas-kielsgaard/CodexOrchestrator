import { fireEvent, render, screen } from '@testing-library/react';
import { recordedFileReviewClient } from '../../dev/fileReview/recordedFileReviewClient';
import { FileReviewScreen } from './FileReviewScreen';

describe('FileReviewScreen', () => {
  it('navigates changed files, expands context, and switches unified and split layouts', async () => {
    const { container } = render(<FileReviewScreen client={recordedFileReviewClient} />);

    expect(await screen.findByText('5 changed files')).toBeVisible();
    expect(screen.getByText('+84')).toBeVisible();
    expect(screen.getByText('−16')).toBeVisible();
    expect(screen.queryByText('Repository identity remains inside the adapter.')).toBeNull();
    expect(screen.queryByRole('button', { name: /edit|save|stage|discard|write/i })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Show 3 unchanged lines above' }));
    expect(screen.getByText('Repository identity remains inside the adapter.')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Split' }));
    expect(screen.getByRole('button', { name: 'Split' })).toHaveAttribute('aria-pressed', 'true');
    expect(container.querySelector('.file-review-split')).not.toBeNull();

    fireEvent.click(
      screen.getByRole('button', {
        name: 'Review src/features/fileReview/FileReviewScreen.tsx',
      }),
    );
    expect(screen.getByText('@@ -0,0 +1,8 @@')).toBeVisible();
  });

  it('reuses safe Markdown rendering and names binary and unsupported states', async () => {
    const { container } = render(<FileReviewScreen client={recordedFileReviewClient} />);

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

  it('loads every provenance class through an opaque source selection', async () => {
    render(<FileReviewScreen client={recordedFileReviewClient} />);

    const source = await screen.findByRole('combobox', { name: 'Review source' });
    for (const [sourceId, expectedDetail] of [
      ['source-staged', 'Index snapshot · ready for commit review'],
      ['source-commit-range', 'main…exploration · three commits'],
      ['source-generated', 'Bootstrap preview · not persisted'],
      ['source-application-owned', 'Accepted review note · durable product record'],
    ] as const) {
      fireEvent.change(source, { target: { value: sourceId } });
      expect(await screen.findByText(expectedDetail)).toBeVisible();
    }

    expect(screen.queryByText(/C:\\|C:\//)).toBeNull();
  });
});
