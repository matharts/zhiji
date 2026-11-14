import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { buildChinaCatalog } from '../tools/birthplaces/generate-china-divisions.ts';

const root = resolve(import.meta.dirname, '..');
const tree = [{
  code: '11',
  name: '北京市',
  children: [{
    code: '1101',
    name: '北京市',
    children: [{ code: '110101', name: '东城区' }],
  }],
}];
const points = [{ code: '1101', name: '北京市', province: '北京市', longitude: 116.4 }];
const countyPoints = [{ code: '110101', name: '东城区', province: '北京市', longitude: 116.42 }];

test('domestic divisions replace old IDs and preserve unrelated timezones', () => {
  const current = { 'cn:old': ['Asia/Shanghai'], 'us:example': ['America/New_York'] };
  const result = buildChinaCatalog(tree, points, countyPoints, current);
  assert.deepEqual(current, { 'cn:old': ['Asia/Shanghai'], 'us:example': ['America/New_York'] });
  assert.equal(result.zones['cn:old'], undefined);
  assert.deepEqual(result.zones['cn:110101'], ['Asia/Shanghai']);
  assert.deepEqual(result.data.provinces[0].cities[0], {
    id: 'cn:110100',
    name: '北京市',
    longitude: 116.4,
    counties: [{ id: 'cn:110101', name: '东城区', longitude: 116.42 }],
  });
});

test('ambiguous representative points reject the whole catalog', () => {
  assert.throws(
    () => buildChinaCatalog(tree, [...points, ...points], countyPoints, {}),
    /Missing or ambiguous coordinate/,
  );
});

test('domestic tool reproduces the committed catalog and timezone order', () => {
  const read = (path: string): any => JSON.parse(readFileSync(join(root, path), 'utf8'));
  const result = buildChinaCatalog(
    read('tools/birthplaces/sources/mca-division-tree-2026-11-10.json').data.children,
    read('tools/birthplaces/sources/mca-birthplace-points-2026-11-10.json').points,
    read('tools/birthplaces/sources/mca-county-points-2026-11-11.json').points,
    read('assets/data/birthplace-timezones.json'),
  );
  assert.equal(JSON.stringify(result.data), readFileSync(join(root, 'assets/data/birthplace-china.json'), 'utf8').trim());
  assert.equal(JSON.stringify(result.zones), readFileSync(join(root, 'assets/data/birthplace-timezones.json'), 'utf8').trim());
});
