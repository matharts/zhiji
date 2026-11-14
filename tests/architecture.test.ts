import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

/** 枚举明确受约束的生产目录；模块外置的单元测试不在检查范围内。 */
function* sources(directory: string): Generator<string> {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) yield* sources(path);
    else if (entry.isFile() && entry.name.endsWith('.rs')) yield path;
  }
}

/** 枚举开发脚本中的遗留 JavaScript 文件；依赖和构建目录不参与检查。 */
function* javascriptSources(directory: string): Generator<string> {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && ['node_modules', 'target'].includes(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) yield* javascriptSources(path);
    else if (entry.isFile() && /\.(?:js|mjs|cjs)$/u.test(entry.name)) yield path;
  }
}

test('开发工具与测试只使用 TypeScript 源文件', () => {
  const legacy = ['tools', 'tests'].flatMap(name => [...javascriptSources(join(root, name))]).map(path => relative(root, path));
  assert.deepEqual(legacy, []);
});

test('功能模块不直接引用应用根状态或应用模块路径', () => {
  const paths = ['workspace', 'preferences', 'updates'].flatMap(name => [...sources(join(root, 'src', name))]);
  paths.push(join(root, 'src/diagnostics.rs'));
  assert.ok(paths.length > 0);
  for (const path of paths) {
    const text = readFileSync(path, 'utf8');
    assert.doesNotMatch(text, /\bAppState\b|\b(?:crate|super)\s*::\s*app\b|\b(?:crate|super)\s*::\s*\{[^;]*\bapp\b/, relative(root, path));
  }
});

test('内部计算包与控件包保持独立的直接依赖', () => {
  const result = spawnSync('cargo', ['metadata', '--locked', '--offline', '--no-deps', '--format-version', '1'], {
    cwd: root, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  const { packages } = JSON.parse(result.stdout) as {
    packages: { name: string; dependencies: { name: string }[] }[];
  };
  const rules = {
    zhiji_core: ['zhiji', 'zhiji_calendar', 'zhiji_ui', 'rune'],
    zhiji_calendar: ['zhiji', 'zhiji_core', 'zhiji_ui', 'rune'],
    zhiji_ui: ['zhiji', 'zhiji_core', 'zhiji_calendar'],
  };
  for (const [name, forbidden] of Object.entries(rules)) {
    const packageInfo = packages.find(item => item.name === name);
    assert.ok(packageInfo, `Missing workspace package: ${name}`);
    for (const dependency of packageInfo.dependencies) {
      assert.ok(!forbidden.includes(dependency.name), `${name} must not depend on ${dependency.name}`);
    }
  }
});
