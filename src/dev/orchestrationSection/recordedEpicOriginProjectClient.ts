import type {
  EpicOriginProject,
  EpicOriginProjectClient,
} from '../../application/epicOriginProject';

const recordedProject: EpicOriginProject = {
  name: 'Codex Orchestrator',
  path: 'C:\\Users\\user\\Documents\\Code Projects\\Codex Orchestrator',
  gitDetected: true,
  repositoryRoot: 'C:\\Users\\user\\Documents\\Code Projects\\Codex Orchestrator',
  branches: [
    {
      name: 'main',
      revision: 'ca8f6729ff6f',
      relationship: 'related',
      ahead: 0,
      behind: 0,
      forkRevision: 'ca8f6729ff6f',
      isCurrent: false,
      isBaseline: true,
    },
    {
      name: 'codex/epic-project-selection',
      revision: 'c7d1e91607b3',
      parentName: 'main',
      relationship: 'related',
      ahead: 4,
      behind: 0,
      forkRevision: 'ca8f6729ff6f',
      isCurrent: true,
      isBaseline: false,
    },
    {
      name: 'codex/epic-workflow-presentation',
      revision: '78d3e08ad244',
      parentName: 'main',
      relationship: 'related',
      ahead: 3,
      behind: 0,
      forkRevision: 'ca8f6729ff6f',
      isCurrent: false,
      isBaseline: false,
    },
    {
      name: 'codex/workflow-engine',
      revision: '5ed4ea80a6cc',
      parentName: 'main',
      relationship: 'related',
      ahead: 2,
      behind: 0,
      forkRevision: 'ca8f6729ff6f',
      isCurrent: false,
      isBaseline: false,
    },
  ],
};

export const recordedEpicOriginProjectClient: EpicOriginProjectClient = {
  async chooseProject() {
    return recordedProject;
  },
  async inspectProject() {
    return recordedProject;
  },
};
