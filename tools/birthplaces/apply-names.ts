// 仅应用名称补充，写入前核对快照，保留地点标识、坐标和时区。
import {
  type WorldCatalog,
  WORLD_PATH,
  json,
  placesRoot,
  readJson,
  writeCatalog,
} from './catalog.ts';
import { NAME_SNAPSHOTS, snapshotPath } from './pipeline.ts';
import { applyNameAssignments } from './name-assignments.ts';
import { runCli } from '../cli.ts';

interface NameAssignment {
  id: string;
  name: string;
  english: string;
  country: string;
  longitude: number;
  oldName: string;
  nameLanguage?: string;
}

export interface NameSnapshot {
  places: NameAssignment[];
  originalUrl?: string;
}

/** 验证每个来源分配仍指向原地点，并把名称改动集中在世界目录对象上。 */
export function applyNames(world: WorldCatalog, source: NameSnapshot): number {
  const assignments = source.places.map(item => ({
    id: item.id,
    english: item.english,
    country: item.country,
    longitude: item.longitude,
    oldName: item.oldName,
    name: item.name,
    validName: /\p{Script=Han}/u.test(item.name) || (item.nameLanguage === 'original' && item.name === item.english),
  }));
  const applied = applyNameAssignments(world, assignments, item => item.validName);
  if (source.originalUrl === 'https://data.gov.tw/dataset/15211') {
    const append = (value: string | undefined, addition: string) => {
      const current = value ?? '';
      return current.split('; ').includes(addition) ? current : `${current}; ${addition}`;
    };
    world.source = append(world.source, '國家教育研究院 外國地名譯名');
    world.sourceUrl = append(world.sourceUrl, source.originalUrl);
    world.license = append(world.license, 'Open Government Data License 1.0');
  }
  return applied;
}

/** CLI 保持单次补名命令，不把批次列表复制到这里。 */
function main(): void {
  const root = placesRoot();
  const snapshot = process.argv[2] ?? snapshotPath(NAME_SNAPSHOTS[0]);
  const source = readJson<NameSnapshot>(root, snapshot);
  const world = readJson<WorldCatalog>(root, WORLD_PATH);
  const applied = applyNames(world, source);
  writeCatalog(root, new Map([[WORLD_PATH, json(world)]]));
  console.log(`Applied ${applied} cross-source place names.`);
}

runCli(main, import.meta.url);
