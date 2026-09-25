import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { CodexHomeGuidance, CodexHomeGuidanceProvider } from './CodexHomeGuidance';

const failure = {
  code: 'runtime_preflight_failed',
  message: 'No native Codex home is selected',
  details: null,
};

describe('CodexHomeGuidance', () => {
  it('offers Technical Settings when no home is selected', async () => {
    const onOpenTechnicalSettings = vi.fn();
    render(
      <CodexHomeGuidanceProvider
        consumer={{ currentSelection: async () => ({ kind: 'none' }) }}
        onOpenTechnicalSettings={onOpenTechnicalSettings}
      >
        <CodexHomeGuidance failure={failure} />
      </CodexHomeGuidanceProvider>,
    );
    expect(
      await screen.findByText('No Codex home is selected in Technical Settings.'),
    ).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Open Technical Settings' }));
    expect(onOpenTechnicalSettings).toHaveBeenCalledOnce();
  });

  it('identifies the currently selected home without offering navigation', async () => {
    render(
      <CodexHomeGuidanceProvider
        consumer={{
          currentSelection: async () => ({
            kind: 'selected',
            profileId: 'profile-1',
            codexHome: 'C:/Users/user/.codex',
          }),
        }}
      >
        <CodexHomeGuidance failure={failure} />
      </CodexHomeGuidanceProvider>,
    );
    expect(await screen.findByText(/C:\/Users\/user\/\.codex/)).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Open Technical Settings' })).toBeNull();
  });

  it('does not add guidance to an unrelated runtime failure', () => {
    render(
      <CodexHomeGuidanceProvider consumer={{ currentSelection: async () => ({ kind: 'none' }) }}>
        <CodexHomeGuidance failure={{ ...failure, message: 'The selected profile is not ready' }} />
      </CodexHomeGuidanceProvider>,
    );
    expect(screen.queryByLabelText('Codex home guidance')).toBeNull();
  });
});
