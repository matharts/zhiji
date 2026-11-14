// 核对独立匹配的 Natural Earth 名称快照后改名，不改变地点身份或坐标。
import {
  type WorldCatalog,
  WORLD_PATH,
  json,
  placesRoot,
  readJson,
  writeCatalog,
} from './catalog.ts';
import { NATURAL_EARTH_SNAPSHOT, snapshotPath } from './pipeline.ts';
import { applyNameAssignments } from './name-assignments.ts';
import { runCli } from '../cli.ts';

interface NaturalEarthName {
  id: string;
  english: string;
  country: string;
  lon: number;
  oldName: string;
  name: string;
}

interface NaturalEarthSnapshot {
  places: NaturalEarthName[];
}

/** 只更新匹配一致的名称，快照失配时保持目录对象不变。 */
export function applyNaturalEarthNames(world: WorldCatalog, source: NaturalEarthSnapshot): number {
  const assignments = source.places.map(item => ({
    id: item.id,
    english: item.english,
    country: item.country,
    longitude: item.lon,
    oldName: item.oldName,
    name: item.name,
  }));
  return applyNameAssignments(world, assignments);
}

/** CLI 仅在完整校验通过后提交世界目录。 */
function main(): void {
  const root = placesRoot();
  const source = readJson<NaturalEarthSnapshot>(root, snapshotPath(NATURAL_EARTH_SNAPSHOT));
  const world = readJson<WorldCatalog>(root, WORLD_PATH);
  const applied = applyNaturalEarthNames(world, source);
  writeCatalog(root, new Map([[WORLD_PATH, json(world)]]));
  console.log(`Applied ${applied} Natural Earth Chinese names.`);
}

runCli(main, import.meta.url);
