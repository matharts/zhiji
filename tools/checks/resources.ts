import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { files, root, run } from './context.ts';

/** 检查仓库未引入 Python 源码或环境，跳过版本控制及构建缓存。 */
function checkToolchain(dir: string): void {
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
function checkColorTokens(): void {
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
export function verifyResources(): void {
  checkToolchain(root);
  checkColorTokens();
  const manifest = JSON.parse(readFileSync(join(root, 'assets/manifest.json'), 'utf8')) as {
    version: number;
    sha256: Record<string, string>;
  };
  if (manifest.version !== 1) throw Error('Unsupported asset manifest version');
  const assets = [...files(join(root, 'assets'))].filter(path => !['README.md', '.DS_Store'].includes(path.split(sep).at(-1) ?? '') && path !== join(root, 'assets/manifest.json'));
  const names = assets.map(path => relative(root, path).split(sep).join('/')).sort();
  const expected = Object.keys(manifest.sha256).sort();
  if (JSON.stringify(names) !== JSON.stringify(expected)) {
    throw Error(`Asset inventory changed: missing=${expected.filter(name => !names.includes(name))}, extra=${names.filter(name => !expected.includes(name))}`);
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
  run([process.execPath, 'tools/birthplaces/audit-names.ts'], process.env);
  run([process.execPath, 'tools/birthplaces/generate-major-cities.ts', '--check'], process.env);
  console.log(`Verified ${assets.length} asset/data files and all literal embedded paths.`);
}
