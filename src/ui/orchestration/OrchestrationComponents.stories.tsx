import type { Meta, StoryObj } from '@storybook/react-vite';
import { Panel } from '../Panel';
import { ActivityTimeline } from './ActivityTimeline';
import { AgentConversationView } from './AgentConversationView';
import { AgentConversationWindow } from './AgentConversationWindow';
import { ConversationThread } from './ConversationThread';
import { CurrentAction } from './CurrentAction';
import { FileList } from './FileList';
import { StageList } from './StageList';
import { StatusPill } from './StatusPill';
import {
  mockAgentConversationStateExamples,
  mockActivityTimeline,
  mockBackendCompletedResponse,
  mockBackendFailedResponse,
  mockCompletedAgentConversation,
  mockConversationMessages,
  mockDraftOrchestration,
  mockInputReadyAgentConversation,
  mockIntegrationPendingState,
  mockLongStageList,
  mockManyConversationMessages,
  mockPreviewOnlyState,
  mockRunningAgentConversation,
  mockReadyToSubmitPromptState,
  mockRuntimeCompletedEvent,
  mockRuntimeRunningEvent,
  mockStageList,
  mockSubmittingPromptState,
  mockUnsupportedAgentConversation,
  mockTruthfulStateExamples,
  mockUploadedFiles,
  mockWaitingForRuntimeAcknowledgementState,
} from './orchestrationMockFixtures';

const meta = {
  title: 'UI/Orchestration/Components',
  parameters: {
    layout: 'padded',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const TruthfulStatusStates: Story = {
  render: () => (
    <div className="ui-orchestration-story-grid">
      {mockTruthfulStateExamples.map((example) => (
        <Panel
          eyebrow="Storybook fixture"
          footer={example.note}
          key={example.title}
          title={example.title}
        >
          <StatusPill showProvenance state={example.state} />
        </Panel>
      ))}
    </div>
  ),
};

export const CurrentActionStates: Story = {
  render: () => (
    <div className="ui-orchestration-story-stack">
      <CurrentAction
        actionLabel="Submit prompt"
        description="Mock/demo state: prompt is locally ready and no runtime output exists yet."
        onAction={() => undefined}
        state={mockReadyToSubmitPromptState}
        title="Ready to submit prompt"
      />
      <CurrentAction
        actionLabel="Submitting"
        busy
        description="Mock/demo state: local pending request waiting for backend acknowledgement."
        onAction={() => undefined}
        state={mockSubmittingPromptState}
        title="Submitting prompt"
      />
      <CurrentAction
        actionLabel="Unavailable"
        description="Mock/demo state: backend integration is intentionally marked unsupported."
        state={mockIntegrationPendingState}
        title="Integration pending"
      />
      <CurrentAction
        actionLabel="View output"
        description="Runtime event-shaped fixture: this demonstrates evidence-backed completion display only."
        onAction={() => undefined}
        state={mockRuntimeCompletedEvent}
        title="Completed from runtime evidence"
      />
    </div>
  ),
};

export const StageListStates: Story = {
  render: () => (
    <div className="ui-orchestration-story-stack">
      <p className="ui-orchestration-story-note">
        Every item below is a Storybook fixture. The only running/completed states are labelled as
        backend/runtime evidence-shaped examples, not a live orchestration run.
      </p>
      <StageList stages={mockStageList} />
    </div>
  ),
};

export const ConversationStates: Story = {
  render: () => (
    <div className="ui-orchestration-story-stack">
      <ConversationThread messages={mockConversationMessages} />
      <ConversationThread
        emptyLabel="No output yet: empty uploaded files and empty conversation state."
        messages={[]}
      />
    </div>
  ),
};

export const FileListStates: Story = {
  render: () => (
    <div className="ui-orchestration-story-stack">
      <FileList files={mockUploadedFiles} />
      <FileList
        emptyLabel="No uploaded files yet. This story has no runtime artifacts."
        files={[]}
      />
    </div>
  ),
};

export const ActivityTimelineStates: Story = {
  render: () => <ActivityTimeline events={mockActivityTimeline} />,
};

export const LongContentAndManyMessages: Story = {
  render: () => (
    <div className="ui-orchestration-story-stack">
      <StageList stages={mockLongStageList} />
      <ConversationThread messages={mockManyConversationMessages} />
    </div>
  ),
};

export const RuntimeEvidenceExamples: Story = {
  render: () => (
    <div className="ui-orchestration-story-grid">
      <Panel
        eyebrow="Backend response-shaped fixture"
        footer="Storybook sample only; no backend command ran here."
        title="Failed from backend response"
      >
        <StatusPill showProvenance state={mockBackendFailedResponse} />
      </Panel>
      <Panel
        eyebrow="Backend response-shaped fixture"
        footer="Storybook sample only; completion needs evidence like this."
        title="Completed from backend response"
      >
        <StatusPill showProvenance state={mockBackendCompletedResponse} />
      </Panel>
      <Panel
        eyebrow="Runtime event-shaped fixture"
        footer="Storybook sample only; not a live run."
        title="Running from runtime event"
      >
        <StatusPill showProvenance state={mockRuntimeRunningEvent} />
      </Panel>
      <Panel
        eyebrow="Mock preview fixture"
        footer="Mock preview is structurally separate from backend/runtime evidence."
        title="Mock preview"
      >
        <StatusPill showProvenance state={mockPreviewOnlyState} />
      </Panel>
    </div>
  ),
};

export const DraftAndWaitingStates: Story = {
  render: () => (
    <div className="ui-orchestration-story-grid">
      <Panel
        eyebrow="Local draft fixture"
        footer="No generated files, thread IDs, or runtime output are implied."
        title="Local draft"
      >
        <StatusPill showProvenance state={mockDraftOrchestration} />
      </Panel>
      <Panel
        eyebrow="Backend response-shaped fixture"
        footer="The story stops at acknowledgement and does not invent later progress."
        title="Waiting for runtime acknowledgement"
      >
        <StatusPill showProvenance state={mockWaitingForRuntimeAcknowledgementState} />
      </Panel>
    </div>
  ),
};

export const AgentConversationFullViewStates: Story = {
  render: () => (
    <div className="ui-orchestration-story-stack">
      <p className="ui-orchestration-story-note">
        Every conversation below is a Storybook fixture. Running and completed labels appear only
        on runtime event-shaped fixture states.
      </p>
      <AgentConversationView conversation={mockInputReadyAgentConversation} />
      <AgentConversationView conversation={mockRunningAgentConversation} />
      <AgentConversationView conversation={mockCompletedAgentConversation} />
      <AgentConversationView conversation={mockUnsupportedAgentConversation} />
    </div>
  ),
};

export const AgentConversationWindowStates: Story = {
  render: () => (
    <div className="ui-orchestration-story-grid">
      {mockAgentConversationStateExamples.map((conversation) => (
        <AgentConversationWindow conversation={conversation} key={conversation.id} />
      ))}
    </div>
  ),
};
