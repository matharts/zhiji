// 仅应用名称补充，写入前核对快照，保留地点标识、坐标和时区。
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

const path = 'assets/data/birthplace-regions.json';
const world = JSON.parse(readFileSync(path, 'utf8'));
const sourcePath = process.argv[2] ?? 'tools/data-sources/overseas-cross-source-names-2026-11-11.json';
const source = JSON.parse(readFileSync(sourcePath, 'utf8'));
const byId = new Map(world.places.map(place => [place[0], place]));
if (byId.size !== world.places.length) throw new Error('Duplicate place ID');
const seen = new Set();
const updates = source.places.map(item => {
  const place = byId.get(item.id);
  const validName = /\p{Script=Han}/u.test(item.name) || (item.nameLanguage === 'original' && item.name === item.english);
  if (seen.has(item.id) || !place || place[2] !== item.english || place[4] !== item.country || place[5] !== item.longitude || ![item.oldName, item.name].includes(place[1]) || !validName) {
    throw new Error(`Invalid or changed place: ${item.id}`);
  }
  seen.add(item.id);
  return [place, item.name];
});
for (const [place, name] of updates) place[1] = name;
if (source.originalUrl === 'https://data.gov.tw/dataset/15211') {
  // 来源说明去重追加，重复重建不会累积相同许可条目。
  const append = (value, addition) => value.split('; ').includes(addition) ? value : `${value}; ${addition}`;
  world.source = append(world.source, '國家教育研究院 外國地名譯名');
  world.sourceUrl = append(world.sourceUrl, source.originalUrl);
  world.license = append(world.license, 'Open Government Data License 1.0');
}
writeFileSync(path, JSON.stringify(world) + '\n');
const manifest = JSON.parse(readFileSync('assets/manifest.json', 'utf8'));
manifest.sha256[path] = createHash('sha256').update(readFileSync(path)).digest('hex');
writeFileSync('assets/manifest.json', JSON.stringify(manifest, null, 2) + '\n');
console.log(`Applied ${updates.length} cross-source place names.`);
