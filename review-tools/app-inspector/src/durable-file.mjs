import { mkdir, open, rename, unlink } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

export async function writeDurableJson(filePath, value) {
  await writeDurableText(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

export async function writeDurableText(filePath, value) {
  const target = path.resolve(filePath);
  await mkdir(path.dirname(target), { recursive: true });
  const temporary = `${target}.${process.pid}.${Date.now()}.tmp`;
  let handle;
  try {
    handle = await open(temporary, 'wx');
    await handle.writeFile(value, 'utf8');
    await handle.sync();
    await handle.close();
    handle = undefined;
    await rename(temporary, target);
  } catch (error) {
    await handle?.close().catch(() => {});
    await unlink(temporary).catch(() => {});
    throw error;
  }
}

export async function createExclusiveDurableJson(filePath, value) {
  const target = path.resolve(filePath);
  await mkdir(path.dirname(target), { recursive: true });
  const handle = await open(target, 'wx');
  try {
    await handle.writeFile(`${JSON.stringify(value, null, 2)}\n`, 'utf8');
    await handle.sync();
  } finally {
    await handle.close();
  }
}
