#!/usr/bin/env node
// 应用已审阅的跨来源迁移，全部转换成功后再提交地点、时区和清单。
import {
  type Place,
  type WorldCatalog,
  type ZoneCatalog,
  WORLD_PATH,
  ZONES_PATH,
  json,
  placesRoot,
  readJson,
  writeCatalog,
} from './catalog.ts';
import { MIGRATION_SNAPSHOTS, snapshotPath } from './pipeline.ts';
import { runCli } from '../cli.ts';

interface MigrationPlace {
  oldId: string;
  id: string;
  oldEnglish: string;
  english: string;
  oldCountry: string;
  country: string;
  oldLongitude: number;
  longitude: number;
  name: string;
  region: string;
  population: number;
  timezones: string[];
}

export interface MigrationSnapshot {
  regionNames: Record<string, string>;
  places: MigrationPlace[];
}

export interface MigrationResult {
  migrated: number;
  localized: number;
}

/** 在内存中完成迁移；任一输入不一致都抛出，调用方不会写入半个目录。 */
export function applyMigration(
  world: WorldCatalog,
  zones: ZoneCatalog,
  migration: MigrationSnapshot,
): MigrationResult {
  const oldIndex = new Map(world.places.map((place, index) => [place[0], index]));
  if (oldIndex.size !== world.places.length) throw Error('Duplicate existing place IDs');
  const selected = new Set<string>();
  for (const item of migration.places) {
    if (selected.has(item.id)) throw Error(`Duplicate migration QID: ${item.id}`);
    selected.add(item.id);
    const old = oldIndex.get(item.oldId);
    const current = oldIndex.get(item.id);
    if ((old === undefined) === (current === undefined)) throw Error(`Missing or ambiguous migration ${item.id}`);
    const index = old ?? current;
    if (index === undefined) throw Error(`Missing migration index ${item.id}`);
    const original = world.places[index];
    if (old !== undefined && (original[2] !== item.oldEnglish || original[4] !== item.oldCountry || original[5] !== item.oldLongitude)) {
      throw Error(`Existing record changed: ${item.oldId}`);
    }
    if (current !== undefined && (original[2] !== item.english || original[4] !== item.country || original[5] !== item.longitude)) {
      throw Error(`Migrated record changed: ${item.id}`);
    }
    if (!Number.isFinite(item.longitude) || item.longitude < -180 || item.longitude > 180 || !item.timezones.length || !Number.isSafeInteger(item.population) || item.population < 0) {
      throw Error(`Invalid migration: ${item.id}`);
    }
    const place: Place = [item.id, item.name, item.english, item.region, item.country, item.longitude, item.population];
    world.places[index] = place;
    zones[item.id] = [...new Set(item.timezones)].sort();
    delete zones[item.oldId];
  }
  for (const place of world.places) {
    const region = migration.regionNames[`${place[4]}|${place[3]}`];
    if (region) place[3] = region;
  }
  world.source = 'GeoNames cities15000; selected places from Wikidata, OpenStreetMap and Natural Earth';
  world.sourceUrl = 'https://www.wikidata.org/; https://www.openstreetmap.org/; https://download.geonames.org/export/dump/; https://www.naturalearthdata.com/';
  world.license = 'CC BY 4.0; CC0; ODbL 1.0';
  return { migrated: migration.places.length, localized: Object.keys(migration.regionNames).length };
}

/** CLI 只负责读取固定快照并委托共享提交层。 */
function main(): void {
  const root = placesRoot();
  const snapshot = process.argv[2] ?? snapshotPath(MIGRATION_SNAPSHOTS[0]);
  const migration = readJson<MigrationSnapshot>(root, snapshot);
  const world = readJson<WorldCatalog>(root, WORLD_PATH);
  const zones = readJson<ZoneCatalog>(root, ZONES_PATH);
  const result = applyMigration(world, zones, migration);
  writeCatalog(root, new Map([
    [WORLD_PATH, json(world)],
    [ZONES_PATH, json(zones)],
  ]));
  console.log(`Migrated ${result.migrated} places and localized ${result.localized} region names.`);
}

runCli(main, import.meta.url);
