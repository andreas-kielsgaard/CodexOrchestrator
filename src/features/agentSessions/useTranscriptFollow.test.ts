import { isNearTranscriptBottom } from './useTranscriptFollow';

describe('transcript follow policy', () => {
  it('follows when the reader remains near the newest content', () => {
    expect(isNearTranscriptBottom({ scrollHeight: 1_000, scrollTop: 520, clientHeight: 400 })).toBe(
      true,
    );
  });

  it('does not follow when the reader intentionally moved into older content', () => {
    expect(isNearTranscriptBottom({ scrollHeight: 1_000, scrollTop: 300, clientHeight: 400 })).toBe(
      false,
    );
  });
});
