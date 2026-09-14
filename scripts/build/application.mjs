import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { randomUUID } from 'node:crypto';
import { acquireLock, cacheLayout, selectCache, logSelection } from './cargo.mjs';
import { BuildError, canonical, contained, executable, run } from './process.mjs';

export function frontendConfigPath(worktree, frontend) {
  const relative = path.relative(path.join(worktree, 'src-tauri'), frontend);
  if (path.isAbsolute(relative))
    throw new BuildError(
      'Frontend staging must share the worktree volume. Set ORCHESTRATOR_BUILD_CACHE_ROOT on that volume.',
      'invalid_request',
    );
  // Tauri interprets absolute Windows drive paths as URLs instead of embedded assets.
  return relative.split(path.sep).join('/');
}

export function publishApplication(source, output, debugging, receipt) {
  if (fs.existsSync(output))
    throw new BuildError('The application output already exists.', 'output_unavailable');
  const staging = `${output}.staging`;
  fs.mkdirSync(staging);
  try {
    fs.copyFileSync(source, path.join(staging, path.basename(source)));
    if (debugging) {
      const symbols = path.join(
        path.dirname(source),
        path.parse(source).name.replaceAll('-', '_') + '.pdb',
      );
      if (process.platform === 'win32' && !fs.existsSync(symbols))
        throw new BuildError(
          'The debugging build did not produce application symbols.',
          'output_unavailable',
        );
      if (fs.existsSync(symbols))
        fs.copyFileSync(symbols, path.join(staging, path.basename(symbols)));
    }
    if (receipt)
      fs.writeFileSync(path.join(staging, 'build.json'), JSON.stringify(receipt, null, 2));
    fs.renameSync(staging, output);
    return path.join(output, path.basename(source));
  } catch (error) {
    fs.rmSync(staging, { recursive: true, force: true });
    throw new BuildError(
      `Could not retain application files: ${error.message}`,
      'output_unavailable',
    );
  }
}

function requireFile(filename) {
  if (!fs.existsSync(filename))
    throw new BuildError(
      `Required build dependency is missing: ${filename}`,
      'toolchain_unavailable',
    );
  return filename;
}

export async function buildApplication(request) {
  if (!['release', 'debug'].includes(request.profile))
    throw new BuildError('Application profile must be release or debug.', 'invalid_request');
  const layout = cacheLayout(request.worktreeRoot, request.targetDir);
  const worktree = layout.worktree;
  const attemptRoot = path.resolve(
    request.attemptRoot ?? path.join(layout.storage, '..', 'builds', layout.key, randomUUID()),
  );
  if (
    attemptRoot === worktree ||
    contained(worktree, attemptRoot) ||
    contained(attemptRoot, worktree) ||
    attemptRoot === layout.target ||
    contained(layout.target, attemptRoot) ||
    contained(attemptRoot, layout.target) ||
    contained(layout.storage, attemptRoot)
  )
    throw new BuildError(
      'Retained application storage must be separate from the worktree and compiler cache.',
      'invalid_request',
    );
  fs.mkdirSync(attemptRoot, { recursive: true });
  if (canonical(attemptRoot) !== attemptRoot)
    throw new BuildError(
      'Application storage must not resolve through a linked directory.',
      'invalid_request',
    );
  const release = acquireLock(layout);
  try {
    const profile = request.profile;
    const selection = selectCache(layout, profile, request.cache);
    logSelection(selection, layout, profile);
    const output = path.join(attemptRoot, 'output');
    if (fs.existsSync(output))
      throw new BuildError(
        'This application output has already been published.',
        'output_unavailable',
      );
    requireFile(path.join(worktree, 'package-lock.json'));
    requireFile(path.join(worktree, 'src-tauri', 'Cargo.toml'));
    if (request.dependencyPolicy === 'install') {
      const npm = executable('npm');
      // Invoke npm's JS entrypoint directly to preserve argument boundaries on Windows.
      const npmCli = requireFile(
        path.join(path.dirname(npm), 'node_modules', 'npm', 'bin', 'npm-cli.js'),
      );
      await run(
        process.execPath,
        [
          npmCli,
          'ci',
          '--include=dev',
          '--prefer-offline',
          '--no-audit',
          '--no-fund',
          ...(request.npmCache ? ['--cache', request.npmCache] : []),
        ],
        { cwd: worktree },
      );
    }
    const modules = path.join(worktree, 'node_modules');
    const tsc = requireFile(path.join(modules, 'typescript', 'bin', 'tsc'));
    const vite = requireFile(path.join(modules, 'vite', 'bin', 'vite.js'));
    const tauri = requireFile(path.join(modules, '@tauri-apps', 'cli', 'tauri.js'));
    const frontend = path.join(layout.ownerRoot, 'frontend', profile);
    fs.mkdirSync(frontend, { recursive: true });
    console.log('== TypeScript typecheck ==');
    await run(process.execPath, [tsc, '--noEmit'], { cwd: worktree });
    console.log('== Frontend build ==');
    await run(process.execPath, [vite, 'build', '--outDir', frontend, '--emptyOutDir'], {
      cwd: worktree,
    });
    const runnerJs = fileURLToPath(new URL('./cargo-runner.mjs', import.meta.url));
    const runner = path.join(
      layout.ownerRoot,
      process.platform === 'win32' ? 'cargo-runner.cmd' : 'cargo-runner',
    );
    fs.writeFileSync(
      runner,
      process.platform === 'win32'
        ? `@"${process.execPath}" "${runnerJs}" %*\r\n`
        : `#!/bin/sh\nexec '${process.execPath.replaceAll("'", "'\\''")}' '${runnerJs.replaceAll("'", "'\\''")}' "$@"\n`,
      { mode: 0o755 },
    );
    const context = path.join(layout.ownerRoot, 'cargo-context.json');
    fs.writeFileSync(context, JSON.stringify({ layout, selection }));
    const config = {
      build: { beforeBuildCommand: null, frontendDist: frontendConfigPath(worktree, frontend) },
      bundle: { active: Boolean(request.bundle) },
    };
    const env = {
      ...process.env,
      CARGO_TARGET_DIR: layout.target,
      ORCHESTRATOR_CARGO_CONTEXT: context,
    };
    delete env.CARGO_BUILD_TARGET;
    console.log(`== Tauri ${profile} build ==`);
    await run(
      process.execPath,
      [
        tauri,
        'build',
        ...(profile === 'debug' ? ['--debug'] : []),
        ...(!request.bundle ? ['--no-bundle'] : []),
        '--runner',
        runner,
        '--config',
        JSON.stringify(config),
      ],
      { cwd: worktree, env },
    );
    const binary = request.cargoBinaryName ?? 'codex-orchestrator';
    if (path.basename(binary) !== binary || binary === '.' || binary === '..')
      throw new BuildError('Invalid application binary name.', 'invalid_request');
    const source = requireFile(
      path.join(layout.target, profile, binary + (process.platform === 'win32' ? '.exe' : '')),
    );
    const executablePath = path.join(output, path.basename(source));
    const result = {
      worktreeRoot: worktree,
      attemptRoot,
      outputRoot: output,
      executable: executablePath,
      profile,
      cacheMode: selection.mode,
      cargoTarget: layout.target,
    };
    publishApplication(source, output, profile === 'debug', result);
    console.log(`Application ready: ${executablePath}\nLaunch is a separate action.`);
    return result;
  } finally {
    release();
  }
}
