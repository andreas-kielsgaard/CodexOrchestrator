import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

export class BuildError extends Error {
  constructor(message, kind = 'build_failed', exitCode = 1) {
    super(message);
    this.kind = kind;
    this.exitCode = exitCode;
  }
}

export function executable(name, env = process.env) {
  const candidates = process.platform === 'win32' ? [`${name}.exe`, `${name}.cmd`, name] : [name];
  for (const directory of (env.PATH ?? env.Path ?? '').split(path.delimiter)) {
    for (const candidate of candidates) {
      const full = path.join(directory.replace(/^"|"$/g, ''), candidate);
      if (fs.existsSync(full) && fs.statSync(full).isFile()) return full;
    }
  }
  throw new BuildError(`${name} is required and could not be found.`, 'toolchain_unavailable');
}

export function capture(program, args, options = {}) {
  return execFileSync(program, args, { encoding: 'utf8', windowsHide: true, ...options }).trim();
}

export function run(program, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(program, args, { stdio: 'inherit', windowsHide: true, ...options });
    child.on('error', (error) =>
      reject(
        new BuildError(`Could not start ${program}: ${error.message}`, 'toolchain_unavailable'),
      ),
    );
    child.on('exit', (code, signal) => {
      if (code === 0) resolve();
      else
        reject(
          new BuildError(
            `${path.basename(program)} failed (${signal ?? code}).`,
            'build_failed',
            code ?? 1,
          ),
        );
    });
  });
}

export function contained(root, item) {
  const relative = path.relative(path.resolve(root), path.resolve(item));
  return (
    relative !== '' &&
    !relative.startsWith(`..${path.sep}`) &&
    relative !== '..' &&
    !path.isAbsolute(relative)
  );
}

export function canonical(directory) {
  return fs.realpathSync.native(directory).replace(/^\\\\\?\\/, '');
}
