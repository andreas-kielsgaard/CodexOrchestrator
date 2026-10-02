export const graphLaneColors = Array.from(
  { length: 7 },
  (_, index) => `var(--graph-lane-${index + 1})`,
);

export function graphColorForKey(key: string): string {
  let hash = 0;
  for (const char of key) hash = ((hash << 5) - hash + char.charCodeAt(0)) | 0;
  return graphLaneColors[Math.abs(hash) % graphLaneColors.length];
}
