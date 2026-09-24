import { fireEvent, render, renderHook, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ProductCommandBar } from './ProductCommandBar';
import { useHistoryMouseButtons } from './useHistoryMouseButtons';

describe('ProductCommandBar', () => {
  it('always renders independently disabled, keyboard-operable Back and Forward commands', () => {
    const onBack = vi.fn();
    const onForward = vi.fn();
    const { rerender } = render(
      <ProductCommandBar
        canGoBack={false}
        canGoForward={true}
        onBack={onBack}
        onForward={onForward}
      />,
    );

    expect(screen.getByRole('button', { name: 'Back' })).toBeDisabled();
    const forward = screen.getByRole('button', { name: 'Forward' });
    expect(forward).toBeEnabled();
    fireEvent.click(forward);
    expect(onForward).toHaveBeenCalledOnce();

    rerender(
      <ProductCommandBar
        canGoBack={true}
        canGoForward={false}
        onBack={onBack}
        onForward={onForward}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    expect(onBack).toHaveBeenCalledOnce();
    expect(screen.getByRole('button', { name: 'Forward' })).toBeDisabled();
  });
});

describe('useHistoryMouseButtons', () => {
  it('routes standard mouse Back and Forward buttons through product commands', () => {
    const onBack = vi.fn();
    const onForward = vi.fn();
    const { rerender } = renderHook(
      ({ canGoBack, canGoForward }) =>
        useHistoryMouseButtons({ canGoBack, canGoForward, onBack, onForward }),
      { initialProps: { canGoBack: true, canGoForward: true } },
    );

    window.dispatchEvent(new MouseEvent('mouseup', { button: 3, cancelable: true }));
    window.dispatchEvent(new MouseEvent('mouseup', { button: 4, cancelable: true }));
    expect(onBack).toHaveBeenCalledOnce();
    expect(onForward).toHaveBeenCalledOnce();

    rerender({ canGoBack: false, canGoForward: false });
    window.dispatchEvent(new MouseEvent('mouseup', { button: 3, cancelable: true }));
    window.dispatchEvent(new MouseEvent('mouseup', { button: 4, cancelable: true }));
    expect(onBack).toHaveBeenCalledOnce();
    expect(onForward).toHaveBeenCalledOnce();
  });
});
