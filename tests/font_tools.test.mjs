import { strict as assert } from 'node:assert';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { coveredCodepoints, renameFont, tables } from '../tools/font-sfnt.mjs';

// 字体改名仅影响名称及必要头部字段，字符覆盖与整体校验和仍须有效。
test('renaming preserves font tables, cmap and OpenType checksum', () => {
  const original = readFileSync(new URL('../assets/fonts/ZhijiHanSupplement.ttf', import.meta.url));
  const result = renameFont(original, 'Zhiji Test Serif', 'ZhijiTestSerif', 'Regular');
  const before = tables(original), after = tables(result);
  for (const [tag, table] of before) {
    if (tag !== 'name' && tag !== 'head' && tag !== 'DSIG') assert.deepEqual(after.get(tag), table, tag);
  }
  assert.deepEqual([...coveredCodepoints(result)].sort((a,b)=>a-b), [...coveredCodepoints(original)].sort((a,b)=>a-b));
  let sum = 0;
  for (let i=0; i<result.length; i+=4) sum = (sum + result.readUInt32BE(i)) >>> 0;
  assert.equal(sum, 0xb1b0afba);
  assert.notDeepEqual(after.get('name'), before.get('name'));
});

// 可变字体改名后保留变体轴数据和原有 Unicode 覆盖。
test('variable font rename retains variation table and Unicode coverage', () => {
  const original = readFileSync(new URL('../assets/fonts/ZhijiChineseSerif-VF.otf', import.meta.url));
  const result = renameFont(original, 'Zhiji Test Serif', 'ZhijiTestSerif', null, true, 'ZhijiTestSerif-ExtraLight');
  assert.deepEqual(tables(result).get('fvar'), tables(original).get('fvar'));
  assert.deepEqual([...coveredCodepoints(result)].sort((a,b)=>a-b), [...coveredCodepoints(original)].sort((a,b)=>a-b));
});

// 中文字体必须同时保留简体、台湾繁体和香港繁体的语言系统及本地化字形特性。
test('chinese font keeps simplified and traditional locale features', () => {
  const font = readFileSync(new URL('../assets/fonts/ZhijiChineseSerif-VF.otf', import.meta.url));
  const text = font.toString('latin1');
  for (const tag of ['ZHS ', 'ZHT ', 'ZHH ', 'locl']) assert.ok(text.includes(tag), tag);
});

// 不完整字体必须显式拒绝，不能输出表面成功的衍生字体。
test('malformed input is rejected', () => {
  assert.throws(() => renameFont(Buffer.alloc(4), 'x', 'x'));
  assert.throws(() => coveredCodepoints(Buffer.alloc(4)));
});

// 生成器遇到既有目标文件应失败，并保留该文件原有字节。
test('generator refuses to replace an existing font', () => {
  const dir = mkdtempSync(join(tmpdir(), 'zhiji-font-test-'));
  try {
    const file = join(dir, 'ZhijiChineseSerif-VF.otf');
    writeFileSync(file, 'existing');
    const result = spawnSync(process.execPath, [fileURLToPath(new URL('../tools/fonts.mjs', import.meta.url)), 'tc', 'unused.otf', '--output', dir], { encoding: 'utf8' });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /already exists/);
    assert.equal(readFileSync(file, 'utf8'), 'existing');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
