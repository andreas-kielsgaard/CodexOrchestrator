import type { AgentIdentityDto, AgentVisualIdentityDto } from './contracts';

export const productDefaultAgentNames = [
  'Ada Lovelace',
  'Antoni Gaudi',
  'Grace Hopper',
  'Alan Turing',
  'Hedy Lamarr',
  'Claude Shannon',
  'Margaret Hamilton',
  'Katherine Johnson',
  'Radia Perlman',
  'Barbara Liskov',
  'Annie Easley',
  'Mary Jackson',
  'Dorothy Vaughan',
  'Jean Bartik',
  'Frances Allen',
  'Karen Sparck Jones',
  'Evelyn Boyd Granville',
  'Joan Clarke',
  'John von Neumann',
  'Donald Knuth',
  'Edsger Dijkstra',
  'Niklaus Wirth',
  'John Backus',
  'Ken Thompson',
  'Dennis Ritchie',
  'Brian Kernighan',
  'Alan Kay',
  'Douglas Engelbart',
  'Vint Cerf',
  'Bob Kahn',
  'Tim Berners-Lee',
  'James Gosling',
  'Guido van Rossum',
  'Yukihiro Matsumoto',
  'Brendan Eich',
  'Linus Torvalds',
  'Ken Kutaragi',
  'Shigeru Miyamoto',
  'Gunpei Yokoi',
  'Roberta Williams',
  'Carol Shaw',
  'Dona Bailey',
  'Ralph Baer',
  'Nolan Bushnell',
  'Sid Meier',
  'Will Wright',
  'Satoshi Tajiri',
  'John Carmack',
  'John Romero',
  'Jordan Mechner',
  'Zaha Hadid',
  'Maya Lin',
  'I M Pei',
  'Frank Lloyd Wright',
  'Eero Saarinen',
  'Lina Bo Bardi',
  'Le Corbusier',
  'Buckminster Fuller',
  'Jane Jacobs',
  'Christopher Wren',
  'Vitruvius',
  'Isambard Brunel',
  'Emily Roebling',
  'Fazlur Rahman Khan',
  'Gustave Eiffel',
  'Joseph Strauss',
  'Santiago Calatrava',
  'Norman Foster',
  'Renzo Piano',
  'Moshe Safdie',
  'Usain Bolt',
  'Wilma Rudolph',
  'Eliud Kipchoge',
  'Kathrine Switzer',
  'Roger Bannister',
  'Florence Griffith Joyner',
  'Abebe Bikila',
  'Joan Benoit',
  'Steve Prefontaine',
  'Allyson Felix',
  'Jackie Joyner-Kersee',
  'Jesse Owens',
  'Emil Zatopek',
  'Paavo Nurmi',
  'Fanny Blankers-Koen',
  'Cathy Freeman',
  'Michael Johnson',
  'Haile Gebrselassie',
  'Mo Farah',
  'Sifan Hassan',
  'Archimedes',
  'Hypatia',
  'Leonardo da Vinci',
  'Johannes Gutenberg',
  'Johannes Kepler',
  'Marie Curie',
  'Niels Bohr',
  'Emmy Noether',
  'Srinivasa Ramanujan',
  'George Boole',
] as const;

export const harnessAgentNamePools = {
  epic_plan_builder: [
    'Antoni Gaudi',
    'Zaha Hadid',
    'Maya Lin',
    'I M Pei',
    'Frank Lloyd Wright',
    'Eero Saarinen',
    'Lina Bo Bardi',
    'Buckminster Fuller',
    'Jane Jacobs',
    'Christopher Wren',
  ],
  epic_bootstrap_generator: [
    'Ada Lovelace',
    'Grace Hopper',
    'Alan Turing',
    'Hedy Lamarr',
    'Claude Shannon',
    'Margaret Hamilton',
    'Radia Perlman',
    'Barbara Liskov',
    'Annie Easley',
    'Jean Bartik',
  ],
  epic_runner: [
    'Usain Bolt',
    'Wilma Rudolph',
    'Eliud Kipchoge',
    'Kathrine Switzer',
    'Roger Bannister',
    'Florence Griffith Joyner',
    'Abebe Bikila',
    'Joan Benoit',
    'Steve Prefontaine',
    'Allyson Felix',
  ],
} as const;

export const harnessVisualIdentities = {
  epic_plan_builder: { token: 'drafting_compass', accent: '#39745a' },
  epic_bootstrap_generator: { token: 'bootstrap_package', accent: '#9a6730' },
  epic_runner: { token: 'runner_route', accent: '#466d98' },
} as const satisfies Record<string, AgentVisualIdentityDto>;

export interface AssignAgentIdentityInput {
  readonly sessionId: string;
  readonly harnessKey: string;
  readonly harnessRole: string;
  readonly harnessRevision: number;
  readonly visualIdentity: AgentVisualIdentityDto;
  readonly permittedNames?: readonly string[] | null;
  readonly existingNames?: readonly string[];
  readonly assignedAt: string;
  readonly assignmentKind?: AgentIdentityDto['assignment']['kind'];
}

/** Assignment runs once at Session creation; persistence, not re-running this function, owns stability. */
export function assignAgentIdentity(input: AssignAgentIdentityInput): AgentIdentityDto {
  const pool = validateAgentNamePool(input.permittedNames ?? productDefaultAgentNames);
  const used = new Set((input.existingNames ?? []).map(normalizeName));
  const start = stableHash(`${input.harnessKey}:${input.sessionId}`) % pool.length;
  let name: string | null = null;
  for (let offset = 0; offset < pool.length; offset += 1) {
    const candidate = pool[(start + offset) % pool.length];
    if (!used.has(normalizeName(candidate))) {
      name = candidate;
      break;
    }
  }
  if (!name) {
    const base = pool[start];
    let suffix = 2;
    name = `${base} ${suffix}`;
    while (used.has(normalizeName(name))) {
      suffix += 1;
      name = `${base} ${suffix}`;
    }
  }
  return {
    name,
    harnessRole: input.harnessRole,
    visualIdentity: input.visualIdentity,
    appliedHarnessRevision: input.harnessRevision,
    assignment: {
      kind: input.assignmentKind ?? 'durable',
      pool: input.permittedNames ? 'harness_subset' : 'product_default',
      assignedAt: input.assignedAt,
    },
  };
}

export function validateAgentNamePool(pool: readonly string[]): readonly string[] {
  if (pool.length === 0) throw new Error('Agent name pool must not be empty.');
  const names = pool.map((name) => name.trim());
  if (names.some((name) => !name)) throw new Error('Agent names must not be blank.');
  if (new Set(names.map(normalizeName)).size !== names.length)
    throw new Error('Agent names must be unique.');
  return names;
}

export function validateCuratedHarnessNamePools(): void {
  const defaults = new Set(productDefaultAgentNames.map(normalizeName));
  for (const pool of Object.values(harnessAgentNamePools)) {
    for (const name of validateAgentNamePool(pool)) {
      if (!defaults.has(normalizeName(name)))
        throw new Error(`Curated Agent name is outside the product pool: ${name}`);
    }
  }
}

function normalizeName(value: string): string {
  return value.trim().toLocaleLowerCase('en-US');
}

function stableHash(value: string): number {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}
