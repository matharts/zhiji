// 构造可选海外主要城市列表，不删除历史地点；--check 只验证而不写资源。
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  type Place,
  MAJOR_CITIES_PATH,
  WORLD_PATH,
  json,
  placesRoot,
  readJson,
  writeCatalog,
} from './catalog.ts';
import { snapshotPath } from './pipeline.ts';
import { runCli } from '../cli.ts';

interface CitySelection {
  geonamesUrbanSubdivisionIds: string[];
  naturalEarth: { id: string }[];
  diasporaCityIds: string[];
}

export interface MajorCityResult {
  ids: string[];
  countries: number;
  content: string;
}

/** 按显式来源清单和人口阈值生成稳定城市集合。 */
export function selectMajorCities(world: Place[], source: CitySelection): MajorCityResult {
  const domestic = new Set(['CN', 'HK', 'MO', 'TW']);
  const places = world.filter(place => !domestic.has(place[4]) && /\p{Script=Han}/u.test(place[1]));
  const byId = new Map(places.map(place => [place[0], place]));
  const subdivisions = new Set(source.geonamesUrbanSubdivisionIds);
  const ids = new Set(places.filter(place => place[6] >= 200000 && !subdivisions.has(place[0])).map(place => place[0]));
  for (const id of [...source.naturalEarth.map(place => place.id), ...source.diasporaCityIds]) {
    if (byId.has(id)) ids.add(id);
  }
  const covered = new Set([...ids].map(id => byId.get(id)?.[4]).filter((country): country is string => Boolean(country)));
  for (const country of new Set(places.map(place => place[4]))) {
    if (covered.has(country)) continue;
    const candidates = places.filter(place => place[4] === country && !subdivisions.has(place[0]));
    candidates.sort((a, b) => b[6] - a[6] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    if (candidates[0]) ids.add(candidates[0][0]);
  }
  const sorted = [...ids].sort();
  return {
    ids: sorted,
    countries: new Set(sorted.map(id => byId.get(id)?.[4]).filter((country): country is string => Boolean(country))).size,
    content: json({ ids: sorted }),
  };
}

function main(): void {
  const root = placesRoot();
  const source = readJson<CitySelection>(root, snapshotPath('overseas-major-city-selection'));
  const world = readJson<{ places: Place[] }>(root, WORLD_PATH).places;
  const result = selectMajorCities(world, source);
  if (process.argv.includes('--check')) {
    if (readFileSync(resolve(root, MAJOR_CITIES_PATH), 'utf8') !== result.content) throw Error('Selectable overseas city list is stale');
    console.log(`Verified ${result.ids.length} selectable overseas cities.`);
    return;
  }
  writeCatalog(root, new Map([[MAJOR_CITIES_PATH, result.content]]));
  console.log(`Selected ${result.ids.length} overseas cities across ${result.countries} countries/regions.`);
}

runCli(main, import.meta.url);
