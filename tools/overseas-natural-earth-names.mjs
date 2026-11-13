// 核对独立匹配的 Natural Earth 名称快照后改名，不改变地点身份或坐标。
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

const path = 'assets/data/birthplace-regions.json';
const world = JSON.parse(readFileSync(path, 'utf8'));
const source = JSON.parse(readFileSync('tools/data-sources/overseas-natural-earth-names-2026-11-11.json', 'utf8'));
const byId = new Map(world.places.map(place => [place[0], place]));
const updates = source.places.map(item => {
  const place = byId.get(item.id);
  if (!place || place[2] !== item.english || place[4] !== item.country || place[5] !== item.lon || ![item.oldName, item.name].includes(place[1])) {
    throw new Error(`Place no longer matches source snapshot: ${item.id}`);
  }
  return [place, item.name];
});
for (const [place, name] of updates) place[1] = name;
writeFileSync(path, JSON.stringify(world) + '\n');
const manifest = JSON.parse(readFileSync('assets/manifest.json', 'utf8'));
manifest.sha256[path] = createHash('sha256').update(readFileSync(path)).digest('hex');
writeFileSync('assets/manifest.json', JSON.stringify(manifest, null, 2) + '\n');
console.log(`Applied ${updates.length} Natural Earth Chinese names.`);
