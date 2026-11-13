#!/usr/bin/env node
// 应用已审阅的跨来源迁移，保持无关地点记录不变。
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = process.env.ZHIJI_MIGRATION_ROOT
  ? resolve(process.env.ZHIJI_MIGRATION_ROOT)
  : resolve(dirname(fileURLToPath(import.meta.url)), '..');
// 相对所选工作根目录读取快照，测试可定向至隔离目录。
const read = path => JSON.parse(readFileSync(resolve(root, path), 'utf8'));
const worldPath = 'assets/data/birthplace-regions.json';
const zonesPath = 'assets/data/birthplace-timezones.json';
const manifestPath = 'assets/manifest.json';
const migrationPath = process.argv[2] ?? 'tools/data-sources/overseas-priority-migration-2026-11-11.json';
const migration = read(migrationPath);
const world = read(worldPath);
const zones = read(zonesPath);
const oldIndex = new Map(world.places.map((place, index) => [place[0], index]));
if (oldIndex.size !== world.places.length) throw Error('Duplicate existing place IDs');
const selected = new Set();
for (const item of migration.places) {
  if (selected.has(item.id)) throw Error(`Duplicate migration QID: ${item.id}`);
  selected.add(item.id);
  const old = oldIndex.get(item.oldId);
  const current = oldIndex.get(item.id);
  if ((old === undefined) === (current === undefined)) throw Error(`Missing or ambiguous migration ${item.id}`);
  const index = old ?? current;
  const original = world.places[index];
  if (old !== undefined && (original[2] !== item.oldEnglish || original[4] !== item.oldCountry || original[5] !== item.oldLongitude)) throw Error(`Existing record changed: ${item.oldId}`);
  if (current !== undefined && (original[2] !== item.english || original[4] !== item.country || original[5] !== item.longitude)) throw Error(`Migrated record changed: ${item.id}`);
  if (!Number.isFinite(item.longitude) || item.longitude < -180 || item.longitude > 180 || !item.timezones.length || !Number.isSafeInteger(item.population) || item.population < 0) throw Error(`Invalid migration: ${item.id}`);
  world.places[index] = [item.id, item.name, item.english, item.region, item.country, item.longitude, item.population];
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
writeFileSync(resolve(root, worldPath), JSON.stringify(world) + '\n');
writeFileSync(resolve(root, zonesPath), JSON.stringify(zones) + '\n');
const manifest = read(manifestPath);
for (const path of [worldPath, zonesPath]) manifest.sha256[path] = createHash('sha256').update(readFileSync(resolve(root, path))).digest('hex');
writeFileSync(resolve(root, manifestPath), JSON.stringify(manifest, null, 2) + '\n');
console.log(`Migrated ${migration.places.length} places and localized ${Object.keys(migration.regionNames).length} region names.`);
