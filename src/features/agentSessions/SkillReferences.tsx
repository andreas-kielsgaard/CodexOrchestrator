import {
  createContext,
  useContext,
  useEffect,
  useId,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
import type {
  AgentSessionQuickFeatures,
  AgentSkillDetails,
} from '../../application/agentSessions/quickFeatures';
import { ModalDialog } from '../../components/ModalDialog';
import { MarkdownContent } from '../../components/MarkdownContent';
import type { ComposerQuickFeatures } from './composerQuickActions';

type Skill = AgentSessionQuickFeatures['skills'][number];
const Context = createContext<{ skills: readonly Skill[]; open(skill: Skill): void }>({
  skills: [],
  open: () => {},
});
const NO_SKILLS: readonly Skill[] = [];

export function SkillReferenceProvider({
  source,
  children,
}: {
  source?: ComposerQuickFeatures;
  children: ReactNode;
}) {
  const [skill, setSkill] = useState<Skill | null>(null);
  const [details, setDetails] = useState<AgentSkillDetails | null>(null);
  const [error, setError] = useState<string | null>(null);
  const id = useId();
  const skills = source?.catalogue?.skills ?? NO_SKILLS;
  const context = useMemo(() => ({ skills, open: setSkill }), [skills]);
  useEffect(() => {
    setDetails(null);
    setError(null);
    if (!skill) return;
    let current = true;
    if (source?.loadSkillDetails)
      void source.loadSkillDetails(skill.id).then(
        (value) => {
          if (current) setDetails(value);
        },
        (cause) => {
          if (current) setError(String(cause));
        },
      );
    else setError('Skill content is unavailable for this session.');
    return () => {
      current = false;
    };
    // Opening a skill, not a controller refresh, performs the content read.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [skill]);
  return (
    <Context.Provider value={context}>
      {children}
      {skill && (
        <ModalDialog
          labelledBy={id}
          onClose={() => setSkill(null)}
          className="skill-details-dialog"
        >
          <header>
            <h2 id={id}>{skill.name}</h2>
            <button type="button" onClick={() => setSkill(null)}>
              Close
            </button>
          </header>
          <p>{skill.description}</p>
          <dl>
            <dt>Source</dt>
            <dd>{details?.source ?? skill.groupId ?? 'Unavailable'}</dd>
            <dt>Path</dt>
            <dd>{details?.path ?? skill.id}</dd>
            {details?.createdAt && (
              <>
                <dt>File creation date</dt>
                <dd>{new Date(details.createdAt).toLocaleString()}</dd>
              </>
            )}
          </dl>
          {error ? (
            <p role="alert">{error}</p>
          ) : details ? (
            <MarkdownContent>{details.content}</MarkdownContent>
          ) : (
            <p role="status">Loading skill content…</p>
          )}
        </ModalDialog>
      )}
    </Context.Provider>
  );
}

export function skillReferenceParts(text: string, skills: readonly Skill[]) {
  const byMention = new Map(skills.map((skill) => [skill.invocationText, skill]));
  // Full mention boundaries avoid highlighting partial names and ordinary prose.
  const parts: Array<{ text: string; skill?: Skill }> = [];
  let previous = 0;
  for (const match of text.matchAll(/\$[\w:./-]+/g)) {
    const mention = match[0].replace(/[.]+$/, '');
    const skill = byMention.get(mention);
    if (!skill) continue;
    parts.push({ text: text.slice(previous, match.index) }, { text: mention, skill });
    previous = match.index! + mention.length;
  }
  parts.push({ text: text.slice(previous) });
  return parts;
}
export function SkillReferenceText({ text }: { text: string }) {
  const { skills, open } = useContext(Context);
  return (
    <>
      {skillReferenceParts(text, skills).map((part, index) =>
        part.skill ? (
          <button
            type="button"
            className="skill-reference"
            key={index}
            onClick={() => open(part.skill!)}
          >
            {part.text}
          </button>
        ) : (
          part.text
        ),
      )}
    </>
  );
}
export function useSkillReferences() {
  return useContext(Context);
}
