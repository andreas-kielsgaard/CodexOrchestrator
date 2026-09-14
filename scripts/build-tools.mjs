#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildApplication } from './build/application.mjs';
import { cargoCommand, clearCache, profileFromArgs } from './build/cargo.mjs';
import { BuildError } from './build/process.mjs';

export function parseArguments(args, cwd = process.cwd()) {
  const request = {
    action: args[0] ?? 'help',
    worktreeRoot: cwd,
    cache: 'auto',
    profile: 'release',
    dependencyPolicy: 'use-existing',
    cargoArgs: [],
  };
  let passthrough = false;
  for (let i = 1; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--') {
      passthrough = true;
      request.cargoArgs.push(arg);
      continue;
    }
    const [name, inline] = arg.split(/=(.*)/s);
    const value = () => {
      const next = inline ?? args[++i];
      if (next === undefined) throw new BuildError(`Missing value for ${name}`, 'invalid_request');
      return next;
    };
    if (name === '--cache') request.cache = value();
    else if (name === '--worktree') request.worktreeRoot = path.resolve(value());
    else if (name === '--target-dir') request.targetDir = path.resolve(value());
    else if (name === '--output') request.attemptRoot = path.resolve(value());
    else if (name === '--request') request.requestFile = path.resolve(value());
    else if (name === '--result') request.resultFile = path.resolve(value());
    else if (name === '--test-debug') request.testDebug = value();
    else if (name === '--debug' && request.action === 'app') request.profile = 'debug';
    else if (name === '--bundle' && request.action === 'app') request.bundle = true;
    else if (name === '--install' && request.action === 'app') request.dependencyPolicy = 'install';
    else if (!passthrough && (name === '--help' || name === '-h')) request.action = 'help';
    else if (name === '--manifest-path')
      throw new BuildError('Use --worktree; this tool owns --manifest-path.', 'invalid_request');
    else request.cargoArgs.push(arg);
  }
  if (request.action === 'app' && request.cargoArgs.length)
    throw new BuildError(
      'Unknown application option: ' + request.cargoArgs.join(' '),
      'invalid_request',
    );
  if (request.action !== 'app') request.profile = profileFromArgs(request.cargoArgs);
  return request;
}

const help = `Application builds and Rust compilation
  npm run build [-- --debug] [-- --cache=auto|local|shared]
  npm run check:rust -- --cache=shared
  npm run test:rust:fast -- --cache=local
  node scripts/build-tools.mjs <app|check|test|build|clear-cache> [options]

  --cache=auto    Prefer local artifacts for this profile; otherwise shared cache if available (default).
  --cache=local   Use normal Cargo reuse/incremental compilation. Best suited to repeated local edits.
  --cache=shared  Use sccache across worktrees; disables incremental compilation for this invocation.
  --worktree DIR Select the checkout to compile. --target-dir DIR overrides its persistent target.
  --output DIR   Retained application destination, outside the checkout (app only).
  --debug        Build a debugging application with symbols; normal applications use release mode.
  --install      Prepare npm dependencies (app only). Otherwise use existing dependencies.
  --bundle       Also generate the configured installer bundles (app only).
  clear-cache    Clear this worktree's compiler target. Retained applications remain runnable.

Build produces a runnable application and prints its path. Launch it separately.
check/test/raw Cargo build preserve Cargo profiles; pass test harness arguments after --.
Use npm run build:frontend when only frontend compilation is needed.
`;

export async function main(args = process.argv.slice(2)) {
  let request;
  try {
    request = parseArguments(args);
    if (request.action === 'help' || request.action === '--help') {
      console.log(help);
      return;
    }
    if (request.requestFile)
      request = {
        ...request,
        ...JSON.parse(fs.readFileSync(request.requestFile, 'utf8')),
        resultFile: request.resultFile,
      };
    let result;
    if (request.action === 'app') result = await buildApplication(request);
    else if (['build', 'check', 'test'].includes(request.action)) await cargoCommand(request);
    else if (request.action === 'clear-cache') clearCache(request);
    else throw new BuildError(`Unknown action: ${request.action}`, 'invalid_request');
    if (request.resultFile)
      fs.writeFileSync(request.resultFile, JSON.stringify({ ok: true, result }));
  } catch (error) {
    if (request?.resultFile)
      fs.writeFileSync(
        request.resultFile,
        JSON.stringify({
          ok: false,
          error: { kind: error.kind ?? 'build_failed', message: error.message },
        }),
      );
    console.error(error.message);
    process.exitCode = error.exitCode ?? 1;
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  await main();
