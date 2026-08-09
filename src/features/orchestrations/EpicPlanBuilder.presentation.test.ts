import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('Epic Plan Builder responsive containment', () => {
  it('keeps three wide panes bounded and stacks the working panes without horizontal overflow', () => {
    const styles = readFileSync(
      resolve('src/features/orchestrations/styles/epicPlanBuilder.css'),
      'utf8',
    );

    expect(styles).toMatch(
      /\.epic-plan-builder__layout\s*\{[\s\S]*min-width: 0;[\s\S]*overflow: hidden;/,
    );
    expect(styles).toMatch(
      /\.epic-plan-builder__workspace\s*\{[\s\S]*grid-template-columns: minmax\(400px, 1fr\) minmax\(320px, 420px\);[\s\S]*overflow: hidden;/,
    );
    expect(styles).toMatch(
      /@media \(max-width: 900px\)\s*\{[\s\S]*\.epic-plan-builder__body\s*\{[\s\S]*overflow-x: hidden;[\s\S]*overflow-y: auto;[\s\S]*\.epic-plan-builder__workspace\s*\{[\s\S]*grid-template-columns: minmax\(0, 1fr\);/,
    );
    expect(styles).toMatch(
      /@media \(max-width: 720px\)\s*\{[\s\S]*\.epic-plan-builder__workspace\s*\{[\s\S]*grid-template-rows: minmax\(460px, 1fr\) minmax\(280px, 0.7fr\);/,
    );
  });
});
