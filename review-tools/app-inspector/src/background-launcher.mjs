import { spawn } from 'node:child_process';
import { mkdir, open } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

import { writeDurableJson } from './durable-file.mjs';

export async function launchDetachedWait({
  scriptPath,
  waitArguments,
  workspaceRoot,
  watcherLogPath,
  launchResultPath,
  waitResultPath,
  evidenceRoot,
  timeoutMs,
  cancelFilePath,
}) {
  const watcherLog = path.resolve(watcherLogPath);
  const launchResult = path.resolve(launchResultPath);
  const logHandle = await openAfterCreatingParent(watcherLog);
  let child;
  try {
    child = spawn(process.execPath, [scriptPath, 'wait', ...waitArguments], {
      cwd: workspaceRoot,
      detached: true,
      shell: false,
      windowsHide: true,
      stdio: ['ignore', logHandle.fd, logHandle.fd],
    });
    child.unref();
    const result = {
      schemaVersion: 'review-app-wait-launch/v1',
      launchedAt: new Date().toISOString(),
      watcherPid: child.pid,
      watcher: {
        executable: process.execPath,
        argumentCount: waitArguments.length + 2,
        shell: false,
        timeoutMs,
      },
      artifacts: {
        launchResult,
        watcherLog,
        waitResult: path.resolve(waitResultPath),
        evidenceRoot: path.resolve(evidenceRoot),
        callbackReceipt: null,
        callbackLog: null,
      },
      cancellation: {
        cancelFile: path.resolve(cancelFilePath),
        powershell: `New-Item -ItemType File -Path "${path.resolve(cancelFilePath)}" -Force`,
        effect: 'Requests graceful watcher cancellation; the watcher finalizes cancelled evidence.',
        emergencyPowershell: `Stop-Process -Id ${child.pid}`,
        emergencyEffect:
          'Forces termination and may prevent cancellation evidence finalization; use only if graceful cancellation does not exit.',
      },
      cleanup: {
        instruction:
          'After the watcher PID has exited, remove only the cancel file, or the explicit evidenceRoot if its retained review evidence is no longer needed.',
        target: path.resolve(evidenceRoot),
      },
    };
    await writeDurableJson(launchResult, result);
    return result;
  } catch (error) {
    if (child?.pid) child.kill();
    throw error;
  } finally {
    await logHandle.close();
  }
}

async function openAfterCreatingParent(filePath) {
  await mkdir(path.dirname(filePath), { recursive: true });
  return open(filePath, 'a');
}
