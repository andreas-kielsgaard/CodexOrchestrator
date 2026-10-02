import { isLongPaste, isImageAttachment } from './composerAttachments';
it('uses a bounded, explicit long-text threshold', () => {
  expect(isLongPaste('x'.repeat(4000))).toBe(false);
  expect(isLongPaste('x'.repeat(4001))).toBe(true);
  expect(isLongPaste(Array(51).fill('line').join('\n'))).toBe(true);
});
it('distinguishes image previews from ordinary file attachments', () => {
  expect(isImageAttachment({ mediaType: 'image/png' })).toBe(true);
  expect(isImageAttachment({ mediaType: 'text/plain' })).toBe(false);
});
