// 在隔离目录完整重放海外目录；全部命令成功后才一次性提交资源。
import { spawnSync } from 'node:child_process';
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  MANIFEST_PATH,
  placesRoot,
  sha256,
  writeFilesAtomically,
} from './catalog.ts';
import { rebuildStages } from './pipeline.ts';
import { runCli } from '../cli.ts';

const toolsDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(toolsDirectory, '../..');

/** 枚举目录内文件，保持相对路径，便于比较暂存目录与仓库。 */
function* files(directory: string): Generator<string> {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) yield* files(path);
    else if (entry.isFile()) yield path;
  }
}

/** 在临时根目录复制资源和来源快照，失败时不会改动仓库。 */
export function rebuild(root = placesRoot(repositoryRoot), options: { quiet?: boolean } = {}): void {
  const stage = mkdtempSync(join(tmpdir(), 'zhiji-places-'));
  try {
    mkdirSync(join(stage, 'assets'), { recursive: true });
    mkdirSync(join(stage, 'tools'), { recursive: true });
    mkdirSync(join(stage, 'tools/birthplaces'), { recursive: true });
    cpSync(join(root, 'assets/data'), join(stage, 'assets/data'), { recursive: true });
    cpSync(join(root, 'tools/birthplaces/sources'), join(stage, 'tools/birthplaces/sources'), { recursive: true });
    cpSync(join(root, MANIFEST_PATH), join(stage, MANIFEST_PATH));
    for (const step of rebuildStages()) {
      const result = spawnSync(process.execPath, [join(toolsDirectory, step.script), ...step.args], {
        cwd: stage,
        env: { ...process.env, ZHIJI_PLACES_ROOT: stage },
        stdio: options.quiet ? 'pipe' : 'inherit',
      });
      if (result.error) throw result.error;
      if (result.status !== 0) throw Error(`${step.script} failed`);
    }
    const stagedManifest = readFileSync(join(stage, MANIFEST_PATH), 'utf8');
    validateDataManifest(stage, stagedManifest);
    const changes = new Map<string, string>();
    const stageData = join(stage, 'assets/data');
    for (const staged of files(stageData)) {
      const path = relative(stage, staged).split(sep).join('/');
      const current = join(root, path);
      const content = readFileSync(staged, 'utf8');
      const changed = !existsSync(current)
        || !statSync(current).isFile()
        || readFileSync(current, 'utf8') !== content;
      if (changed) changes.set(path, content);
    }
    changes.set(MANIFEST_PATH, stagedManifest);
    writeFilesAtomically(root, changes);
  } finally {
    rmSync(stage, { recursive: true, force: true });
  }
}

/** 提交前核对暂存数据与清单，避免把生成步骤漏更新的摘要写入仓库。 */
function validateDataManifest(stage: string, serialized: string): void {
  const manifest = JSON.parse(serialized) as { sha256?: Record<string, string> };
  if (!manifest.sha256) throw Error('Staged manifest has no checksum table');
  const dataRoot = join(stage, 'assets/data');
  const expected = Object.keys(manifest.sha256).filter(path => path.startsWith('assets/data/'));
  const actual = [...files(dataRoot)]
    .filter(path => !['README.md', '.DS_Store'].includes(path.split(sep).at(-1) ?? ''))
    .map(path => relative(stage, path).split(sep).join('/'));
  if (expected.length !== actual.length || expected.some(path => !actual.includes(path))) {
    throw Error('Staged data inventory does not match manifest');
  }
  for (const path of actual) {
    if (manifest.sha256[path] !== sha256(readFileSync(join(stage, path)))) {
      throw Error(`Staged resource checksum mismatch: ${path}`);
    }
  }
}

function main(): void {
  rebuild();
  console.log('Overseas catalog rebuild completed.');
}

runCli(main, import.meta.url);
