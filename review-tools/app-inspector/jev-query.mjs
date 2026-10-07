#!/usr/bin/env node
import { readFile, writeFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { invokeNativeCommand } from './native-commands.mjs';

const [requestFile, outputFile] = process.argv.slice(2);
if (!requestFile || requestFile === '--help') {
  console.log('Usage: node review-tools/app-inspector/jev-query.mjs request.json [result.json]');
  console.log(
    'Request: exe, pid, debugUrl, targetUrl, evaluation; optional model, title, sessionId, invocationId.',
  );
} else {
  try {
    const request = JSON.parse(await readFile(requestFile, 'utf8'));
    const { exe, pid, debugUrl, targetUrl } = request;
    const owner = { exe, pid, debugUrl, targetUrl };
    const call = async (command, args) =>
      (await invokeNativeCommand({ ...owner, command, args })).result;
    const sessionId =
      request.sessionId ??
      (
        await call('create_jev_api_session', {
          input: { title: request.title ?? 'Jev CLI query', model: request.model ?? 'jev-latest' },
        })
      ).id;
    const invocationId = request.invocationId ?? randomUUID();
    const receipt = await call('invoke_jev_api_session', {
      input: { sessionId, invocationId, evaluation: request.evaluation },
    });
    const deadline = Date.now() + 45_000;
    let view;
    do {
      view = await call('load_jev_api_invocation', { sessionId, invocationId });
      if (['completed', 'failed', 'canceled', 'interrupted'].includes(view.invocation.status))
        break;
      if (Date.now() >= deadline)
        throw new Error(
          `Timed out waiting for Jev invocation ${invocationId}; inspect its durable state.`,
        );
      await new Promise((resolve) => setTimeout(resolve, 100));
    } while (true);
    const output = { sessionId, invocationId, launchAccepted: receipt.launchAccepted, ...view };
    const json = JSON.stringify(output, null, 2) + '\n';
    if (outputFile) await writeFile(outputFile, json);
    else process.stdout.write(json);
    if (view.invocation.status !== 'completed' || !view.result) process.exitCode = 1;
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
