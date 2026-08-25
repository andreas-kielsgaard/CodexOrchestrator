import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const workspace = resolve(dirname(scriptPath), '..', '..');
const retainedProfile = Object.freeze({
  id: 'native-profile-ed6f9ea9-f8a8-411b-ba0b-490859dc6126',
  home: 'C:\\Users\\user\\.codex',
});
const receiptPath = join(workspace, '.dev', 'worktree-runtime', 'ps2-live-20260808', 'review-evidence', 'native-sandbox-setup-live.json');
const preservedEnvironmentKeys = Object.freeze([
  'APPDATA', 'COMSPEC', 'HOMEDRIVE', 'HOMEPATH', 'LOCALAPPDATA', 'PATH', 'PATHEXT',
  'SYSTEMROOT', 'TEMP', 'TMP', 'USERPROFILE', 'WINDIR',
]);

export function setupArguments(home = retainedProfile.home) {
  if (home !== retainedProfile.home) throw new Error('This support tool is bound to the retained selected profile home.');
  return ['sandbox', 'setup', '--elevated', '--current-user', '--codex-home', home];
}

export function setupEnvironment(home = retainedProfile.home, source = process.env) {
  setupArguments(home);
  const environment = { CODEX_HOME: home };
  for (const key of preservedEnvironmentKeys) {
    if (source[key]?.trim()) environment[key] = source[key];
  }
  return environment;
}

export function resolveNativeCodex(source = process.env, platform = process.platform, arch = process.arch) {
  if (platform !== 'win32' || arch !== 'x64') throw new Error('This support tool only runs for the retained Windows x64 route.');
  for (const directory of (source.PATH ?? '').split(';').filter(Boolean)) {
    const npmShim = join(directory, 'codex.cmd');
    const native = join(directory, 'node_modules', '@openai', 'codex', 'node_modules', '@openai', 'codex-win32-x64', 'vendor', 'x86_64-pc-windows-msvc', 'bin', 'codex.exe');
    if (existsSync(npmShim) && existsSync(native)) return native;
  }
  throw new Error('No native npm Codex executable is discoverable on PATH.');
}

function atomicWrite(value) {
  mkdirSync(dirname(receiptPath), { recursive: true });
  const temporary = `${receiptPath}.tmp`;
  writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  renameSync(temporary, receiptPath);
}

function currentReceipt() {
  return existsSync(receiptPath) ? JSON.parse(readFileSync(receiptPath, 'utf8')) : null;
}

function isAlive(pid) {
  try { process.kill(pid, 0); return true; } catch { return false; }
}

function versionOf(executable, environment) {
  const result = spawnSync(executable, ['--version'], { cwd: retainedProfile.home, env: environment, encoding: 'utf8', windowsHide: true });
  if (result.status !== 0) return 'unavailable';
  return result.stdout.trim() || 'unavailable';
}

function baseReceipt(executable, environment) {
  return {
    schemaVersion: 'native-sandbox-setup-support/v1',
    operation: 'official_windows_sandbox_setup',
    profile: { id: retainedProfile.id, home: retainedProfile.home },
    executable,
    version: versionOf(executable, environment),
    arguments: setupArguments(),
    workingDirectory: retainedProfile.home,
    environmentKeys: Object.keys(environment).sort(),
    stdout: 'discarded_not_retained',
    stderr: 'discarded_not_retained',
    uac: {
      disposition: 'unobserved',
      boundary: 'The Windows secure desktop is not inspected. A process state never proves UAC interaction or approval.',
    },
  };
}

async function supervise() {
  const environment = setupEnvironment();
  let executable;
  try {
    executable = resolveNativeCodex();
  } catch (error) {
    atomicWrite({ schemaVersion: 'native-sandbox-setup-support/v1', operation: 'official_windows_sandbox_setup', state: 'launch_failed', requestedAt: new Date().toISOString(), error: error instanceof Error ? error.message : 'native executable resolution failed', uac: { disposition: 'unobserved' } });
    return;
  }
  const receipt = { ...baseReceipt(executable, environment), state: 'launching', requestedAt: new Date().toISOString() };
  try {
    const child = spawn(executable, setupArguments(), { cwd: retainedProfile.home, env: environment, stdio: 'ignore', windowsHide: false });
    receipt.state = 'pending';
    receipt.pid = child.pid;
    receipt.launchAcceptedAt = new Date().toISOString();
    atomicWrite(receipt);
    const terminal = await new Promise((resolveTerminal) => child.once('close', (exitCode, signal) => resolveTerminal({ exitCode, signal })));
    atomicWrite({ ...receipt, state: terminal.exitCode === 0 ? 'terminal_succeeded' : 'terminal_failed', exitCode: terminal.exitCode, signal: terminal.signal, settledAt: new Date().toISOString() });
  } catch (error) {
    atomicWrite({ ...receipt, state: 'launch_failed', settledAt: new Date().toISOString(), error: error instanceof Error ? error.message : 'native launch failed' });
  }
}

async function launch() {
  const existing = currentReceipt();
  if (existing?.state === 'pending' && Number.isInteger(existing.pid) && isAlive(existing.pid)) {
    process.stdout.write(`${JSON.stringify({ disposition: 'already_pending', receiptPath, pid: existing.pid })}\n`);
    return;
  }
  const supervisor = spawn(process.execPath, [scriptPath, 'supervise'], { detached: true, stdio: 'ignore', windowsHide: false });
  supervisor.unref();
  process.stdout.write(`${JSON.stringify({ disposition: 'supervisor_started', receiptPath, supervisorPid: supervisor.pid })}\n`);
}

if (process.argv[1] === scriptPath) {
  if (process.argv[2] === 'launch') await launch();
  else if (process.argv[2] === 'supervise') await supervise();
  else throw new Error('Usage: native-sandbox-setup.mjs <launch|supervise>');
}
