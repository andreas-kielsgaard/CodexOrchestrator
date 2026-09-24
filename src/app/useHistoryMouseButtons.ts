import { useEffect } from 'react';

/** Maps the standard mouse history buttons to the product navigation commands. */
export function useHistoryMouseButtons({
  canGoBack,
  canGoForward,
  onBack,
  onForward,
}: {
  readonly canGoBack: boolean;
  readonly canGoForward: boolean;
  readonly onBack: () => void;
  readonly onForward: () => void;
}) {
  useEffect(() => {
    const handleMouseHistory = (event: MouseEvent) => {
      if (event.button === 3 && canGoBack) {
        event.preventDefault();
        onBack();
      } else if (event.button === 4 && canGoForward) {
        event.preventDefault();
        onForward();
      }
    };
    window.addEventListener('mouseup', handleMouseHistory);
    return () => window.removeEventListener('mouseup', handleMouseHistory);
  }, [canGoBack, canGoForward, onBack, onForward]);
}
