// 地点目录的共享类型、JSON 读取和资源提交，避免各生成脚本各自维护写入顺序。
import { createHash, randomUUID } from 'node:crypto';
import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, resolve, sep } from 'node:path';

/** 地点行的固定字段顺序；数组格式属于现有资源协议。 */
export type Place = [
  id: string,
  name: string,
  english: string,
  region: string,
  country: string,
  longitude: number,
  population: number,
];

export interface WorldCatalog {
  places: Place[];
  source?: string;
  sourceUrl?: string;
  license?: string;
  [key: string]: unknown;
}

export type ZoneCatalog = Record<string, string[]>;

export interface Manifest {
  version: number;
  sha256: Record<string, string>;
}

export const WORLD_PATH = 'assets/data/birthplace-regions.json';
export const ZONES_PATH = 'assets/data/birthplace-timezones.json';
export const COUNTRIES_PATH = 'assets/data/birthplace-countries.json';
export const MAJOR_CITIES_PATH = 'assets/data/birthplace-major-cities.json';
export const CHINA_PATH = 'assets/data/birthplace-china.json';
export const MANIFEST_PATH = 'assets/manifest.json';

/** 默认以当前工作目录为仓库根；测试和重建可通过参数或环境变量换根。 */
export function placesRoot(override?: string): string {
  return resolve(override ?? process.env.ZHIJI_PLACES_ROOT ?? process.cwd());
}

/** 只允许访问根目录内的相对路径，防止生成脚本写到工作区之外。 */
export function catalogPath(root: string, path: string): string {
  if (isAbsolute(path)) throw Error(`Catalog path must be relative: ${path}`);
  const target = resolve(root, path);
  const prefix = root.endsWith(sep) ? root : `${root}${sep}`;
  if (target !== root && !target.startsWith(prefix)) throw Error(`Catalog path escapes root: ${path}`);
  return target;
}

/** 读取并解析脚本固定的 JSON 输入；解析失败由调用方直接中止。 */
export function readJson<T>(root: string, path: string): T {
  const target = isAbsolute(path) ? path : catalogPath(root, path);
  return JSON.parse(readFileSync(target, 'utf8')) as T;
}

/** 资源文件保持单行，清单保持可审阅的缩进格式。 */
export function json(value: unknown, pretty = false): string {
  return `${JSON.stringify(value, null, pretty ? 2 : undefined)}\n`;
}

/** 返回资源清单使用的稳定 SHA-256 表示。 */
export function sha256(content: string | Buffer): string {
  return createHash('sha256').update(content).digest('hex');
}

/** 在替换前先把所有内容写为同目录临时文件，任一失败都不触碰目标文件。 */
export function writeFilesAtomically(root: string, files: Map<string, string>): void {
  const staged = new Map<string, string>();
  const originals = new Map<string, Buffer | null>();
  try {
    for (const [path, content] of files) {
      const target = catalogPath(root, path);
      mkdirSync(dirname(target), { recursive: true });
      originals.set(target, existsSync(target) ? readFileSync(target) : null);
      const temporary = `${target}.tmp-${process.pid}-${randomUUID()}`;
      try {
        writeDurable(temporary, content);
        staged.set(target, temporary);
      } catch (error) {
        rmSync(temporary, { force: true });
        throw error;
      }
    }
    const committed: string[] = [];
    try {
      for (const [target, temporary] of staged) {
        renameSync(temporary, target);
        committed.push(target);
      }
    } catch (error) {
      for (const target of committed.reverse()) restore(target, originals.get(target) ?? null);
      throw error;
    }
  } finally {
    for (const temporary of staged.values()) rmSync(temporary, { force: true });
  }
}

/** 同步更新清单后再一次性提交资源，清单解析失败时不会先改资源。 */
export function writeCatalog(
  root: string,
  files: Map<string, string>,
): Manifest {
  const manifest = readJson<Manifest>(root, MANIFEST_PATH);
  if (manifest.version !== 1 || !manifest.sha256 || typeof manifest.sha256 !== 'object') {
    throw Error('Unsupported asset manifest');
  }
  for (const [path, content] of files) {
    if (path.startsWith('assets/')) manifest.sha256[path] = sha256(content);
  }
  files.set(MANIFEST_PATH, json(manifest, true));
  writeFilesAtomically(root, files);
  return manifest;
}

/** 写入并同步单个临时文件，替换操作仍在调用方最后执行。 */
function writeDurable(path: string, content: string | Buffer): void {
  const descriptor = openSync(path, 'wx', 0o666);
  try {
    writeFileSync(descriptor, content);
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

/** 尽力恢复已经替换的目标；恢复失败不能覆盖原始写入错误。 */
function restore(path: string, original: Buffer | null): void {
  try {
    if (original === null) {
      rmSync(path, { force: true });
      return;
    }
    const temporary = `${path}.restore-${process.pid}-${randomUUID()}`;
    writeDurable(temporary, original);
    renameSync(temporary, path);
  } catch {
    // 原始错误更有诊断价值，恢复失败由下一次目录校验发现。
  }
}
