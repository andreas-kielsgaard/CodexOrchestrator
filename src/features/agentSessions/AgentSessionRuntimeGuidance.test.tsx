import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import {
  AgentSessionRuntimeGuidance,
  AgentSessionRuntimeGuidanceProvider,
} from './AgentSessionRuntimeGuidance';

const failure = {
  code: 'runtime_preflight_failed',
  message: 'No native Codex home is selected',
  details: null,
};

describe('AgentSessionRuntimeGuidance', () => {
  it('offers Technical Settings when no home is selected', async () => {
    const onOpenTechnicalSettings = vi.fn();
    render(
      <AgentSessionRuntimeGuidanceProvider
        consumer={{ currentSelection: async () => ({ kind: 'none' }) }}
        onOpenTechnicalSettings={onOpenTechnicalSettings}
      >
        <AgentSessionRuntimeGuidance failure={failure} />
      </AgentSessionRuntimeGuidanceProvider>,
    );
    expect(
      await screen.findByText('No Codex home is selected in Technical Settings.'),
    ).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Open Technical Settings' }));
    expect(onOpenTechnicalSettings).toHaveBeenCalledOnce();
  });

  it('identifies the currently selected home without offering navigation', async () => {
    render(
      <AgentSessionRuntimeGuidanceProvider
        consumer={{
          currentSelection: async () => ({
            kind: 'selected',
            profileId: 'profile-1',
            codexHome: 'C:/Users/user/.codex',
          }),
        }}
      >
        <AgentSessionRuntimeGuidance failure={failure} />
      </AgentSessionRuntimeGuidanceProvider>,
    );
    expect(await screen.findByText(/C:\/Users\/user\/\.codex/)).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Open Technical Settings' })).toBeNull();
  });

  it('does not add guidance to an unrelated runtime failure', () => {
    render(
      <AgentSessionRuntimeGuidanceProvider
        consumer={{ currentSelection: async () => ({ kind: 'none' }) }}
      >
        <AgentSessionRuntimeGuidance
          failure={{ ...failure, message: 'The selected profile is not ready' }}
        />
      </AgentSessionRuntimeGuidanceProvider>,
    );
    expect(screen.queryByLabelText('Codex home guidance')).toBeNull();
  });
});
