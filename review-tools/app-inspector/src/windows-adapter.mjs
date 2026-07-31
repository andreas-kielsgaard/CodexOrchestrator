import { Buffer } from 'node:buffer';
import { execFile } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { TextDecoder } from 'node:util';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const payloadPrefix = 'REVIEW_APP_JSON_V1:';

export async function inspectWindowsApplication({
  toolRoot,
  executablePath,
  pid,
  screenshotPath,
  skipAccessibility = false,
  signal,
  timeoutMs = 15_000,
}) {
  if (process.platform !== 'win32') {
    const reason = 'The provisional native-window adapter currently supports Windows only.';
    return {
      process: unavailable(reason),
      window: unavailable(reason),
      screenshot: unavailable(reason),
      accessibility: unavailable(reason),
    };
  }

  const script = path.join(toolRoot, 'adapters', 'windows-window.ps1');
  const arguments_ = [
    '-NoProfile',
    '-ExecutionPolicy',
    'Bypass',
    '-File',
    script,
    '-ExecutablePath',
    executablePath,
  ];
  if (pid) arguments_.push('-ProcessId', String(pid));
  if (screenshotPath) arguments_.push('-ScreenshotPath', screenshotPath);
  if (skipAccessibility) arguments_.push('-SkipAccessibility');

  try {
    const { stdout, stderr } = await execFileAsync('powershell.exe', arguments_, {
      encoding: 'utf8',
      windowsHide: true,
      timeout: timeoutMs,
      signal,
      maxBuffer: 2 * 1024 * 1024,
    });
    return {
      ...parseWindowsAdapterOutput(stdout),
      diagnostics: adapterDiagnostics(stderr),
    };
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    return {
      process: unavailable(`Windows process inspection failed: ${detail}`),
      window: unavailable('No window observation was produced.'),
      screenshot: unavailable('No screenshot was produced.'),
      accessibility: unavailable('No accessibility observation was produced.'),
      diagnostics: adapterDiagnostics(error?.stderr),
    };
  }
}

export function parseWindowsAdapterOutput(stdout) {
  const frames = String(stdout ?? '')
    .split(/\r?\n/u)
    .filter((line) => line.startsWith(payloadPrefix));
  if (frames.length !== 1) {
    throw new Error(
      `Windows adapter expected exactly one ${payloadPrefix} frame; observed ${frames.length}.`,
    );
  }

  const encoded = frames[0].slice(payloadPrefix.length);
  if (
    encoded.length === 0 ||
    encoded.length % 4 !== 0 ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u.test(encoded)
  ) {
    throw new Error('Windows adapter frame is not canonical base64.');
  }

  const bytes = Buffer.from(encoded, 'base64');
  if (bytes.toString('base64') !== encoded) {
    throw new Error('Windows adapter frame is not canonical base64.');
  }

  let json;
  try {
    json = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    throw new Error('Windows adapter frame is not valid UTF-8.');
  }

  let value;
  try {
    value = JSON.parse(json);
  } catch (error) {
    throw new Error(`Windows adapter frame contains malformed JSON: ${error.message}`);
  }
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('Windows adapter frame must contain a JSON object.');
  }
  return value;
}

function adapterDiagnostics(stderr) {
  const text = typeof stderr === 'string' ? stderr.trim() : '';
  return {
    source: 'PowerShell stderr',
    stderr: text ? text.slice(0, 4096) : null,
    truncated: text.length > 4096,
  };
}

function unavailable(reason) {
  return { disposition: 'unavailable', reason };
}
