import assert from 'node:assert/strict';
import test from 'node:test';
import {
  deriveRequestBinding,
  fixedInvokeExpression,
} from '../epic-initiation-confirmation-request-dispatch.mjs';

const current = {
  draft: { id: 'epic-planning-draft-ff5cdb5043454f168b6079f92f47a164', status: 'active' },
  proposal: { id: 'proposal-revision-1ef8571e-a322-42f4-a144-e6a5e62e6f86', draftId: 'epic-planning-draft-ff5cdb5043454f168b6079f92f47a164', revisionToken: 'token' },
  sessionAssociation: { draftId: 'epic-planning-draft-ff5cdb5043454f168b6079f92f47a164', sessionId: 'be35756b-3a9d-4f99-ae86-1ebf19e00b9d' },
  initiationCount: 0,
};

test('derives only the current retained draft proposal and deterministic idempotency key', () => {
  assert.deepEqual(deriveRequestBinding(current), {
    epicPlanningDraftId: 'epic-planning-draft-ff5cdb5043454f168b6079f92f47a164',
    proposalRevisionId: 'proposal-revision-1ef8571e-a322-42f4-a144-e6a5e62e6f86',
    expectedRevisionToken: 'token',
    idempotencyKey: 'initiate:epic-planning-draft-ff5cdb5043454f168b6079f92f47a164:proposal-revision-1ef8571e-a322-42f4-a144-e6a5e62e6f86',
  });
  assert.throws(() => deriveRequestBinding({ ...current, initiationCount: 1 }), /already exists/);
  assert.throws(() => deriveRequestBinding({ ...current, proposal: { ...current.proposal, draftId: 'other' } }), /current proposal/);
});

test('invokes only the request command and never provides confirmation or root authority', () => {
  const expression = fixedInvokeExpression(deriveRequestBinding(current));
  assert.match(expression, /request_epic_initiation_confirmation/);
  assert.match(expression, /proposal-revision-1ef8571e-a322-42f4-a144-e6a5e62e6f86/);
  assert.doesNotMatch(expression, /resolve_epic_initiation_confirmation/);
  assert.doesNotMatch(expression, /rootBranch|root_branch|codex\/epic-workflow-ux-test/);
});
