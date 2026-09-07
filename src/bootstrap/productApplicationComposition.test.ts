import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createProductApplicationComposition } from './productApplicationComposition';

describe('product application composition', () => {
  it('boots with an honest unavailable native-query state and unsupported effect boundaries', async () => {
    const composition = createProductApplicationComposition();

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
    expect(composition.contextualFileReviewClient).toBeDefined();
    expect(composition.productDecisionClient).toBeDefined();
  });

  it('keeps product startup free of development fixture authority', () => {
    for (const file of ['src/main.tsx', 'src/app/App.tsx']) {
      expect(readFileSync(resolve(file), 'utf8')).not.toMatch(
        /disposableRecordedOrchestrationView|recordedDevelopment|recordedOrchestrationClient/,
      );
    }
  });

  it('mounts replacement execution, Workflow, identity, Session Profile, and event boundaries', () => {
    const composition = createProductApplicationComposition();

    expect(composition.executionConfigurationClient).toBeDefined();
    expect(composition.workflowAuthoringClient).toBeDefined();
    expect(composition.workflowInstanceClient).toBeDefined();
    expect(composition.draftCloseGuard).toBeDefined();
    expect(composition.identityManagementClient).toBeDefined();
    expect(composition.agentSessionProfileClient).toBeDefined();
    expect(composition.sessionEventQueryClient).toBeDefined();
    expect(composition.agentSessionHarnessManagementSource).toBeUndefined();
  });
});
