// 统计海外显示名称是否含汉字，不据此判定译名质量；只输出报告，不修改目录。
import { type Place, WORLD_PATH, placesRoot, readJson } from './catalog.ts';
import { runCli } from '../cli.ts';

export function nameCoverage(world: Place[], includePlaces = false): Record<string, unknown> {
  const places = world.filter(place => !['CN', 'HK', 'MO', 'TW'].includes(place[4]));
  const missing = places.filter(place => !/\p{Script=Han}/u.test(place[1]));
  const countries: Record<string, number> = {};
  for (const place of missing) countries[place[4]] = (countries[place[4]] ?? 0) + 1;
  const summary: Record<string, unknown> = {
    total: places.length,
    containsHan: places.length - missing.length,
    withoutHan: missing.length,
    missingByCountry: Object.fromEntries(
      Object.entries(countries).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])),
    ),
  };
  if (includePlaces) {
    summary.places = missing.map(place => ({
      id: place[0],
      name: place[1],
      english: place[2],
      region: place[3],
      country: place[4],
      longitude: place[5],
    }));
  }
  return summary;
}

function main(): void {
  const world = readJson<{ places: Place[] }>(placesRoot(), WORLD_PATH);
  console.log(JSON.stringify(nameCoverage(world.places, process.argv.includes('--places')), null, 2));
}

runCli(main, import.meta.url);
