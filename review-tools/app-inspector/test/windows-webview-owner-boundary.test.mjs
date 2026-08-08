import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import net from 'node:net';
import path from 'node:path';
import process from 'node:process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const here = path.dirname(fileURLToPath(import.meta.url));
const adapter = path.resolve(here, '..', 'adapters', 'windows-webview-owner.ps1');

test('owner adapter rejects a debug port with zero loopback listener endpoints', async () => {
  const port = await reserveThenReleasePort();
  await assert.rejects(
    runOwnerAdapter({ ownerPid: process.pid, port }),
    /exactly one loopback listener endpoint.*observed 0/u,
  );
});

test('owner adapter rejects a non-loopback debug listener endpoint', async () => {
  const listener = await startListener('0.0.0.0');
  try {
    await assert.rejects(
      runOwnerAdapter({ ownerPid: listener.pid, port: listener.port }),
      /non-loopback listener endpoint/u,
    );
  } finally {
    listener.child.kill();
  }
});

function runOwnerAdapter({ ownerPid, port }) {
  return execFileAsync('powershell.exe', [
    '-NoProfile',
    '-File',
    adapter,
    '-OwnerExecutablePath',
    process.execPath,
    '-OwnerProcessId',
    String(ownerPid),
    '-DebugPort',
    String(port),
  ]);
}

async function reserveThenReleasePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen({ host: '127.0.0.1', port: 0 }, resolve);
  });
  const port = server.address().port;
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  return port;
}

function startListener(host) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      [
        '-e',
        "const net=require('net');const server=net.createServer();server.listen({host:process.argv[1],port:0},()=>console.log(server.address().port));setInterval(()=>{},1000)",
        host,
      ],
      { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true },
    );
    let output = '';
    child.stdout.setEncoding('utf8');
    child.stdout.on('data', (chunk) => {
      output += chunk;
      const port = Number.parseInt(output.trim(), 10);
      if (Number.isSafeInteger(port) && port > 0) {
        resolve({ child, pid: child.pid, port });
      }
    });
    child.once('error', reject);
    child.once('exit', (code) => reject(new Error(`listener exited before binding (${code})`)));
  });
}
