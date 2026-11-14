import assert from 'node:assert/strict';
import test from 'node:test';
import type { Place, WorldCatalog } from '../tools/birthplaces/catalog.ts';
import { applyNameAssignments, type NameAssignment } from '../tools/birthplaces/name-assignments.ts';

const initial: Place = ['wd:Q1', '旧名', 'Old', 'R', 'US', 1, 100];

function catalog(): WorldCatalog {
  return { places: [[...initial]] };
}

function assignment(overrides: Partial<NameAssignment> = {}): NameAssignment {
  return {
    id: 'wd:Q1',
    english: 'Old',
    country: 'US',
    longitude: 1,
    oldName: '旧名',
    name: '新名',
    ...overrides,
  };
}

test('matching name assignments are applied after validation', () => {
  const world = catalog();
  assert.equal(applyNameAssignments(world, [assignment()]), 1);
  assert.equal(world.places[0][1], '新名');
});

test('identity drift or a rejected name leaves the catalog unchanged', () => {
  const world = catalog();
  assert.throws(() => applyNameAssignments(world, [assignment({ longitude: 2 })]), /Invalid or changed place/);
  assert.throws(() => applyNameAssignments(world, [assignment({ oldName: '其他' })]), /Invalid or changed place/);
  assert.throws(() => applyNameAssignments(world, [assignment()], () => false), /Invalid or changed place/);
  assert.equal(world.places[0][1], '旧名');
});

test('duplicate assignment IDs fail before any name is written', () => {
  const world = catalog();
  assert.throws(() => applyNameAssignments(world, [assignment(), assignment({ name: '另一个' })]), /Invalid or changed place/);
  assert.equal(world.places[0][1], '旧名');
});
