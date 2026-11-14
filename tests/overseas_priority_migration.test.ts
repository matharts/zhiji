import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const script = resolve(import.meta.dirname, '../tools/birthplaces/migrate-catalog.ts');

// 即使前一行有效，后续非法人口值也必须使迁移整体失败且不写入资源。
test('invalid population rejects migration without changing any resources', () => {
  const dir = mkdtempSync(join(tmpdir(), 'zhiji-migration-'));
  try {
    mkdirSync(join(dir, 'assets/data'), { recursive: true });
    mkdirSync(join(dir, 'tools/birthplaces/sources'), { recursive: true });
    const world = { places: [['1', 'A', 'A', 'R', 'US', -1, 100], ['2', 'B', 'B', 'R', 'US', -2, 100]] };
    const zones = { 1: ['America/New_York'], 2: ['America/New_York'] };
    const manifest = { version: 1, sha256: {} };
    // 构造完整迁移行，仅改变目标字段以验证非法人口值的原子拒绝。
    const row = (oldId: string, id: string, english: string, longitude: number, population: number) => ({
      oldId, id, oldEnglish: english, english, oldCountry: 'US', country: 'US',
      oldLongitude: longitude, longitude, name: english, region: 'R', population,
      timezones: ['America/New_York'],
    });
    const snapshot = { regionNames: {}, places: [row('1', 'wd:Q1', 'A', -1, 100), row('2', 'wd:Q2', 'B', -2, 101.568)] };
    const files: [string, unknown][] = [
      ['assets/data/birthplace-regions.json', world],
      ['assets/data/birthplace-timezones.json', zones],
      ['assets/manifest.json', manifest],
      ['tools/birthplaces/sources/overseas-priority-migration-2026-11-11.json', snapshot],
    ];
    for (const [path, content] of files) writeFileSync(join(dir, path), JSON.stringify(content));
    const before = ['assets/data/birthplace-regions.json', 'assets/data/birthplace-timezones.json', 'assets/manifest.json']
      .map(path => readFileSync(join(dir, path), 'utf8'));
    const result = spawnSync(process.execPath, [script], {
      env: { ...process.env, ZHIJI_PLACES_ROOT: dir }, encoding: 'utf8',
    });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Invalid migration: wd:Q2/);
    assert.deepEqual(['assets/data/birthplace-regions.json', 'assets/data/birthplace-timezones.json', 'assets/manifest.json']
      .map(path => readFileSync(join(dir, path), 'utf8')), before);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});

// 清单损坏时必须在任何资源替换前失败，不能让地点或时区留下半成品。
test('invalid manifest rejects migration without changing resources', () => {
  const dir = mkdtempSync(join(tmpdir(), 'zhiji-manifest-'));
  try {
    mkdirSync(join(dir, 'assets/data'), { recursive: true });
    mkdirSync(join(dir, 'tools/birthplaces/sources'), { recursive: true });
    const world = { places: [['1', 'A', 'A', 'R', 'US', -1, 100]] };
    const zones = { 1: ['America/New_York'] };
    const migration = {
      regionNames: {},
      places: [{
        oldId: '1', id: 'wd:Q1', oldEnglish: 'A', english: 'A', oldCountry: 'US', country: 'US',
        oldLongitude: -1, longitude: -1, name: 'A', region: 'R', population: 100,
        timezones: ['America/New_York'],
      }],
    };
    const files: [string, unknown][] = [
      ['assets/data/birthplace-regions.json', world],
      ['assets/data/birthplace-timezones.json', zones],
      ['tools/birthplaces/sources/overseas-priority-migration-2026-11-11.json', migration],
    ];
    for (const [path, content] of files) writeFileSync(join(dir, path), JSON.stringify(content));
    writeFileSync(join(dir, 'assets/manifest.json'), '{broken');
    const paths = ['assets/data/birthplace-regions.json', 'assets/data/birthplace-timezones.json'];
    const before = paths.map(path => readFileSync(join(dir, path), 'utf8'));
    const result = spawnSync(process.execPath, [script], {
      env: { ...process.env, ZHIJI_PLACES_ROOT: dir }, encoding: 'utf8',
    });
    assert.notEqual(result.status, 0);
    assert.deepEqual(paths.map(path => readFileSync(join(dir, path), 'utf8')), before);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});
