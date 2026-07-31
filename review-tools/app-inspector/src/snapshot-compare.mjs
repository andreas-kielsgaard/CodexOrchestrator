export function compareSnapshots(before, after, sources = {}) {
  assertSnapshot(before, 'before');
  assertSnapshot(after, 'after');

  const beforeState = before.application.durableState.value ?? null;
  const afterState = after.application.durableState.value ?? null;
  const changes = [];
  collectChanges(beforeState, afterState, 'application.durableState.value', changes);

  const beforeScreenshot = before.application.screenshot.value?.sha256 ?? null;
  const afterScreenshot = after.application.screenshot.value?.sha256 ?? null;
  if (beforeScreenshot !== afterScreenshot) {
    changes.push({
      path: 'application.screenshot.value.sha256',
      before: beforeScreenshot,
      after: afterScreenshot,
    });
  }

  const beforeProcess = before.application.process.value?.pid ?? null;
  const afterProcess = after.application.process.value?.pid ?? null;
  if (beforeProcess !== afterProcess) {
    changes.push({
      path: 'application.process.value.pid',
      before: beforeProcess,
      after: afterProcess,
    });
  }

  return {
    schemaVersion: 'review-app-comparison/v1',
    comparedAt: new Date().toISOString(),
    sources,
    changed: changes.length > 0,
    summary: {
      durableStateChanged: beforeState?.fingerprint !== afterState?.fingerprint,
      screenshotChanged: beforeScreenshot !== afterScreenshot,
      processChanged: beforeProcess !== afterProcess,
    },
    changes,
  };
}

function assertSnapshot(value, label) {
  if (value?.schemaVersion !== 'review-app-observation/v1' || !value.application) {
    throw new Error(`${label} is not a review-app-observation/v1 snapshot.`);
  }
}

function collectChanges(before, after, currentPath, changes) {
  if (Object.is(before, after)) return;
  if (isRecord(before) && isRecord(after)) {
    const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
    for (const key of [...keys].sort()) {
      collectChanges(before[key], after[key], `${currentPath}.${key}`, changes);
    }
    return;
  }
  if (Array.isArray(before) && Array.isArray(after)) {
    const length = Math.max(before.length, after.length);
    for (let index = 0; index < length; index += 1) {
      collectChanges(before[index], after[index], `${currentPath}[${index}]`, changes);
    }
    return;
  }
  changes.push({ path: currentPath, before, after });
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
