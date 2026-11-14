import type { Place, WorldCatalog } from './catalog.ts';

export interface NameAssignment {
  id: string;
  english: string;
  country: string;
  longitude: number;
  oldName: string;
  name: string;
}

/**
 * 仅在快照仍指向同一地点、身份字段未漂移且调用方接受名称时改名。
 * 先完成全部校验再修改目录，任一分配失败都不留下部分更新。
 */
export function applyNameAssignments<T extends NameAssignment>(
  world: WorldCatalog,
  assignments: readonly T[],
  accept: (assignment: T) => boolean = () => true,
): number {
  const byId = new Map(world.places.map(place => [place[0], place]));
  if (byId.size !== world.places.length) throw Error('Duplicate place ID');
  const seen = new Set<string>();
  const updates: [Place, string][] = [];
  for (const item of assignments) {
    const place = byId.get(item.id);
    if (seen.has(item.id) || !place || place[2] !== item.english || place[4] !== item.country || place[5] !== item.longitude || ![item.oldName, item.name].includes(place[1]) || !accept(item)) {
      throw Error(`Invalid or changed place: ${item.id}`);
    }
    seen.add(item.id);
    updates.push([place, item.name]);
  }
  for (const [place, name] of updates) place[1] = name;
  return updates.length;
}
