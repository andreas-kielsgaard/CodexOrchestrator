import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { graphColorForKey, graphLaneColors } from './graphColors';

const sourceRoot = resolve('src');
const sourceFiles = readdirSync(sourceRoot, { recursive: true }) as string[];
const styles = sourceFiles
  .filter((file) => file.endsWith('.css'))
  .map((file) => ({ file, source: readFileSync(resolve(sourceRoot, file), 'utf8') }));
const paletteSource = readFileSync(resolve(sourceRoot, 'theme/tokens.css'), 'utf8');
const tokens = Object.fromEntries(
  [...paletteSource.matchAll(/(--[\w-]+):\s*([^;]+);/g)].map((match) => [match[1], match[2]]),
);

function contrast(first: string, second: string): number {
  function luminance(hex: string) {
    const [red, green, blue] = [1, 3, 5].map((offset) => {
      const channel = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
      return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
    });
    return red * 0.2126 + green * 0.7152 + blue * 0.0722;
  }
  const a = luminance(first);
  const b = luminance(second);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

function expectContrast(foreground: string, background: string, minimum = 4.5) {
  expect(
    contrast(tokens[foreground], tokens[background]),
    `${foreground} on ${background}`,
  ).toBeGreaterThanOrEqual(minimum);
}

function blendedColor(foreground: string, background: string, opacity: number): string {
  return `#${[1, 3, 5]
    .map((offset) => {
      const foregroundChannel = Number.parseInt(foreground.slice(offset, offset + 2), 16);
      const backgroundChannel = Number.parseInt(background.slice(offset, offset + 2), 16);
      return Math.round(foregroundChannel * opacity + backgroundChannel * (1 - opacity))
        .toString(16)
        .padStart(2, '0');
    })
    .join('')}`;
}

it('keeps literal presentation colors in the application palette across every stylesheet', () => {
  const literals =
    /#[\da-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|lab|lch|color)\s*\(|(?:^|[\s:])(?:white|black|red|green|blue|gray|grey|orange|yellow|purple|pink|cyan|magenta)(?=[\s;,]|$)/i;
  for (const { file, source } of styles) {
    if (file.replaceAll('\\', '/') === 'theme/tokens.css') continue;
    expect(source.replace(/\/\*[\s\S]*?\*\//g, ''), file).not.toMatch(literals);
  }
});

it('defines every CSS token, including properties supplied by component styles', () => {
  const sources = sourceFiles
    .filter((file) => /\.(?:css|tsx)$/.test(file))
    .map((file) => readFileSync(resolve(sourceRoot, file), 'utf8'));
  const definitions = new Set(
    sources.flatMap((source) =>
      [...source.matchAll(/["']?(--[\w-]+)["']?\s*:/g)].map((match) => match[1]),
    ),
  );
  for (const { file, source } of styles) {
    for (const match of source.matchAll(/var\((--[\w-]+)/g)) {
      expect(definitions.has(match[1]), `${file}: ${match[1]}`).toBe(true);
    }
  }
});

it('themes session flyouts and skill-reference editing with shared color roles', () => {
  const navigation = readFileSync(
    resolve(sourceRoot, 'features/agentSessions/sessionNavigation.css'),
    'utf8',
  );
  const chrome = readFileSync(
    resolve(sourceRoot, 'features/agentSessions/compactSessionChrome.css'),
    'utf8',
  );
  expect(navigation).toMatch(
    /\.session-branch-submenu\s*\{[^}]*background:\s*var\(--surface-raised\)/,
  );
  expect(chrome).toMatch(/\.skill-reference\s*\{[^}]*color:\s*var\(--info\)/);
  expect(chrome).toMatch(/\.skill-reference:hover\s*\{[^}]*background:\s*var\(--info-surface\)/);
  expect(chrome).toMatch(/textarea\.composer-skill-input\s*\{[^}]*caret-color:\s*var\(--accent\)/);
});

it('keeps body text, metadata, semantic states and primary actions readable', () => {
  const surfaces = [
    '--canvas',
    '--surface',
    '--surface-raised',
    '--surface-muted',
    '--surface-hover',
    '--surface-selected',
    '--control-background',
    '--code-background',
  ];
  for (const text of ['--text-primary', '--text-secondary', '--text-muted']) {
    for (const surface of surfaces) expectContrast(text, surface);
  }
  for (const state of ['success', 'warning', 'danger', 'info', 'concern']) {
    expectContrast(`--${state}`, `--${state}-surface`);
    for (const surface of surfaces) expectContrast(`--${state}`, surface);
  }
  for (const solid of ['--accent-solid', '--success-solid', '--info-solid']) {
    expectContrast('--text-on-accent', solid);
  }
});

it('checks foreground/background pairs declared together by feature styles', () => {
  for (const { file, source } of styles) {
    for (const rule of source.matchAll(/([^{}]+)\{([^{}]+)\}/g)) {
      const text = rule[2].match(/(?:^|;)\s*color:\s*var\((--[\w-]+)\)\s*;/)?.[1];
      const surface = rule[2].match(
        /(?:^|;)\s*background(?:-color)?:\s*var\((--[\w-]+)\)\s*;/,
      )?.[1];
      if (!text || !surface || !tokens[text]?.startsWith('#') || !tokens[surface]?.startsWith('#'))
        continue;
      expect(
        contrast(tokens[text], tokens[surface]),
        `${file}: ${rule[1].trim()}`,
      ).toBeGreaterThanOrEqual(4.5);
    }
  }
});

it('makes graph lanes, focus and control boundaries visible on dark surfaces', () => {
  for (const color of graphLaneColors) {
    const token = color.slice(4, -1);
    expectContrast(token, '--canvas', 3);
    expectContrast(token, '--surface', 3);
  }
  expectContrast('--focus-ring', '--surface-raised', 3);
  expectContrast('--control-border', '--control-background', 3);
  expect(graphColorForKey('branch:old')).toBe(graphColorForKey('branch:old'));
  expect(graphColorForKey('branch:old')).not.toBe(graphColorForKey('branch:new'));
  expect(graphLaneColors).toContain(graphColorForKey('branch:old'));
});

it('keeps branch connections and anchor outlines visible after their opacity is applied', () => {
  const graphStyles = styles.find(
    ({ file }) => file.replaceAll('\\', '/') === 'features/branches/branchGraph.css',
  )!;
  const rules = [...graphStyles.source.matchAll(/([^{}]+)\{([^{}]+)\}/g)];
  for (const selector of ['.branch-graph__connection', '.branch-graph__anchor circle']) {
    const rule = rules.find((rule) => rule[1].trim() === selector)!;
    const opacity = Number(rule[2].match(/opacity:\s*([\d.]+);/)![1]);
    for (const color of graphLaneColors) {
      const visible = blendedColor(tokens[color.slice(4, -1)], tokens['--canvas'], opacity);
      expect(contrast(visible, tokens['--canvas']), `${selector}: ${color}`).toBeGreaterThanOrEqual(
        3,
      );
    }
  }
});

it('keeps workflow and Sprint connectors readable against their canvas', () => {
  for (const [file, selector, background] of [
    [
      'features/workflowGraph/workflowGraph.css',
      '.workflow-canvas__connections .workflow-connection__visible',
      '--canvas',
    ],
    ['features/orchestrations/styles/sprintFlowMap.css', '.sprint-flow__connector', '--surface'],
  ]) {
    const source = styles.find((style) => style.file.replaceAll('\\', '/') === file)!;
    const rule = [...source.source.matchAll(/([^{}]+)\{([^{}]+)\}/g)].find(
      (match) => match[1].trim() === selector,
    )!;
    const stroke = rule[2].match(/stroke:\s*var\((--[\w-]+)\)/)![1];
    expectContrast(stroke, background, 3);
  }
});

it('uses the dark scheme before React loads and aligns the native window background', () => {
  expect(paletteSource).toMatch(/color-scheme:\s*dark;/);
  expect(readFileSync(resolve('index.html'), 'utf8')).toContain(
    '<meta name="color-scheme" content="dark"',
  );
  const config = JSON.parse(readFileSync(resolve('src-tauri/tauri.conf.json'), 'utf8'));
  expect(config.app.windows[0]).toMatchObject({
    theme: 'Dark',
    backgroundColor: tokens['--canvas'],
  });
});
