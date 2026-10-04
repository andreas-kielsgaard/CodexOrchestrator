import { fireEvent, render, screen } from '@testing-library/react';
import { skillReferenceParts, SkillReferenceProvider } from './SkillReferences';
import { AgentSessionComposer } from './AgentSessionComposer';
import type { ComposerQuickFeatures } from './composerQuickActions';
it('highlights only complete references in the current catalogue', () => {
  const skill = { id: 'path', name: 'review', description: 'Review', invocationText: '$review' };
  const parts = skillReferenceParts('Use $review and $review-extra, not $other.', [skill]);
  expect(parts.filter((part) => part.skill)).toEqual([{ text: '$review', skill }]);
  expect(parts.map((part) => part.text).join('')).toBe(
    'Use $review and $review-extra, not $other.',
  );
});

it('keeps the skill mirror inside the text box and aligns it on resize and scroll', () => {
  const callbacks: Array<() => void> = [];
  const disconnect = vi.fn();
  vi.stubGlobal(
    'ResizeObserver',
    class {
      constructor(callback: () => void) {
        callbacks.push(callback);
      }
      observe() {}
      disconnect = disconnect;
    },
  );
  try {
    const catalogue = {
      configuration: { provider: 'codex' as const, configurationId: 'test' },
      defaults: { model: null, reasoningMode: null },
      models: [],
      limitations: [],
      skills: [{ id: 'path', name: 'review', description: 'Review', invocationText: '$review' }],
    };
    const source: ComposerQuickFeatures = {
      contextKey: 'test',
      catalogue,
      load: async () => catalogue,
      selection: { model: null, reasoningMode: null },
      setSelection: vi.fn(),
    };
    const { container, unmount } = render(
      <SkillReferenceProvider source={source}>
        <AgentSessionComposer
          draft={'Before $review\n\nAfter'}
          workingDirectory=""
          isNewSession={false}
          sending={false}
          active={false}
          canceling={false}
          showWorkingDirectory={false}
          keyboardHint="hidden"
          onDraftChange={vi.fn()}
          onWorkingDirectoryChange={vi.fn()}
          onSend={vi.fn()}
          onCancel={vi.fn()}
        />
      </SkillReferenceProvider>,
    );
    const input = screen.getByRole('textbox', { name: 'Message' });
    const mirror = container.querySelector<HTMLElement>('.composer-skill-mirror')!;
    expect(input.parentElement).toBe(mirror.parentElement);
    expect(input.parentElement).toHaveClass('composer-text-editor');
    Object.defineProperty(input, 'clientWidth', { value: 320 });
    input.scrollTop = 42;
    callbacks.forEach((callback) => callback());
    expect(mirror.style.width).toBe('320px');
    expect(mirror.scrollTop).toBe(42);
    input.scrollTop = 84;
    fireEvent.scroll(input);
    expect(mirror.scrollTop).toBe(84);
    expect(mirror.textContent).toBe('Before $review\n\nAfter\n');
    unmount();
    expect(disconnect).toHaveBeenCalled();
  } finally {
    vi.unstubAllGlobals();
  }
});
