import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import { execFile } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { parseInteractionOutput } from '../interact-app.mjs';

const prefix = 'REVIEW_APP_INTERACTION_V1:';
const execFileAsync = promisify(execFile);
const here = path.dirname(fileURLToPath(import.meta.url));
const toolRoot = path.resolve(here, '..');
const frame = (value) =>
  `${prefix}${Buffer.from(JSON.stringify(value), 'utf8').toString('base64')}`;

test('parses one interaction receipt while ignoring incidental native output', () => {
  const expected = { transport: { foregrounded: false, delivery: 'window_messages_acknowledged' } };
  assert.deepEqual(parseInteractionOutput(`banner\n${frame(expected)}\ntrailer`), expected);
});

test('rejects missing and duplicate interaction receipts', () => {
  assert.throws(() => parseInteractionOutput('none'), /observed 0/u);
  const value = frame({ transport: {} });
  assert.throws(() => parseInteractionOutput(`${value}\n${value}`), /observed 2/u);
});

test('removes paste authority before any target is selected', async () => {
  const executable = path.join(toolRoot, 'interact-app.mjs');
  await assert.rejects(
    execFileAsync(process.execPath, [
      executable,
      'paste',
      '--exe',
      'C:\\not-a-target.exe',
      '--pid',
      '1',
      '--x',
      '0',
      '--y',
      '0',
    ]),
    /Unknown action: paste/u,
  );
});

test('PowerShell boundary rejects unsafe coordinates before process lookup', async () => {
  const adapter = path.join(toolRoot, 'adapters', 'windows-interaction.ps1');
  await assert.rejects(
    execFileAsync('powershell.exe', [
      '-NoProfile',
      '-File',
      adapter,
      '-ExecutablePath',
      'C:\\not-a-target.exe',
      '-ProcessId',
      '1',
      '-Action',
      'click',
      '-X',
      '-1',
      '-Y',
      '0',
    ]),
    /0 through 32767/u,
  );
});

test('Windows adapter contains no clipboard operation', async () => {
  const adapter = await readFile(
    path.join(toolRoot, 'adapters', 'windows-interaction.ps1'),
    'utf8',
  );
  assert.doesNotMatch(adapter, /Set-Clipboard|Get-Clipboard|REVIEW_APP_INTERACTION_TEXT/u);
});
