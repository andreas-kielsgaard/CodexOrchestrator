import assert from 'node:assert/strict';
import { mkdir, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';

describe('native Tauri review proof', () => {
  it('launches the real shell and crosses the active native-query IPC boundary', async () => {
    const evidenceDir = process.env.NATIVE_REVIEW_EVIDENCE_DIR;
    assert.ok(evidenceDir, 'the evidence directory is configured');
    await mkdir(evidenceDir, { recursive: true });

    const title = await browser.getTitle();
    assert.equal(title, 'Codex Orchestrator');

    const root = await browser.$('#root');
    assert.equal(await root.isDisplayed(), true, 'the application root is displayed');

    const nativeQuery = await browser.tauri.execute(async ({ core }) => {
      console.info('[native-review] invoking load_orchestration_native_query');
      const result = await core.invoke('load_orchestration_native_query');
      console.info('[native-review] native query returned');
      return result;
    });

    assert.equal(nativeQuery.contractVersion, 'orchestration-native-query/v2');
    assert.match(nativeQuery.generatedAt, /^\d{4}-\d{2}-\d{2}T/);
    for (const field of [
      'planningDrafts',
      'agentSessionAssociations',
      'proposalRevisions',
      'recordedProposalEvents',
      'provenanceLinks',
      'initiationCommands',
      'initiationResults',
      'initiationEvents',
      'initiationProvenance',
      'materialSnapshots',
      'initiatedEpics',
      'initiatedSprints',
    ]) {
      assert.deepEqual(nativeQuery[field], [], `${field} starts empty in the isolated app data`);
    }

    const windowSize = await browser.getWindowSize();
    const screenshotPath = path.join(evidenceDir, 'native-shell.png');
    await browser.saveScreenshot(screenshotPath);
    const screenshot = await stat(screenshotPath);
    assert.ok(screenshot.size > 0, 'the native shell screenshot is non-empty');

    await writeFile(
      path.join(evidenceDir, 'assertions.json'),
      `${JSON.stringify(
        {
          title,
          rootDisplayed: true,
          windowSize,
          nativeQuery: {
            contractVersion: nativeQuery.contractVersion,
            generatedAt: nativeQuery.generatedAt,
            emptyCollections: true,
          },
          screenshot: {
            path: screenshotPath,
            bytes: screenshot.size,
          },
        },
        null,
        2,
      )}\n`,
      'utf8',
    );
  });
});
