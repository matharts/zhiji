import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const root = resolve(import.meta.dirname, '..');
const script = resolve(root, 'tools/overseas-candidate-audit.mjs');
// 在独立进程中运行审计，保留退出码与标准错误供失败断言使用。
const run = (...args) => spawnSync(process.execPath, [script, ...args], { cwd: root, encoding: 'utf8' });

// 保留可确认的城市名称，将国家归属冲突及未核验关系留在审阅结果中。
test('candidate audit keeps city names and quarantines unresolved claims', () => {
  const dir = mkdtempSync(join(tmpdir(), 'zhiji-overseas-'));
  try {
    const result = run('tools/data-sources/overseas-candidates-2026-11-11.json', join(dir, 'audit.json'));
    assert.equal(result.status, 0, result.stderr);
    const { candidates } = JSON.parse(readFileSync(join(dir, 'audit.json'), 'utf8'));
    // 按稳定实体 ID 取回候选，避免断言依赖输出顺序。
    const get = id => candidates.find(item => item.id === id);
    assert.deepEqual([get('Q60').country, get('Q60').chinese], ['US', '纽约']);
    assert.deepEqual([get('Q84').country, get('Q84').chinese], ['GB', '伦敦']);
    assert.deepEqual([get('Q334').country, get('Q334').chinese], ['SG', '新加坡']);
    assert.deepEqual([get('Q231318').country, get('Q231318').chinese], ['MY', '新山']);
    assert.equal(get('Q220').country, null);
    assert.deepEqual(get('Q220').geographicCountries, ['IT']);
    assert.deepEqual(get('Q84').geographicCountries, ['GB']);
    assert.ok(get('Q220').reasons.includes('multiple-current-country-claims'));
    assert.ok(candidates.every(item => item.reasons.includes('hierarchy-unverified') && item.reasons.includes('timezone-unverified')));
  } finally { rmSync(dir, { recursive: true, force: true }); }
});

// 候选实体 ID 重复时拒绝审计，防止后行静默覆盖前行。
test('candidate audit rejects duplicate IDs rather than silently overwriting', () => {
  const dir = mkdtempSync(join(tmpdir(), 'zhiji-overseas-'));
  try {
    const row = { id: 'Q1', lat: 0, lon: 0, countries: [], types: [] };
    const file = join(dir, 'bad.json');
    writeFileSync(file, JSON.stringify({ countries: {}, supplements: [], candidates: [row, row] }));
    const result = run(file);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Invalid or duplicate QID/);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});
