import { createHash } from 'node:crypto';
import { execFile } from 'node:child_process';
import { createReadStream } from 'node:fs';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export async function inspectSourceAndExecutable({ workspaceRoot, executablePath }) {
  const [source, executable, product] = await Promise.all([
    inspectGit(workspaceRoot),
    inspectExecutable(executablePath),
    inspectProduct(workspaceRoot),
  ]);
  return { source, executable, product };
}

async function inspectGit(workspaceRoot) {
  try {
    const [head, branch, status] = await Promise.all([
      git(workspaceRoot, ['rev-parse', 'HEAD']),
      git(workspaceRoot, ['branch', '--show-current']),
      git(workspaceRoot, ['status', '--porcelain=v1', '--untracked-files=all']),
    ]);
    return observed('Git CLI in requested workspace', {
      workspaceRoot,
      head,
      branch: branch || null,
      dirty: status.length > 0,
      changedPathCount: status ? status.split(/\r?\n/u).length : 0,
    });
  } catch (error) {
    return unavailable(`Git source inspection failed: ${message(error)}`);
  }
}

async function inspectExecutable(executablePath) {
  try {
    const details = await stat(executablePath);
    if (!details.isFile()) return unavailable(`${executablePath} is not a regular file.`);
    return observed('filesystem metadata and SHA-256', {
      path: executablePath,
      bytes: details.size,
      modifiedAt: details.mtime.toISOString(),
      sha256: await hashFile(executablePath),
    });
  } catch (error) {
    return unavailable(`Executable inspection failed: ${message(error)}`);
  }
}

async function inspectProduct(workspaceRoot) {
  try {
    const [packageText, tauriText] = await Promise.all([
      readFile(path.join(workspaceRoot, 'package.json'), 'utf8'),
      readFile(path.join(workspaceRoot, 'src-tauri', 'tauri.conf.json'), 'utf8'),
    ]);
    const packageJson = JSON.parse(packageText);
    const tauri = JSON.parse(tauriText);
    return observed('workspace package and Tauri configuration', {
      packageName: packageJson.name ?? null,
      productName: tauri.productName ?? null,
      version: tauri.version ?? packageJson.version ?? null,
      identifier: tauri.identifier ?? null,
    });
  } catch (error) {
    return unavailable(`Product metadata inspection failed: ${message(error)}`);
  }
}

async function git(cwd, arguments_) {
  const { stdout } = await execFileAsync('git', ['-C', cwd, ...arguments_], {
    encoding: 'utf8',
    windowsHide: true,
    timeout: 10_000,
  });
  return stdout.trim();
}

async function hashFile(filePath) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(filePath)) hash.update(chunk);
  return hash.digest('hex');
}

function observed(source, value) {
  return { disposition: 'observed', source, value };
}

function unavailable(reason) {
  return { disposition: 'unavailable', reason };
}

function message(error) {
  return error instanceof Error ? error.message : String(error);
}
