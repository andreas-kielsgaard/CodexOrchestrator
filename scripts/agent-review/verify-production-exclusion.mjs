import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const forbiddenBuildStrings = [
  'Agent Review Lab',
  'recorded-plan-builder',
  '--remote-debugging-port',
  'DevToolsActivePort',
  'tauri-plugin-wdio',
  '@wdio/',
];
const forbiddenProductionConfigStrings = [
  'withGlobalTauri',
  '--remote-debugging-port',
  'DevToolsActivePort',
  'wdio',
];

const buildFiles = await listFiles(path.join(root, 'dist'));
const buildViolations = await findMatches(buildFiles, forbiddenBuildStrings);
const productionConfigFiles = [
  path.join(root, 'src-tauri', 'tauri.conf.json'),
  path.join(root, 'src-tauri', 'capabilities', 'default.json'),
];
const configViolations = await findMatches(productionConfigFiles, forbiddenProductionConfigStrings);
const violations = [...buildViolations, ...configViolations];

if (violations.length > 0) {
  throw new Error(
    `Development review facilities entered production output:\n${violations.join('\n')}`,
  );
}

process.stdout.write(
  `Production exclusion verified across ${buildFiles.length} built files and ${productionConfigFiles.length} normal Tauri configuration files.\n`,
);

async function listFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(
    entries.map((entry) => {
      const candidate = path.join(directory, entry.name);
      return entry.isDirectory() ? listFiles(candidate) : [candidate];
    }),
  );
  return nested.flat();
}

async function findMatches(files, patterns) {
  const matches = [];
  for (const file of files) {
    const contents = await readFile(file, 'utf8');
    for (const pattern of patterns) {
      if (contents.includes(pattern)) matches.push(`${path.relative(root, file)}: ${pattern}`);
    }
  }
  return matches;
}
