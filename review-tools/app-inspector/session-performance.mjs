import { writeFileSync } from 'node:fs';
import { assertOwnedDebugger, resolveTarget, withProtocol } from './owned-webview.mjs';

// Read-only navigation and disposable draft typing; never submits a message.
const [exe, pid, port, output] = process.argv.slice(2);
await assertOwnedDebugger({ executablePath: exe, pid: Number(pid), debugPort: Number(port) });
const target = await resolveTarget(`http://127.0.0.1:${port}`, 'http://tauri.localhost/');
await withProtocol(target.webSocketDebuggerUrl, async (protocol) => {
  const evaluate = async (expression) => {
    const response = await protocol.request('Runtime.evaluate', {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (response.exceptionDetails) throw new Error(response.exceptionDetails.text);
    return response.result.value;
  };
  await evaluate(`(() => {
    window.sessionPerf = { ipc: [], input: [], longTasks: [] };
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.sessionPerf.invoke = async function(command, ...args) {
      const start = performance.now();
      try { return await invoke.call(this, command, ...args); }
      finally { window.sessionPerf.ipc.push({command, ms: performance.now() - start}); }
    };
    new PerformanceObserver(list => window.sessionPerf.longTasks.push(...list.getEntries().map(e => ({start:e.startTime, ms:e.duration})))).observe({type:'longtask', buffered:false});
  })()`);
  const ipc = await evaluate(`(async () => {
    const invoke = window.sessionPerf.invoke;
    const navigation = await invoke('load_agent_session_navigation');
    const sessions = [...navigation.summaries].sort((a,b) => b.invocationCount - a.invocationCount).slice(0,3);
    for (const session of sessions) {
      const start = performance.now();
      const details = await invoke('load_agent_session', {query:{sessionId:session.id}});
      window.sessionPerf.ipc.at(-1).turns = details.invocations.length;
      window.sessionPerf.ipc.at(-1).bytes = JSON.stringify(details).length;
    }
    const tab = [...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Agent Sessions');
    tab?.click();
    return sessions.map(s=>s.title);
  })()`);
  const wait = async (condition, timeout = 30000) => {
    const start = Date.now();
    while (!(await evaluate(condition))) {
      if (Date.now() - start > timeout) throw new Error('Timed out: ' + condition);
      await new Promise((r) => setTimeout(r, 100));
    }
  };
  await wait(`document.querySelector('.session-entry') !== null`);
  await evaluate(
    `(() => { const row = [...document.querySelectorAll('.session-entry')].find(e => e.getAttribute('aria-label') === ${JSON.stringify(ipc[0])}); (row ?? document.querySelector('.session-entry')).click(); })()`,
  );
  await wait(
    `document.querySelector('textarea[aria-label="Message"]') !== null && !document.querySelector('.agent-session-transcript-loading')`,
  );
  await protocol.request('Profiler.enable');
  await protocol.request('Profiler.start');
  const original = await evaluate(
    `(() => {const e=document.querySelector('textarea[aria-label="Message"]');e.focus();return e.value;})()`,
  );
  for (let i = 0; i < 80; i++) {
    await evaluate(`window.sessionPerf.start = performance.now()`);
    await protocol.request('Input.insertText', { text: 'x' });
    await evaluate(
      `new Promise(resolve => requestAnimationFrame(() => {window.sessionPerf.input.push(performance.now()-window.sessionPerf.start);resolve();}))`,
    );
  }
  const profile = await protocol.request('Profiler.stop');
  const metrics = await evaluate('window.sessionPerf');
  await evaluate(
    `(() => {const e=document.querySelector('textarea[aria-label="Message"]');Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set.call(e,${JSON.stringify(original)});e.dispatchEvent(new Event('input',{bubbles:true}));})()`,
  );
  writeFileSync(output, JSON.stringify({ ...metrics, profile: profile.profile }, null, 2));
  const sorted = [...metrics.input].sort((a, b) => a - b);
  console.log(
    JSON.stringify(
      {
        ipc: metrics.ipc,
        input: {
          median: sorted[Math.floor(sorted.length / 2)],
          p95: sorted[Math.floor(sorted.length * 0.95)],
          max: sorted.at(-1),
        },
        longTasks: metrics.longTasks,
      },
      null,
      2,
    ),
  );
});
