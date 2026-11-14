#!/usr/bin/env node
// 桌面统一检查：node tools/verify.mjs [--offline] resources|rust|gpu|native|updates
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, readdirSync, rmSync, existsSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const exports = [
  'export::render::tests::export_contains_current_color_and_square_corners',
  'export::render::tests::rasterizer_honors_every_export_scale',
  'export::render::tests::export_restores_the_interface_font_language',
  'export::render::tests::export_uses_chart_variant_independently_of_interface',
  'export::tests::flow::chosen_path_exports_a_real_chart_via_the_background_worker',
  'export::tests::flow::flying_export_contains_the_four_default_colors',
  'export::tests::flow::export_matches_visual_fixture_after_worker_completes',
  'export::tests::flow::hidden_stem_or_palace_export_preserves_grid_layout',
  'export::copy::tests::copy_freezes_the_visible_chart_and_hidden_marks_are_excluded',
  'export::tests::flow::exported_image_freezes_edited_drawings_and_text_before_the_dialog',
];
/** 递归枚举实际文件，不把目录作为资源条目。 */
function* files(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* files(path);
    else if (entry.isFile()) yield path;
  }
}
/** 检查仓库未引入 Python 源码或环境，跳过版本控制及构建缓存。 */
function checkToolchain(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (['.git', 'target', '.serena', '.playwright-cli', '.unlazy'].includes(entry.name)) continue;
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (['.venv', 'venv', '__pycache__'].includes(entry.name)) throw Error(`Unexpected Python environment: ${relative(root, path)}`);
      checkToolchain(path);
    } else if (entry.isFile() && (/\.(py|pyi|pyc|pyw)$/.test(entry.name) || /^requirements(?:[-.].*)?\.txt$/.test(entry.name) || ['pyvenv.cfg', 'pyproject.toml', '.python-version', 'Pipfile', 'Pipfile.lock', 'poetry.lock', 'uv.lock'].includes(entry.name))) {
      throw Error(`Unexpected Python file: ${relative(root, path)}`);
    }
  }
}
/** 检查生产 Rust 界面使用语义颜色，禁止在使用处新增 HEX 字面量。 */
function checkColorTokens() {
  const tokenFile = join(root, 'crates/zhiji_ui/src/tokens.rs');
  if (!existsSync(tokenFile)) throw Error('Missing semantic color token definitions');
  const roots = [join(root, 'src'), join(root, 'crates/zhiji_ui/src')];
  for (const dir of roots) {
    for (const source of files(dir)) {
      if (!source.endsWith('.rs') || resolve(source) === resolve(tokenFile)) continue;
      const text = readFileSync(source, 'utf8');
      const match = /"#[0-9a-fA-F]{3,8}"/.exec(text);
      if (match) {
        const before = text.slice(0, match.index).split('\n');
        throw Error(`Raw interface color in ${relative(root, source)}:${before.length}: ${match[0]}. Use ColorToken.`);
      }
    }
  }
}
/** 核验资源清单、摘要、内嵌路径及地点名称目录一致性。 */
function resources() {
  checkToolchain(root);
  checkColorTokens();
  const manifest = JSON.parse(readFileSync(join(root, 'assets/manifest.json'), 'utf8'));
  if (manifest.version !== 1) throw Error('Unsupported asset manifest version');
  const assets = [...files(join(root, 'assets'))].filter(p => !['README.md', '.DS_Store'].includes(p.split(sep).at(-1)) && p !== join(root, 'assets/manifest.json'));
  const names = assets.map(p => relative(root, p).split(sep).join('/')).sort();
  const expected = Object.keys(manifest.sha256).sort();
  if (JSON.stringify(names) !== JSON.stringify(expected)) {
    throw Error(`Asset inventory changed: missing=${expected.filter(n => !names.includes(n))}, extra=${names.filter(n => !expected.includes(n))}`);
  }
  for (const path of assets) {
    const name = relative(root, path).split(sep).join('/');
    if (createHash('sha256').update(readFileSync(path)).digest('hex') !== manifest.sha256[name]) throw Error(`Asset checksum changed: ${name}`);
  }
  for (const source of files(join(root, 'src'))) {
    if (!source.endsWith('.rs')) continue;
    for (const match of readFileSync(source, 'utf8').matchAll(/include_(?:bytes|str)!\(\s*"([^"]+)"/g)) {
      const embedded = resolve(dirname(source), match[1]);
      if (!existsSync(embedded) || !statSync(embedded).isFile()) throw Error(`Missing embedded file in ${source}: ${match[1]}`);
    }
  }
  run([process.execPath, 'tools/overseas-name-audit.mjs'], process.env);
  run([process.execPath, 'tools/overseas-major-cities.mjs', '--check'], process.env);
  console.log(`Verified ${assets.length} asset/data files and all literal embedded paths.`);
}
/** 在仓库根目录执行命令，保留输出并将启动或退出失败向上报告。 */
function run(args, env) {
  console.log('+ ' + args.map(s => JSON.stringify(s)).join(' '));
  const r = spawnSync(args[0], args.slice(1), { cwd: root, env, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  if (r.error) throw r.error;
  if (r.status !== 0) throw Error(`Command exited ${r.status}: ${args[0]}`);
  return (r.stdout ?? '') + (r.stderr ?? '');
}
/** 先列出待选测试再执行，拒绝零项、数量不符或被忽略的检查。 */
export function selected(args, env, exact = false, execute = run) {
  const listing = execute([...args, '--list'], env);
  const chosen = [...listing.matchAll(/^(.+): test\r?$/gm)];
  if (!chosen.length || (exact && chosen.length !== 1)) throw Error(`Unexpected test selection: ${chosen.length}`);
  const output = execute(args, env);
  const counts = [...output.matchAll(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)];
  if (counts.length !== 1 || Number(counts[0][1]) !== chosen.length || counts[0][2] !== '0' || counts[0][3] !== '0') throw Error('Executed test count does not match the inspected selection');
  return chosen.length;
}
/** 校验参数并在隔离配置目录中执行对应检查，退出时清理临时环境。 */
function main() {
  const args = process.argv.slice(2);
  const mode = args.find(x => ['resources', 'rust', 'gpu', 'native', 'updates'].includes(x));
  const offline = args.includes('--offline');
  const bundleIndex = args.indexOf('--app-bundle');
  const valid = args.filter((x, i) => x !== mode && x !== '--offline' && x !== '--app-bundle' && !(bundleIndex !== -1 && i === bundleIndex + 1));
  if (!mode || valid.length || (bundleIndex !== -1 && !args[bundleIndex + 1])) throw Error('Usage: node tools/verify.mjs [--offline] [--app-bundle PATH] resources|rust|gpu|native|updates');
  resources();
  if (mode === 'resources') return;
  const dir = mkdtempSync(join(tmpdir(), 'zhiji-check-'));
  const env = { ...process.env, ZHIJI_CONFIG_DIR: dir };
  try {
    if (mode === 'native') {
      if (process.platform !== 'darwin') throw Error('Native macOS checks require macOS');
      const bundle = resolve(bundleIndex === -1 ? join(root, 'target/release/知几.app') : args[bundleIndex + 1]);
      const name = run(['plutil', '-extract', 'CFBundleExecutable', 'raw', '-o', '-', join(bundle, 'Contents/Info.plist')], env).trim();
      const executable = join(bundle, 'Contents/MacOS', name);
      if (!existsSync(executable)) throw Error(`Missing bundle executable: ${executable}`);
      const main = join(dir, 'main.swift');
      writeFileSync(main, readFileSync(join(root, 'tests/macos_accessibility.swift')));
      const binary = join(dir, 'macos_accessibility');
      run(['swiftc', '-swift-version', '6', '-default-isolation', 'MainActor', '-D', 'ZHIJI_NATIVE_BUNDLE', join(root, 'tests/macos_test_application.swift'), main, '-o', binary], env);
      run([binary, executable], env);
      console.log('Desktop native macOS checks passed: 1 suite.');
      return;
    }
    const locked = ['--locked', ...(offline ? ['--offline'] : [])];
    if (mode === 'updates') {
      const count = selected(['cargo', 'test', ...locked, '-p', 'zhiji', '--bin', 'zhiji', 'updates::tests::packaged_update_signatures_match_client', '--', '--ignored', '--exact'], env, true);
      console.log(`Desktop update artifact checks passed: ${count} test.`);
    } else if (mode === 'rust') {
      run([process.execPath, '--test', 'tests/font_tools.test.mjs', 'tests/updates.test.mjs', 'tests/verify.test.mjs', 'tests/architecture.test.mjs'], env);
      run(['cargo', 'fmt', '--all', '--', '--check'], env);
      run(['cargo', 'clippy', ...locked, '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings'], env);
      run(['cargo', 'test', ...locked, '-p', 'zhiji', '--test', 'scene_configuration'], env);
      const output = run(['cargo', 'test', ...locked, '--workspace', '--all-targets', '--all-features', '--', '--test-threads=1'], env);
      const passed = [...output.matchAll(/test result: ok\. (\d+) passed;/g)].reduce((n, m) => n + Number(m[1]), 0);
      if (!passed) throw Error('No workspace tests executed');
      console.log(`Desktop Rust checks passed: ${passed} tests.`);
    } else {
      let count = 0;
      for (const name of exports) count += selected(['cargo', 'test', ...locked, '-p', 'zhiji', '--bin', 'zhiji', '--all-features', name, '--', '--ignored', '--exact', '--test-threads=1'], env, true);
      count += selected(['cargo', 'test', ...locked, '-p', 'zhiji', '--test', 'visual_regression', '--all-features', '--', '--ignored', '--test-threads=1'], env);
      console.log(`Desktop GPU checks passed: ${count} tests.`);
    }
  } finally { rmSync(dir, { recursive: true, force: true }); }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
