import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Presentation, PresentationFile } from '@oai/artifact-tool';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');
const screenshots = path.join(root, 'screenshots', 'merged-2026-09-07');
const rendered = path.join(root, 'rendered-v7');
const outputDir = path.join(root, 'output');
const taskWorkspace = process.env.CODEX_PRESENTATION_WORKSPACE ?? process.cwd();
const workspaceDir = path.join(taskWorkspace, '.codex-presentation-finalizer-session-event-v7');
const candidatePath = path.join(workspaceDir, 'candidate.pptx');
const finalPath = path.join(outputDir, 'Session Event UI Demo Walkthrough v7.pptx');
const validatedPath = path.join(workspaceDir, 'output', 'Session Event UI Demo Walkthrough v7.validated.pptx');
const receiptPath = path.join(workspaceDir, 'validation.json');
const SKILL_DIR = process.env.SKILL_DIR;
const RUNTIME_PYTHON = process.env.RUNTIME_PYTHON;

if (!SKILL_DIR || !RUNTIME_PYTHON) {
  throw new Error('SKILL_DIR and RUNTIME_PYTHON are required');
}

const W = 1280;
const H = 720;
const C = {
  ink: '#122019',
  muted: '#657069',
  paper: '#F8F7F2',
  white: '#FFFFFF',
  line: '#CBD1CC',
  green: '#2D5D48',
  pale: '#EAF1EA',
  red: '#9E3328',
  redPale: '#F8E8E5',
};

const deck = Presentation.create({ slideSize: { width: W, height: H } });

function textbox(slide, name, text, position, style = {}) {
  const shape = slide.shapes.add({
    geometry: 'textbox',
    name,
    position,
    fill: 'none',
    line: { fill: 'none', width: 0 },
  });
  shape.text = text;
  shape.text.style = {
    typeface: 'Arial',
    fontSize: 20,
    color: C.ink,
    verticalAlignment: 'top',
    autoFit: 'shrinkText',
    insets: { top: 0, right: 0, bottom: 0, left: 0 },
    ...style,
  };
  return shape;
}

function box(slide, name, position, fill = C.white, line = C.line) {
  return slide.shapes.add({
    geometry: 'roundRect',
    name,
    position,
    fill,
    line: { fill: line, width: 1 },
    borderRadius: 'rounded-xl',
  });
}

function addHeader(slide, title, number, section) {
  textbox(slide, `section-${number}`, section, { left: 42, top: 26, width: 360, height: 18 }, {
    fontSize: 11,
    bold: true,
    color: C.green,
    letterSpacing: 1.4,
  });
  textbox(slide, `title-${number}`, title, { left: 42, top: 48, width: 1120, height: 52 }, {
    fontSize: 32,
    bold: true,
  });
  textbox(slide, `number-${number}`, String(number).padStart(2, '0'), { left: 1198, top: 31, width: 38, height: 18 }, {
    fontSize: 11,
    color: C.muted,
    alignment: 'right',
  });
}

function addNotes(slide, talkTrack, screenshot) {
  slide.speakerNotes.textFrame.setText(`${talkTrack}\n\n[Sources]\n- Local in-app browser capture: ${screenshot}\n[/Sources]`);
}

function addCallouts(slide, items, issue) {
  let top = 132;
  items.forEach((item, index) => {
    textbox(slide, `callout-number-${index}-${top}`, String(index + 1), { left: 42, top: top + 1, width: 30, height: 28 }, {
      fontSize: 18,
      bold: true,
      color: C.green,
    });
    textbox(slide, `callout-title-${index}-${top}`, item.title, { left: 76, top, width: 330, height: 27 }, {
      fontSize: 19,
      bold: true,
    });
    textbox(slide, `callout-body-${index}-${top}`, item.body, { left: 76, top: top + 31, width: 330, height: 62 }, {
      fontSize: 16,
      color: C.muted,
    });
    top += 116;
  });
  if (issue) {
    box(slide, `issue-${top}`, { left: 42, top: 566, width: 368, height: 98 }, C.redPale, '#E4A099');
    textbox(slide, `issue-text-${top}`, issue, { left: 58, top: 584, width: 336, height: 64 }, {
      fontSize: 16,
      bold: true,
      color: C.red,
    });
  }
}

async function addScreenshot(slide, filename, alt) {
  const position = { left: 448, top: 112, width: 790, height: 596 };
  box(slide, `frame-${filename}`, position, C.white, '#AEB8B1');
  const bytes = await fs.readFile(path.join(screenshots, filename));
  slide.images.add({
    blob: bytes,
    contentType: 'image/png',
    alt,
    fit: 'contain',
    geometry: 'roundRect',
    borderRadius: 'rounded-xl',
    position: { left: 454, top: 118, width: 778, height: 584 },
  });
  box(slide, `evidence-${filename}`, { left: 464, top: 128, width: 164, height: 26 }, C.pale, C.pale);
  textbox(slide, `evidence-text-${filename}`, 'LIVE LOCAL FIXTURE', { left: 476, top: 135, width: 140, height: 14 }, {
    fontSize: 10,
    bold: true,
    color: C.green,
    alignment: 'center',
  });
}

const slides = [
  {
    title: 'Capability Profile list and details',
    section: 'CAPABILITY PROFILES',
    image: '01-capability-profile.png',
    alt: 'Capability Profile list and profile details',
    items: [
      { title: 'Saved profiles', body: 'The left side lists each profile and its revision.' },
      { title: 'Stable ID', body: 'The profile name can change. The profile ID stays fixed.' },
      { title: 'Read-only runtime', body: 'The selected native runtime is shown as inherited.' },
    ],
    note: 'The new Capability Profiles screen is separate from Workflow authoring. The left side changes the selected profile. The right side edits the profile and shows the inherited runtime.',
  },
  {
    title: 'Allowed capabilities',
    section: 'CAPABILITY PROFILES',
    image: '02-allowed-capabilities.png',
    alt: 'Capability Profile controls for models, reasoning, MCP, skills, and sandbox',
    items: [
      { title: 'Models and reasoning', body: 'Tick the choices that nodes may use.' },
      { title: 'Inherited items', body: 'MCP tools and skills stay locked in this build.' },
      { title: 'Save or delete', body: 'Both actions sit at the end of the profile form.' },
    ],
    note: 'This is the lower part of the same profile. Model and reasoning choices can be narrowed. MCP, skills, and sandbox show what comes from the selected runtime.',
  },
  {
    title: 'Workflow canvas',
    section: 'WORKFLOW',
    image: '03-workflow-canvas.png',
    alt: 'Restored Workflow canvas with two nodes and instance list',
    items: [
      { title: 'The canvas is back', body: 'Nodes and connections are shown as a flow again.' },
      { title: 'Edit tools', body: 'Add, connect, copy, delete, drag, undo, and redo live above the canvas.' },
      { title: 'Instances stay visible', body: 'Saved Workflow instances sit below the recipe list.' },
    ],
    note: 'The merge fixes the largest problem in the last demo. The node list no longer replaces the flow. The canvas, recipe list, and instance list now share one screen.',
  },
  {
    title: 'Node profile editor',
    section: 'WORKFLOW NODE',
    image: '04-node-profile.png',
    alt: 'Node Profile editor beside the Workflow canvas',
    items: [
      { title: 'Editor beside the flow', body: 'Selecting a node opens its form on the right.' },
      { title: 'Separate choices', body: 'Agent identity and Capability Profile have their own fields.' },
      { title: 'Collapsible sections', body: 'Node details, copy, prompt, limits, defaults, and runtime can fold.' },
    ],
    note: 'The flow remains visible while the node form opens. The first screen shows the node name, identity, profile, copy section, and creation-only prompt.',
  },
  {
    title: 'Copy node setup',
    section: 'WORKFLOW NODE',
    image: '05-copy-node-profile.png',
    alt: 'Expanded copy node configuration section',
    items: [
      { title: 'Choose a source', body: 'Pick another node in the same Workflow.' },
      { title: 'Copy the setup', body: 'The button copies that node into this node.' },
      { title: 'Edit it here', body: 'The copied values become separate local node state.' },
    ],
    note: 'There is no shared Node Profile catalog. This section supports the agreed copy-and-edit flow without creating reusable node objects.',
  },
  {
    title: 'Node limits and defaults',
    section: 'WORKFLOW NODE',
    image: '06-node-limits-defaults.png',
    alt: 'Node capability limits and pinned defaults',
    items: [
      { title: 'Inherited access', body: 'MCP, skills, and sandbox show their runtime source.' },
      { title: 'Workflow defaults', body: 'Model and reasoning are set for Workflow messages.' },
      { title: 'Locked sandbox', body: 'The sandbox is visible but cannot change here.' },
    ],
    note: 'The node may narrow its Capability Profile. The node also sets the model and reasoning used when the Workflow sends a message. The sandbox stays inherited.',
  },
  {
    title: 'Connection trigger',
    section: 'WORKFLOW CONNECTION',
    image: '07-connection-trigger.png',
    alt: 'Connection editor with source, destination, and trigger',
    items: [
      { title: 'The edge sets direction', body: 'Author is the source. Reviewer is the destination.' },
      { title: 'The trigger is explicit', body: 'This connection starts after an invocation completes.' },
      { title: 'Prompt logic follows', body: 'The ordered prompt parts sit in the next section.' },
    ],
    note: 'Selecting the line opens the connection editor. The top of the form names the nodes and the trigger that starts the Session Event.',
  },
  {
    title: 'Prompt and Session target',
    section: 'WORKFLOW CONNECTION',
    image: '08-connection-prompt-target.png',
    alt: 'Connection prompt and Session addressing controls',
    items: [
      { title: 'Fixed prompt', body: 'The connection can append its own text.' },
      { title: 'Match rules', body: 'Choose the Session order and running state.' },
      { title: 'No match rule', body: 'The event can create a Session when none match.' },
    ],
    issue: 'Needs polish: the recorded-source checkbox sits too far from its label at this width.',
    note: 'This part controls prompt text and Session lookup. The behavior is present, but the checkbox layout is still awkward at the reviewed width.',
  },
  {
    title: 'MCP trigger references',
    section: 'WORKFLOW CONNECTION',
    image: '17-mcp-trigger.png',
    alt: 'MCP trigger with server and tool references',
    items: [
      { title: 'MCP call trigger', body: 'The trigger type can change from completion to MCP call.' },
      { title: 'Server reference', body: 'Namespace, kind, and ID identify the MCP server.' },
      { title: 'Tool reference', body: 'A second reference identifies the MCP tool.' },
    ],
    note: 'This state shows the built-in MCP trigger. The connection stores separate typed references for the server and the tool.',
  },
  {
    title: 'Ordered prompt sources',
    section: 'WORKFLOW CONNECTION',
    image: '19-prompt-sources.png',
    alt: 'Two ordered prompt source rows',
    items: [
      { title: 'One row per source', body: 'Each row has a type and the fields for that type.' },
      { title: 'Order controls', body: 'Move a row up or down, or remove it.' },
      { title: 'More prompt parts', body: 'Add another source or fixed connection text.' },
    ],
    note: 'The screenshot shows an MCP argument and referenced file content in one ordered list. The order on screen is the order used in the prompt.',
  },
  {
    title: 'Recorded-source filter',
    section: 'WORKFLOW CONNECTION',
    image: '18-recorded-source-target.png',
    alt: 'Session lookup filtered by the recorded creating event',
    items: [
      { title: 'Optional filter', body: 'Limit matches to Sessions made by a recorded source.' },
      { title: 'Event reference', body: 'Namespace, kind, and ID name the creating event.' },
      { title: 'Fallback', body: 'Create a Session when no Session matches.' },
    ],
    issue: 'Needs polish: checkbox size and label spacing are still uneven.',
    note: 'This state shows the deeper Session addressing rule. It can match the event or Session that created the target. The fields work, but the checkbox layout needs another pass.',
  },
  {
    title: 'Create Workflow instance',
    section: 'WORKFLOW INSTANCE',
    image: '09-create-instance.png',
    alt: 'Create Workflow instance dialog',
    items: [
      { title: 'Choose a saved Workflow', body: 'The picker uses an active saved revision.' },
      { title: 'Name the instance', body: 'The run gets a clear name in the instance list.' },
      { title: 'Choose a worktree', body: 'The dialog states that creation does not start an agent.' },
    ],
    note: 'The missing instance flow is restored. The user chooses the Workflow, instance name, and worktree. Creating the instance saves it but does not send a request.',
  },
  {
    title: 'Saved Workflow instance',
    section: 'WORKFLOW INSTANCE',
    image: '10-saved-instance.png',
    alt: 'Saved Workflow instance with node request form',
    items: [
      { title: 'Pinned recipe', body: 'The header shows the saved Workflow revision and worktree.' },
      { title: 'Send to a node', body: 'Choose a node and enter the request for that node.' },
      { title: 'Session list', body: 'Sessions created for the instance appear beside the request form.' },
    ],
    note: 'Opening a saved instance shows its pinned recipe and worktree. Requests are sent to a selected node. The Session list records what belongs to the instance.',
  },
  {
    title: 'Session inside an instance',
    section: 'WORKFLOW INSTANCE',
    image: '11-instance-session.png',
    alt: 'Workflow instance with selected Agent Session',
    items: [
      { title: 'Workflow context stays open', body: 'The instance and node request remain on the left.' },
      { title: 'Session opens on the right', body: 'The selected Session shows its identity and conversation.' },
      { title: 'Back returns to the last view', body: 'The product navigation keeps this as one joined flow.' },
    ],
    note: 'A Session can be opened without leaving the Workflow instance. This gives the instance a useful working view while preserving normal Agent Session controls.',
  },
  {
    title: 'Direct message controls',
    section: 'AGENT SESSION',
    image: '07-session-settings.png',
    alt: 'Agent Session with per-message model and reasoning controls',
    items: [
      { title: 'Next message only', body: 'Model and reasoning choices apply to one direct user message.' },
      { title: 'Pinned Session profile', body: 'The resolved runtime, profile revision, and digest are read only.' },
      { title: 'Workflow messages stay pinned', body: 'They continue to use the node defaults.' },
    ],
    issue: 'Needs polish: the conversation card overlaps the open settings at this width.',
    note: 'The user can choose any model and reasoning level exposed by the attached runtime for the next direct message. The Session Profile remains pinned. The open panel still collides with the conversation layer at this width.',
  },
  {
    title: 'Session profile details',
    section: 'AGENT SESSION',
    image: '13-session-runtime-and-deliveries.png',
    alt: 'Pinned defaults, node capabilities, attached runtime, and delivery sections',
    items: [
      { title: 'Pinned defaults', body: 'Model, reasoning, and sandbox show their node source.' },
      { title: 'Node capabilities', body: 'The Session shows what Workflow messages may use.' },
      { title: 'Runtime and deliveries', body: 'Both have separate collapsible sections below.' },
    ],
    issue: 'Needs polish: fixed conversation content covers the lower profile sections.',
    note: 'The source labels make the pinned values easy to trace. The structure is clear, but the conversation and composer layers cover part of the open configuration on this screen.',
  },
  {
    title: 'Delivery details',
    section: 'SESSION EVENT',
    image: '14-delivery-details.png',
    alt: 'Expanded Session Event delivery record in a Workflow instance',
    items: [
      { title: 'Delivery status', body: 'The card shows that the request was dispatched.' },
      { title: 'Recorded fields', body: 'Target, creation, sequence, and invocation are kept together.' },
      { title: 'Grouped with the instance', body: 'The delivery sits beside the Session it addressed.' },
    ],
    issue: 'Needs polish: long IDs break into hard-to-read lines in the narrow column.',
    note: 'Expanding the delivery shows the stored delivery record. The information is useful, but raw identifiers need truncation, copy actions, or a wider detail view.',
  },
  {
    title: 'Agent identity',
    section: 'AGENT SESSION',
    image: '12-agent-identity.png',
    alt: 'Agent identity dialog with initials, color, and shape controls',
    items: [
      { title: 'Initials preview', body: 'The preview uses the Agent name inside the chosen shape.' },
      { title: 'Color picker', body: 'The selected color and hex value appear together.' },
      { title: 'Three shapes', body: 'Circle, square, and hexagon are available.' },
    ],
    note: 'The requested identity controls are present in the Agent Session. The dialog changes display name, color, and shape for that Session.',
  },
  {
    title: 'Remaining Harness Management screen',
    section: 'LEGACY SURFACE',
    image: '16-legacy-harness-management.png',
    alt: 'Existing Harness Management screen with prompt, skills, and tools',
    items: [
      { title: 'Still in the product', body: 'Harness Management remains in the top navigation.' },
      { title: 'Old mixed object', body: 'It still groups prompt, skills, tools, model, and sandbox.' },
      { title: 'Not yet retired', body: 'The new screens now work beside this older screen.' },
    ],
    issue: 'Open decision: keep this only as a migration view, or remove it when the new UI is accepted.',
    note: 'The merge did not remove the old Harness Management screen. It now sits beside Capability Profiles, Workflow node profiles, and Session Profiles. That overlap should be resolved in a later retirement pass.',
  },
];

// 1. Cover
{
  const slide = deck.slides.add();
  slide.background.fill = C.paper;
  textbox(slide, 'cover-kicker', 'CODEX ORCHESTRATOR', { left: 54, top: 48, width: 360, height: 22 }, {
    fontSize: 12,
    bold: true,
    color: C.green,
    letterSpacing: 1.6,
  });
  textbox(slide, 'cover-title', 'Merged Harness UI demo', { left: 54, top: 190, width: 820, height: 86 }, {
    fontSize: 52,
    bold: true,
  });
  textbox(slide, 'cover-subtitle', 'A screen-by-screen review of the current branch', { left: 58, top: 292, width: 700, height: 54 }, {
    fontSize: 25,
    color: C.muted,
  });
  box(slide, 'cover-summary', { left: 54, top: 408, width: 738, height: 132 }, C.pale, C.pale);
  textbox(slide, 'cover-summary-text', 'The Workflow canvas and instance flow are restored.\nThis deck also marks the layout issues still visible in the merged UI.', { left: 82, top: 438, width: 682, height: 80 }, {
    fontSize: 22,
    bold: true,
  });
  textbox(slide, 'cover-meta', '20 slides  ·  branch 2e378bc  ·  local fake clients  ·  no provider call', { left: 56, top: 646, width: 820, height: 24 }, {
    fontSize: 14,
    color: C.muted,
  });
  slide.speakerNotes.textFrame.setText('This walkthrough uses fresh in-app browser captures from the merged Harness branch. The fixture mounts the real React components with local fake clients. No provider request was sent.');
}

for (const [index, data] of slides.entries()) {
  const slideNumber = index + 2;
  const slide = deck.slides.add();
  slide.background.fill = C.paper;
  addHeader(slide, data.title, slideNumber, data.section);
  addCallouts(slide, data.items, data.issue);
  await addScreenshot(slide, data.image, data.alt);
  addNotes(slide, data.note, data.image);
}

await fs.mkdir(rendered, { recursive: true });
await fs.mkdir(outputDir, { recursive: true });
await fs.mkdir(workspaceDir, { recursive: true });
await fs.mkdir(path.dirname(validatedPath), { recursive: true });

for (const [index, slide] of deck.slides.items.entries()) {
  const stem = `slide-${String(index + 1).padStart(2, '0')}`;
  const png = await deck.export({ slide, format: 'png', scale: 1 });
  await fs.writeFile(path.join(rendered, `${stem}.png`), new Uint8Array(await png.arrayBuffer()));
  const layout = await slide.export({ format: 'layout' });
  await fs.writeFile(path.join(rendered, `${stem}.layout.json`), await layout.text());
}

const montage = await deck.export({ format: 'webp', montage: true, scale: 0.5 });
await fs.writeFile(path.join(rendered, 'walkthrough-montage.webp'), new Uint8Array(await montage.arrayBuffer()));

await (await PresentationFile.exportPptx(deck)).save(candidatePath);
const { finalizePresentation } = await import(
  pathToFileURL(path.join(SKILL_DIR, 'container_tools', 'artifact_tool_utils.mjs')).href
);
const result = await finalizePresentation({
  explicitTotalSlideCount: 20,
  requiredNativeTableOwnerSlides: [],
  requiredNativeChartOwnerSlides: [],
  workspaceDir,
  candidatePath,
  finalPath: validatedPath,
  pythonExecutable: RUNTIME_PYTHON,
  integrityValidatorPath: path.join(SKILL_DIR, 'container_tools', 'inspect_presentation_package_integrity.py'),
  layoutValidatorPath: path.join(SKILL_DIR, 'container_tools', 'inspect_presentation_layout_geometry.py'),
  layoutArgs: [
    '--expected-slide-size-emu',
    '12192000,6858000',
    '--validate-bullet-geometry',
    '--validate-heading-fit',
  ],
  fontPolicy: { basis: 'design', families: ['Arial'] },
  verifyArtifactToolImport: true,
  receiptPath,
});

await fs.copyFile(validatedPath, finalPath);
console.log(JSON.stringify({ finalPath, result }, null, 2));
