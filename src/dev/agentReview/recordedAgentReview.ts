import type {
  AgentReviewDisposition,
  AgentReviewEvidenceLane,
} from '../../application/agentReview';

export type AgentReviewLaneStatus = 'verified' | 'under-evaluation';

export interface AgentReviewEvidenceFile {
  readonly label: string;
  readonly path: string;
}

export interface AgentReviewLaneRecord {
  readonly id: string;
  readonly ordinal: string;
  readonly title: string;
  readonly scope: string;
  readonly evidenceLane: AgentReviewEvidenceLane;
  readonly dispositionKind: AgentReviewDisposition;
  readonly status: AgentReviewLaneStatus;
  readonly statusLabel: string;
  readonly request: string;
  readonly evidence: string;
  readonly disposition: string;
  readonly metadata: readonly Readonly<{ label: string; value: string }>[];
  readonly evidenceFiles: readonly AgentReviewEvidenceFile[];
  readonly reproduction: readonly string[];
  readonly reproductionNote?: string;
  readonly action?: Readonly<{ label: string; href: string }>;
  readonly unverifiedClaims: readonly string[];
}

export interface AgentReviewLabRecord {
  readonly repository: Readonly<{
    revision: string;
    branch: string;
    worktree: string;
  }>;
  readonly lanes: readonly AgentReviewLaneRecord[];
  readonly worktreeHandoff: Readonly<{
    status: string;
    request: string;
    instance: string;
    review: string;
    unverified: string;
  }>;
  readonly boundaries: readonly Readonly<{ label: string; detail: string }>[];
}

const repository = {
  revision: 'f23f5fdcd3ae9298261e81db52366854c00dc4a0',
  branch: 'codex/explore-agent-app-review',
  worktree: 'C:\\Users\\user\\.codex\\worktrees\\5634\\Codex Orchestrator',
} as const;

export const recordedAgentReviewLab: AgentReviewLabRecord = {
  repository,
  lanes: [
    {
      id: 'deterministic-renderer',
      ordinal: '01',
      title: 'Deterministic renderer',
      scope: 'Recorded browser scenario',
      evidenceLane: 'deterministic-verification',
      dispositionKind: 'user-review-required',
      status: 'verified',
      statusLabel: 'Verified run',
      request: 'Inspect the recorded Plan Builder layout and exercise its semantic Sprint control.',
      evidence:
        'Playwright Test opened the development-only scenario, asserted the workspace and proposal rail, and collapsed and expanded Sprint 1.',
      disposition:
        'Behavior assertions passed. The retained screenshots still require review before visual fidelity is accepted.',
      metadata: [
        { label: 'Application', value: 'Vite development server · recorded Plan Builder' },
        { label: 'Driver', value: 'Playwright Test 1.61.1 · Microsoft Edge 150.0.4078.99' },
        { label: 'Platform', value: 'Windows x64 · 1920 × 1080 viewport' },
        { label: 'Starting state', value: 'Effect-limited recorded orchestration overview' },
      ],
      evidenceFiles: [
        {
          label: 'Full screenshot',
          path: 'docs/agent-review/evidence/renderer/recorded-plan-builder/plan-builder-1920x1080.png',
        },
        {
          label: 'Focused proposal rail',
          path: 'docs/agent-review/evidence/renderer/recorded-plan-builder/proposal-rail.png',
        },
        {
          label: 'Semantic snapshot',
          path: 'docs/agent-review/evidence/renderer/recorded-plan-builder/semantic-snapshot.yml',
        },
        {
          label: 'Trace',
          path: 'docs/agent-review/evidence/renderer/recorded-plan-builder/trace.zip',
        },
        {
          label: 'Manifest',
          path: 'docs/agent-review/evidence/renderer/recorded-plan-builder/manifest.json',
        },
      ],
      reproduction: ['npm run review:renderer'],
      action: {
        label: 'Open recorded Plan Builder',
        href: '?recorded-plan-builder',
      },
      unverifiedClaims: [
        'This run does not verify a Tauri window, native IPC, or production behavior.',
        'A DOM assertion or screenshot alone does not establish visual acceptance.',
      ],
    },
    {
      id: 'windows-webview2-attachment',
      ordinal: '02',
      title: 'Windows Tauri / WebView2 attachment',
      scope: 'Real development host',
      evidenceLane: 'exploratory-control',
      dispositionKind: 'accepted',
      status: 'verified',
      statusLabel: 'Verified attachment',
      request:
        'Inspect the launched Windows Tauri host through a process-scoped development endpoint.',
      evidence:
        'Playwright connectOverCDP attached to exactly one WebView2 150 page target discovered through DevToolsActivePort, then exercised the recorded Plan Builder.',
      disposition:
        'Accepted for bounded Windows attachment feasibility. Playwright CDP was proven; Chrome DevTools MCP was not invoked.',
      metadata: [
        {
          label: 'Application',
          value: 'Tauri development shell · recorded Plan Builder composition',
        },
        { label: 'Driver', value: 'Playwright connectOverCDP 1.61.1 · WebView2 150.0.4078.99' },
        {
          label: 'Endpoint',
          value: '--remote-debugging-port=0 · observed port recorded in the manifest',
        },
        { label: 'Target', value: 'Exactly one page · Tauri internals observed' },
      ],
      evidenceFiles: [
        {
          label: 'Attachment manifest',
          path: 'docs/agent-review/evidence/windows-attachment/attachment-manifest.json',
        },
        {
          label: 'Native window screenshot',
          path: 'docs/agent-review/evidence/windows-attachment/tauri-webview2.png',
        },
        {
          label: 'Semantic snapshot',
          path: 'docs/agent-review/evidence/windows-attachment/semantic-snapshot.yml',
        },
        {
          label: 'Lifecycle cleanup',
          path: 'docs/agent-review/evidence/windows-attachment/lifecycle.json',
        },
      ],
      reproduction: [
        'powershell -NoProfile -ExecutionPolicy Bypass -File scripts/agent-review/run-windows-attachment.ps1 -FrontendPort 1442',
      ],
      reproductionNote:
        'The script discovers the selected loopback port, attaches only to the launched target, stops the app/server process tree, verifies closure, and deletes the worktree-scoped profile.',
      unverifiedClaims: [
        'Chrome DevTools MCP itself was not invoked in this run.',
        'The attachment does not establish native IPC correctness or support on macOS or Linux.',
      ],
    },
    {
      id: 'native-tauri-e2e',
      ordinal: '03',
      title: 'Native Tauri E2E',
      scope: 'Shell and IPC contract',
      evidenceLane: 'deterministic-verification',
      dispositionKind: 'accepted',
      status: 'verified',
      statusLabel: 'Verified native IPC',
      request: 'Verify a real Tauri shell and active Rust IPC contract through an isolated build.',
      evidence:
        'WebdriverIO launched the release Tauri shell, displayed #root, and browser.tauri.execute returned orchestration-native-query/v2 with 12 empty collections from isolated app data.',
      disposition:
        'Accepted for this Windows shell and native-query contract. Command mocking, cross-platform behavior, visual fidelity, and production behavior remain unverified.',
      metadata: [
        { label: 'Application', value: 'Release Tauri shell · native-review feature' },
        {
          label: 'Driver',
          value: '@wdio/tauri-service 1.2.0 · embedded provider · msedge 150',
        },
        { label: 'Platform', value: 'Windows x64 · observed window 1618 × 1072' },
        { label: 'Starting state', value: 'Fresh isolated app data and WebView2 profile' },
      ],
      evidenceFiles: [
        {
          label: 'Native proof manifest',
          path: 'docs/agent-review/evidence/native-tauri-wdio/manifest.json',
        },
        {
          label: 'IPC assertions',
          path: 'docs/agent-review/evidence/native-tauri-wdio/assertions.json',
        },
        {
          label: 'Native shell screenshot',
          path: 'docs/agent-review/evidence/native-tauri-wdio/native-shell.png',
        },
        {
          label: 'Frontend/backend service log',
          path: 'docs/agent-review/evidence/native-tauri-wdio/wdio-service.txt',
        },
      ],
      reproduction: ['npm run review:native'],
      reproductionNote:
        'The runner builds only the alternate feature/config, selects an available loopback port, owns the session, then verifies closure and deletes isolated application/browser state.',
      unverifiedClaims: [
        'Command mocking remains unproven after the frontend invoke-interception warning.',
        'Forwarded frontend logs contain non-fatal JSON deserialization warnings.',
        'The screenshot is an observation, not visual-fidelity proof.',
        'This Windows run does not establish macOS, Linux, or production behavior.',
      ],
    },
  ],
  worktreeHandoff: {
    status: 'Interface defined · application integration unproven',
    request:
      'The application requests one development/test instance for an expected worktree path, commit, and source fingerprint, with isolated data, scrubbed credentials, ephemeral ports, required capabilities, and mandatory cleanup.',
    instance:
      'The worktree runtime owns build and launch, then returns named instance/build/session identity, an HTTP endpoint or opaque window reference, semantic capabilities, and runtime/review evidence roots.',
    review:
      'A lane adapter validates the handoff, performs driver-specific interaction, retains evidence beside the runtime manifest, and returns neutral evidence for a separate review judgement.',
    unverified:
      'The current proofs were launched by dedicated scripts. This branch defines but does not yet integrate the application-to-worktree-runtime lifecycle port.',
  },
  boundaries: [
    {
      label: 'Worktree instance',
      detail:
        'Build, launch, endpoint/window ownership, and cleanup belong to the worktree runtime. Review adapters receive semantic references, not process or driver authority.',
    },
    {
      label: 'Security',
      detail:
        'Remote debugging is development-only. The recorded endpoint was consumed through loopback and must never be enabled in production.',
    },
    {
      label: 'Dependency risk',
      detail:
        'The 2026-07-27 npm audit reports one unchanged-baseline high production finding and 28 full-tree package findings; 22 affected packages belong to the development-only WDIO chain.',
    },
    {
      label: 'Sandbox',
      detail:
        'Renderer scenarios are effect-limited. CDP page access grants inspection only, not orchestration or native authority.',
    },
    {
      label: 'Lifecycle',
      detail:
        'The launched app/server process tree was cleaned up after capture; the endpoint is no longer active.',
    },
    {
      label: 'Process ownership',
      detail:
        'Attach only to the launched development host and its single expected target, never unrelated browser processes.',
    },
    {
      label: 'Port',
      detail:
        'Port 0 selects an ephemeral port. Discover it from DevToolsActivePort; do not hard-code or expose it.',
    },
    {
      label: 'Credentials',
      detail:
        'The launch adapters removed CODEX_HOME and credential-shaped environment variables. The scenarios required and retained no credentials.',
    },
    {
      label: 'Production exclusion',
      detail:
        'This lab, recorded clients, debugging arguments, and scoped user-data folder belong only to development/test composition.',
    },
  ],
};
