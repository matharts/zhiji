import assert from 'node:assert/strict';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join, relative, resolve } from 'node:path';
import test from 'node:test';
import { nameCoverage } from '../tools/birthplaces/report-name-coverage.ts';
import { rebuild } from '../tools/birthplaces/rebuild-catalog.ts';
import {
  MIGRATION_SNAPSHOTS,
  NAME_SNAPSHOTS,
  nameAuditSnapshots,
  rebuildStages,
  snapshotPath,
} from '../tools/birthplaces/pipeline.ts';

const root = resolve(import.meta.dirname, '..');

/** 递归枚举资源文件，用于比较重建前后的字节内容。 */
function* files(directory: string): Generator<string> {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) yield* files(path);
    else if (entry.isFile()) yield path;
  }
}

/** 只复制重放所需资源，避免测试污染仓库。 */
function fixture() {
  const directory = mkdtempSync(join(tmpdir(), 'zhiji-places-test-'));
  mkdirSync(join(directory, 'assets'), { recursive: true });
  mkdirSync(join(directory, 'tools'), { recursive: true });
  mkdirSync(join(directory, 'tools/birthplaces'), { recursive: true });
  cpSync(join(root, 'assets/data'), join(directory, 'assets/data'), { recursive: true });
  cpSync(join(root, 'assets/manifest.json'), join(directory, 'assets/manifest.json'));
  cpSync(join(root, 'tools/birthplaces/sources'), join(directory, 'tools/birthplaces/sources'), { recursive: true });
  return directory;
}

/** 返回资源和清单的相对路径摘要，失败时也能比较是否发生写入。 */
function hashes(directory: string): Record<string, string> {
  const paths = [...files(join(directory, 'assets/data'))].sort();
  paths.push(join(directory, 'assets/manifest.json'));
  return Object.fromEntries(paths.map(path => [
    relative(directory, path),
    createHash('sha256').update(readFileSync(path)).digest('hex'),
  ]));
}

test('pipeline snapshots are unique and present', () => {
  const snapshots = nameAuditSnapshots();
  assert.equal(new Set(snapshots).size, snapshots.length);
  const required = [
    ...MIGRATION_SNAPSHOTS.map(snapshotPath),
    ...snapshots,
    snapshotPath('overseas-major-city-selection'),
    snapshotPath('overseas-candidates'),
  ];
  for (const path of required) assert.ok(existsSync(join(root, path)), path);
  const stages = rebuildStages();
  assert.equal(stages.filter(stage => stage.script === 'migrate-catalog.ts').length, MIGRATION_SNAPSHOTS.length);
  assert.equal(stages.filter(stage => stage.script === 'apply-names.ts').length, NAME_SNAPSHOTS.length);
  assert.equal(stages.filter(stage => stage.script === 'audit-names.ts').length, 1);
});

test('name coverage excludes domestic places and reports overseas gaps', () => {
  const summary = nameCoverage([
    ['cn:1', '北京', 'Beijing', '北京', 'CN', 116.4, 100],
    ['wd:Q1', '东京', 'Tokyo', '东京', 'JP', 139.7, 100],
    ['wd:Q2', 'Example', 'Example', 'Region', 'US', -1, 100],
  ]);
  assert.equal(summary.total, 2);
  assert.equal(summary.containsHan, 1);
  assert.equal(summary.withoutHan, 1);
  assert.deepEqual(summary.missingByCountry, { US: 1 });
});

test('isolated rebuild reproduces current resources', () => {
  const directory = fixture();
  try {
    const before = hashes(directory);
    rebuild(directory, { quiet: true });
    assert.deepEqual(hashes(directory), before);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test('failed rebuild leaves every resource unchanged', () => {
  const directory = fixture();
  try {
    writeFileSync(join(directory, 'tools/birthplaces/sources/overseas-global-migration-2026-11-11.json'), '{broken');
    const before = hashes(directory);
    assert.throws(() => rebuild(directory, { quiet: true }), /failed|JSON|Unexpected/);
    assert.deepEqual(hashes(directory), before);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
