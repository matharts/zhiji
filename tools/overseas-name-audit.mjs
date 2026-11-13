// 核对补名快照、内嵌目录和来源实体关联，拒绝重复分配或位置、时区漂移。
import { readFileSync } from 'node:fs';
// 读取固定本地快照，任何缺失或解析失败都会中止审计。
const read = path => JSON.parse(readFileSync(path, 'utf8'));
const files = [
  'overseas-natural-earth-names', 'overseas-cross-source-names', 'overseas-naer-names', 'overseas-osm-extra-names',
  'overseas-further-names', 'overseas-normalized-names', 'overseas-english-alias-names',
  'overseas-english-alias-wide-names', 'overseas-additional-types-names', 'overseas-name-corrections', 'overseas-cldr-names', 'overseas-multilingual-names', 'overseas-multilingual-wide-names', 'overseas-multilingual-alias-names', 'overseas-chinese-alias-names', 'overseas-multilingual-chinese-alias-names', 'overseas-wikidata-current-names', 'overseas-chinese-locales-names', 'overseas-naer-alias-names', 'overseas-naer-alias-wide-names', 'overseas-geonames-zh-names', 'overseas-osm-locality-names', 'overseas-osm-thailand-names', 'overseas-osm-vietnam-names',
];
const catalog = read('assets/data/birthplace-regions.json').places;
const zones = read('assets/data/birthplace-timezones.json');
const byId = new Map(catalog.map(place => [place[0], place]));
if (byId.size !== catalog.length) throw new Error('Duplicate catalog ID');
const assigned = new Map();
const assignedNodes = new Map();
const ids = new Set();
for (const file of files) {
  const snapshot = read(`tools/data-sources/${file}-2026-11-11.json`);
  for (const item of snapshot.places) {
    const place = byId.get(item.id);
    const validName = /\p{Script=Han}/u.test(item.name) || (item.nameLanguage === 'original' && item.name === item.english);
    if (ids.has(item.id) || !place || place[1] !== item.name || place[2] !== item.english || place[4] !== item.country || place[5] !== (item.longitude ?? item.lon) || !validName) {
      throw new Error(`Changed or duplicate name assignment: ${file} ${item.id}`);
    }
    ids.add(item.id);
    if (item.zone && !(zones[item.id] ?? []).includes(item.zone)) throw new Error(`Timezone changed: ${item.id}`);
    const node = String(item.sourceId ?? '').replace(/^node\/(\d+)$/, 'osm:node:$1');
    if (/^osm:node:\d+$/.test(node)) {
      const prior = assignedNodes.get(node) ?? (byId.has(node) ? node : item.id);
      if (prior !== item.id) throw new Error(`OSM node ${node} maps to ${prior} and ${item.id}`);
      assignedNodes.set(node, item.id);
    }
    const qid = item.sourceQid ?? item.sourceId;
    if (/^Q\d+$/.test(qid ?? '')) {
      const prior = assigned.get(qid) ?? (byId.has(`wd:${qid}`) ? `wd:${qid}` : item.id);
      if (prior !== item.id) throw new Error(`Wikidata item ${qid} maps to ${prior} and ${item.id}`);
      assigned.set(qid, item.id);
    }
  }
}
console.log(`Validated ${ids.size} distinct name assignments and source associations.`);
