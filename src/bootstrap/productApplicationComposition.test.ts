import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createProductApplicationComposition } from './productApplicationComposition';

describe('product application composition', () => {
  it('boots with an honest unavailable native-query state and unsupported effect boundaries', async () => {
    const composition = createProductApplicationComposition();

    expect(composition.developmentSurface).toBeUndefined();
    await expect(composition.orchestrationClient.load()).resolves.toEqual({
      kind: 'unavailable',
      reason: 'The durable orchestration query is unavailable.',
    });
    await expect(
      composition.artifactAccessController?.copyPath({
        documentRefId: 'document-1',
        title: 'Product document',
        artifactIds: ['artifact-1'],
      }),
    ).resolves.toMatchObject({ status: 'unsupported' });
    await expect(
      composition.sprintAutomaticContinuationPolicyController?.updatePolicy({
        level: 'sprint',
        sprintId: 'sprint-1',
        policyId: 'policy-1',
        automaticEnabled: true,
      }),
    ).resolves.toMatchObject({ status: 'unsupported' });
  });

  it('keeps product startup free of development fixture authority', () => {
    for (const file of ['src/main.tsx', 'src/app/App.tsx']) {
      expect(readFileSync(resolve(file), 'utf8')).not.toMatch(
        /disposableRecordedOrchestrationView|recordedDevelopment|recordedOrchestrationClient/,
      );
    }
  });
});
