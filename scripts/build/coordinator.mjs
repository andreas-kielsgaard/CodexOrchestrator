import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  cacheLayout,
  acquireLock,
  selectCache,
  cargoEnvironment,
  reportStatistics,
} from './cargo.mjs';
import { capture, executable, run } from './process.mjs';

export async function buildCoordinator(layout, selection, profile, staging) {
  const env = cargoEnvironment(selection);
  delete env.CARGO_BUILD_TARGET;
  const cargo = executable('cargo');
  try {
    await run(
      cargo,
      [
        'build',
        '--locked',
        '--manifest-path',
        path.join(layout.worktree, 'crates/orchid-coordinator/Cargo.toml'),
        '--target-dir',
        layout.target,
        ...(profile === 'release' ? ['--release'] : []),
      ],
      { cwd: layout.worktree, env },
    );
  } finally {
    reportStatistics(selection, env);
  }
  const suffix = process.platform === 'win32' ? '.exe' : '';
  const binary = path.join(layout.target, profile, `orchid-coordinator${suffix}`);
  const host = capture(executable('rustc'), ['-vV']).match(/^host: (.+)$/m)?.[1];
  if (!host) throw new Error('Cannot determine coordinator packaging target');
  fs.mkdirSync(staging, { recursive: true });
  const externalBin = path.join(staging, 'orchid-coordinator');
  fs.copyFileSync(binary, `${externalBin}-${host}${suffix}`);
  return { binary, externalBin };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const layout = cacheLayout(process.cwd());
  const release = acquireLock(layout);
  try {
    const selection = selectCache(layout, 'debug', 'auto');
    const result = await buildCoordinator(
      layout,
      selection,
      'debug',
      path.join(layout.ownerRoot, 'coordinator'),
    );
    const dest = path.join(process.cwd(), 'src-tauri/target/debug');
    fs.mkdirSync(dest, { recursive: true });
    if (path.resolve(result.binary) !== path.join(dest, path.basename(result.binary)))
      fs.copyFileSync(result.binary, path.join(dest, path.basename(result.binary)));
  } finally {
    release();
  }
}
