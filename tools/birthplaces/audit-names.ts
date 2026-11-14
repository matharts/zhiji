// 核对补名快照、内嵌目录和来源实体关联，拒绝重复分配或位置、时区漂移。
import {
  type Place,
  type ZoneCatalog,
  WORLD_PATH,
  ZONES_PATH,
  placesRoot,
  readJson,
} from './catalog.ts';
import { nameAuditSnapshots } from './pipeline.ts';
import { runCli } from '../cli.ts';

interface NameAssignment {
  id: string;
  name: string;
  english: string;
  country: string;
  longitude?: number;
  lon?: number;
  nameLanguage?: string;
  zone?: string;
  sourceId?: string;
  sourceQid?: string;
}

interface NameSnapshot {
  places: NameAssignment[];
}

/** 验证同一快照列表同时覆盖重放批次，并返回已审计的分配数量。 */
export function auditNames(root: string): number {
  const catalog = readJson<{ places: Place[] }>(root, WORLD_PATH).places;
  const zones = readJson<ZoneCatalog>(root, ZONES_PATH);
  const byId = new Map(catalog.map(place => [place[0], place]));
  if (byId.size !== catalog.length) throw Error('Duplicate catalog ID');
  const assigned = new Map<string, string>();
  const assignedNodes = new Map<string, string>();
  const ids = new Set<string>();
  for (const path of nameAuditSnapshots()) {
    const snapshot = readJson<NameSnapshot>(root, path);
    for (const item of snapshot.places) {
      const place = byId.get(item.id);
      const validName = /\p{Script=Han}/u.test(item.name) || (item.nameLanguage === 'original' && item.name === item.english);
      if (ids.has(item.id) || !place || place[1] !== item.name || place[2] !== item.english || place[4] !== item.country || place[5] !== (item.longitude ?? item.lon) || !validName) {
        throw Error(`Changed or duplicate name assignment: ${path} ${item.id}`);
      }
      ids.add(item.id);
      if (item.zone && !(zones[item.id] ?? []).includes(item.zone)) throw Error(`Timezone changed: ${item.id}`);
      const node = String(item.sourceId ?? '').replace(/^node\/(\d+)$/, 'osm:node:$1');
      if (/^osm:node:\d+$/.test(node)) {
        const prior = assignedNodes.get(node) ?? (byId.has(node) ? node : item.id);
        if (prior !== item.id) throw Error(`OSM node ${node} maps to ${prior} and ${item.id}`);
        assignedNodes.set(node, item.id);
      }
      const qid = item.sourceQid ?? item.sourceId;
      if (qid !== undefined && /^Q\d+$/.test(qid)) {
        const prior = assigned.get(qid) ?? (byId.has(`wd:${qid}`) ? `wd:${qid}` : item.id);
        if (prior !== item.id) throw Error(`Wikidata item ${qid} maps to ${prior} and ${item.id}`);
        assigned.set(qid, item.id);
      }
    }
  }
  return ids.size;
}

function main(): void {
  const count = auditNames(placesRoot());
  console.log(`Validated ${count} distinct name assignments and source associations.`);
}

runCli(main, import.meta.url);
