import { canClaimAgentConversationActiveWork } from '../domain/agentConversation';
import { mapBuildPackageToAgentConversation, type OrchestrationBuildPackage } from './orchestrationClient';

describe('mapBuildPackageToAgentConversation', () => {
  it('maps unsupported draft packages without claiming a runtime thread or active work', () => {
    const buildPackage: OrchestrationBuildPackage = {
      clientState: {
        currentAction: 'Draft saved locally; runtime route unavailable.',
        notices: [
          {
            id: 'notice-1',
            kind: 'missing_capability',
            message: 'Plan Builder runtime is not wired for this draft.',
            title: 'Runtime unsupported',
            truth: {
              provenance: 'unsupported',
              status: 'integration_pending',
            },
          },
        ],
        persisted: true,
        provenance: 'unsupported',
        runtimeSupported: false,
        status: 'integration_pending',
      },
      createdAt: '2026-07-09T10:00:00.000Z',
      files: [
        {
          id: 'file-1',
          name: 'notes.md',
          size: 42,
        },
      ],
      folderPath: 'C:\\fixture',
      generatedFiles: [],
      id: 'build-package-1',
      messages: [
        {
          body: 'Storybook/test fixture user input only.',
          createdAt: '2026-07-09T10:01:00.000Z',
          id: 'message-1',
          role: 'user',
          truth: {
            provenance: 'user_input',
            status: 'ready',
          },
        },
      ],
      planPreview: [],
      sourcePrompt: 'Storybook/test fixture user input only.',
      stages: [],
      title: 'Fixture draft',
      updatedAt: '2026-07-09T10:02:00.000Z',
    };

    const conversation = mapBuildPackageToAgentConversation(buildPackage);

    expect(conversation.id).toBe('build-package-1');
    expect(conversation.externalThreadId).toBeUndefined();
    expect(conversation.mode).toBe('read_only');
    expect(conversation.state.streamStatus).toBe('unsupported');
    expect(conversation.state.unavailable?.detail).toContain('not wired');
    expect(canClaimAgentConversationActiveWork(conversation)).toBe(false);
    expect(conversation.turns[0]).toMatchObject({
      body: 'Storybook/test fixture user input only.',
      role: 'user',
      truth: {
        provenance: 'user_input',
        status: 'ready',
      },
    });
    expect(conversation.attachments[0]).toMatchObject({
      kind: 'uploaded',
      name: 'notes.md',
      truth: {
        provenance: 'local_draft',
        status: 'draft',
      },
    });
  });
});

