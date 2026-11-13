// 构造可选海外主要城市列表，不删除历史地点；--check 只验证而不写资源。
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

// 所有输入从固定本地 JSON 读取，避免构建时隐式抓取数据。
const read = path => JSON.parse(readFileSync(path, 'utf8'));
const source = read('tools/data-sources/overseas-major-city-selection-2026-11-11.json');
const world = read('assets/data/birthplace-regions.json').places;
const domestic = new Set(['CN', 'HK', 'MO', 'TW']);
const places = world.filter(p => !domestic.has(p[4]) && /\p{Script=Han}/u.test(p[1]));
const byId = new Map(places.map(p => [p[0], p]));
const subdivisions = new Set(source.geonamesUrbanSubdivisionIds);
const ids = new Set(places.filter(p => p[6] >= 200000 && !subdivisions.has(p[0])).map(p => p[0]));
for (const id of [...source.naturalEarth.map(p => p.id), ...source.diasporaCityIds]) {
  if (byId.has(id)) ids.add(id);
}
// 某国没有地点达到条件时，保留其最大的具名非分区地点。
const covered = new Set([...ids].map(id => byId.get(id)[4]));
for (const country of new Set(places.map(p => p[4]))) {
  if (covered.has(country)) continue;
  const candidates = places.filter(p => p[4] === country && !subdivisions.has(p[0]));
  candidates.sort((a, b) => b[6] - a[6] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
  if (candidates[0]) ids.add(candidates[0][0]);
}
const path = 'assets/data/birthplace-major-cities.json';
const output = JSON.stringify({ ids: [...ids].sort() }) + '\n';
if (process.argv.includes('--check')) {
  if (readFileSync(path, 'utf8') !== output) throw Error('Selectable overseas city list is stale');
  console.log(`Verified ${ids.size} selectable overseas cities.`);
  process.exit(0);
}
writeFileSync(path, output);
const manifest = read('assets/manifest.json');
manifest.sha256[path] = createHash('sha256').update(readFileSync(path)).digest('hex');
writeFileSync('assets/manifest.json', JSON.stringify(manifest, null, 2) + '\n');
console.log(`Selected ${ids.size} overseas cities across ${new Set([...ids].map(id => byId.get(id)[4])).size} countries/regions.`);
