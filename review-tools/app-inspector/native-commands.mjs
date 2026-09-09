#!/usr/bin/env node
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  assertOwnedDebugger,
  resolveTarget,
  withProtocol,
  loopbackUrl,
  debuggerPort,
} from './owned-webview.mjs';

/** Invokes native application commands. It performs no DOM inspection or UI input. */
export async function invokeNativeCommand({ exe, pid, debugUrl, targetUrl, command, args = {} }) {
  if (!Number.isSafeInteger(pid) || pid <= 0)
    throw new Error('An exact application PID is required');
  if (!/^[a-z][a-z0-9_]*$/.test(command)) throw new Error('Expected a native command name');
  const endpoint = loopbackUrl(debugUrl);
  const ownership = await assertOwnedDebugger({
    executablePath: path.resolve(exe),
    pid,
    debugPort: debuggerPort(endpoint),
  });
  const target = await resolveTarget(endpoint, targetUrl);
  const result = await withProtocol(target.webSocketDebuggerUrl, async (protocol) => {
    const evaluated = await protocol.request('Runtime.evaluate', {
      expression: `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`,
      awaitPromise: true,
      returnByValue: true,
    });
    if (evaluated.exceptionDetails)
      throw new Error(
        evaluated.exceptionDetails.exception?.description ??
          evaluated.exceptionDetails.exception?.value ??
          evaluated.exceptionDetails.text,
      );
    return evaluated.result.value;
  });
  return { command, ownership, result };
}

async function main() {
  const [requestFile, outputFile] = process.argv.slice(2);
  if (!requestFile) throw new Error('Usage: node native-commands.mjs request.json [receipt.json]');
  const receipt = await invokeNativeCommand(JSON.parse(await readFile(requestFile, 'utf8')));
  if (outputFile) await writeFile(outputFile, JSON.stringify(receipt, null, 2));
  else process.stdout.write(JSON.stringify(receipt, null, 2) + '\n');
}
if (import.meta.url === pathToFileURL(path.resolve(process.argv[1] ?? '')).href) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
