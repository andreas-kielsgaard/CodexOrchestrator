import '@testing-library/jest-dom/vitest';

if (!Range.prototype.getBoundingClientRect)
  Range.prototype.getBoundingClientRect = () => new DOMRect();
if (!Range.prototype.getClientRects)
  Range.prototype.getClientRects = () => [] as unknown as DOMRectList;
if (!HTMLElement.prototype.scrollIntoView) HTMLElement.prototype.scrollIntoView = () => undefined;
if (!HTMLElement.prototype.hasPointerCapture) HTMLElement.prototype.hasPointerCapture = () => false;
if (!HTMLElement.prototype.setPointerCapture)
  HTMLElement.prototype.setPointerCapture = () => undefined;
if (!HTMLElement.prototype.releasePointerCapture)
  HTMLElement.prototype.releasePointerCapture = () => undefined;
