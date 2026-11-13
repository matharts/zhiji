// 按依赖顺序重放目录迁移和补名，任一步失败立即停止后续写入。
import { spawnSync } from 'node:child_process';
// 固定来源快照日期，使重建不依赖网络或最新数据。
const dated = name => `tools/data-sources/${name}-2026-11-11.json`;
const migrations = [
  'overseas-priority-migration', 'overseas-global-migration', 'overseas-osm-migration',
  'overseas-osm-fallback-migration', 'overseas-wikidata-followup',
];
const names = [
  'overseas-cross-source-names', 'overseas-naer-names', 'overseas-osm-extra-names',
  'overseas-further-names', 'overseas-normalized-names', 'overseas-english-alias-names',
  'overseas-english-alias-wide-names', 'overseas-additional-types-names',
  'overseas-name-corrections', 'overseas-cldr-names', 'overseas-multilingual-names', 'overseas-multilingual-wide-names', 'overseas-multilingual-alias-names', 'overseas-chinese-alias-names', 'overseas-multilingual-chinese-alias-names', 'overseas-wikidata-current-names', 'overseas-chinese-locales-names', 'overseas-naer-alias-names', 'overseas-naer-alias-wide-names', 'overseas-geonames-zh-names', 'overseas-osm-locality-names', 'overseas-osm-thailand-names', 'overseas-osm-vietnam-names',
];
const commands = [
  ...migrations.map(name => ['tools/overseas-priority-migration.mjs', dated(name)]),
  ['tools/overseas-natural-earth-names.mjs'],
  ...names.map(name => ['tools/overseas-cross-source-names.mjs', dated(name)]),
  ['tools/overseas-continents.mjs'],
  ['tools/overseas-name-audit.mjs'],
  ['tools/overseas-major-cities.mjs'],
];
for (const args of commands) {
  const result = spawnSync(process.execPath, args, { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
console.log('Overseas catalog rebuild completed.');
