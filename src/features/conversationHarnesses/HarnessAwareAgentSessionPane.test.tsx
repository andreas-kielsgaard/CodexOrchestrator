import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import {
  createRecordedHarnessManagementSource,
  recordedHarnessInspectorSessionId,
} from '../../dev/conversationHarnesses/recordedHarnessInspectorSource';
import { ConversationHarnessManagement } from './ConversationHarnessInspector';
import { HarnessAwareAgentSessionPane } from './HarnessAwareAgentSessionPane';

async function openHarnessManagement() {
  fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
  await screen.findByRole('heading', { name: 'Harness details' });
}

async function setHarnessDefault(model: string, reasoning: string) {
  chooseClosedOption('Harness default model', modelLabel(model));
  await waitFor(() =>
    expect(screen.getByLabelText('Harness default model')).toHaveValue(modelLabel(model)),
  );
  chooseClosedOption('Harness default reasoning', reasoningLabel(reasoning));
  await waitFor(() =>
    expect(screen.getByLabelText('Harness default reasoning')).toHaveValue(
      reasoningLabel(reasoning),
    ),
  );
}

async function expectHarnessDefault(model: string, reasoning: string) {
  await waitFor(() => {
    expect(screen.getByLabelText('Harness default model')).toHaveValue(modelLabel(model));
    expect(screen.getByLabelText('Harness default reasoning')).toHaveValue(
      reasoningLabel(reasoning),
    );
  });
}

async function setTerraMaximum(index: number) {
  await waitFor(() =>
    expect(
      screen.getByRole('combobox', {
        name: 'Harness GPT-5.6 Terra maximum reasoning',
      }),
    ).toBeEnabled(),
  );
  chooseClosedOption(
    'Harness GPT-5.6 Terra maximum reasoning',
    ['Low', 'Medium', 'High', 'Xhigh'][index],
  );
}

function chooseClosedOption(label: string, option: string | RegExp) {
  const selector = screen.getByRole('combobox', { name: label });
  fireEvent.focus(selector);
  const listbox = document.getElementById(selector.getAttribute('aria-controls') ?? '');
  if (!listbox) throw new Error(`${label} did not open its option list.`);
  fireEvent.click(within(listbox).getByRole('option', { name: option }));
}

async function toggleAllowedModel(model: string) {
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Allowed models' })).toBeEnabled(),
  );
  chooseClosedOption('Allowed models', model);
}

function modelLabel(model: string): string {
  if (!model) return 'Caller choice';
  return model === 'gpt-5.6-terra' ? 'GPT-5.6 Terra' : 'GPT-5.6 Sol';
}

function reasoningLabel(reasoning: string): string {
  return reasoning ? reasoning[0].toUpperCase() + reasoning.slice(1) : 'Caller choice';
}

describe('HarnessAwareAgentSessionPane', () => {
  it('offers management only after the Session-owned source returns a harness relationship', async () => {
    const source = createRecordedHarnessManagementSource();
    const { rerender } = render(
      <HarnessAwareAgentSessionPane sessionId="session-without-harness">
        <div>Neutral conversation</div>
      </HarnessAwareAgentSessionPane>,
    );
    expect(screen.queryByRole('button', { name: 'Manage harness' })).toBeNull();

    rerender(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <div>Product conversation</div>
      </HarnessAwareAgentSessionPane>,
    );
    expect(await screen.findByRole('button', { name: 'Manage harness' })).toBeVisible();
  });

  it('keeps individual model and effort choices in the Agent Session view within Harness constraints', async () => {
    const source = createRecordedHarnessManagementSource();
    render(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );

    const model = await screen.findByLabelText('Session model');
    const effort = screen.getByLabelText('Session effort');
    expect(screen.getByText('This Session · v3 constraints')).toBeVisible();
    expect(model).toHaveValue('');
    expect(effort).toBeDisabled();
    expect(within(model).getByRole('option', { name: 'GPT-5.6 Terra' })).toBeVisible();
    expect(within(model).getByRole('option', { name: 'GPT-5.6 Sol' })).toBeVisible();

    fireEvent.change(model, { target: { value: 'gpt-5.6-terra' } });
    await waitFor(() => expect(model).toHaveValue('gpt-5.6-terra'));
    expect(effort).toBeEnabled();
    expect(within(effort).getByRole('option', { name: 'xhigh' })).toBeVisible();
    fireEvent.change(effort, { target: { value: 'xhigh' } });
    await waitFor(() => expect(effort).toHaveValue('xhigh'));

    await openHarnessManagement();
    expect(screen.queryByLabelText('Session model')).toBeNull();
    expect(screen.queryByLabelText('Session effort')).toBeNull();
    expect(screen.queryByText('Current Session resolves to')).toBeNull();
    expect(screen.queryByRole('region', { name: 'Current Session model override' })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    expect(await screen.findByLabelText('Session model')).toHaveValue('gpt-5.6-terra');
    expect(screen.getByLabelText('Session effort')).toHaveValue('xhigh');
  });

  it('defaults to the Session version and presents the corrected management hierarchy', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <section aria-label="Product conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    expect(await screen.findByRole('heading', { name: 'Harness details' })).toBeVisible();
    expect(screen.getByTestId('harness-definition-editor')).toBeVisible();
    expect(screen.queryByLabelText('Product conversation')).toBeNull();
    expect(screen.getByLabelText('Viewed harness version').getAttribute('value')).toContain('v3');
    expect(
      screen.getByRole('button', {
        name: 'Newest pushed: v4 · Next-prompt update policy',
      }),
    ).toBeVisible();
    expect(screen.queryByText('Viewed version is not pushed')).toBeNull();
    fireEvent.focus(screen.getByLabelText('Viewed harness version'));
    expect(screen.queryByText(/Session version ·/)).toBeNull();
    expect(screen.queryByText(/Current pushed ·/)).toBeNull();
    expect(screen.getByRole('option', { name: 'v3 · Session binding baseline' })).toBeVisible();
    expect(screen.getByRole('option', { name: 'v4 · Next-prompt update policy' })).toBeVisible();
    expect(screen.getByText('Epic Plan Builder', { selector: 'strong' })).toBeVisible();
    expect(screen.queryByLabelText('Harness role')).toBeNull();
    expect(screen.getByRole('heading', { name: 'Prompt prefix' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Skills' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Tools and MCP exposure' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Models and reasoning' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Application hooks' })).toBeVisible();
    expect(screen.getByText(/Proposed application hook reference:/)).toBeVisible();
    expect(screen.getByText('Proposed')).toBeVisible();
    expect(screen.queryByText(/Connected application hook:/)).toBeNull();
    expect(screen.queryByText('Exposed')).toBeNull();
    expect(screen.getByRole('table')).toHaveAccessibleName('');
    expect(screen.getByRole('heading', { name: 'Version history' })).toBeVisible();
    expect(screen.queryByRole('heading', { name: 'Agent Session updates' })).toBeNull();
    expect(
      screen.getByRole('combobox', { name: 'Harness GPT-5.6 Terra minimum reasoning' }),
    ).toBeEnabled();
    expect(screen.getAllByLabelText('Avery, Epic Plan Builder').length).toBeGreaterThan(0);
    expect(
      screen.queryByText(/validation|provenance|delivery not evidenced|future invocation/i),
    ).toBeNull();

    expect(screen.getByText('epic-plan-builder', { selector: 'output' })).toBeVisible();
    expect(screen.getAllByText(/submit_epic_plan_proposal/).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await waitFor(() => expect(screen.getByLabelText('Product conversation')).toBeVisible());
  });

  it('inspects and edits the Harness name subset without renaming the existing Session', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });

    expect(screen.getByText(/Eero Saarinen/, { selector: 'output' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    await waitFor(() => expect(screen.getByLabelText('Permitted Agent names')).toBeEnabled());
    const permitted = screen.getByLabelText('Permitted Agent names');
    fireEvent.focus(permitted);
    fireEvent.change(permitted, { target: { value: 'Grace Hopper' } });
    fireEvent.click(screen.getByRole('option', { name: 'Grace Hopper' }));
    await waitFor(() => expect(permitted).toHaveAttribute('placeholder', '11 selected'));

    expect(screen.getAllByLabelText('Avery, Epic Plan Builder').length).toBeGreaterThan(0);
    expect(
      screen.getByText('Working draft · uncommitted', {
        selector: '.harness-management__badge',
      }),
    ).toBeVisible();
  }, 10_000);

  it('changes selected-skill applicability through the shared searchable definition control', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });
    expect(screen.getByText('Available', { selector: 'output' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    await waitFor(() =>
      expect(screen.getByLabelText('epic-plan-builder applicability')).toBeEnabled(),
    );
    const dialog = await screen.findByRole('dialog', { name: 'epic-plan-builder' });
    expect(within(dialog).getByText(/# Product Epic Plan Builder/)).toBeVisible();
    expect(
      within(dialog).getByText(/application can derive it from the calling session/),
    ).toBeVisible();
    const applicability = within(dialog).getByLabelText('epic-plan-builder details applicability');
    expect(applicability).toBeDisabled();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Edit skill policy' }));
    await waitFor(() => expect(applicability).toBeEnabled());
    fireEvent.change(applicability, { target: { value: 'always_applicable' } });
    await waitFor(() => expect(applicability).toHaveValue('always_applicable'));
    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Close epic-plan-builder details' }),
    );
  });

  it('keeps one prompt value and caret-safe rendered/Plain source across navigation and remount', async () => {
    const source = createRecordedHarnessManagementSource();
    const first = render(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <section aria-label="Planning view conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Edit harness' }));
    await waitFor(() => expect(screen.getByLabelText('Harness name')).toBeEnabled());
    expect(screen.getByRole('region', { name: 'Prompt prefix rendered Markdown' })).toBeVisible();
    expect(screen.queryByRole('toolbar')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    const plain = screen.getByLabelText('Prompt prefix plain Markdown') as HTMLTextAreaElement;
    const revisedPrefix = '# Revised prefix\n\nKeep this durable working copy.';
    fireEvent.change(plain, {
      target: {
        value: revisedPrefix,
      },
    });
    expect(
      await screen.findByText('Working draft · uncommitted', {
        selector: '.harness-management__badge',
      }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Rendered' }));
    expect(screen.getByRole('heading', { name: 'Revised prefix' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    expect(screen.getByLabelText('Prompt prefix plain Markdown')).toHaveValue(revisedPrefix);
    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await screen.findByLabelText('Planning view conversation');
    first.unmount();

    render(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <section aria-label="Agent Sessions view conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });

    expect(screen.getByLabelText('Viewed harness version').getAttribute('value')).toContain('v3');
    expect(screen.getByText('Working draft has uncommitted changes')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Edit draft' })).toBeVisible();
    chooseClosedOption('Viewed harness version', /Working draft/);
    const renderedHeading = await screen.findByRole('heading', { name: 'Revised prefix' });
    expect(renderedHeading).toBeVisible();
    expect(renderedHeading.closest('.agent-markdown')).toHaveClass(
      'harness-management__markdown-view',
    );
  });

  it('searches, adds, categorizes, and removes skills in the shared definition control', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });
    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    const skills = await waitFor(() => screen.getByRole('combobox', { name: 'Harness skills' }));
    fireEvent.focus(skills);
    fireEvent.change(skills, { target: { value: 'sprint' } });
    fireEvent.click(await screen.findByRole('option', { name: /sprint-runner/ }));
    const applicability = await screen.findByLabelText('sprint-runner applicability');
    chooseClosedOption('sprint-runner applicability', 'Always applicable');
    await waitFor(() => expect(applicability).toHaveValue('Always applicable'));

    fireEvent.focus(skills);
    fireEvent.click(await screen.findByRole('option', { name: /sprint-runner/ }));
    await waitFor(() => expect(screen.queryByLabelText('sprint-runner applicability')).toBeNull());
  });

  it('edits and removes runtime-owned tools through the shared searchable definition control', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });
    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    const exposure = await waitFor(() =>
      screen.getByRole('combobox', { name: 'request_epic_initiation exposure timing' }),
    );
    chooseClosedOption('request_epic_initiation exposure timing', 'Initial invocation only');
    await waitFor(() => expect(exposure).toHaveValue('Initial invocation only'));

    const tools = screen.getByLabelText('Harness tools');
    fireEvent.focus(tools);
    fireEvent.change(tools, { target: { value: 'submit_epic_plan_proposal' } });
    fireEvent.click(screen.getByRole('option', { name: /submit_epic_plan_proposal/ }));
    await waitFor(() =>
      expect(screen.queryByLabelText('submit_epic_plan_proposal exposure timing')).toBeNull(),
    );
    expect(screen.getByText(/schemas remain runtime-owned/i)).toBeVisible();
  });

  it('changes only the current Session identity with a color and shape', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });

    fireEvent.click(screen.getByRole('button', { name: 'Edit Agent identity for Avery' }));
    const dialog = await screen.findByRole('dialog', { name: 'Current Agent identity' });
    expect(within(dialog).getByText('Agent name')).toBeVisible();
    expect(within(dialog).getByText('Available names')).toBeVisible();
    expect(
      within(within(dialog).getByLabelText('Available Agent names')).getAllByRole('button'),
    ).toHaveLength(100);
    fireEvent.change(within(dialog).getByLabelText('Search available Agent names'), {
      target: { value: 'grcehpr' },
    });
    expect(await within(dialog).findByRole('button', { name: 'Grace Hopper' })).toBeVisible();
    fireEvent.change(within(dialog).getByLabelText('Agent name'), {
      target: { value: 'Mildred Plot Twist' },
    });
    fireEvent.change(within(dialog).getByLabelText('Agent identity color'), {
      target: { value: '#6f4ab5' },
    });
    fireEvent.click(within(dialog).getByRole('radio', { name: 'hexagon' }));
    fireEvent.click(within(dialog).getByRole('button', { name: 'Apply to this Session' }));

    expect(
      await screen.findByRole('button', {
        name: 'Edit Agent identity for Mildred Plot Twist',
      }),
    ).toBeVisible();
    expect(
      screen.getAllByLabelText('Mildred Plot Twist, Epic Plan Builder').length,
    ).toBeGreaterThan(0);
    expect(document.querySelector('.agent-identity-marker.is-hexagon')).toHaveStyle({
      '--agent-identity-accent': '#6f4ab5',
    });
  });

  it('enforces revision-owned edit mode while allowing delegated shared policy in view mode', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    await openHarnessManagement();

    const terraMaximum = screen.getByRole('combobox', {
      name: 'Harness GPT-5.6 Terra maximum reasoning',
    });
    expect(terraMaximum).toBeEnabled();
    expect(screen.getByText('Shared by recorded Sessions using v3.')).toBeVisible();
    expect(screen.getByLabelText('Harness GPT-5.6 Terra minimum reasoning')).toHaveValue('Low');
    expect(terraMaximum).toHaveValue('Xhigh');

    chooseClosedOption('Viewed harness version', /v4.*Next-prompt update policy/);
    await waitFor(() =>
      expect(
        screen.queryByRole('combobox', {
          name: 'Harness GPT-5.6 Terra maximum reasoning',
        }),
      ).toBeNull(),
    );
    expect(
      screen.getByText('Fixed by this revision; edit the Harness to change it.'),
    ).toBeVisible();
    expect(screen.queryByRole('combobox', { name: 'Harness default model' })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    await waitFor(() =>
      expect(
        screen.getByRole('combobox', {
          name: 'Harness GPT-5.6 Terra maximum reasoning',
        }),
      ).toBeEnabled(),
    );
    expect(screen.getByLabelText('Harness default model')).toBeEnabled();
    expect(screen.getByLabelText('Allowed models')).toBeEnabled();
  });

  it('restores untouched model and range defaults across delegated shared-policy cache writes', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    await openHarnessManagement();
    await setHarnessDefault('gpt-5.6-terra', 'high');

    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-sol', 'medium');
    expect(await screen.findByText('Recorded shared adjustment')).toBeVisible();
    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-terra', 'high');

    await setTerraMaximum(1);
    await expectHarnessDefault('gpt-5.6-terra', 'medium');
    await setTerraMaximum(3);
    await expectHarnessDefault('gpt-5.6-terra', 'high');

    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-sol', 'medium');
    chooseClosedOption('Harness default model', 'Caller choice');
    await expectHarnessDefault('', '');
    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('', '');

    await setHarnessDefault('gpt-5.6-terra', 'high');
    await setTerraMaximum(1);
    await expectHarnessDefault('gpt-5.6-terra', 'medium');
    chooseClosedOption('Harness default reasoning', 'Low');
    await expectHarnessDefault('gpt-5.6-terra', 'low');
    await setTerraMaximum(3);
    await expectHarnessDefault('gpt-5.6-terra', 'low');
  });

  it('keeps fallback memory through working-copy auto-cache and clears it at Finish editing', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    await openHarnessManagement();
    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    await waitFor(() => expect(screen.getByLabelText('Harness default model')).toBeEnabled());
    await setHarnessDefault('gpt-5.6-terra', 'high');

    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-sol', 'medium');
    expect(
      await screen.findByText('Working draft · uncommitted', {
        selector: '.harness-management__badge',
      }),
    ).toBeVisible();
    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-terra', 'high');

    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-sol', 'medium');
    fireEvent.click(screen.getByRole('button', { name: 'Finish editing' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Edit draft' }));
    await waitFor(() => expect(screen.getByLabelText('Allowed models')).toBeEnabled());
    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-sol', 'medium');
  });

  it('clears range and model fallback memory when leaving and returning', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    await openHarnessManagement();
    await setHarnessDefault('gpt-5.6-terra', 'high');

    await setTerraMaximum(1);
    await expectHarnessDefault('gpt-5.6-terra', 'medium');
    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await screen.findByText('Conversation body');
    await openHarnessManagement();
    await setTerraMaximum(3);
    await expectHarnessDefault('gpt-5.6-terra', 'medium');

    await setHarnessDefault('gpt-5.6-terra', 'high');
    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-sol', 'medium');
    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await screen.findByText('Conversation body');
    await openHarnessManagement();
    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-sol', 'medium');
  });

  it('clears combined model and range fallback memory when the draft is committed', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    await openHarnessManagement();
    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    await waitFor(() => expect(screen.getByLabelText('Harness default model')).toBeEnabled());
    await setHarnessDefault('gpt-5.6-terra', 'high');

    await setTerraMaximum(1);
    await expectHarnessDefault('gpt-5.6-terra', 'medium');
    await toggleAllowedModel('GPT-5.6 Terra');
    await expectHarnessDefault('gpt-5.6-sol', 'medium');

    fireEvent.click(screen.getByRole('button', { name: 'Commit' }));
    const confirmation = screen.getByRole('alertdialog', {
      name: 'Commit this harness version?',
    });
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Commit version' }));
    await waitFor(() =>
      expect(screen.getByLabelText('Viewed harness version').getAttribute('value')).toContain('v5'),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Finish editing' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Edit harness' }));
    await waitFor(() => expect(screen.getByLabelText('Allowed models')).toBeEnabled());

    await toggleAllowedModel('GPT-5.6 Terra');
    await setTerraMaximum(3);
    await expectHarnessDefault('gpt-5.6-sol', 'medium');
  });

  it('confirmation-gates Session changes, commit, push, and bulk next-prompt queues', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });

    chooseClosedOption('Viewed harness version', /v4.*Next-prompt update policy/);
    fireEvent.click(screen.getByRole('button', { name: 'Use v4 for this Session' }));
    let confirmation = screen.getByRole('alertdialog', {
      name: 'Change this Session to v4?',
    });
    expect(
      within(confirmation).getByText(/recorded next-prompt update is consumed/i),
    ).toBeVisible();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByText('Queued for next prompt')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    await waitFor(() => expect(screen.getByLabelText('Harness name')).toBeEnabled());
    fireEvent.change(screen.getByLabelText('Harness name'), {
      target: { value: 'Epic Plan Builder Plus' },
    });
    await screen.findByText('Working draft · uncommitted', {
      selector: '.harness-management__badge',
    });
    fireEvent.click(screen.getByRole('button', { name: 'Commit' }));
    confirmation = screen.getByRole('alertdialog', {
      name: 'Commit this harness version?',
    });
    expect(within(confirmation).getByText(/does not push.*Sessions/i)).toBeVisible();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Commit version' }));
    await waitFor(() =>
      expect(screen.getByLabelText('Viewed harness version').getAttribute('value')).toContain('v5'),
    );
    expect(screen.getByRole('button', { name: 'v5 · Harness settings update' })).toBeVisible();
    expect(screen.getByText('Committed', { selector: '.harness-management__badge' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Push' }));
    confirmation = screen.getByRole('alertdialog', { name: 'Push harness v5?' });
    expect(
      within(confirmation).getByText(/queues it.*next prompt.*does not contact a remote/i),
    ).toBeVisible();
    expect(within(confirmation).queryByText(/interrupt now/i)).toBeNull();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Push v5' }));
    await waitFor(() => expect(screen.getByText('Queued for next prompt')).toBeVisible());
    expect(screen.getByText('Current pushed')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Change all to v4' }));
    confirmation = screen.getByRole('alertdialog', {
      name: 'Change all relevant Sessions to v4?',
    });
    expect(
      within(confirmation).getByText(/each recorded next-prompt update is consumed/i),
    ).toBeVisible();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Queue v4 for all' }));
    await waitFor(() => expect(screen.getByText('Queued for next prompt')).toBeVisible());
    expect(screen.queryByRole('button', { name: /interrupt/i })).toBeNull();
  });

  it('keeps unavailable and unbound reads explicit', () => {
    const { rerender } = render(
      <ConversationHarnessManagement
        read={{ kind: 'unavailable', reason: 'The application query failed.' }}
        onBack={() => undefined}
      />,
    );
    expect(screen.getByText('Harness unavailable')).toBeVisible();
    expect(screen.getByText('The application query failed.')).toBeVisible();

    rerender(
      <ConversationHarnessManagement
        read={{ kind: 'unbound', reason: 'The Session has no harness relationship.' }}
        onBack={() => undefined}
      />,
    );
    expect(screen.getByText('No harness assigned')).toBeVisible();
    expect(screen.getByText('The Session has no harness relationship.')).toBeVisible();
  });

  it('does not offer a control when the source reports an unbound Session', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId="unknown-session"
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    expect(await screen.findByText('Conversation body')).toBeVisible();
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: 'Manage harness' })).toBeNull(),
    );
  });
});
