import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  Presentation,
  PresentationFile,
} from 'file:///C:/Users/user/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/@oai/artifact-tool/dist/artifact_tool.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');
const screenshots = path.join(root, 'screenshots');
const rendered = path.join(root, 'rendered');
const output = path.join(root, 'Session Event Model UI Walkthrough.pptx');

const W = 1280;
const H = 720;
const C = {
  ink: '#101914',
  muted: '#66706A',
  paper: '#F8F7F2',
  white: '#FFFFFF',
  line: '#C8CEC9',
  green: '#2D5D48',
  pale: '#EAF1EA',
  blue: '#2F80ED',
  amber: '#A55B16',
};

const deck = Presentation.create({ slideSize: { width: W, height: H } });

function textbox(slide, name, text, position, style = {}) {
  const shape = slide.shapes.add({
    geometry: 'textbox',
    name,
    position,
    fill: 'none',
    line: { style: 'solid', fill: 'none', width: 0 },
  });
  shape.text = text;
  shape.text.style = {
    typeface: 'Arial',
    fontSize: 24,
    color: C.ink,
    verticalAlignment: 'top',
    autoFit: 'shrinkText',
    insets: { top: 0, right: 0, bottom: 0, left: 0 },
    ...style,
  };
  return shape;
}

function box(slide, name, position, fill = C.white, line = C.line, radius = 'rounded-xl') {
  return slide.shapes.add({
    geometry: 'roundRect',
    name,
    position,
    fill,
    line: { style: 'solid', fill: line, width: 1 },
    borderRadius: radius,
  });
}

function addHeader(slide, title, number, eyebrow = 'SESSION EVENT MODEL') {
  textbox(slide, `eyebrow-${number}`, eyebrow, { left: 42, top: 34, width: 340, height: 22 }, {
    fontSize: 12,
    bold: true,
    color: C.green,
    letterSpacing: 1.6,
  });
  textbox(slide, `title-${number}`, title, { left: 42, top: 62, width: 1120, height: 58 }, {
    fontSize: 36,
    bold: true,
  });
  textbox(slide, `slide-number-${number}`, String(number).padStart(2, '0'), { left: 1190, top: 42, width: 45, height: 20 }, {
    fontSize: 12,
    color: C.muted,
    alignment: 'right',
  });
}

function addEvidenceLabel(slide, text, left, top, width, kind = 'live') {
  const fill = kind === 'seeded' ? '#F6EBDD' : C.pale;
  const color = kind === 'seeded' ? C.amber : C.green;
  box(slide, `evidence-${text}`, { left, top, width, height: 28 }, fill, fill, 'rounded-full');
  textbox(slide, `evidence-text-${text}`, text, { left: left + 12, top: top + 7, width: width - 24, height: 16 }, {
    fontSize: 10,
    bold: true,
    color,
    alignment: 'center',
  });
}

async function addScreenshot(slide, filename, position, alt, label, labelKind = 'live') {
  box(slide, `frame-${filename}`, position, C.white, '#ABB5AF', 'rounded-xl');
  const bytes = await fs.readFile(path.join(screenshots, filename));
  slide.images.add({
    blob: bytes,
    contentType: 'image/jpeg',
    alt,
    fit: 'contain',
    geometry: 'roundRect',
    borderRadius: 'rounded-xl',
    position: {
      left: position.left + 6,
      top: position.top + 6,
      width: position.width - 12,
      height: position.height - 12,
    },
  });
  if (label) addEvidenceLabel(slide, label, position.left + 14, position.top + 14, label.length * 6.5 + 28, labelKind);
}

function addBullets(slide, items, position, name) {
  const lineHeight = 62;
  items.forEach((item, index) => {
    slide.shapes.add({
      geometry: 'ellipse',
      name: `${name}-dot-${index}`,
      position: { left: position.left, top: position.top + index * lineHeight + 7, width: 9, height: 9 },
      fill: C.green,
      line: { style: 'solid', fill: C.green, width: 0 },
    });
    textbox(slide, `${name}-${index}`, item, {
      left: position.left + 22,
      top: position.top + index * lineHeight,
      width: position.width - 22,
      height: 48,
    }, { fontSize: 17, color: C.ink });
  });
}

function notes(slide, talkTrack, sources) {
  slide.speakerNotes.textFrame.setText(`${talkTrack}\n\n[Sources]\n${sources.map((source) => `- ${source}`).join('\n')}\n[/Sources]`);
  slide.speakerNotes.setVisible(true);
}

function setBackground(slide, fill = C.paper) {
  slide.background.fill = fill;
}

// 1 — cover
{
  const slide = deck.slides.add();
  setBackground(slide);
  textbox(slide, 'cover-eyebrow', 'CODEX ORCHESTRATOR · UI WALKTHROUGH', { left: 42, top: 38, width: 540, height: 24 }, {
    fontSize: 12,
    bold: true,
    color: C.green,
    letterSpacing: 1.8,
  });
  textbox(slide, 'cover-title', 'From Harness UX\nto Session Events', { left: 42, top: 196, width: 560, height: 220 }, {
    fontSize: 58,
    bold: true,
  });
  textbox(slide, 'cover-subtitle', 'A screenshot-led walkthrough of capability exposure, node configuration, event compilation, and pinned runtime truth.', { left: 46, top: 442, width: 520, height: 108 }, {
    fontSize: 21,
    color: C.muted,
  });
  await addScreenshot(slide, '08-session-profile-and-message-controls.jpg', { left: 664, top: 46, width: 570, height: 575 }, 'Agent Session showing message-local controls and pinned Session Profile', 'SEEDED INSPECTION FIXTURE', 'seeded');
  addEvidenceLabel(slide, 'BRANCH aaca806', 42, 624, 142, 'live');
  addEvidenceLabel(slide, 'NO PROVIDER DISPATCH', 198, 624, 164, 'live');
  textbox(slide, 'cover-footer', 'Captured 2 Sep 2026', { left: 1080, top: 670, width: 154, height: 18 }, { fontSize: 11, color: C.muted, alignment: 'right' });
  notes(slide,
    'Frame the walkthrough as a model and UI boundary review, not a claim of production completion. The Capability Profile and Workflow authoring states are live in the isolated desktop demo. The Agent Session is a seeded inspection fixture so the presentation can show pinned truth and provenance without triggering a provider.',
    ['Local capture: 08-session-profile-and-message-controls.jpg']);
}

// 2 — conceptual map
{
  const slide = deck.slides.add();
  setBackground(slide);
  addHeader(slide, 'The UI now exposes five distinct layers', 2, 'CONCEPTUAL MAP');
  const stages = [
    ['1', 'Selected runtime', 'Provider-managed exposure'],
    ['2', 'Capability Profile', 'Reusable technical boundary'],
    ['3', 'Node Profile', 'Embedded operational config'],
    ['4', 'Session Event', 'Trigger + prompt + target'],
    ['5', 'Session Profile', 'Pinned runtime truth'],
  ];
  const start = 58;
  const gap = 14;
  const cardW = 220;
  stages.forEach(([n, title, detail], index) => {
    const left = start + index * (cardW + gap);
    box(slide, `stage-${n}`, { left, top: 198, width: cardW, height: 244 }, index === 4 ? C.pale : C.white, index === 4 ? C.green : C.line);
    textbox(slide, `stage-number-${n}`, n, { left: left + 20, top: 218, width: 44, height: 44 }, { fontSize: 30, bold: true, color: C.green });
    textbox(slide, `stage-title-${n}`, title, { left: left + 20, top: 292, width: cardW - 40, height: 72 }, { fontSize: 22, bold: true });
    textbox(slide, `stage-detail-${n}`, detail, { left: left + 20, top: 375, width: cardW - 40, height: 48 }, { fontSize: 15, color: C.muted });
    if (index < stages.length - 1) {
      textbox(slide, `arrow-${n}`, '→', { left: left + cardW + 1, top: 302, width: gap + 12, height: 42 }, { fontSize: 28, bold: true, color: C.green, alignment: 'center' });
    }
  });
  textbox(slide, 'map-caption', 'The boundaries are intentionally legible before they are made extensible: authoring owns definitions; Session Events own execution semantics; Agent Sessions expose resolved truth.', { left: 90, top: 512, width: 1100, height: 80 }, { fontSize: 21, color: C.ink, alignment: 'center' });
  notes(slide,
    'Walk left to right. The selected runtime is observed, not configured here. A Capability Profile narrows what is available. Each Workflow node embeds its own Node Profile. Nodes and connections compile into Session Event definitions. Creation resolves and pins a Session Profile. The key architectural property is that each layer has a clear consumption contract.',
    ['Derived from the captured live authoring and seeded inspection surfaces in this deck.']);
}

// 3 — capability profile
{
  const slide = deck.slides.add();
  setBackground(slide);
  addHeader(slide, 'Capability Profile: reusable technical exposure', 3, 'AUTHORING SURFACE');
  addBullets(slide, [
    'The globally selected native runtime is visible as read-only inherited truth.',
    'Models, reasoning modes, and sandbox modes can be narrowed for downstream nodes.',
    'MCP tools and skills are shown as inherited and disabled in this bounded slice.',
  ], { left: 52, top: 170, width: 332 }, 'capability-bullet');
  await addScreenshot(slide, '01-capability-profile-overview.jpg', { left: 414, top: 142, width: 396, height: 264 }, 'Capability Profile details and selected runtime summary', 'LIVE AUTHORING STATE');
  await addScreenshot(slide, '02-capability-allowed-selections.jpg', { left: 826, top: 142, width: 396, height: 264 }, 'Allowed models and reasoning modes', 'LIVE AUTHORING STATE');
  box(slide, 'capability-takeaway', { left: 414, top: 438, width: 808, height: 140 }, C.pale, C.pale);
  textbox(slide, 'capability-takeaway-text', 'Boundary rule\nA Capability Profile may only narrow the attached runtime. Provider connection setup remains a separate concern.', { left: 446, top: 466, width: 744, height: 86 }, { fontSize: 22, bold: true });
  notes(slide,
    'Start with the profile details, then point to the runtime summary. The UI is explicit that this object consumes one selected native runtime rather than configuring providers. Move to the second capture to show the editable narrowing surface and the inherited MCP/skill treatment.',
    ['Local capture: 01-capability-profile-overview.jpg', 'Local capture: 02-capability-allowed-selections.jpg']);
}

// 4 — node profile
{
  const slide = deck.slides.add();
  setBackground(slide);
  addHeader(slide, 'Node Profile: embedded, copyable operational context', 4, 'WORKFLOW NODE');
  addBullets(slide, [
    'The initial prompt is included only when this node creates a Session.',
    'Node capabilities can narrow the selected Capability Profile again.',
    'Pinned defaults govern workflow-addressed messages; identity remains separate.',
  ], { left: 52, top: 170, width: 330 }, 'node-bullet');
  await addScreenshot(slide, '03-workflow-node-profile.jpg', { left: 414, top: 142, width: 396, height: 264 }, 'Node details and creation prompt', 'LIVE AUTHORING STATE');
  await addScreenshot(slide, '04-node-restrictions-and-defaults.jpg', { left: 826, top: 142, width: 396, height: 264 }, 'Node restrictions and defaults', 'LIVE AUTHORING STATE');
  box(slide, 'node-takeaway', { left: 414, top: 438, width: 808, height: 140 }, C.white, C.line);
  textbox(slide, 'node-takeaway-text', 'Current reuse decision\nThere is no shared Node Profile catalog. Copying creates independent node state that can be edited locally.', { left: 446, top: 466, width: 744, height: 86 }, { fontSize: 22, bold: true });
  notes(slide,
    'Use the first capture to separate operational naming, Agent identity, Capability Profile selection, and the creation-only prompt. Use the second to show that restrictions and defaults are visible in the same node editor. Emphasize that Node Profile reuse is deliberately deferred; the supported operation is copy then edit.',
    ['Local capture: 03-workflow-node-profile.jpg', 'Local capture: 04-node-restrictions-and-defaults.jpg']);
}

// 5 — connection to event
{
  const slide = deck.slides.add();
  setBackground(slide);
  addHeader(slide, 'A visual edge compiles into a Session Event definition', 5, 'WORKFLOW CONNECTION');
  addBullets(slide, [
    'Trigger: invocation completed for the source node address.',
    'Prompt: ordered sources plus optional fixed connection text.',
    'Target: destination node address, selection order, running filter, and no-match policy.',
  ], { left: 52, top: 170, width: 330 }, 'event-bullet');
  await addScreenshot(slide, '05-connection-trigger-and-prompt.jpg', { left: 414, top: 142, width: 396, height: 264 }, 'Connection trigger and ordered prompt sources', 'LIVE AUTHORING STATE');
  await addScreenshot(slide, '06-connection-prompt-and-target.jpg', { left: 826, top: 142, width: 396, height: 264 }, 'Prompt materialization and target resolution', 'LIVE AUTHORING STATE');
  box(slide, 'event-takeaway', { left: 414, top: 438, width: 808, height: 140 }, C.pale, C.pale);
  textbox(slide, 'event-takeaway-text', 'Abstraction boundary\nThe Workflow editor arranges recipe concepts. The compiled result is expressed entirely in the generic Session Event model.', { left: 446, top: 466, width: 744, height: 86 }, { fontSize: 22, bold: true });
  notes(slide,
    'Describe the connection as an authoring convenience rather than execution truth. The source and destination nodes derive logical addresses. The trigger, ordered prompt sources, target cardinality/order/filter, and create-on-missing behavior are already explicit in the editor. This is the functional layer a future script or custom provider would target.',
    ['Local capture: 05-connection-trigger-and-prompt.jpg', 'Local capture: 06-connection-prompt-and-target.jpg']);
}

// 6 — compile boundary
{
  const slide = deck.slides.add();
  setBackground(slide);
  addHeader(slide, 'Compilation is explicit; dispatch is separate', 6, 'COMPILE AND RUN');
  addBullets(slide, [
    'A Workflow instance ID scopes all logical Session addresses for the run.',
    'The active recipe compiles into one or more Session Event definitions.',
    'The user-triggered occurrence is staged independently from dispatch.',
  ], { left: 52, top: 182, width: 326 }, 'compile-bullet');
  await addScreenshot(slide, '07-compile-and-run.jpg', { left: 414, top: 142, width: 808, height: 536 }, 'Compiled active recipe with a staged user request', 'LIVE · NOT DISPATCHED');
  addEvidenceLabel(slide, '0 PROVIDER INVOCATIONS', 52, 542, 184, 'live');
  addEvidenceLabel(slide, '0 RUNTIME SESSIONS', 52, 582, 166, 'live');
  notes(slide,
    'This is the safety and legibility checkpoint. Compilation produced a Session Event definition for the named Workflow instance, but the request was never dispatched. The UI makes the boundary visible: authoring and compilation can be reviewed before any provider work begins.',
    ['Local capture: 07-compile-and-run.jpg']);
}

// 7 — pinned session truth
{
  const slide = deck.slides.add();
  setBackground(slide);
  addHeader(slide, 'Sessions pin truth; user prompts keep local choice', 7, 'AGENT SESSION');
  addBullets(slide, [
    'The Session stores the resolved runtime reference, profile revision, and digest.',
    'Workflow messages use pinned model, reasoning, and sandbox defaults.',
    'A direct user message may choose any attached runtime model and reasoning mode for that message only.',
  ], { left: 52, top: 170, width: 332 }, 'session-bullet');
  await addScreenshot(slide, '08-session-profile-and-message-controls.jpg', { left: 414, top: 142, width: 396, height: 264 }, 'Per-message controls and pinned resolution', 'SEEDED FIXTURE', 'seeded');
  await addScreenshot(slide, '09-pinned-defaults-and-node-capabilities.jpg', { left: 826, top: 142, width: 396, height: 264 }, 'Pinned defaults and node capabilities', 'SEEDED FIXTURE', 'seeded');
  box(slide, 'session-takeaway', { left: 414, top: 438, width: 808, height: 140 }, '#F6EBDD', '#F6EBDD');
  textbox(slide, 'session-takeaway-text', 'Important distinction\nThe message selectors are not a mutable Session override profile. They resolve invocation-local user authority against the attached runtime.', { left: 446, top: 466, width: 744, height: 86 }, { fontSize: 22, bold: true, color: C.ink });
  notes(slide,
    'This slide uses a seeded inspection record. Point first to the model and reasoning selectors: they apply only to the next direct user message. Then point to the immutable resolution metadata. Workflow-addressed messages continue to obey the pinned Node Profile defaults, while direct user prompts can use the wider attached runtime exposure.',
    ['Local seeded capture: 08-session-profile-and-message-controls.jpg', 'Local seeded capture: 09-pinned-defaults-and-node-capabilities.jpg']);
}

// 8 — runtime and provenance / closing
{
  const slide = deck.slides.add();
  setBackground(slide);
  addHeader(slide, 'Runtime truth and provenance are inspectable from the Session', 8, 'WALKTHROUGH CLOSE');
  await addScreenshot(slide, '10-attached-runtime-and-delivery-provenance.jpg', { left: 42, top: 142, width: 664, height: 442 }, 'Attached runtime and recorded delivery section', 'SEEDED FIXTURE', 'seeded');
  box(slide, 'questions', { left: 742, top: 142, width: 496, height: 442 }, C.white, C.line);
  textbox(slide, 'questions-title', 'Questions for review', { left: 778, top: 180, width: 420, height: 42 }, { fontSize: 27, bold: true });
  addBullets(slide, [
    'Are the five boundaries legible enough to edit confidently?',
    'Does the Session view expose enough provenance for day-to-day work?',
    'Which run-level projection should be designed next—without assuming it is a graph?',
  ], { left: 778, top: 254, width: 408 }, 'closing-bullet');
  textbox(slide, 'closing-note', 'Deferred: canonical run UI, robust custom providers, role concepts, and provider connection management.', { left: 778, top: 486, width: 410, height: 68 }, { fontSize: 16, color: C.muted });
  textbox(slide, 'closing-footer', 'Next conversation: refine the presentation surface around proven functional boundaries.', { left: 42, top: 630, width: 1040, height: 36 }, { fontSize: 24, bold: true });
  notes(slide,
    'Close by showing that attached runtime exposure and recorded delivery provenance are separate from pinned node truth. The Session Event delivery section exists, but the canonical run projection is intentionally deferred. Use the three questions to guide the review and resist choosing a graph or role abstraction prematurely.',
    ['Local seeded capture: 10-attached-runtime-and-delivery-provenance.jpg']);
}

await fs.mkdir(rendered, { recursive: true });
for (const [index, slide] of deck.slides.items.entries()) {
  const stem = `slide-${String(index + 1).padStart(2, '0')}`;
  const png = await deck.export({ slide, format: 'png', scale: 1 });
  await fs.writeFile(path.join(rendered, `${stem}.png`), new Uint8Array(await png.arrayBuffer()));
  const layout = await slide.export({ format: 'layout' });
  await fs.writeFile(path.join(rendered, `${stem}.layout.json`), await layout.text());
}
const montage = await deck.export({ format: 'webp', montage: true, scale: 1 });
await fs.writeFile(path.join(rendered, 'walkthrough-montage.webp'), new Uint8Array(await montage.arrayBuffer()));
const pptx = await PresentationFile.exportPptx(deck);
await pptx.save(output);
console.log(output);
