import { ArrowLeft, CheckCircle2, CircleHelp, LockKeyhole, ShieldAlert } from 'lucide-react';
import type { ReactNode } from 'react';
import type { ConversationHarnessInspectorRead } from '../../application/conversationHarnesses';

export interface ConversationHarnessInspectorProps {
  readonly read: ConversationHarnessInspectorRead | null;
  onBack(): void;
}

export function ConversationHarnessInspector({ read, onBack }: ConversationHarnessInspectorProps) {
  if (!read)
    return (
      <InspectorShell onBack={onBack}>
        <p className="harness-inspector__loading" role="status">
          Loading harness configuration...
        </p>
      </InspectorShell>
    );

  if (read.kind === 'unavailable')
    return (
      <InspectorShell onBack={onBack}>
        <div className="harness-inspector__unavailable" role="alert">
          <ShieldAlert size={20} aria-hidden="true" />
          <div>
            <h2>Harness configuration unavailable</h2>
            <p>{read.reason}</p>
          </div>
        </div>
      </InspectorShell>
    );

  const { snapshot } = read;
  return (
    <InspectorShell onBack={onBack}>
      <header className="harness-inspector__title">
        <div>
          <p className="eyebrow">Conversation Harness</p>
          <h2>{snapshot.profile.title}</h2>
          <p>
            {snapshot.profile.key} · profile v{snapshot.profile.version}
          </p>
        </div>
        <div className="harness-inspector__title-badges" aria-label="Inspector status">
          <StatusBadge
            tone={snapshot.validation.status === 'valid' ? 'positive' : 'caution'}
            label={snapshot.validation.status === 'valid' ? 'Validated snapshot' : 'Unverified'}
          />
          <StatusBadge tone="neutral" label="Read-only exploration" />
        </div>
      </header>

      <div className="harness-inspector__boundary-note">
        <LockKeyhole size={18} aria-hidden="true" />
        <p>
          Delivered context is locked to this session. Controls marked{' '}
          <strong>Next invocation</strong> show the proposed editing surface, but this prototype
          cannot apply changes.
        </p>
      </div>

      <div className="harness-inspector__grid">
        <InspectorCard
          title="Prompt and context"
          stateLabel="Delivered · immutable"
          stateTone="locked"
          description={snapshot.promptContext.state.reason}
          wide
        >
          <label className="harness-inspector__field">
            <span>Initial context prefix</span>
            <textarea value={snapshot.promptContext.content} readOnly rows={8} />
          </label>
          <p className="harness-inspector__field-note">
            Delivery: {humanize(snapshot.promptContext.delivery)}
          </p>
        </InspectorCard>

        <InspectorCard
          title="Skills"
          stateLabel="Next invocation · read only"
          stateTone="future"
          description={snapshot.skills.state.reason}
        >
          <div className="harness-inspector__choice-list">
            {snapshot.skills.items.map((skill) => (
              <label key={skill.name} className="harness-inspector__choice">
                <input type="checkbox" checked disabled readOnly />
                <span>
                  <strong>{skill.name}</strong>
                  <small>{skill.purpose}</small>
                  <code>{skill.path}</code>
                  <small>Use when {skill.useWhen}</small>
                </span>
              </label>
            ))}
          </div>
        </InspectorCard>

        <InspectorCard
          title="MCP tools"
          stateLabel="Next invocation · read only"
          stateTone="future"
          description={snapshot.mcp.state.reason}
        >
          <div className="harness-inspector__choice-list">
            {snapshot.mcp.tools.map((tool) => (
              <label key={tool} className="harness-inspector__choice">
                <input type="checkbox" checked disabled readOnly />
                <span>
                  <strong>{tool}</strong>
                  <small>Enabled by the profile allow-list</small>
                </span>
              </label>
            ))}
          </div>
          <p className="harness-inspector__field-note">
            MCP server: {snapshot.mcp.required ? 'required' : 'optional'}
          </p>
        </InspectorCard>

        <InspectorCard
          title="Model and reasoning"
          stateLabel="Next invocation · read only"
          stateTone="future"
          description={snapshot.runtime.state.reason}
        >
          <div className="harness-inspector__field-row">
            <label className="harness-inspector__field">
              <span>Model</span>
              <select value={snapshot.runtime.model ?? 'inherited'} disabled>
                <option value="inherited">Inherited</option>
                {snapshot.runtime.model && (
                  <option value={snapshot.runtime.model}>{snapshot.runtime.model}</option>
                )}
              </select>
            </label>
            <label className="harness-inspector__field">
              <span>Reasoning effort</span>
              <select value={snapshot.runtime.reasoningEffort ?? 'inherited'} disabled>
                <option value="inherited">Inherited</option>
                {snapshot.runtime.reasoningEffort && (
                  <option value={snapshot.runtime.reasoningEffort}>
                    {snapshot.runtime.reasoningEffort}
                  </option>
                )}
              </select>
            </label>
          </div>
        </InspectorCard>

        <InspectorCard
          title="Sandbox and authority"
          stateLabel="Next invocation · policy bounded"
          stateTone="future"
          description={snapshot.runtime.authorityBoundary}
        >
          <div className="harness-inspector__field-row">
            <label className="harness-inspector__field">
              <span>Sandbox</span>
              <select value={snapshot.runtime.sandbox} disabled>
                <option value={snapshot.runtime.sandbox}>
                  {humanize(snapshot.runtime.sandbox)}
                </option>
              </select>
            </label>
            <label className="harness-inspector__field">
              <span>Approval policy</span>
              <select value={snapshot.runtime.approvalPolicy} disabled>
                <option value={snapshot.runtime.approvalPolicy}>
                  {snapshot.runtime.approvalPolicy}
                </option>
              </select>
            </label>
          </div>
        </InspectorCard>

        <InspectorCard
          title="Application hooks"
          stateLabel="Application owned"
          stateTone="locked"
          description={snapshot.hooks.state.reason}
          wide
        >
          <ul className="harness-inspector__hook-list">
            {snapshot.hooks.items.map((hook) => (
              <li key={hook.name}>
                <div>
                  <strong>{hook.name}</strong>
                  <p>{hook.detail}</p>
                </div>
                <StatusBadge
                  tone={hook.status === 'configured' ? 'positive' : 'caution'}
                  label={humanize(hook.status)}
                />
              </li>
            ))}
          </ul>
        </InspectorCard>

        <InspectorCard
          title="Validation and provenance"
          stateLabel={`Catalog schema v${snapshot.profile.catalogSchemaVersion}`}
          stateTone="neutral"
          description={snapshot.provenance.summary}
          wide
        >
          <dl className="harness-inspector__provenance">
            <div>
              <dt>Source</dt>
              <dd>{snapshot.provenance.source}</dd>
            </div>
            <div>
              <dt>Read boundary</dt>
              <dd>{humanize(snapshot.provenance.kind)}</dd>
            </div>
            <div>
              <dt>Session binding</dt>
              <dd>{snapshot.sessionId}</dd>
            </div>
          </dl>
          <ul className="harness-inspector__validation-list">
            {snapshot.validation.checks.map((check) => (
              <li key={check.label}>
                {check.status === 'passed' ? (
                  <CheckCircle2 size={17} aria-hidden="true" />
                ) : (
                  <CircleHelp size={17} aria-hidden="true" />
                )}
                <span>
                  <strong>{check.label}</strong>
                  <small>{check.detail}</small>
                </span>
              </li>
            ))}
          </ul>
        </InspectorCard>
      </div>

      <section className="harness-inspector__apply" aria-label="Safe apply semantics">
        <div>
          <p className="eyebrow">Safe apply boundary</p>
          <h3>Changes would create future configuration, not rewrite this session.</h3>
          <ul>
            {snapshot.apply.safeSemantics.map((semantic) => (
              <li key={semantic}>{semantic}</li>
            ))}
          </ul>
          <p>{snapshot.apply.reason}</p>
        </div>
        <button type="button" disabled>
          Apply to future invocation
        </button>
      </section>
    </InspectorShell>
  );
}

function InspectorShell({ children, onBack }: { readonly children: ReactNode; onBack(): void }) {
  return (
    <section className="harness-inspector" aria-label="Conversation Harness inspector">
      <div className="harness-inspector__toolbar">
        <button type="button" onClick={onBack}>
          <ArrowLeft size={16} aria-hidden="true" />
          Back to conversation
        </button>
      </div>
      <div className="harness-inspector__scroll">{children}</div>
    </section>
  );
}

function InspectorCard({
  title,
  stateLabel,
  stateTone,
  description,
  wide = false,
  children,
}: {
  readonly title: string;
  readonly stateLabel: string;
  readonly stateTone: 'locked' | 'future' | 'neutral';
  readonly description: string;
  readonly wide?: boolean;
  readonly children: ReactNode;
}) {
  return (
    <section className={`harness-inspector__card${wide ? ' is-wide' : ''}`}>
      <header>
        <h3>{title}</h3>
        <span className={`harness-inspector__scope is-${stateTone}`}>{stateLabel}</span>
      </header>
      <p className="harness-inspector__description">{description}</p>
      {children}
    </section>
  );
}

function StatusBadge({
  tone,
  label,
}: {
  readonly tone: 'positive' | 'caution' | 'neutral';
  readonly label: string;
}) {
  return <span className={`harness-inspector__status is-${tone}`}>{label}</span>;
}

function humanize(value: string): string {
  return value.replaceAll('_', ' ');
}
